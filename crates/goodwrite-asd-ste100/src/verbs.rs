use regex::Regex;

use goodwrite_core::{
    Applicability, CheckContext, Diagnostic, Rule, RuleInput, Severity, Suggestion, WritingMode,
};

use crate::dict::lookup::DictionaryLookup;

pub struct VerbFormRule;
pub struct ApprovedTensesRule;
pub struct PastParticipleAsAdjectiveRule;
pub struct NoComplexAuxiliaryRule;
pub struct IngFormRestrictionRule;
pub struct PassiveVoiceRule;
pub struct NominalizationRule;

impl Rule for VerbFormRule {
    fn id(&self) -> &str {
        "asd-ste100/verb-forms"
    }

    fn name(&self) -> &str {
        "Use approved dictionary verb forms"
    }

    fn profiles(&self) -> &[&str] {
        &["asd-ste100"]
    }

    fn default_severity(&self) -> Severity {
        Severity::Warning
    }

    fn check(&self, input: &RuleInput, ctx: &CheckContext) -> Vec<Diagnostic> {
        let dictionary = DictionaryLookup::for_overlay(ctx.glossary_data.as_ref());
        let mut diagnostics = Vec::new();

        for sentence in &input.sentences {
            for pair in sentence.tokens.windows(2) {
                let prev = pair[0].text.to_ascii_lowercase();
                if !matches!(prev.as_str(), "shall" | "must" | "can" | "may") {
                    continue;
                }

                let lower = pair[1].text.to_ascii_lowercase();
                if dictionary.is_approved_form(&lower) {
                    continue;
                }

                if dictionary.known_non_approved(&lower) {
                    continue;
                }

                if !lower
                    .chars()
                    .all(|ch| ch.is_ascii_alphabetic() || ch == '-')
                {
                    continue;
                }

                diagnostics.push(
                    Diagnostic::new(
                        self.id(),
                        self.default_severity(),
                        format!("`{}` is not an approved verb form", pair[1].text),
                        pair[1].range,
                    )
                    .with_note("ASD-STE100 Rule 3.1")
                    .with_help("replace with an approved base verb"),
                );
            }
        }

        diagnostics
    }
}

impl Rule for ApprovedTensesRule {
    fn id(&self) -> &str {
        "asd-ste100/approved-tenses"
    }

    fn name(&self) -> &str {
        "Use approved tenses"
    }

    fn profiles(&self) -> &[&str] {
        &["asd-ste100"]
    }

    fn default_severity(&self) -> Severity {
        Severity::Warning
    }

    fn check(&self, input: &RuleInput, _ctx: &CheckContext) -> Vec<Diagnostic> {
        static PERFECT_RE: std::sync::LazyLock<Regex> = std::sync::LazyLock::new(|| {
            Regex::new(r"(?i)\b(has|have|had)\s+[a-z]+ed\b")
                .unwrap_or_else(|error| panic!("valid perfect-tense regex: {error}"))
        });
        static PROGRESSIVE_RE: std::sync::LazyLock<Regex> = std::sync::LazyLock::new(|| {
            Regex::new(r"(?i)\b(am|is|are|was|were|being)\s+[a-z]+ing\b")
                .unwrap_or_else(|error| panic!("valid progressive regex: {error}"))
        });

        let mut diagnostics = Vec::new();

        for found in PERFECT_RE.find_iter(&input.span.text) {
            diagnostics.push(
                Diagnostic::new(
                    self.id(),
                    self.default_severity(),
                    "complex perfect tense detected",
                    absolute_range(input, found.start(), found.end()),
                )
                .with_note("ASD-STE100 Rule 3.2")
                .with_help("prefer simple present/past/future or imperative"),
            );
        }

        for found in PROGRESSIVE_RE.find_iter(&input.span.text) {
            diagnostics.push(
                Diagnostic::new(
                    self.id(),
                    self.default_severity(),
                    "progressive tense detected",
                    absolute_range(input, found.start(), found.end()),
                )
                .with_note("ASD-STE100 Rule 3.2")
                .with_help("prefer simple tenses unless used as technical noun/modifier"),
            );
        }

        diagnostics
    }
}

impl Rule for PastParticipleAsAdjectiveRule {
    fn id(&self) -> &str {
        "asd-ste100/past-participle-as-adjective"
    }

    fn name(&self) -> &str {
        "Past participles should be adjectival"
    }

    fn profiles(&self) -> &[&str] {
        &["asd-ste100"]
    }

    fn default_severity(&self) -> Severity {
        Severity::Warning
    }

    fn check(&self, input: &RuleInput, _ctx: &CheckContext) -> Vec<Diagnostic> {
        let mut diagnostics = Vec::new();
        for sentence in &input.sentences {
            for triple in sentence.tokens.windows(3) {
                let be = triple[0].text.to_ascii_lowercase();
                let middle = triple[1].text.to_ascii_lowercase();
                let next = triple[2].text.to_ascii_lowercase();

                if !matches!(be.as_str(), "is" | "are" | "was" | "were" | "be") {
                    continue;
                }
                if !middle.ends_with("ed") {
                    continue;
                }
                if matches!(next.as_str(), "by" | "to") {
                    diagnostics.push(
                        Diagnostic::new(
                            self.id(),
                            self.default_severity(),
                            "past participle appears in verbal construction",
                            triple[1].range,
                        )
                        .with_note("ASD-STE100 Rule 3.3")
                        .with_help("use participle as adjective before noun, or rewrite"),
                    );
                }
            }
        }

        diagnostics
    }
}

impl Rule for NoComplexAuxiliaryRule {
    fn id(&self) -> &str {
        "asd-ste100/no-complex-auxiliary"
    }

    fn name(&self) -> &str {
        "Avoid complex auxiliary constructions"
    }

    fn profiles(&self) -> &[&str] {
        &["asd-ste100"]
    }

    fn default_severity(&self) -> Severity {
        Severity::Warning
    }

    fn check(&self, input: &RuleInput, _ctx: &CheckContext) -> Vec<Diagnostic> {
        static AUX_RE: std::sync::LazyLock<Regex> = std::sync::LazyLock::new(|| {
            Regex::new(r"(?i)\b(has|have|had)\s+[a-z]+ed\b|\b(is|are|was|were|be)\s+to\s+[a-z]+\b")
                .unwrap_or_else(|error| panic!("valid auxiliary regex: {error}"))
        });

        AUX_RE
            .find_iter(&input.span.text)
            .map(|found| {
                Diagnostic::new(
                    self.id(),
                    self.default_severity(),
                    "auxiliary verb chain detected",
                    absolute_range(input, found.start(), found.end()),
                )
                .with_note("ASD-STE100 Rule 3.4")
                .with_help("rewrite with a simpler verb construction")
            })
            .collect()
    }
}

impl Rule for IngFormRestrictionRule {
    fn id(&self) -> &str {
        "asd-ste100/ing-form-restriction"
    }

    fn name(&self) -> &str {
        "Restrict -ing forms"
    }

    fn profiles(&self) -> &[&str] {
        &["asd-ste100"]
    }

    fn default_severity(&self) -> Severity {
        Severity::Warning
    }

    fn check(&self, input: &RuleInput, ctx: &CheckContext) -> Vec<Diagnostic> {
        let glossary = ctx.glossary.as_ref();
        let mut diagnostics = Vec::new();

        for sentence in &input.sentences {
            for (idx, token) in sentence.tokens.iter().enumerate() {
                let lower = token.text.to_ascii_lowercase();
                if !lower.ends_with("ing") {
                    continue;
                }

                if glossary.is_some_and(|entries| entries.has_term(&lower)) {
                    continue;
                }

                let next_is_noun = sentence
                    .tokens
                    .get(idx + 1)
                    .is_some_and(|next| next.text.chars().all(|ch| ch.is_ascii_alphabetic()));
                if next_is_noun {
                    continue;
                }

                diagnostics.push(
                    Diagnostic::new(
                        self.id(),
                        self.default_severity(),
                        format!("`{}` uses restricted -ing form", token.text),
                        token.range,
                    )
                    .with_note("ASD-STE100 Rule 3.5")
                    .with_help("use imperative or simple tense verb"),
                );
            }
        }

        diagnostics
    }
}

impl Rule for PassiveVoiceRule {
    fn id(&self) -> &str {
        "asd-ste100/passive-voice"
    }

    fn name(&self) -> &str {
        "Use active voice"
    }

    fn profiles(&self) -> &[&str] {
        &["asd-ste100"]
    }

    fn default_severity(&self) -> Severity {
        Severity::Warning
    }

    fn check(&self, input: &RuleInput, _ctx: &CheckContext) -> Vec<Diagnostic> {
        static PASSIVE_RE: std::sync::LazyLock<Regex> = std::sync::LazyLock::new(|| {
            Regex::new(r"(?i)\b(is|are|was|were|be|been|being)\s+([a-z]+ed)\b")
                .unwrap_or_else(|error| panic!("valid passive regex: {error}"))
        });

        let mode = input.span.annotations.effective_mode();
        let mut diagnostics = Vec::new();

        for found in PASSIVE_RE.find_iter(&input.span.text) {
            let clause = &input.span.text[found.start()..found.end()];
            if matches!(mode, WritingMode::Descriptive)
                && !input.span.text[found.end()..].contains(" by ")
            {
                continue;
            }

            let absolute = absolute_range(input, found.start(), found.end());

            diagnostics.push(
                Diagnostic::new(
                    self.id(),
                    self.default_severity(),
                    format!("passive voice detected: `{clause}`"),
                    absolute,
                )
                .with_note("ASD-STE100 Rule 3.6")
                .with_help("rewrite with an explicit subject performing the action")
                .with_suggestion(Suggestion {
                    span: absolute,
                    replacement: clause.to_string(),
                    applicability: Applicability::MaybeIncorrect,
                    message: "manual rewrite to active voice required".to_string(),
                }),
            );
        }

        diagnostics
    }
}

impl Rule for NominalizationRule {
    fn id(&self) -> &str {
        "asd-ste100/nominalization"
    }

    fn name(&self) -> &str {
        "Prefer verbs over nominalizations"
    }

    fn profiles(&self) -> &[&str] {
        &["asd-ste100"]
    }

    fn default_severity(&self) -> Severity {
        Severity::Warning
    }

    fn check(&self, input: &RuleInput, _ctx: &CheckContext) -> Vec<Diagnostic> {
        static NOMINALIZATION_RE: std::sync::LazyLock<Regex> = std::sync::LazyLock::new(|| {
            Regex::new(r"(?i)\b[a-z]+(tion|ment|ance|ence)\s+of\b")
                .unwrap_or_else(|error| panic!("valid nominalization regex: {error}"))
        });

        NOMINALIZATION_RE
            .find_iter(&input.span.text)
            .map(|found| {
                Diagnostic::new(
                    self.id(),
                    self.default_severity(),
                    "nominalization detected",
                    absolute_range(input, found.start(), found.end()),
                )
                .with_note("ASD-STE100 Rule 3.7")
                .with_help("rewrite with an action verb")
            })
            .collect()
    }
}

fn absolute_range(
    input: &RuleInput,
    local_start: usize,
    local_end: usize,
) -> goodwrite_core::SourceRange {
    goodwrite_core::SourceRange::new(
        input.span.range.start + local_start,
        input.span.range.start + local_end,
    )
}
