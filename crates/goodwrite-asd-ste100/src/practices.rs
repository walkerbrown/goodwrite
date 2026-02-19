use goodwrite_core::{
    Applicability, CheckContext, Diagnostic, Rule, RuleInput, Severity, Suggestion,
};

pub struct ThatWhichRule;
pub struct AmbiguousWithRule;
pub struct PronounAntecedentRule;
pub struct ThisReferentRule;
pub struct FalseFriendsRule;
pub struct LatinAbbreviationRule;
pub struct InclusiveLanguageRule;
pub struct PossessiveFormRule;

impl Rule for ThatWhichRule {
    fn id(&self) -> &str {
        "asd-ste100/gr-that-which"
    }

    fn name(&self) -> &str {
        "GR-1 that vs which"
    }

    fn profiles(&self) -> &[&str] {
        &["asd-ste100"]
    }

    fn default_severity(&self) -> Severity {
        Severity::Warning
    }

    fn check(&self, input: &RuleInput, _ctx: &CheckContext) -> Vec<Diagnostic> {
        let lower = input.span.text.to_ascii_lowercase();
        let mut diagnostics = Vec::new();

        let mut from = 0usize;
        while let Some(idx) = lower[from..].find(" which ") {
            let start = from + idx;
            let before = &lower[..start];
            let has_comma = before.ends_with(',');
            if !has_comma {
                let span = absolute_range(input, start + 1, start + 6);
                diagnostics.push(
                    Diagnostic::new(
                        self.id(),
                        self.default_severity(),
                        "consider `that` for defining clause",
                        span,
                    )
                    .with_note("ASD-STE100 GR-1")
                    .with_suggestion(Suggestion {
                        span,
                        replacement: "that".to_string(),
                        applicability: Applicability::MaybeIncorrect,
                        message: "replace `which` with `that`".to_string(),
                    }),
                );
            }
            from = start + 7;
        }

        diagnostics
    }
}

impl Rule for AmbiguousWithRule {
    fn id(&self) -> &str {
        "asd-ste100/gr-ambiguous-with"
    }

    fn name(&self) -> &str {
        "GR-2 ambiguous with"
    }

    fn profiles(&self) -> &[&str] {
        &["asd-ste100"]
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
                if !(lower.contains(" with ") && (lower.contains(" and ") || lower.contains(','))) {
                    return None;
                }

                Some(
                    Diagnostic::new(
                        self.id(),
                        self.default_severity(),
                        "`with` clause may be ambiguously attached",
                        sentence.range,
                    )
                    .with_note("ASD-STE100 GR-2")
                    .with_help("rewrite to make attachment explicit"),
                )
            })
            .collect()
    }
}

impl Rule for PronounAntecedentRule {
    fn id(&self) -> &str {
        "asd-ste100/gr-pronoun-antecedent"
    }

    fn name(&self) -> &str {
        "GR-3 pronoun antecedent"
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
            for (idx, token) in sentence.tokens.iter().enumerate() {
                let lower = token.text.to_ascii_lowercase();
                if !matches!(lower.as_str(), "it" | "they" | "them" | "this" | "that") {
                    continue;
                }

                let noun_candidates = sentence.tokens[..idx]
                    .iter()
                    .filter(|tok| {
                        tok.text.chars().all(|ch| ch.is_ascii_alphabetic()) && tok.text.len() > 2
                    })
                    .count();
                if noun_candidates < 2 {
                    continue;
                }

                diagnostics.push(
                    Diagnostic::new(
                        self.id(),
                        self.default_severity(),
                        format!("pronoun `{}` may have unclear antecedent", token.text),
                        token.range,
                    )
                    .with_note("ASD-STE100 GR-3")
                    .with_help("repeat the noun for clarity"),
                );
            }
        }

        diagnostics
    }
}

impl Rule for ThisReferentRule {
    fn id(&self) -> &str {
        "asd-ste100/gr-this-referent"
    }

    fn name(&self) -> &str {
        "GR-4 this referent"
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
            for pair in sentence.tokens.windows(2) {
                if !pair[0].text.eq_ignore_ascii_case("this") {
                    continue;
                }

                let next = &pair[1].text;
                if next.chars().all(|ch| ch.is_ascii_alphabetic()) && next.len() > 2 {
                    continue;
                }

                diagnostics.push(
                    Diagnostic::new(
                        self.id(),
                        self.default_severity(),
                        "`this` should be followed by a noun",
                        pair[0].range,
                    )
                    .with_note("ASD-STE100 GR-4")
                    .with_help("replace `this` with `this <noun>`"),
                );
            }
        }

        diagnostics
    }
}

impl Rule for FalseFriendsRule {
    fn id(&self) -> &str {
        "asd-ste100/gr-false-friends"
    }

    fn name(&self) -> &str {
        "GR-5 false friends"
    }

    fn profiles(&self) -> &[&str] {
        &["asd-ste100"]
    }

    fn default_severity(&self) -> Severity {
        Severity::Info
    }

    fn check(&self, input: &RuleInput, _ctx: &CheckContext) -> Vec<Diagnostic> {
        const WORDS: &[&str] = &["actual", "eventual", "sensible", "assist", "fabric"];

        input
            .sentences
            .iter()
            .flat_map(|sentence| sentence.tokens.iter())
            .filter(|token| WORDS.contains(&token.text.to_ascii_lowercase().as_str()))
            .map(|token| {
                Diagnostic::new(
                    self.id(),
                    self.default_severity(),
                    format!(
                        "`{}` can be a false friend for non-native readers",
                        token.text
                    ),
                    token.range,
                )
                .with_note("ASD-STE100 GR-5")
            })
            .collect()
    }
}

impl Rule for LatinAbbreviationRule {
    fn id(&self) -> &str {
        "asd-ste100/gr-latin-abbrev"
    }

    fn name(&self) -> &str {
        "GR-6 avoid Latin abbreviations"
    }

    fn profiles(&self) -> &[&str] {
        &["asd-ste100"]
    }

    fn default_severity(&self) -> Severity {
        Severity::Warning
    }

    fn check(&self, input: &RuleInput, _ctx: &CheckContext) -> Vec<Diagnostic> {
        const LATIN: &[(&str, &str)] = &[
            ("i.e.", "that is"),
            ("e.g.", "for example"),
            ("etc.", "and so on"),
            ("cf.", "compare"),
        ];

        let lower = input.span.text.to_ascii_lowercase();
        let mut diagnostics = Vec::new();

        for (latin, replacement) in LATIN {
            for (start, end) in find_token_boundary_matches(&lower, latin) {
                let span = absolute_range(input, start, end);

                diagnostics.push(
                    Diagnostic::new(
                        self.id(),
                        self.default_severity(),
                        format!("avoid Latin abbreviation `{latin}`"),
                        span,
                    )
                    .with_note("ASD-STE100 GR-6")
                    .with_help(format!("use `{replacement}`"))
                    .with_suggestion(Suggestion {
                        span,
                        replacement: (*replacement).to_string(),
                        applicability: Applicability::MachineApplicable,
                        message: "replace Latin abbreviation".to_string(),
                    }),
                );
            }
        }

        diagnostics
    }
}

impl Rule for InclusiveLanguageRule {
    fn id(&self) -> &str {
        "asd-ste100/gr-inclusive-language"
    }

    fn name(&self) -> &str {
        "GR-7 inclusive language"
    }

    fn profiles(&self) -> &[&str] {
        &["asd-ste100"]
    }

    fn default_severity(&self) -> Severity {
        Severity::Warning
    }

    fn check(&self, input: &RuleInput, _ctx: &CheckContext) -> Vec<Diagnostic> {
        const NON_INCLUSIVE: &[(&str, &str)] = &[
            ("he", "they"),
            ("she", "they"),
            ("manpower", "workforce"),
            ("chairman", "chair"),
        ];

        let lower = input.span.text.to_ascii_lowercase();
        let mut diagnostics = Vec::new();

        for (word, replacement) in NON_INCLUSIVE {
            for (start, end) in find_token_boundary_matches(&lower, word) {
                let span = absolute_range(input, start, end);

                diagnostics.push(
                    Diagnostic::new(
                        self.id(),
                        self.default_severity(),
                        format!("consider inclusive alternative to `{word}`"),
                        span,
                    )
                    .with_note("ASD-STE100 GR-7")
                    .with_help(format!("consider `{replacement}`"))
                    .with_suggestion(Suggestion {
                        span,
                        replacement: (*replacement).to_string(),
                        applicability: Applicability::MaybeIncorrect,
                        message: "replace with inclusive language".to_string(),
                    }),
                );
            }
        }

        diagnostics
    }
}

impl Rule for PossessiveFormRule {
    fn id(&self) -> &str {
        "asd-ste100/gr-possessive-form"
    }

    fn name(&self) -> &str {
        "GR-8 possessive form"
    }

    fn profiles(&self) -> &[&str] {
        &["asd-ste100"]
    }

    fn default_severity(&self) -> Severity {
        Severity::Info
    }

    fn check(&self, input: &RuleInput, _ctx: &CheckContext) -> Vec<Diagnostic> {
        let mut diagnostics = Vec::new();
        // Normalize apostrophes so both `'s` and `’s` are detected.
        let lower = input.span.text.to_ascii_lowercase().replace('’', "'");
        let mut from = 0usize;
        while let Some(idx) = lower[from..].find("'s") {
            let start = from + idx;
            let end = start + 2;
            diagnostics.push(
                Diagnostic::new(
                    self.id(),
                    self.default_severity(),
                    "consider whether possessive form is necessary",
                    absolute_range(input, start, end),
                )
                .with_note("ASD-STE100 GR-8"),
            );
            from = end;
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

#[cfg(test)]
mod tests {
    use super::find_token_boundary_matches;

    #[test]
    fn boundary_match_does_not_hit_substring_inside_word() {
        let matches = find_token_boundary_matches("the system is ready", "he");
        assert!(matches.is_empty());
    }

    #[test]
    fn boundary_match_finds_standalone_token() {
        let matches = find_token_boundary_matches("he is ready", "he");
        assert_eq!(matches, vec![(0, 2)]);
    }
}
