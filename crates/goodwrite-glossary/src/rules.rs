use goodwrite_core::{
    Applicability, CheckContext, Diagnostic, Rule, RuleInput, Severity, Suggestion,
};

pub struct UndefinedTermRule;
pub struct SynonymRule;
pub struct CasingRule;

impl Rule for UndefinedTermRule {
    fn id(&self) -> &str {
        "glossary/undefined-term"
    }

    fn name(&self) -> &str {
        "Undefined technical term"
    }

    fn profiles(&self) -> &[&str] {
        &["glossary"]
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
            .span
            .text
            .split(|ch: char| !ch.is_ascii_alphanumeric() && ch != '-')
            .filter(|token| token.len() >= 3)
        {
            if !token
                .chars()
                .all(|ch| ch.is_ascii_uppercase() || ch.is_ascii_digit() || ch == '-')
            {
                continue;
            }

            let lower = token.to_ascii_lowercase();
            if matches!(lower.as_str(), "warning" | "caution" | "note" | "req") {
                continue;
            }

            if glossary.has_term(&lower) {
                continue;
            }

            if let Some(start) = input.span.text.find(token) {
                diagnostics.push(
                    Diagnostic::new(
                        self.id(),
                        self.default_severity(),
                        format!("`{token}` is not defined in glossary"),
                        goodwrite_core::SourceRange::new(
                            input.span.range.start + start,
                            input.span.range.start + start + token.len(),
                        ),
                    )
                    .with_help(
                        "add the term to glossary.toml [[approved]] or use a defined canonical term",
                    ),
                );
            }
        }

        diagnostics
    }
}

impl Rule for SynonymRule {
    fn id(&self) -> &str {
        "glossary/synonym-enforce"
    }

    fn name(&self) -> &str {
        "Use canonical glossary terms"
    }

    fn profiles(&self) -> &[&str] {
        &["glossary"]
    }

    fn default_severity(&self) -> Severity {
        Severity::Warning
    }

    fn check(&self, input: &RuleInput, ctx: &CheckContext) -> Vec<Diagnostic> {
        let Some(glossary) = ctx.glossary.as_ref() else {
            return Vec::new();
        };

        let lower_text = input.span.text.to_ascii_lowercase();
        let mut diagnostics = Vec::new();

        for entry in glossary.not_approved() {
            let synonym_lower = entry.word.to_ascii_lowercase();
            for start in find_all_token_boundary(&lower_text, &synonym_lower) {
                let end = start + synonym_lower.len();
                let absolute = goodwrite_core::SourceRange::new(
                    input.span.range.start + start,
                    input.span.range.start + end,
                );

                let canonical = entry
                    .alternatives
                    .first()
                    .map(|alt| alt.word.clone())
                    .unwrap_or_else(String::new);

                if canonical.is_empty() {
                    continue;
                }

                diagnostics.push(
                    Diagnostic::new(
                        self.id(),
                        self.default_severity(),
                        format!(
                            "`{}` is a glossary synonym; prefer canonical term `{}`",
                            entry.word, canonical
                        ),
                        absolute,
                    )
                    .with_help(format!("replace with `{}`", canonical))
                    .with_suggestion(Suggestion {
                        span: absolute,
                        replacement: canonical.clone(),
                        applicability: Applicability::MachineApplicable,
                        message: "use canonical glossary term".to_string(),
                    }),
                );
            }
        }

        diagnostics
    }
}

impl Rule for CasingRule {
    fn id(&self) -> &str {
        "glossary/casing"
    }

    fn name(&self) -> &str {
        "Canonical glossary casing"
    }

    fn profiles(&self) -> &[&str] {
        &["glossary"]
    }

    fn default_severity(&self) -> Severity {
        Severity::Info
    }

    fn check(&self, input: &RuleInput, ctx: &CheckContext) -> Vec<Diagnostic> {
        let Some(glossary) = ctx.glossary.as_ref() else {
            return Vec::new();
        };

        let mut diagnostics = Vec::new();
        for entry in glossary.approved() {
            let canonical_lower = entry.word.to_ascii_lowercase();
            let text_lower = input.span.text.to_ascii_lowercase();

            for start in find_all_token_boundary(&text_lower, &canonical_lower) {
                let end = start + canonical_lower.len();
                let found = &input.span.text[start..end];
                if found == entry.word {
                    continue;
                }

                let absolute = goodwrite_core::SourceRange::new(
                    input.span.range.start + start,
                    input.span.range.start + end,
                );

                diagnostics.push(
                    Diagnostic::new(
                        self.id(),
                        self.default_severity(),
                        format!("use canonical casing `{}`", entry.word),
                        absolute,
                    )
                    .with_suggestion(Suggestion {
                        span: absolute,
                        replacement: entry.word.clone(),
                        applicability: Applicability::MachineApplicable,
                        message: "normalize glossary casing".to_string(),
                    }),
                );
            }
        }

        diagnostics
    }
}

fn find_all(haystack: &str, needle: &str) -> Vec<usize> {
    if needle.is_empty() {
        return Vec::new();
    }

    let mut out = Vec::new();
    let mut from = 0usize;

    while let Some(idx) = haystack[from..].find(needle) {
        let absolute = from + idx;
        out.push(absolute);
        from = absolute + needle.len();
    }

    out
}

fn find_all_token_boundary(haystack: &str, needle: &str) -> Vec<usize> {
    find_all(haystack, needle)
        .into_iter()
        .filter(|start| {
            let end = *start + needle.len();
            let before_ok = if *start == 0 {
                true
            } else {
                !haystack.as_bytes()[*start - 1].is_ascii_alphanumeric()
            };
            let after_ok = if end >= haystack.len() {
                true
            } else {
                !haystack.as_bytes()[end].is_ascii_alphanumeric()
            };
            before_ok && after_ok
        })
        .collect()
}
