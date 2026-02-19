use std::{collections::BTreeSet, sync::Arc};

use crate::{
    CheckContext, Diagnostic, ProseSpan, Sentence, Severity, SourceRange, UnsafeAnnotation,
    UnsafeAnnotationState, profile,
};

/// Rule input assembled per extracted prose span.
#[derive(Debug, Clone)]
pub struct RuleInput {
    pub file_path: String,
    pub span: ProseSpan,
    pub sentences: Vec<Sentence>,
}

/// A lint rule.
pub trait Rule: Send + Sync {
    fn id(&self) -> &str;
    fn name(&self) -> &str;
    fn profiles(&self) -> &[&str];
    fn default_severity(&self) -> Severity;
    fn check(&self, input: &RuleInput, ctx: &CheckContext) -> Vec<Diagnostic>;
}

/// Registered rule collection and execution helper.
#[derive(Default)]
pub struct RuleSet {
    rules: Vec<Arc<dyn Rule>>,
}

impl RuleSet {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn register<R>(&mut self, rule: R)
    where
        R: Rule + 'static,
    {
        self.rules.push(Arc::new(rule));
    }

    pub fn extend(&mut self, rules: impl IntoIterator<Item = Arc<dyn Rule>>) {
        self.rules.extend(rules);
    }

    pub fn rules(&self) -> &[Arc<dyn Rule>] {
        &self.rules
    }

    pub fn run(&self, input: &mut RuleInput, ctx: &CheckContext) -> Vec<Diagnostic> {
        let mut all = Vec::new();
        let mut registered_rule_ids = BTreeSet::new();

        for rule in &self.rules {
            registered_rule_ids.insert(rule.id().to_string());
        }

        for rule in &self.rules {
            if !rule
                .profiles()
                .iter()
                .all(|profile_name| ctx.profile_enabled(profile_name))
            {
                continue;
            }

            // Requirement rules are routed through span requirement metadata.
            let is_ears_rule = rule
                .profiles()
                .iter()
                .any(|profile_name| profile_name.eq_ignore_ascii_case("ears"));
            if is_ears_rule {
                // Requirement rules never run on descriptive/procedural prose.
                // They require an explicit requirement span marker.
                let Some(ruleset) = input
                    .span
                    .annotations
                    .effective_requirement_ruleset(&ctx.config.requirements.default_ruleset)
                else {
                    continue;
                };

                if !ruleset.eq_ignore_ascii_case("ears")
                    || !ctx.config.requirement_ruleset_enabled(ruleset)
                {
                    // Source text cannot force a ruleset; config/CLI controls
                    // active requirement rulesets centrally.
                    continue;
                }
            }

            let Some(effective_severity) =
                profile::effective_severity(&ctx.config, rule.id(), rule.default_severity())
            else {
                continue;
            };

            let mut diagnostics = rule.check(input, ctx);
            if consume_matching_unsafe_annotation_if_violated(
                &mut input.span.unsafe_annotations,
                rule.id(),
                !diagnostics.is_empty(),
            ) {
                continue;
            }
            for diagnostic in &mut diagnostics {
                diagnostic.severity = effective_severity;
            }
            all.extend(diagnostics);
        }

        all.extend(validate_unsafe_annotations(
            &mut input.span.unsafe_annotations,
            &registered_rule_ids,
        ));

        all
    }
}

fn consume_matching_unsafe_annotation_if_violated(
    annotations: &mut [UnsafeAnnotation],
    rule_id: &str,
    has_violation: bool,
) -> bool {
    if !has_violation {
        return false;
    }

    for annotation in annotations {
        if !annotation.rule_id.eq_ignore_ascii_case(rule_id) {
            continue;
        }

        if !matches!(annotation.state, UnsafeAnnotationState::Pending) {
            continue;
        }

        annotation.state = UnsafeAnnotationState::Consumed;
        return true;
    }

    false
}

fn validate_unsafe_annotations(
    annotations: &mut [UnsafeAnnotation],
    known_rules: &BTreeSet<String>,
) -> Vec<Diagnostic> {
    let mut diagnostics = Vec::new();

    for annotation in annotations {
        let state = annotation.state.clone();
        match state {
            UnsafeAnnotationState::Consumed => {}
            UnsafeAnnotationState::Invalid(message) => diagnostics.push(
                Diagnostic::new(
                    "goodwrite/unsafe-invalid",
                    Severity::Error,
                    format!(
                        "invalid unsafe annotation for `{}`: {message}",
                        annotation.rule_id
                    ),
                    annotation.range,
                )
                .with_help("use format `goodwrite:unsafe(<rule-id>): <reason>`"),
            ),
            UnsafeAnnotationState::Pending => {
                if !known_rules
                    .iter()
                    .any(|rule_id| rule_id.eq_ignore_ascii_case(&annotation.rule_id))
                {
                    diagnostics.push(
                        Diagnostic::new(
                            "goodwrite/unsafe-unknown-rule",
                            Severity::Error,
                            format!(
                                "unsafe annotation references unknown rule `{}`",
                                annotation.rule_id
                            ),
                            annotation.range,
                        )
                        .with_help("use a valid rule id from `goodwrite list-rules`"),
                    );
                    annotation.state = UnsafeAnnotationState::Invalid(
                        "unknown rule id for current ruleset".to_string(),
                    );
                    continue;
                }

                diagnostics.push(
                    Diagnostic::new(
                        "goodwrite/unsafe-stale",
                        Severity::Error,
                        format!(
                            "unsafe annotation for `{}` did not match a violation",
                            annotation.rule_id
                        ),
                        SourceRange::new(annotation.range.start, annotation.range.end),
                    )
                    .with_help(
                        "remove the unsafe annotation or attach it to the exact violating span",
                    ),
                );
                annotation.state =
                    UnsafeAnnotationState::Invalid("stale unsafe annotation".to_string());
            }
        }
    }

    diagnostics
}

/// Helper macro for lightweight rule declarations.
#[macro_export]
macro_rules! declare_rule {
    (
        $(#[$meta:meta])*
        $vis:vis struct $name:ident {
            id: $id:expr,
            title: $title:expr,
            profiles: [$($profile:expr),* $(,)?],
            severity: $severity:expr,
        }
    ) => {
        $(#[$meta])*
        $vis struct $name;

        impl $crate::Rule for $name {
            fn id(&self) -> &str { $id }
            fn name(&self) -> &str { $title }
            fn profiles(&self) -> &[&str] { &[$($profile),*] }
            fn default_severity(&self) -> $crate::Severity { $severity }
            fn check(
                &self,
                _input: &$crate::RuleInput,
                _ctx: &$crate::CheckContext,
            ) -> Vec<$crate::Diagnostic> {
                Vec::new()
            }
        }
    };
}
