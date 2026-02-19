use std::fmt;
use std::sync::Arc;

use goodwrite_core::{Applicability, Diagnostic as CoreDiagnostic, Severity};
use miette::{Diagnostic, LabeledSpan, NamedSource, SourceCode};

/// Shared source text for a single file, so multiple diagnostics
/// referencing the same file don't each allocate their own copy.
#[derive(Clone)]
pub struct SharedSource(Arc<NamedSource<String>>);

impl SharedSource {
    pub fn new(name: impl AsRef<str>, source: String) -> Self {
        Self(Arc::new(NamedSource::new(name, source)))
    }
}

impl SourceCode for SharedSource {
    fn read_span<'a>(
        &'a self,
        span: &miette::SourceSpan,
        context_lines_before: usize,
        context_lines_after: usize,
    ) -> Result<Box<dyn miette::SpanContents<'a> + 'a>, miette::MietteError> {
        self.0
            .read_span(span, context_lines_before, context_lines_after)
    }
}

/// Wraps a core `Diagnostic` with source text so it can be rendered by miette.
pub struct RenderDiagnostic {
    message: String,
    rule_id: String,
    severity: miette::Severity,
    help_text: Option<String>,
    label: LabeledSpan,
    source: SharedSource,
}

impl RenderDiagnostic {
    pub fn new(diagnostic: &CoreDiagnostic, source: SharedSource) -> Self {
        let severity = match diagnostic.severity {
            Severity::Error => miette::Severity::Error,
            Severity::Warning => miette::Severity::Warning,
            Severity::Info => miette::Severity::Advice,
        };

        let label = LabeledSpan::at(
            diagnostic.span.start..diagnostic.span.end,
            &diagnostic.rule_id,
        );

        let mut help_parts = Vec::new();
        if let Some(help) = &diagnostic.help {
            help_parts.push(help.clone());
        }
        if let Some(note) = &diagnostic.note {
            help_parts.push(format!("note: {note}"));
        }
        for suggestion in &diagnostic.suggestions {
            let applicability = match suggestion.applicability {
                Applicability::MachineApplicable => "machine-applicable",
                Applicability::MaybeIncorrect => "maybe-incorrect",
                Applicability::HasPlaceholders => "has-placeholders",
            };
            help_parts.push(format!(
                "suggestion: {} ({applicability})",
                suggestion.message,
            ));
        }

        let help_text = if help_parts.is_empty() {
            None
        } else {
            Some(help_parts.join("\n"))
        };

        Self {
            message: diagnostic.message.clone(),
            rule_id: diagnostic.rule_id.clone(),
            severity,
            help_text,
            label,
            source,
        }
    }
}

impl fmt::Display for RenderDiagnostic {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.message)
    }
}

impl fmt::Debug for RenderDiagnostic {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Display::fmt(self, f)
    }
}

impl std::error::Error for RenderDiagnostic {}

impl Diagnostic for RenderDiagnostic {
    fn code<'a>(&'a self) -> Option<Box<dyn fmt::Display + 'a>> {
        Some(Box::new(&self.rule_id))
    }

    fn severity(&self) -> Option<miette::Severity> {
        Some(self.severity)
    }

    fn help<'a>(&'a self) -> Option<Box<dyn fmt::Display + 'a>> {
        self.help_text
            .as_ref()
            .map(|h| Box::new(h) as Box<dyn fmt::Display>)
    }

    fn labels(&self) -> Option<Box<dyn Iterator<Item = LabeledSpan> + '_>> {
        Some(Box::new(std::iter::once(self.label.clone())))
    }

    fn source_code(&self) -> Option<&dyn SourceCode> {
        Some(&self.source)
    }
}
