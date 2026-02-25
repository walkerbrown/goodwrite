use goodwrite_core::{
    Applicability, CheckContext, Diagnostic, Rule, RuleInput, Severity, Suggestion,
};

use crate::compliance::{ComplianceAction, ComplianceEngine, ComplianceState};
use crate::dict::lookup::DictionaryLookup;
use crate::guidance::{GuidanceScenario, guidance_for};

pub struct UnapprovedWordRule;
pub struct ApprovedWordPosMismatchRule;
pub struct AmbiguousPosRule;
pub struct AlternativePosMismatchRule;
pub struct ApprovedMeaningRule;
pub struct ApprovedFormRule;
pub struct TechnicalNounCategoryRule;
pub struct NonApprovedTechnicalNounRule;
pub struct TechnicalNounAsVerbRule;
pub struct CompanyApprovedTechnicalNounRule;
pub struct TechnicalNounLengthRule;
pub struct NoJargonTechnicalNounRule;
pub struct ConsistentTechnicalNounRule;
pub struct TechnicalVerbCategoryRule;
pub struct TechnicalVerbAsNounRule;
pub struct AmericanSpellingRule;

impl Rule for UnapprovedWordRule {
    fn id(&self) -> &str {
        "asd-ste100/unapproved-word"
    }

    fn name(&self) -> &str {
        "Use approved ASD-STE100 words"
    }

    fn profiles(&self) -> &[&str] {
        &["asd-ste100"]
    }

    fn default_severity(&self) -> Severity {
        Severity::Error
    }

    fn check(&self, input: &RuleInput, ctx: &CheckContext) -> Vec<Diagnostic> {
        let dictionary = DictionaryLookup::for_overlay(ctx.glossary_data.as_ref());
        let compliance = ComplianceEngine::new(ctx);
        let mode = input.span.annotations.effective_mode();
        let mut diagnostics = Vec::new();

        for sentence in &input.sentences {
            for decision in compliance.analyze_sentence(sentence, mode) {
                if !dictionary.known_non_approved(&decision.normalized) {
                    continue;
                }
                if covered_by_actionable_glossary_synonym(input, ctx, decision.token.range) {
                    continue;
                }

                let Some(alternatives) = dictionary.alternatives_for_word(&decision.normalized)
                else {
                    continue;
                };
                let alternatives_text = alternatives
                    .iter()
                    .map(|alternative| alternative.word.clone())
                    .collect::<Vec<_>>()
                    .join(", ");

                let mut diagnostic = Diagnostic::new(
                    self.id(),
                    self.default_severity(),
                    format!("`{}` is not approved in ASD-STE100", decision.token.text),
                    decision.token.range,
                )
                .with_note("ASD-STE100 Rule 1.1")
                .with_help(format!(
                    "{}; use one of: {}",
                    guidance_for(mode, GuidanceScenario::NonApprovedWord),
                    alternatives_text
                ));

                if let Some(replacement) = decision.actions.iter().find_map(|action| match action {
                    ComplianceAction::UseAlternative { word } => Some(word),
                    _ => None,
                }) {
                    diagnostic = diagnostic.with_suggestion(Suggestion {
                        span: decision.token.range,
                        replacement: replacement.clone(),
                        applicability: Applicability::MaybeIncorrect,
                        message: "replace with approved alternative".to_string(),
                    });
                }

                diagnostics.push(diagnostic);
            }
        }

        diagnostics
    }
}

impl Rule for ApprovedWordPosMismatchRule {
    fn id(&self) -> &str {
        "asd-ste100/approved-word-pos-mismatch"
    }

    fn name(&self) -> &str {
        "Approved words must be used with approved POS"
    }

    fn profiles(&self) -> &[&str] {
        &["asd-ste100"]
    }

    fn default_severity(&self) -> Severity {
        Severity::Warning
    }

    fn check(&self, input: &RuleInput, ctx: &CheckContext) -> Vec<Diagnostic> {
        let dictionary = DictionaryLookup::for_overlay(ctx.glossary_data.as_ref());
        let compliance = ComplianceEngine::new(ctx);
        let mode = input.span.annotations.effective_mode();
        let mut diagnostics = Vec::new();

        for sentence in &input.sentences {
            for decision in compliance.analyze_sentence(sentence, mode) {
                if !dictionary.is_approved_form(&decision.normalized) {
                    continue;
                }

                if !matches!(decision.state, ComplianceState::NeedsRewrite) {
                    continue;
                }

                if !decision
                    .actions
                    .iter()
                    .any(|action| matches!(action, ComplianceAction::ClarifyPartOfSpeech))
                {
                    continue;
                }

                let allowed = dictionary
                    .allowed_pos(&decision.normalized)
                    .map(|values| values.iter().cloned().collect::<Vec<_>>().join(", "))
                    .unwrap_or_else(|| "unknown".to_string());

                diagnostics.push(
                    Diagnostic::new(
                        self.id(),
                        self.default_severity(),
                        format!(
                            "`{}` does not match approved part-of-speech for this context (allowed: `{}`)",
                            decision.token.text, allowed
                        ),
                        decision.token.range,
                    )
                    .with_note("ASD-STE100 Rule 1.2")
                    .with_help(guidance_for(mode, GuidanceScenario::PosMismatch)),
                );
            }
        }

        diagnostics
    }
}

impl Rule for AmbiguousPosRule {
    fn id(&self) -> &str {
        "asd-ste100/ambiguous-pos"
    }

    fn name(&self) -> &str {
        "Part-of-speech is ambiguous"
    }

    fn profiles(&self) -> &[&str] {
        &["asd-ste100"]
    }

    fn default_severity(&self) -> Severity {
        Severity::Warning
    }

    fn check(&self, input: &RuleInput, ctx: &CheckContext) -> Vec<Diagnostic> {
        let compliance = ComplianceEngine::new(ctx);
        let dictionary = DictionaryLookup::for_overlay(ctx.glossary_data.as_ref());
        let mode = input.span.annotations.effective_mode();
        let mut diagnostics = Vec::new();

        for sentence in &input.sentences {
            for decision in compliance.analyze_sentence(sentence, mode) {
                if !dictionary.is_known_word(&decision.normalized) {
                    continue;
                }

                if !matches!(decision.state, ComplianceState::Ambiguous) {
                    continue;
                }

                if !decision
                    .actions
                    .iter()
                    .any(|action| matches!(action, ComplianceAction::ClarifyPartOfSpeech))
                {
                    continue;
                }

                diagnostics.push(
                    Diagnostic::new(
                        self.id(),
                        self.default_severity(),
                        format!(
                            "part-of-speech for `{}` is ambiguous in this sentence",
                            decision.token.text
                        ),
                        decision.token.range,
                    )
                    .with_note("ASD compliance engine requires resolved POS for this decision")
                    .with_help(guidance_for(mode, GuidanceScenario::AmbiguousPos)),
                );
            }
        }

        diagnostics
    }
}

impl Rule for AlternativePosMismatchRule {
    fn id(&self) -> &str {
        "asd-ste100/alternative-pos-mismatch"
    }

    fn name(&self) -> &str {
        "No same-POS approved alternative"
    }

    fn profiles(&self) -> &[&str] {
        &["asd-ste100"]
    }

    fn default_severity(&self) -> Severity {
        Severity::Warning
    }

    fn check(&self, input: &RuleInput, ctx: &CheckContext) -> Vec<Diagnostic> {
        let compliance = ComplianceEngine::new(ctx);
        let mode = input.span.annotations.effective_mode();
        let mut diagnostics = Vec::new();

        for sentence in &input.sentences {
            for decision in compliance.analyze_sentence(sentence, mode) {
                if !decision
                    .actions
                    .iter()
                    .any(|action| matches!(action, ComplianceAction::AlternativePosMismatch))
                {
                    continue;
                }

                diagnostics.push(
                    Diagnostic::new(
                        self.id(),
                        self.default_severity(),
                        format!(
                            "no same-POS approved alternative for `{}`",
                            decision.token.text
                        ),
                        decision.token.range,
                    )
                    .with_note("ASD compliance engine: replacement must preserve part-of-speech")
                    .with_help(guidance_for(mode, GuidanceScenario::AlternativePosMismatch)),
                );
            }
        }

        diagnostics
    }
}

impl Rule for ApprovedMeaningRule {
    fn id(&self) -> &str {
        "asd-ste100/approved-meaning"
    }

    fn name(&self) -> &str {
        "Approved words should keep approved meanings"
    }

    fn profiles(&self) -> &[&str] {
        &["asd-ste100"]
    }

    fn default_severity(&self) -> Severity {
        Severity::Info
    }

    fn check(&self, input: &RuleInput, _ctx: &CheckContext) -> Vec<Diagnostic> {
        const AMBIGUOUS: &[&str] = &["set", "control", "support", "clear", "normal"];

        input
            .sentences
            .iter()
            .flat_map(|sentence| sentence.tokens.iter())
            .filter(|token| AMBIGUOUS.contains(&token.text.to_ascii_lowercase().as_str()))
            .map(|token| {
                Diagnostic::new(
                    self.id(),
                    self.default_severity(),
                    format!(
                        "`{}` can be ambiguous; confirm approved meaning in context",
                        token.text
                    ),
                    token.range,
                )
                .with_note("ASD-STE100 Rule 1.3")
            })
            .collect()
    }
}

impl Rule for ApprovedFormRule {
    fn id(&self) -> &str {
        "asd-ste100/approved-word-form"
    }

    fn name(&self) -> &str {
        "Use approved verb/adjective forms"
    }

    fn profiles(&self) -> &[&str] {
        &["asd-ste100"]
    }

    fn default_severity(&self) -> Severity {
        Severity::Error
    }

    fn check(&self, input: &RuleInput, ctx: &CheckContext) -> Vec<Diagnostic> {
        let dictionary = DictionaryLookup::for_overlay(ctx.glossary_data.as_ref());
        let mut diagnostics = Vec::new();

        for token in input
            .sentences
            .iter()
            .flat_map(|sentence| sentence.tokens.iter())
        {
            if !is_word_token(&token.text) {
                continue;
            }

            let lower = token.text.to_ascii_lowercase();
            if dictionary.is_approved_form(&lower) || dictionary.known_non_approved(&lower) {
                continue;
            }

            let Some(lemma) = heuristic_lemma(&lower) else {
                continue;
            };

            if !dictionary.is_approved(&lemma) {
                continue;
            }

            diagnostics.push(
                Diagnostic::new(
                    self.id(),
                    self.default_severity(),
                    format!("`{}` is not an approved form of `{lemma}`", token.text),
                    token.range,
                )
                .with_note("ASD-STE100 Rule 1.4")
                .with_help(format!("use an approved form of `{lemma}`")),
            );
        }

        diagnostics
    }
}

impl Rule for TechnicalNounCategoryRule {
    fn id(&self) -> &str {
        "asd-ste100/technical-noun-category"
    }

    fn name(&self) -> &str {
        "Technical nouns should be categorized"
    }

    fn profiles(&self) -> &[&str] {
        &["asd-ste100"]
    }

    fn default_severity(&self) -> Severity {
        Severity::Info
    }

    fn check(&self, input: &RuleInput, ctx: &CheckContext) -> Vec<Diagnostic> {
        let Some(glossary) = ctx.glossary.as_ref() else {
            return Vec::new();
        };

        let mut diagnostics = Vec::new();
        for token in input
            .sentences
            .iter()
            .flat_map(|sentence| sentence.tokens.iter())
        {
            if !looks_technical_noun_candidate(&token.text) {
                continue;
            }

            let lower = token.text.to_ascii_lowercase();
            if glossary.has_term(&lower) {
                continue;
            }

            diagnostics.push(
                Diagnostic::new(
                    self.id(),
                    self.default_severity(),
                    format!(
                        "technical noun candidate `{}` is not in glossary",
                        token.text
                    ),
                    token.range,
                )
                .with_note("ASD-STE100 Rule 1.5")
                .with_help(
                    "add term with category to glossary.toml [[terms]] if this is intentional",
                ),
            );
        }

        diagnostics
    }
}

impl Rule for NonApprovedTechnicalNounRule {
    fn id(&self) -> &str {
        "asd-ste100/non-approved-as-technical-noun"
    }

    fn name(&self) -> &str {
        "Non-approved words allowed only as technical nouns"
    }

    fn profiles(&self) -> &[&str] {
        &["asd-ste100"]
    }

    fn default_severity(&self) -> Severity {
        Severity::Info
    }

    fn check(&self, input: &RuleInput, ctx: &CheckContext) -> Vec<Diagnostic> {
        let dictionary = DictionaryLookup::for_overlay(ctx.glossary_data.as_ref());
        let glossary = ctx.glossary.as_ref();

        input
            .sentences
            .iter()
            .flat_map(|sentence| sentence.tokens.iter())
            .filter_map(|token| {
                let lower = token.text.to_ascii_lowercase();
                if !dictionary.known_non_approved(&lower) {
                    return None;
                }
                if covered_by_actionable_glossary_synonym(input, ctx, token.range) {
                    return None;
                }

                let in_glossary = glossary.is_some_and(|loaded| loaded.has_term(&lower));
                if in_glossary {
                    return None;
                }

                Some(
                    Diagnostic::new(
                        self.id(),
                        self.default_severity(),
                        format!(
                            "`{}` is non-approved and not recognized as technical noun",
                            token.text
                        ),
                        token.range,
                    )
                    .with_note("ASD-STE100 Rule 1.6"),
                )
            })
            .collect()
    }
}

impl Rule for TechnicalNounAsVerbRule {
    fn id(&self) -> &str {
        "asd-ste100/technical-noun-as-verb"
    }

    fn name(&self) -> &str {
        "Do not use technical nouns as verbs"
    }

    fn profiles(&self) -> &[&str] {
        &["asd-ste100"]
    }

    fn default_severity(&self) -> Severity {
        Severity::Warning
    }

    fn check(&self, input: &RuleInput, ctx: &CheckContext) -> Vec<Diagnostic> {
        let Some(glossary) = ctx.glossary.as_ref() else {
            return Vec::new();
        };

        let mut diagnostics = Vec::new();
        for sentence in &input.sentences {
            for pair in sentence.tokens.windows(2) {
                let prev = pair[0].text.to_ascii_lowercase();
                let word = pair[1].text.to_ascii_lowercase();
                if !matches!(prev.as_str(), "shall" | "must" | "can" | "may") {
                    continue;
                }

                let Some(entry) = glossary
                    .approved()
                    .iter()
                    .find(|entry| entry.word.eq_ignore_ascii_case(&word))
                else {
                    continue;
                };

                if !entry.pos.eq_ignore_ascii_case("noun") {
                    continue;
                }

                diagnostics.push(
                    Diagnostic::new(
                        self.id(),
                        self.default_severity(),
                        format!("technical noun `{}` appears in verb position", entry.word),
                        pair[1].range,
                    )
                    .with_note("ASD-STE100 Rule 1.7")
                    .with_help("rewrite with an approved verb"),
                );
            }
        }

        diagnostics
    }
}

impl Rule for CompanyApprovedTechnicalNounRule {
    fn id(&self) -> &str {
        "asd-ste100/company-approved-technical-noun"
    }

    fn name(&self) -> &str {
        "Use company-approved technical nouns"
    }

    fn profiles(&self) -> &[&str] {
        &["asd-ste100"]
    }

    fn default_severity(&self) -> Severity {
        Severity::Info
    }

    fn check(&self, input: &RuleInput, ctx: &CheckContext) -> Vec<Diagnostic> {
        let Some(glossary) = ctx.glossary.as_ref() else {
            return Vec::new();
        };

        let mut diagnostics = Vec::new();
        for token in input
            .sentences
            .iter()
            .flat_map(|sentence| sentence.tokens.iter())
        {
            if !looks_technical_noun_candidate(&token.text) {
                continue;
            }

            let lower = token.text.to_ascii_lowercase();
            if glossary.has_term(&lower) {
                continue;
            }

            if lower == "warning" || lower == "caution" || lower == "note" {
                continue;
            }

            diagnostics.push(
                Diagnostic::new(
                    self.id(),
                    self.default_severity(),
                    format!(
                        "`{}` looks like a technical term not present in company glossary",
                        token.text
                    ),
                    token.range,
                )
                .with_note("ASD-STE100 Rule 1.8"),
            );
        }

        diagnostics
    }
}

impl Rule for TechnicalNounLengthRule {
    fn id(&self) -> &str {
        "asd-ste100/technical-noun-length"
    }

    fn name(&self) -> &str {
        "Technical nouns should be short"
    }

    fn profiles(&self) -> &[&str] {
        &["asd-ste100"]
    }

    fn default_severity(&self) -> Severity {
        Severity::Warning
    }

    fn check(&self, input: &RuleInput, ctx: &CheckContext) -> Vec<Diagnostic> {
        let Some(glossary) = ctx.glossary.as_ref() else {
            return Vec::new();
        };

        let lower = input.span.text.to_ascii_lowercase();
        let mut diagnostics = Vec::new();

        let mut all_words = Vec::new();
        all_words.extend(
            glossary
                .approved()
                .iter()
                .map(|e| e.word.to_ascii_lowercase()),
        );
        all_words.extend(
            glossary
                .not_approved()
                .iter()
                .map(|e| e.word.to_ascii_lowercase()),
        );

        for candidate in all_words {
            let words = candidate.split_whitespace().count();
            if words <= 3 {
                continue;
            }

            let mut matches = find_token_boundary_matches(&lower, &candidate);
            if matches.is_empty() {
                // Multi-word technical terms can be tokenized with punctuation or
                // formatting around them; fallback to substring matching so the
                // glossary-driven phrase still gets flagged.
                if let Some(start) = lower.find(&candidate) {
                    matches.push((start, start + candidate.len()));
                }
            }

            for (start, end) in matches {
                diagnostics.push(
                    Diagnostic::new(
                        self.id(),
                        self.default_severity(),
                        format!("technical noun phrase has {words} words"),
                        absolute_range(input, start, end),
                    )
                    .with_note("ASD-STE100 Rule 1.9")
                    .with_help("reduce to three words or fewer"),
                );
            }
        }

        if diagnostics.is_empty() {
            static LONG_TECHNICAL_PHRASE_RE: std::sync::LazyLock<regex::Regex> =
                std::sync::LazyLock::new(|| {
                    regex::Regex::new(
                        r"(?i)\b[a-z]+(?:\s+[a-z]+){3,}\s+(valve|module|assembly|bracket|connector)\b",
                    )
                    .unwrap_or_else(|error| panic!("valid technical noun length regex: {error}"))
                });

            for found in LONG_TECHNICAL_PHRASE_RE.find_iter(&input.span.text) {
                let phrase = &input.span.text[found.start()..found.end()];
                let words = phrase.split_whitespace().count();
                diagnostics.push(
                    Diagnostic::new(
                        self.id(),
                        self.default_severity(),
                        format!("technical noun phrase has {words} words"),
                        absolute_range(input, found.start(), found.end()),
                    )
                    .with_note("ASD-STE100 Rule 1.9")
                    .with_help("reduce to three words or fewer"),
                );
            }
        }

        diagnostics
    }
}

impl Rule for NoJargonTechnicalNounRule {
    fn id(&self) -> &str {
        "asd-ste100/no-jargon-technical-noun"
    }

    fn name(&self) -> &str {
        "Avoid jargon or slang terms"
    }

    fn profiles(&self) -> &[&str] {
        &["asd-ste100"]
    }

    fn default_severity(&self) -> Severity {
        Severity::Warning
    }

    fn check(&self, input: &RuleInput, _ctx: &CheckContext) -> Vec<Diagnostic> {
        const JARGON: &[&str] = &[
            "obviously",
            "basically",
            "thingy",
            "stuff",
            "gonna",
            "wanna",
        ];

        input
            .sentences
            .iter()
            .flat_map(|sentence| sentence.tokens.iter())
            .filter(|token| JARGON.contains(&token.text.to_ascii_lowercase().as_str()))
            .map(|token| {
                Diagnostic::new(
                    self.id(),
                    self.default_severity(),
                    format!("avoid jargon term `{}`", token.text),
                    token.range,
                )
                .with_note("ASD-STE100 Rule 1.10")
            })
            .collect()
    }
}

impl Rule for ConsistentTechnicalNounRule {
    fn id(&self) -> &str {
        "asd-ste100/consistent-technical-noun"
    }

    fn name(&self) -> &str {
        "Use one term for one item"
    }

    fn profiles(&self) -> &[&str] {
        &["asd-ste100"]
    }

    fn default_severity(&self) -> Severity {
        Severity::Warning
    }

    fn check(&self, input: &RuleInput, ctx: &CheckContext) -> Vec<Diagnostic> {
        let Some(glossary) = ctx.glossary.as_ref() else {
            return Vec::new();
        };

        let lower = input.span.text.to_ascii_lowercase();
        let mut diagnostics = Vec::new();
        for entry in glossary.not_approved() {
            let synonym_lower = entry.word.to_ascii_lowercase();
            let synonym_matches = find_token_boundary_matches(&lower, &synonym_lower);
            if synonym_matches.is_empty() {
                continue;
            }

            for alt in &entry.alternatives {
                let canonical = alt.word.to_ascii_lowercase();
                if !find_token_boundary_matches(&lower, &canonical).is_empty() {
                    for (start, end) in &synonym_matches {
                        diagnostics.push(
                            Diagnostic::new(
                                self.id(),
                                self.default_severity(),
                                format!(
                                    "both `{}` and synonym `{}` appear in same context",
                                    alt.word, entry.word
                                ),
                                absolute_range(input, *start, *end),
                            )
                            .with_note("ASD-STE100 Rule 1.11")
                            .with_help(format!("prefer `{}` consistently", alt.word)),
                        );
                    }
                }
            }
        }

        diagnostics
    }
}

impl Rule for TechnicalVerbCategoryRule {
    fn id(&self) -> &str {
        "asd-ste100/technical-verb-category"
    }

    fn name(&self) -> &str {
        "Technical verbs should be categorized"
    }

    fn profiles(&self) -> &[&str] {
        &["asd-ste100"]
    }

    fn default_severity(&self) -> Severity {
        Severity::Info
    }

    fn check(&self, input: &RuleInput, _ctx: &CheckContext) -> Vec<Diagnostic> {
        let mut diagnostics = Vec::new();

        for sentence in &input.sentences {
            for pair in sentence.tokens.windows(2) {
                let prev = pair[0].text.to_ascii_lowercase();
                let word = pair[1].text.to_ascii_lowercase();
                if !matches!(prev.as_str(), "shall" | "must" | "can" | "may") {
                    continue;
                }
                if !word.ends_with("ate") {
                    continue;
                }
                diagnostics.push(
                    Diagnostic::new(
                        self.id(),
                        self.default_severity(),
                        format!("technical verb `{}` has no category", pair[1].text),
                        pair[1].range,
                    )
                    .with_note("ASD-STE100 Rule 1.12"),
                );
                break;
            }
            if !diagnostics.is_empty() {
                break;
            }
        }

        diagnostics
    }
}

impl Rule for TechnicalVerbAsNounRule {
    fn id(&self) -> &str {
        "asd-ste100/technical-verb-as-noun"
    }

    fn name(&self) -> &str {
        "Do not use technical verbs as nouns"
    }

    fn profiles(&self) -> &[&str] {
        &["asd-ste100"]
    }

    fn default_severity(&self) -> Severity {
        Severity::Warning
    }

    fn check(&self, input: &RuleInput, ctx: &CheckContext) -> Vec<Diagnostic> {
        let Some(glossary) = ctx.glossary.as_ref() else {
            return Vec::new();
        };

        let mut diagnostics = Vec::new();
        for sentence in &input.sentences {
            for pair in sentence.tokens.windows(2) {
                let prev = pair[0].text.to_ascii_lowercase();
                if !matches!(prev.as_str(), "the" | "a" | "an") {
                    continue;
                }

                let candidate = pair[1]
                    .text
                    .trim_matches(|ch: char| !ch.is_ascii_alphanumeric() && ch != '-')
                    .to_ascii_lowercase();
                let is_glossary_verb = glossary.approved().iter().any(|entry| {
                    entry.word.eq_ignore_ascii_case(&candidate)
                        && entry.pos.eq_ignore_ascii_case("verb")
                });
                let is_verb_like_fallback = candidate.ends_with("ate");

                if !(is_glossary_verb || is_verb_like_fallback) {
                    continue;
                }

                diagnostics.push(
                    Diagnostic::new(
                        self.id(),
                        self.default_severity(),
                        format!("technical verb `{}` appears in noun position", pair[1].text),
                        pair[1].range,
                    )
                    .with_note("ASD-STE100 Rule 1.13")
                    .with_help("use a noun form or rephrase"),
                );
            }
        }

        diagnostics
    }
}

impl Rule for AmericanSpellingRule {
    fn id(&self) -> &str {
        "asd-ste100/american-spelling"
    }

    fn name(&self) -> &str {
        "Use American English spelling"
    }

    fn profiles(&self) -> &[&str] {
        &["asd-ste100"]
    }

    fn default_severity(&self) -> Severity {
        Severity::Error
    }

    fn check(&self, input: &RuleInput, _ctx: &CheckContext) -> Vec<Diagnostic> {
        const BRITISH_TO_US: &[(&str, &str)] = &[
            ("colour", "color"),
            ("favour", "favor"),
            ("labour", "labor"),
            ("centre", "center"),
            ("metre", "meter"),
            ("analyse", "analyze"),
            ("organise", "organize"),
            ("behaviour", "behavior"),
            ("initialise", "initialize"),
        ];

        let lower = input.span.text.to_ascii_lowercase();
        let mut diagnostics = Vec::new();

        for (british, american) in BRITISH_TO_US {
            for (start, end) in find_token_boundary_matches(&lower, british) {
                let span = absolute_range(input, start, end);

                diagnostics.push(
                    Diagnostic::new(
                        self.id(),
                        self.default_severity(),
                        format!("`{british}` is not American spelling"),
                        span,
                    )
                    .with_note("ASD-STE100 Rule 1.14")
                    .with_help(format!("use `{american}`"))
                    .with_suggestion(Suggestion {
                        span,
                        replacement: (*american).to_string(),
                        applicability: Applicability::MachineApplicable,
                        message: "convert to American spelling".to_string(),
                    }),
                );
            }
        }

        diagnostics
    }
}

fn is_word_token(token: &str) -> bool {
    token
        .chars()
        .all(|ch| ch.is_ascii_alphanumeric() || ch == '-' || ch == '\'' || ch == '’')
}

fn looks_technical_noun_candidate(token: &str) -> bool {
    if token.len() < 3
        || !token
            .chars()
            .all(|ch| ch.is_ascii_alphanumeric() || ch == '-')
    {
        return false;
    }

    let all_caps_or_symbols = token
        .chars()
        .all(|ch| !ch.is_ascii_lowercase() && (ch.is_ascii_alphanumeric() || ch == '-'));
    let has_internal_caps = token.chars().skip(1).any(|ch| ch.is_ascii_uppercase());
    let has_digits = token.chars().any(|ch| ch.is_ascii_digit());

    all_caps_or_symbols || has_internal_caps || has_digits
}

fn heuristic_lemma(word: &str) -> Option<String> {
    if let Some(stem) = word.strip_suffix("ing") {
        return Some(stem.to_string());
    }
    if let Some(stem) = word.strip_suffix("ed") {
        return Some(stem.to_string());
    }
    if let Some(stem) = word.strip_suffix('s') {
        return Some(stem.to_string());
    }
    None
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

// Phrase-level glossary synonym matches are higher-confidence rewrites than
// token-level ASD replacements. When a token is inside an actionable glossary
// synonym span (for example, `flux drive` -> `FluxDrive`), suppress overlapping
// token diagnostics to avoid contradictory guidance.
fn covered_by_actionable_glossary_synonym(
    input: &RuleInput,
    ctx: &CheckContext,
    token_range: goodwrite_core::SourceRange,
) -> bool {
    let Some(glossary) = ctx.glossary.as_ref() else {
        return false;
    };

    if token_range.start < input.span.range.start || token_range.end > input.span.range.end {
        return false;
    }

    let local_start = token_range.start - input.span.range.start;
    let local_end = token_range.end - input.span.range.start;
    let lower = input.span.text.to_ascii_lowercase();

    for entry in glossary.not_approved() {
        // Mirror glossary/synonym-enforce behavior: only suppress when a
        // canonical replacement exists and is therefore actionable.
        let canonical = entry
            .alternatives
            .first()
            .map(|alt| alt.word.trim())
            .unwrap_or_default();
        if canonical.is_empty() {
            continue;
        }

        let synonym_lower = entry.word.to_ascii_lowercase();
        for (start, end) in find_token_boundary_matches(&lower, &synonym_lower) {
            if local_start >= start && local_end <= end {
                return true;
            }
        }
    }

    false
}

fn find_token_boundary_matches(haystack: &str, needle: &str) -> Vec<(usize, usize)> {
    if needle.is_empty() {
        return Vec::new();
    }

    let mut matches = Vec::new();
    let mut from = 0usize;
    while let Some(idx) = haystack[from..].find(needle) {
        let start = from + idx;
        let end = start + needle.len();

        let before_ok = if start == 0 {
            true
        } else {
            !haystack.as_bytes()[start - 1].is_ascii_alphanumeric()
        };
        let after_ok = if end >= haystack.len() {
            true
        } else {
            !haystack.as_bytes()[end].is_ascii_alphanumeric()
        };

        if before_ok && after_ok {
            matches.push((start, end));
        }

        from = end;
    }

    matches
}

#[cfg(test)]
mod tests {
    use super::{TechnicalNounLengthRule, TechnicalVerbAsNounRule, TechnicalVerbCategoryRule};
    use goodwrite_core::{
        CheckContext, GlossaryData, GoodwriteConfig, ProseSpan, Rule, RuleInput, SourceRange,
        SpanAnnotations,
    };

    fn context_with_glossary() -> CheckContext {
        CheckContext {
            config: GoodwriteConfig::default(),
            glossary: Some(GlossaryData::new(
                vec![
                    goodwrite_core::GlossaryApprovedEntry {
                        word: "calibrate".to_string(),
                        pos: "verb".to_string(),
                        forms: Vec::new(),
                        approved_meaning: String::new(),
                        goodwrite_example: String::new(),
                        wrongwrite_example: String::new(),
                    },
                    goodwrite_core::GlossaryApprovedEntry {
                        word: "control".to_string(),
                        pos: "verb".to_string(),
                        forms: Vec::new(),
                        approved_meaning: String::new(),
                        goodwrite_example: String::new(),
                        wrongwrite_example: String::new(),
                    },
                    goodwrite_core::GlossaryApprovedEntry {
                        word: "high pressure fuel shutoff valve".to_string(),
                        pos: "noun".to_string(),
                        forms: Vec::new(),
                        approved_meaning: String::new(),
                        goodwrite_example: String::new(),
                        wrongwrite_example: String::new(),
                    },
                ],
                Vec::new(),
            )),
            glossary_data: None,
            file_has_mode_annotations: true,
        }
    }

    fn input_for(text: &str) -> RuleInput {
        let span = ProseSpan::new(
            text,
            SourceRange::new(0, text.len()),
            SpanAnnotations {
                writing_mode: None,
                requirement: false,
                requirement_type: None,
            },
        );
        let sentences = goodwrite_tokenize::tokenize_span(&span);
        RuleInput {
            file_path: "test.md".to_string(),
            span,
            sentences,
        }
    }

    #[test]
    fn technical_verb_category_rule_flags_uncategorized_verb() {
        let rule = TechnicalVerbCategoryRule;
        let input = input_for("The operator shall calibrate the valve.");
        let diagnostics = rule.check(&input, &context_with_glossary());
        assert!(!diagnostics.is_empty());
    }

    #[test]
    fn technical_verb_as_noun_rule_flags_article_usage() {
        let rule = TechnicalVerbAsNounRule;
        let input = input_for("The control is active.");
        let diagnostics = rule.check(&input, &context_with_glossary());
        assert!(!diagnostics.is_empty());
    }

    #[test]
    fn technical_noun_length_rule_flags_long_phrase() {
        let rule = TechnicalNounLengthRule;
        let input = input_for("The high pressure fuel shutoff valve leaks.");
        let diagnostics = rule.check(&input, &context_with_glossary());
        assert!(!diagnostics.is_empty());
    }
}
