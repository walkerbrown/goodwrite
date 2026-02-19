use goodwrite_core::{CheckContext, Diagnostic, Rule, RuleInput, Severity, WritingMode};

pub struct SentenceLengthProceduralRule;
pub struct OneInstructionPerSentenceRule;
pub struct ImperativeProceduralRule;
pub struct ConditionBeforeCommandRule;
pub struct NoteNoImperativeRule;

impl Rule for SentenceLengthProceduralRule {
    fn id(&self) -> &str {
        "asd-ste100/sentence-length-procedural"
    }

    fn name(&self) -> &str {
        "Procedural sentence length"
    }

    fn profiles(&self) -> &[&str] {
        &["asd-ste100"]
    }

    fn default_severity(&self) -> Severity {
        Severity::Error
    }

    fn check(&self, input: &RuleInput, ctx: &CheckContext) -> Vec<Diagnostic> {
        let mode = input.span.annotations.effective_mode();
        if !matches!(
            mode,
            WritingMode::Procedural | WritingMode::SafetyInstruction
        ) {
            return Vec::new();
        }

        let max_words = ctx.config.rule_max_words(self.id()).unwrap_or(20);
        input
            .sentences
            .iter()
            .filter_map(|sentence| {
                if sentence.ste_word_count <= max_words {
                    return None;
                }

                Some(
                    Diagnostic::new(
                        self.id(),
                        self.default_severity(),
                        format!(
                            "procedural sentence has {} words; maximum is {}",
                            sentence.ste_word_count, max_words
                        ),
                        sentence.range,
                    )
                    .with_note("ASD-STE100 Rule 5.1")
                    .with_help("split into shorter instructions"),
                )
            })
            .collect()
    }
}

impl Rule for OneInstructionPerSentenceRule {
    fn id(&self) -> &str {
        "asd-ste100/one-instruction-per-sentence"
    }

    fn name(&self) -> &str {
        "One instruction per sentence"
    }

    fn profiles(&self) -> &[&str] {
        &["asd-ste100"]
    }

    fn default_severity(&self) -> Severity {
        Severity::Warning
    }

    fn check(&self, input: &RuleInput, _ctx: &CheckContext) -> Vec<Diagnostic> {
        if input.span.annotations.effective_mode() != WritingMode::Procedural {
            return Vec::new();
        }

        input
            .sentences
            .iter()
            .filter_map(|sentence| {
                let verb_count = sentence
                    .tokens
                    .iter()
                    .filter(|token| is_imperative_candidate(&token.text))
                    .count();
                if verb_count <= 1 {
                    return None;
                }

                Some(
                    Diagnostic::new(
                        self.id(),
                        self.default_severity(),
                        format!("sentence contains {verb_count} instruction verbs"),
                        sentence.range,
                    )
                    .with_note("ASD-STE100 Rule 5.2")
                    .with_help("split into separate instruction sentences"),
                )
            })
            .collect()
    }
}

impl Rule for ImperativeProceduralRule {
    fn id(&self) -> &str {
        "asd-ste100/imperative-procedural"
    }

    fn name(&self) -> &str {
        "Procedural instructions should be imperative"
    }

    fn profiles(&self) -> &[&str] {
        &["asd-ste100"]
    }

    fn default_severity(&self) -> Severity {
        Severity::Warning
    }

    fn check(&self, input: &RuleInput, _ctx: &CheckContext) -> Vec<Diagnostic> {
        if input.span.annotations.effective_mode() != WritingMode::Procedural {
            return Vec::new();
        }

        input
            .sentences
            .iter()
            .filter_map(|sentence| {
                let first = sentence
                    .tokens
                    .first()
                    .map(|token| token.text.to_ascii_lowercase())
                    .unwrap_or_default();

                if matches!(first.as_str(), "if" | "when" | "before" | "after") {
                    return None;
                }

                if is_imperative_candidate(&first) {
                    return None;
                }

                Some(
                    Diagnostic::new(
                        self.id(),
                        self.default_severity(),
                        "instruction may not be in imperative form",
                        sentence.range,
                    )
                    .with_note("ASD-STE100 Rule 5.3")
                    .with_help("start sentence with an imperative verb"),
                )
            })
            .collect()
    }
}

impl Rule for ConditionBeforeCommandRule {
    fn id(&self) -> &str {
        "asd-ste100/condition-before-command"
    }

    fn name(&self) -> &str {
        "Put condition before command"
    }

    fn profiles(&self) -> &[&str] {
        &["asd-ste100"]
    }

    fn default_severity(&self) -> Severity {
        Severity::Warning
    }

    fn check(&self, input: &RuleInput, _ctx: &CheckContext) -> Vec<Diagnostic> {
        if input.span.annotations.effective_mode() != WritingMode::Procedural {
            return Vec::new();
        }

        input
            .sentences
            .iter()
            .filter_map(|sentence| {
                let lower = sentence.text.to_ascii_lowercase();
                let has_command_start = sentence
                    .tokens
                    .first()
                    .map(|token| is_imperative_candidate(&token.text))
                    .unwrap_or(false);

                if !has_command_start {
                    return None;
                }

                if lower.contains(", if") || lower.contains(", when") {
                    return Some(
                        Diagnostic::new(
                            self.id(),
                            self.default_severity(),
                            "condition appears after command",
                            sentence.range,
                        )
                        .with_note("ASD-STE100 Rule 5.4")
                        .with_help("move condition clause before command and separate with comma"),
                    );
                }

                None
            })
            .collect()
    }
}

impl Rule for NoteNoImperativeRule {
    fn id(&self) -> &str {
        "asd-ste100/note-no-imperative"
    }

    fn name(&self) -> &str {
        "Notes should be informational"
    }

    fn profiles(&self) -> &[&str] {
        &["asd-ste100"]
    }

    fn default_severity(&self) -> Severity {
        Severity::Warning
    }

    fn check(&self, input: &RuleInput, _ctx: &CheckContext) -> Vec<Diagnostic> {
        if input.span.annotations.effective_mode() != WritingMode::Note {
            return Vec::new();
        }

        let mut diagnostics = Vec::new();
        for sentence in &input.sentences {
            let first = sentence
                .tokens
                .first()
                .map(|token| token.text.to_ascii_lowercase())
                .unwrap_or_default();

            if is_imperative_candidate(&first) {
                diagnostics.push(
                    Diagnostic::new(
                        self.id(),
                        self.default_severity(),
                        "note uses imperative verb",
                        sentence.range,
                    )
                    .with_note("ASD-STE100 Rule 5.5")
                    .with_help("rewrite note as descriptive information"),
                );
            }

            if sentence.ste_word_count > 25 {
                diagnostics.push(
                    Diagnostic::new(
                        self.id(),
                        self.default_severity(),
                        format!(
                            "note sentence has {} words; maximum is 25",
                            sentence.ste_word_count
                        ),
                        sentence.range,
                    )
                    .with_note("ASD-STE100 Rule 5.5")
                    .with_help("shorten the note sentence"),
                );
            }
        }

        diagnostics
    }
}

fn is_imperative_candidate(token: &str) -> bool {
    matches!(
        token.to_ascii_lowercase().as_str(),
        "open"
            | "close"
            | "remove"
            | "install"
            | "press"
            | "set"
            | "check"
            | "verify"
            | "connect"
            | "disconnect"
            | "tighten"
            | "loosen"
            | "move"
            | "turn"
            | "apply"
            | "stop"
            | "start"
            | "drain"
            | "fill"
            | "keep"
            | "use"
    )
}
