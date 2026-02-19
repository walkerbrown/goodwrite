use regex::Regex;

use goodwrite_core::{CheckContext, Diagnostic, Rule, RuleInput, Severity};
use goodwrite_tokenize::{PosClass, PosResolution, TokenAnalysis};

use crate::compliance::analyze_sentence_tokens;

pub struct MultiWordNounLengthRule;
pub struct AbbreviationFirstUseRule;

impl Rule for MultiWordNounLengthRule {
    fn id(&self) -> &str {
        "asd-ste100/multi-word-noun-length"
    }

    fn name(&self) -> &str {
        "Multi-word noun maximum length"
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
            let mut run_start = 0usize;
            let mut run_len = 0usize;

            for (idx, analysis) in analyses.iter().enumerate() {
                if is_noun_phrase_part(analysis) {
                    if run_len == 0 {
                        run_start = idx;
                    }
                    run_len += 1;
                    continue;
                }

                if run_len > 3 {
                    let start = analyses[run_start].token.range.start;
                    let end = analyses[idx - 1].token.range.end;
                    diagnostics.push(
                        Diagnostic::new(
                            self.id(),
                            self.default_severity(),
                            format!("noun phrase has {run_len} words; maximum is 3"),
                            goodwrite_core::SourceRange::new(start, end),
                        )
                        .with_note("ASD-STE100 Rule 2.1")
                        .with_help("shorten or split the noun phrase"),
                    );
                }

                run_len = 0;
            }

            if run_len > 3 {
                let start = analyses[run_start].token.range.start;
                let end = analyses[run_start + run_len - 1].token.range.end;
                diagnostics.push(
                    Diagnostic::new(
                        self.id(),
                        self.default_severity(),
                        format!("noun phrase has {run_len} words; maximum is 3"),
                        goodwrite_core::SourceRange::new(start, end),
                    )
                    .with_note("ASD-STE100 Rule 2.1")
                    .with_help("shorten or split the noun phrase"),
                );
            }
        }

        diagnostics
    }
}

impl Rule for AbbreviationFirstUseRule {
    fn id(&self) -> &str {
        "asd-ste100/abbreviation-first-use"
    }

    fn name(&self) -> &str {
        "Define abbreviation at first use"
    }

    fn profiles(&self) -> &[&str] {
        &["asd-ste100"]
    }

    fn default_severity(&self) -> Severity {
        Severity::Warning
    }

    fn check(&self, input: &RuleInput, _ctx: &CheckContext) -> Vec<Diagnostic> {
        static DEFINITION_RE: std::sync::LazyLock<Regex> = std::sync::LazyLock::new(|| {
            Regex::new(r"\b[A-Za-z][A-Za-z\s]{3,}\(([A-Z]{2,})\)")
                .unwrap_or_else(|error| panic!("valid abbreviation regex: {error}"))
        });

        let mut defined = std::collections::HashSet::new();
        for caps in DEFINITION_RE.captures_iter(&input.span.text) {
            if let Some(abbr) = caps.get(1) {
                defined.insert(abbr.as_str().to_string());
            }
        }

        let mut seen = std::collections::HashSet::new();
        let mut diagnostics = Vec::new();

        for token in input
            .sentences
            .iter()
            .flat_map(|sentence| sentence.tokens.iter())
        {
            if !looks_like_abbreviation(&token.text) {
                continue;
            }

            if matches!(token.text.as_str(), "WARNING" | "CAUTION" | "NOTE") {
                continue;
            }

            if !seen.insert(token.text.clone()) {
                continue;
            }

            if defined.contains(&token.text) {
                continue;
            }

            diagnostics.push(
                Diagnostic::new(
                    self.id(),
                    self.default_severity(),
                    format!("abbreviation `{}` is used before definition", token.text),
                    token.range,
                )
                .with_note("ASD-STE100 Rule 2.2")
                .with_help(format!(
                    "introduce full term first, then `{}` in parentheses",
                    token.text
                )),
            );
        }

        diagnostics
    }
}

fn is_noun_phrase_part(analysis: &TokenAnalysis) -> bool {
    match &analysis.resolution {
        PosResolution::Resolved(candidate) => {
            matches!(
                candidate.pos,
                PosClass::Noun | PosClass::Adjective | PosClass::Participle | PosClass::Number
            )
        }
        PosResolution::Ambiguous(_) | PosResolution::Unresolved => false,
    }
}

fn looks_like_abbreviation(token: &str) -> bool {
    token.len() >= 2
        && token
            .chars()
            .all(|ch| ch.is_ascii_uppercase() || ch.is_ascii_digit())
}
