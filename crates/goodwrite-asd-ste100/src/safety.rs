use goodwrite_core::{CheckContext, Diagnostic, Rule, RuleInput, Severity, WritingMode};

pub struct SafetyRiskLevelRule;
pub struct SafetyCommandOrConditionRule;
pub struct SafetyRiskExplanationRule;

impl Rule for SafetyRiskLevelRule {
    fn id(&self) -> &str {
        "asd-ste100/safety-risk-level"
    }

    fn name(&self) -> &str {
        "Safety instruction risk level"
    }

    fn profiles(&self) -> &[&str] {
        &["asd-ste100"]
    }

    fn default_severity(&self) -> Severity {
        Severity::Warning
    }

    fn check(&self, input: &RuleInput, _ctx: &CheckContext) -> Vec<Diagnostic> {
        if input.span.annotations.effective_mode() != WritingMode::SafetyInstruction {
            return Vec::new();
        }

        let lower = input.span.text.to_ascii_lowercase();
        if lower.contains("warning") || lower.contains("caution") {
            return Vec::new();
        }

        vec![
            Diagnostic::new(
                self.id(),
                self.default_severity(),
                "safety instruction missing WARNING/CAUTION label",
                input.span.range,
            )
            .with_note("ASD-STE100 Rule 7.1")
            .with_help("start instruction with `WARNING:` or `CAUTION:`"),
        ]
    }
}

impl Rule for SafetyCommandOrConditionRule {
    fn id(&self) -> &str {
        "asd-ste100/safety-start-command-or-condition"
    }

    fn name(&self) -> &str {
        "Safety instruction opening structure"
    }

    fn profiles(&self) -> &[&str] {
        &["asd-ste100"]
    }

    fn default_severity(&self) -> Severity {
        Severity::Warning
    }

    fn check(&self, input: &RuleInput, _ctx: &CheckContext) -> Vec<Diagnostic> {
        if input.span.annotations.effective_mode() != WritingMode::SafetyInstruction {
            return Vec::new();
        }

        let Some(first_sentence) = input.sentences.first() else {
            return Vec::new();
        };

        let first = first_sentence
            .tokens
            .first()
            .map(|token| token.text.to_ascii_lowercase())
            .unwrap_or_default();

        if matches!(first.as_str(), "if" | "when" | "before" | "after") || is_command_verb(&first) {
            return Vec::new();
        }

        vec![
            Diagnostic::new(
                self.id(),
                self.default_severity(),
                "safety text should start with command or condition",
                first_sentence.range,
            )
            .with_note("ASD-STE100 Rule 7.2")
            .with_help("start with imperative command or explicit condition"),
        ]
    }
}

impl Rule for SafetyRiskExplanationRule {
    fn id(&self) -> &str {
        "asd-ste100/safety-risk-explanation"
    }

    fn name(&self) -> &str {
        "Safety instruction risk explanation"
    }

    fn profiles(&self) -> &[&str] {
        &["asd-ste100"]
    }

    fn default_severity(&self) -> Severity {
        Severity::Warning
    }

    fn check(&self, input: &RuleInput, _ctx: &CheckContext) -> Vec<Diagnostic> {
        if input.span.annotations.effective_mode() != WritingMode::SafetyInstruction {
            return Vec::new();
        }

        let lower = input.span.text.to_ascii_lowercase();
        let has_explanation = lower.contains("because")
            || lower.contains("can cause")
            || lower.contains("to prevent")
            || lower.contains("risk")
            || lower.contains("injury")
            || lower.contains("damage");

        if has_explanation {
            return Vec::new();
        }

        vec![
            Diagnostic::new(
                self.id(),
                self.default_severity(),
                "safety instruction does not explain the risk",
                input.span.range,
            )
            .with_note("ASD-STE100 Rule 7.3")
            .with_help("add consequence or risk statement"),
        ]
    }
}

fn is_command_verb(word: &str) -> bool {
    matches!(
        word,
        "keep"
            | "do"
            | "wear"
            | "install"
            | "remove"
            | "disconnect"
            | "connect"
            | "stop"
            | "open"
            | "close"
            | "turn"
            | "avoid"
            | "prevent"
    )
}
