use regex::Regex;

use goodwrite_core::{
    Applicability, CheckContext, Diagnostic, Rule, RuleInput, Severity, Suggestion,
};

pub struct SemicolonRule;
pub struct HyphenRelatedWordsRule;
pub struct ParenthesesUsageRule;

impl Rule for SemicolonRule {
    fn id(&self) -> &str {
        "asd-ste100/semicolons"
    }

    fn name(&self) -> &str {
        "Do not use semicolons"
    }

    fn profiles(&self) -> &[&str] {
        &["asd-ste100"]
    }

    fn default_severity(&self) -> Severity {
        Severity::Error
    }

    fn check(&self, input: &RuleInput, _ctx: &CheckContext) -> Vec<Diagnostic> {
        let mut diagnostics = Vec::new();
        for (idx, ch) in input.span.text.char_indices() {
            if ch != ';' {
                continue;
            }

            let absolute = absolute_range(input, idx, idx + ch.len_utf8());

            diagnostics.push(
                Diagnostic::new(
                    self.id(),
                    self.default_severity(),
                    "semicolon is not allowed in ASD-STE100",
                    absolute,
                )
                .with_note("ASD-STE100 Rule 8.1")
                .with_help("replace with a period or split the sentence")
                .with_suggestion(Suggestion {
                    span: absolute,
                    replacement: ".".to_string(),
                    applicability: Applicability::MachineApplicable,
                    message: "replace semicolon with period".to_string(),
                }),
            );
        }

        diagnostics
    }
}

impl Rule for HyphenRelatedWordsRule {
    fn id(&self) -> &str {
        "asd-ste100/hyphen-related-words"
    }

    fn name(&self) -> &str {
        "Use hyphens for related words"
    }

    fn profiles(&self) -> &[&str] {
        &["asd-ste100"]
    }

    fn default_severity(&self) -> Severity {
        Severity::Info
    }

    fn check(&self, input: &RuleInput, _ctx: &CheckContext) -> Vec<Diagnostic> {
        const COMPOUNDS: &[(&str, &str)] = &[
            ("real time", "real-time"),
            ("full scale", "full-scale"),
            ("high level", "high-level"),
            ("low pressure", "low-pressure"),
        ];

        let lower = input.span.text.to_ascii_lowercase();
        let mut diagnostics = Vec::new();

        for (space_form, hyphen_form) in COMPOUNDS {
            let mut from = 0usize;
            while let Some(idx) = lower[from..].find(space_form) {
                let start = from + idx;
                let end = start + space_form.len();
                let span = absolute_range(input, start, end);

                diagnostics.push(
                    Diagnostic::new(
                        self.id(),
                        self.default_severity(),
                        format!("compound expression `{space_form}` may need hyphen"),
                        span,
                    )
                    .with_note("ASD-STE100 Rule 8.2")
                    .with_help(format!("consider `{hyphen_form}`"))
                    .with_suggestion(Suggestion {
                        span,
                        replacement: (*hyphen_form).to_string(),
                        applicability: Applicability::MaybeIncorrect,
                        message: "hyphenate compound modifier".to_string(),
                    }),
                );

                from = end;
            }
        }

        diagnostics
    }
}

impl Rule for ParenthesesUsageRule {
    fn id(&self) -> &str {
        "asd-ste100/parentheses-usage"
    }

    fn name(&self) -> &str {
        "Parentheses usage rules"
    }

    fn profiles(&self) -> &[&str] {
        &["asd-ste100"]
    }

    fn default_severity(&self) -> Severity {
        Severity::Info
    }

    fn check(&self, input: &RuleInput, _ctx: &CheckContext) -> Vec<Diagnostic> {
        static PAREN_RE: std::sync::LazyLock<Regex> = std::sync::LazyLock::new(|| {
            Regex::new(r"\([^)]*\)").unwrap_or_else(|error| panic!("valid paren regex: {error}"))
        });

        let open = input.span.text.matches('(').count();
        let close = input.span.text.matches(')').count();
        let mut diagnostics = Vec::new();

        if open != close {
            diagnostics.push(
                Diagnostic::new(
                    self.id(),
                    self.default_severity(),
                    "parentheses are unbalanced",
                    input.span.range,
                )
                .with_note("ASD-STE100 Rule 8.3")
                .with_help("balance opening and closing parentheses"),
            );
        }

        let groups = PAREN_RE.find_iter(&input.span.text).count();
        if groups > 1 {
            diagnostics.push(
                Diagnostic::new(
                    self.id(),
                    self.default_severity(),
                    format!("sentence contains {groups} parenthesized groups"),
                    input.span.range,
                )
                .with_note("ASD-STE100 Rule 8.3")
                .with_help("limit parentheses to essential clarifications"),
            );
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
