use goodwrite_core::{
    Applicability, CheckContext, Diagnostic, Rule, RuleInput, Severity, Suggestion,
};
use goodwrite_tokenize::{PosClass, PosResolution};

use crate::compliance::analyze_sentence_tokens;

pub struct ContractionRule;
pub struct UseVerticalListRule;
pub struct ConnectingWordsRule;
pub struct ArticlesBeforeNounsRule;

impl Rule for ContractionRule {
    fn id(&self) -> &str {
        "asd-ste100/contractions"
    }

    fn name(&self) -> &str {
        "Do not use contractions"
    }

    fn profiles(&self) -> &[&str] {
        &["asd-ste100"]
    }

    fn default_severity(&self) -> Severity {
        Severity::Error
    }

    fn check(&self, input: &RuleInput, _ctx: &CheckContext) -> Vec<Diagnostic> {
        const CONTRACTIONS: &[(&str, &str)] = &[
            ("can't", "cannot"),
            ("won't", "will not"),
            ("don't", "do not"),
            ("doesn't", "does not"),
            ("isn't", "is not"),
            ("aren't", "are not"),
            ("it's", "it is"),
            ("you're", "you are"),
            ("they're", "they are"),
            ("we're", "we are"),
            ("i've", "i have"),
            ("you've", "you have"),
            ("we've", "we have"),
            ("they've", "they have"),
            ("i'll", "i will"),
        ];

        // Markdown/Typst sources may use either ASCII apostrophes (`'`) or
        // typographic apostrophes (`’`). Normalize both before matching.
        let lower = input.span.text.to_ascii_lowercase().replace('’', "'");
        let mut diagnostics = Vec::new();

        for (contraction, expanded) in CONTRACTIONS {
            for (start, end) in find_token_boundary_matches(&lower, contraction) {
                let absolute = absolute_range(input, start, end);

                diagnostics.push(
                    Diagnostic::new(
                        self.id(),
                        self.default_severity(),
                        format!("contraction `{contraction}` is not permitted"),
                        absolute,
                    )
                    .with_note("ASD-STE100 Rule 4.2")
                    .with_help(format!("use `{expanded}`"))
                    .with_suggestion(Suggestion {
                        span: absolute,
                        replacement: (*expanded).to_string(),
                        applicability: Applicability::MachineApplicable,
                        message: "expand contraction".to_string(),
                    }),
                );
            }
        }

        diagnostics
    }
}

impl Rule for UseVerticalListRule {
    fn id(&self) -> &str {
        "asd-ste100/use-vertical-lists"
    }

    fn name(&self) -> &str {
        "Use vertical lists for complex text"
    }

    fn profiles(&self) -> &[&str] {
        &["asd-ste100"]
    }

    fn default_severity(&self) -> Severity {
        Severity::Info
    }

    fn check(&self, input: &RuleInput, _ctx: &CheckContext) -> Vec<Diagnostic> {
        input
            .sentences
            .iter()
            .filter_map(|sentence| {
                let comma_count = sentence.text.matches(',').count();
                if sentence.ste_word_count < 22 || comma_count < 2 {
                    return None;
                }

                Some(
                    Diagnostic::new(
                        self.id(),
                        self.default_severity(),
                        "long sentence with multiple clauses may be clearer as vertical list",
                        sentence.range,
                    )
                    .with_note("ASD-STE100 Rule 4.3")
                    .with_help("consider splitting into a vertical list"),
                )
            })
            .collect()
    }
}

impl Rule for ConnectingWordsRule {
    fn id(&self) -> &str {
        "asd-ste100/connecting-words"
    }

    fn name(&self) -> &str {
        "Use connecting words for related sentences"
    }

    fn profiles(&self) -> &[&str] {
        &["asd-ste100"]
    }

    fn default_severity(&self) -> Severity {
        Severity::Info
    }

    fn check(&self, input: &RuleInput, _ctx: &CheckContext) -> Vec<Diagnostic> {
        if input.sentences.len() < 2 {
            return Vec::new();
        }

        const CONNECTORS: &[&str] = &[
            "then",
            "therefore",
            "however",
            "next",
            "first",
            "second",
            "finally",
            "thus",
        ];

        let mut diagnostics = Vec::new();
        for sentence in input.sentences.iter().skip(1) {
            let first = sentence
                .text
                .split(|ch: char| !ch.is_ascii_alphabetic())
                .find(|part| !part.is_empty())
                .unwrap_or_default()
                .to_ascii_lowercase();

            if CONNECTORS.contains(&first.as_str()) {
                continue;
            }

            if matches!(first.as_str(), "it" | "this" | "that" | "they") {
                diagnostics.push(
                    Diagnostic::new(
                        self.id(),
                        self.default_severity(),
                        "sentence may need a connecting word for clarity",
                        sentence.range,
                    )
                    .with_note("ASD-STE100 Rule 4.4")
                    .with_help("consider adding a connector such as `then` or `therefore`"),
                );
            }
        }

        diagnostics
    }
}

impl Rule for ArticlesBeforeNounsRule {
    fn id(&self) -> &str {
        "asd-ste100/articles-before-nouns"
    }

    fn name(&self) -> &str {
        "Use articles before nouns"
    }

    fn profiles(&self) -> &[&str] {
        &["asd-ste100"]
    }

    fn default_severity(&self) -> Severity {
        Severity::Warning
    }

    fn check(&self, input: &RuleInput, ctx: &CheckContext) -> Vec<Diagnostic> {
        let mut diagnostics = Vec::new();
        let mode = input.span.annotations.effective_mode();

        for sentence in &input.sentences {
            let analyses = analyze_sentence_tokens(sentence, mode, ctx);
            for pair in analyses.windows(2) {
                let current = match &pair[1].resolution {
                    PosResolution::Resolved(candidate) => candidate,
                    PosResolution::Ambiguous(_) | PosResolution::Unresolved => continue,
                };

                if current.pos != PosClass::Noun {
                    continue;
                }

                let previous_pos = match &pair[0].resolution {
                    PosResolution::Resolved(candidate) => Some(candidate.pos),
                    PosResolution::Ambiguous(_) | PosResolution::Unresolved => None,
                };
                if matches!(previous_pos, Some(PosClass::Determiner)) {
                    continue;
                }

                let previous = pair[0].token.text.to_ascii_lowercase();
                if matches!(
                    previous.as_str(),
                    "of" | "to" | "with" | "in" | "on" | "from"
                ) {
                    continue;
                }

                diagnostics.push(
                    Diagnostic::new(
                        self.id(),
                        self.default_severity(),
                        format!("noun `{}` may need an article", pair[1].token.text),
                        pair[1].token.range,
                    )
                    .with_note("ASD-STE100 Rule 4.5")
                    .with_help("add `the`, `a`, or `an` if appropriate"),
                );
            }
        }

        diagnostics
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
