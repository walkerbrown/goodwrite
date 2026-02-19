use goodwrite_core::{CheckContext, Diagnostic, Rule, RuleInput, Severity};

use crate::patterns::clause_after_keyword;

pub struct UntestableResponseRule;
pub struct AmbiguousTriggerRule;
pub struct VaguePreconditionRule;

impl Rule for UntestableResponseRule {
    fn id(&self) -> &str {
        "ears/untestable-response"
    }

    fn name(&self) -> &str {
        "Response should be testable"
    }

    fn profiles(&self) -> &[&str] {
        &["ears"]
    }

    fn default_severity(&self) -> Severity {
        Severity::Warning
    }

    fn check(&self, input: &RuleInput, _ctx: &CheckContext) -> Vec<Diagnostic> {
        const VAGUE: &[&str] = &["appropriate", "sufficient", "normal", "as necessary"];

        input
            .sentences
            .iter()
            .filter_map(|sentence| {
                let lower = sentence.text.to_ascii_lowercase();
                if !lower.contains("shall") {
                    return None;
                }

                for term in VAGUE {
                    if lower.contains(term) {
                        return Some(
                            Diagnostic::new(
                                self.id(),
                                self.default_severity(),
                                format!("response contains vague term `{term}`"),
                                sentence.range,
                            )
                            .with_help("replace with measurable, verifiable behavior"),
                        );
                    }
                }

                None
            })
            .collect()
    }
}

impl Rule for AmbiguousTriggerRule {
    fn id(&self) -> &str {
        "ears/ambiguous-trigger"
    }

    fn name(&self) -> &str {
        "When trigger should be discrete"
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
                let trigger = clause_after_keyword(&sentence.text, "when")?;
                let lower = trigger.to_ascii_lowercase();
                let words = lower.split_whitespace().count();
                let conjunctive = lower.contains(" and ") || lower.contains(" or ");

                if !conjunctive && words <= 12 {
                    return None;
                }

                Some(
                    Diagnostic::new(
                        self.id(),
                        self.default_severity(),
                        "`When` clause may describe multiple triggers",
                        sentence.range,
                    )
                    .with_help("make trigger event singular and discrete"),
                )
            })
            .collect()
    }
}

impl Rule for VaguePreconditionRule {
    fn id(&self) -> &str {
        "ears/vague-precondition"
    }

    fn name(&self) -> &str {
        "While precondition should be testable"
    }

    fn profiles(&self) -> &[&str] {
        &["ears"]
    }

    fn default_severity(&self) -> Severity {
        Severity::Info
    }

    fn check(&self, input: &RuleInput, _ctx: &CheckContext) -> Vec<Diagnostic> {
        const VAGUE: &[&str] = &["normal", "sufficient", "appropriate", "stable"];

        input
            .sentences
            .iter()
            .filter_map(|sentence| {
                let clause = clause_after_keyword(&sentence.text, "while")?;
                let lower = clause.to_ascii_lowercase();
                if !VAGUE.iter().any(|term| lower.contains(term)) {
                    return None;
                }

                Some(
                    Diagnostic::new(
                        self.id(),
                        self.default_severity(),
                        "`While` clause contains vague state description",
                        sentence.range,
                    )
                    .with_help("replace with measurable state condition"),
                )
            })
            .collect()
    }
}
