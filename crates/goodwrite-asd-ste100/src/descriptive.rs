use std::collections::HashSet;

use goodwrite_core::{CheckContext, Diagnostic, Rule, RuleInput, Severity, WritingMode};

pub struct SentenceLengthDescriptiveRule;
pub struct OneSubjectPerSentenceRule;
pub struct KeyWordsForStructureRule;
pub struct RelatedInfoParagraphRule;
pub struct OneTopicPerParagraphRule;
pub struct MaxSentencesPerParagraphRule;

impl Rule for SentenceLengthDescriptiveRule {
    fn id(&self) -> &str {
        "asd-ste100/sentence-length-descriptive"
    }

    fn name(&self) -> &str {
        "Descriptive sentence length"
    }

    fn profiles(&self) -> &[&str] {
        &["asd-ste100"]
    }

    fn default_severity(&self) -> Severity {
        Severity::Error
    }

    fn check(&self, input: &RuleInput, ctx: &CheckContext) -> Vec<Diagnostic> {
        let mode = input.span.annotations.effective_mode();
        if !matches!(mode, WritingMode::Descriptive | WritingMode::Note) {
            return Vec::new();
        }

        let max_words = ctx.config.rule_max_words(self.id()).unwrap_or(25);
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
                            "descriptive sentence has {} words; maximum is {}",
                            sentence.ste_word_count, max_words
                        ),
                        sentence.range,
                    )
                    .with_note("ASD-STE100 Rule 6.3")
                    .with_help("split into shorter descriptive sentences"),
                )
            })
            .collect()
    }
}

impl Rule for OneSubjectPerSentenceRule {
    fn id(&self) -> &str {
        "asd-ste100/one-subject-per-sentence"
    }

    fn name(&self) -> &str {
        "One subject per descriptive sentence"
    }

    fn profiles(&self) -> &[&str] {
        &["asd-ste100"]
    }

    fn default_severity(&self) -> Severity {
        Severity::Info
    }

    fn check(&self, input: &RuleInput, _ctx: &CheckContext) -> Vec<Diagnostic> {
        if input.span.annotations.effective_mode() != WritingMode::Descriptive {
            return Vec::new();
        }

        input
            .sentences
            .iter()
            .filter_map(|sentence| {
                let subject_markers = sentence
                    .text
                    .split_whitespace()
                    .filter(|part| part.eq_ignore_ascii_case("the"))
                    .count();
                if subject_markers <= 2 {
                    return None;
                }

                Some(
                    Diagnostic::new(
                        self.id(),
                        self.default_severity(),
                        "sentence may contain multiple subjects",
                        sentence.range,
                    )
                    .with_note("ASD-STE100 Rule 6.1")
                    .with_help("simplify to one main subject per sentence"),
                )
            })
            .collect()
    }
}

impl Rule for KeyWordsForStructureRule {
    fn id(&self) -> &str {
        "asd-ste100/key-words-structure"
    }

    fn name(&self) -> &str {
        "Use key words for logical structure"
    }

    fn profiles(&self) -> &[&str] {
        &["asd-ste100"]
    }

    fn default_severity(&self) -> Severity {
        Severity::Info
    }

    fn check(&self, input: &RuleInput, _ctx: &CheckContext) -> Vec<Diagnostic> {
        if input.span.annotations.effective_mode() != WritingMode::Descriptive {
            return Vec::new();
        }

        let mut diagnostics = Vec::new();
        for pair in input.sentences.windows(2) {
            let a = keywords(&pair[0].text);
            let b = keywords(&pair[1].text);
            if !a.is_empty() && !b.is_empty() && a.is_disjoint(&b) {
                diagnostics.push(
                    Diagnostic::new(
                        self.id(),
                        self.default_severity(),
                        "adjacent sentences may lack shared key words",
                        pair[1].range,
                    )
                    .with_note("ASD-STE100 Rule 6.2")
                    .with_help("repeat key term to reinforce logical flow"),
                );
            }
        }

        diagnostics
    }
}

impl Rule for RelatedInfoParagraphRule {
    fn id(&self) -> &str {
        "asd-ste100/paragraph-related-info"
    }

    fn name(&self) -> &str {
        "Use paragraphs for related information"
    }

    fn profiles(&self) -> &[&str] {
        &["asd-ste100"]
    }

    fn default_severity(&self) -> Severity {
        Severity::Info
    }

    fn check(&self, input: &RuleInput, _ctx: &CheckContext) -> Vec<Diagnostic> {
        if input.span.annotations.effective_mode() != WritingMode::Descriptive {
            return Vec::new();
        }

        if input.sentences.len() >= 4 && !input.span.text.contains('\n') {
            return vec![
                Diagnostic::new(
                    self.id(),
                    self.default_severity(),
                    "long descriptive block may need paragraph breaks",
                    input.span.range,
                )
                .with_note("ASD-STE100 Rule 6.4")
                .with_help("split into smaller paragraphs by topic"),
            ];
        }

        Vec::new()
    }
}

impl Rule for OneTopicPerParagraphRule {
    fn id(&self) -> &str {
        "asd-ste100/one-topic-per-paragraph"
    }

    fn name(&self) -> &str {
        "One topic per paragraph"
    }

    fn profiles(&self) -> &[&str] {
        &["asd-ste100"]
    }

    fn default_severity(&self) -> Severity {
        Severity::Info
    }

    fn check(&self, input: &RuleInput, _ctx: &CheckContext) -> Vec<Diagnostic> {
        if input.span.annotations.effective_mode() != WritingMode::Descriptive {
            return Vec::new();
        }

        let lower = input.span.text.to_ascii_lowercase();
        if lower.contains("however") && lower.contains("meanwhile") {
            return vec![
                Diagnostic::new(
                    self.id(),
                    self.default_severity(),
                    "paragraph may mix multiple topics",
                    input.span.range,
                )
                .with_note("ASD-STE100 Rule 6.5")
                .with_help("split into separate topic-focused paragraphs"),
            ];
        }

        Vec::new()
    }
}

impl Rule for MaxSentencesPerParagraphRule {
    fn id(&self) -> &str {
        "asd-ste100/max-sentences-paragraph"
    }

    fn name(&self) -> &str {
        "Paragraph sentence count"
    }

    fn profiles(&self) -> &[&str] {
        &["asd-ste100"]
    }

    fn default_severity(&self) -> Severity {
        Severity::Warning
    }

    fn check(&self, input: &RuleInput, _ctx: &CheckContext) -> Vec<Diagnostic> {
        if input.span.annotations.effective_mode() != WritingMode::Descriptive {
            return Vec::new();
        }

        if input.sentences.len() <= 6 {
            return Vec::new();
        }

        vec![
            Diagnostic::new(
                self.id(),
                self.default_severity(),
                format!(
                    "paragraph has {} sentences; maximum recommended is 6",
                    input.sentences.len()
                ),
                input.span.range,
            )
            .with_note("ASD-STE100 Rule 6.6")
            .with_help("split paragraph into shorter sections"),
        ]
    }
}

fn keywords(text: &str) -> HashSet<String> {
    text.split(|ch: char| !ch.is_ascii_alphabetic())
        .filter(|word| word.len() >= 5)
        .map(|word| word.to_ascii_lowercase())
        .collect::<HashSet<_>>()
}
