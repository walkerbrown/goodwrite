use regex::Regex;

use goodwrite_core::{CheckContext, Diagnostic, Rule, RuleInput, Severity};

use crate::patterns::{
    RequirementType, SUPPORTED_REQUIREMENT_TYPES, contains_keyword, count_keyword, detect_pattern,
    keyword_position,
};

pub struct InvalidRequirementTypeRule;
pub struct MissingShallRule;
pub struct MultipleShallRule;
pub struct MissingSystemNameRule;
pub struct MissingPatternRule;
pub struct MissingConditionKeywordRule;
pub struct PassiveShallRule;

impl Rule for InvalidRequirementTypeRule {
    fn id(&self) -> &str {
        "ears/invalid-requirement-type"
    }

    fn name(&self) -> &str {
        "Requirement type must be supported by EARS ruleset"
    }

    fn profiles(&self) -> &[&str] {
        &["ears"]
    }

    fn default_severity(&self) -> Severity {
        Severity::Warning
    }

    fn check(&self, input: &RuleInput, _ctx: &CheckContext) -> Vec<Diagnostic> {
        let Some(raw_type) = input.span.annotations.requirement_type.as_deref() else {
            return Vec::new();
        };

        if RequirementType::from_annotation(raw_type).is_some() {
            return Vec::new();
        }

        vec![
            Diagnostic::new(
                self.id(),
                self.default_severity(),
                format!("unknown requirement type `{raw_type}` for active EARS ruleset"),
                input.span.range,
            )
            .with_help(format!(
                "use one of: {}",
                SUPPORTED_REQUIREMENT_TYPES.join(", ")
            )),
        ]
    }
}

impl Rule for MissingShallRule {
    fn id(&self) -> &str {
        "ears/missing-shall"
    }

    fn name(&self) -> &str {
        "Requirement must include shall"
    }

    fn profiles(&self) -> &[&str] {
        &["ears"]
    }

    fn default_severity(&self) -> Severity {
        Severity::Warning
    }

    fn check(&self, input: &RuleInput, _ctx: &CheckContext) -> Vec<Diagnostic> {
        let mut diagnostics = Vec::new();

        for sentence in &input.sentences {
            if contains_keyword(&sentence.text, "shall") {
                continue;
            }

            diagnostics.push(
                Diagnostic::new(
                    self.id(),
                    self.default_severity(),
                    "EARS requirement must contain `shall`",
                    sentence.range,
                )
                .with_help("rewrite as `the <system> shall <response>`"),
            );
        }

        diagnostics
    }
}

impl Rule for MultipleShallRule {
    fn id(&self) -> &str {
        "ears/multiple-shall"
    }

    fn name(&self) -> &str {
        "One shall per requirement"
    }

    fn profiles(&self) -> &[&str] {
        &["ears"]
    }

    fn default_severity(&self) -> Severity {
        Severity::Warning
    }

    fn check(&self, input: &RuleInput, _ctx: &CheckContext) -> Vec<Diagnostic> {
        input
            .sentences
            .iter()
            .filter_map(|sentence| {
                let count = count_keyword(&sentence.text, "shall");
                if count <= 1 {
                    return None;
                }

                Some(
                    Diagnostic::new(
                        self.id(),
                        self.default_severity(),
                        format!("requirement contains {count} occurrences of `shall`"),
                        sentence.range,
                    )
                    .with_help("split into separate requirements with one `shall` each"),
                )
            })
            .collect()
    }
}

impl Rule for MissingSystemNameRule {
    fn id(&self) -> &str {
        "ears/missing-system-name"
    }

    fn name(&self) -> &str {
        "System name before shall"
    }

    fn profiles(&self) -> &[&str] {
        &["ears"]
    }

    fn default_severity(&self) -> Severity {
        Severity::Warning
    }

    fn check(&self, input: &RuleInput, _ctx: &CheckContext) -> Vec<Diagnostic> {
        const SYSTEM_HINTS: &[&str] = &[
            "system",
            "module",
            "controller",
            "software",
            "application",
            "engine",
            "display",
            "device",
        ];

        input
            .sentences
            .iter()
            .filter_map(|sentence| {
                let lower = sentence.text.to_ascii_lowercase();
                let shall_pos = lower.find(" shall ")?;
                let before = &lower[..shall_pos];
                if SYSTEM_HINTS.iter().any(|hint| before.contains(hint)) {
                    return None;
                }

                Some(
                    Diagnostic::new(
                        self.id(),
                        self.default_severity(),
                        "system name should appear before `shall`",
                        sentence.range,
                    )
                    .with_help("use form: `the <system-name> shall ...`"),
                )
            })
            .collect()
    }
}

impl Rule for MissingPatternRule {
    fn id(&self) -> &str {
        "ears/missing-pattern"
    }

    fn name(&self) -> &str {
        "Requirement should match an EARS pattern"
    }

    fn profiles(&self) -> &[&str] {
        &["ears"]
    }

    fn default_severity(&self) -> Severity {
        Severity::Info
    }

    fn check(&self, input: &RuleInput, _ctx: &CheckContext) -> Vec<Diagnostic> {
        input
            .sentences
            .iter()
            .filter_map(|sentence| {
                let lower = sentence.text.to_ascii_lowercase();
                if !lower.contains("shall") {
                    return None;
                }

                if detect_pattern(&sentence.text).is_none() {
                    return Some(
                        Diagnostic::new(
                            self.id(),
                            self.default_severity(),
                            "sentence with `shall` does not match known EARS patterns",
                            sentence.range,
                        )
                        .with_help(
                            "use ubiquitous, event-driven, state-driven, optional, unwanted, or complex pattern",
                        ),
                    );
                }

                let detected = detect_pattern(&sentence.text)?;
                let declared = input
                    .span
                    .annotations
                    .requirement_type
                    .as_deref()
                    .and_then(RequirementType::from_annotation)?;

                if declared == RequirementType::Auto || declared == detected {
                    return None;
                }

                Some(
                    Diagnostic::new(
                        self.id(),
                        self.default_severity(),
                        format!(
                            "declared requirement type `{}` does not match detected EARS type `{}`",
                            declared.as_str(),
                            detected.as_str()
                        ),
                        sentence.range,
                    )
                    .with_help("update annotation or rewrite sentence to declared pattern"),
                )
            })
            .collect()
    }
}

impl Rule for MissingConditionKeywordRule {
    fn id(&self) -> &str {
        "ears/missing-condition-keyword"
    }

    fn name(&self) -> &str {
        "Condition should use EARS keyword"
    }

    fn profiles(&self) -> &[&str] {
        &["ears"]
    }

    fn default_severity(&self) -> Severity {
        Severity::Warning
    }

    fn check(&self, input: &RuleInput, _ctx: &CheckContext) -> Vec<Diagnostic> {
        input
            .sentences
            .iter()
            .filter_map(|sentence| {
                let lower = sentence.text.to_ascii_lowercase();

                if lower.contains(" if ") && !lower.contains(" then ") {
                    return Some(
                        Diagnostic::new(
                            self.id(),
                            self.default_severity(),
                            "`if` appears without `then`",
                            sentence.range,
                        )
                        .with_help("use `If <condition>, then ... shall ...`"),
                    );
                }

                let conditional_without_keyword = (lower.contains("in case of")
                    || lower.contains("whenever")
                    || lower.contains("provided that"))
                    && !lower.contains(" when ")
                    && !lower.contains(" while ")
                    && !lower.contains(" if ");

                if conditional_without_keyword {
                    return Some(
                        Diagnostic::new(
                            self.id(),
                            self.default_severity(),
                            "conditional phrasing missing EARS keyword",
                            sentence.range,
                        )
                        .with_help("use explicit `When`, `While`, or `If ... then` clause"),
                    );
                }

                None
            })
            .collect()
    }
}

impl Rule for PassiveShallRule {
    fn id(&self) -> &str {
        "ears/passive-shall"
    }

    fn name(&self) -> &str {
        "Response after shall should be active"
    }

    fn profiles(&self) -> &[&str] {
        &["ears"]
    }

    fn default_severity(&self) -> Severity {
        Severity::Warning
    }

    fn check(&self, input: &RuleInput, _ctx: &CheckContext) -> Vec<Diagnostic> {
        static PASSIVE_AFTER_SHALL: std::sync::LazyLock<Regex> = std::sync::LazyLock::new(|| {
            Regex::new(r"(?i)\bshall\s+be\s+[a-z]+ed\b")
                .unwrap_or_else(|error| panic!("valid passive-shall regex: {error}"))
        });

        input
            .sentences
            .iter()
            .filter_map(|sentence| {
                let found = PASSIVE_AFTER_SHALL.find(&sentence.text)?;
                Some(
                    Diagnostic::new(
                        self.id(),
                        self.default_severity(),
                        "response after `shall` appears passive",
                        goodwrite_core::SourceRange::new(
                            sentence.range.start + found.start(),
                            sentence.range.start + found.end(),
                        ),
                    )
                    .with_help("rewrite response in active voice"),
                )
            })
            .collect()
    }
}

#[allow(dead_code)]
fn _clause_positions(sentence: &str) -> [Option<usize>; 5] {
    [
        keyword_position(sentence, "where"),
        keyword_position(sentence, "while"),
        keyword_position(sentence, "when"),
        keyword_position(sentence, "if"),
        keyword_position(sentence, "shall"),
    ]
}
