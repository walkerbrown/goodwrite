use goodwrite_core::{Diagnostic as CoreDiagnostic, Severity};
use tower_lsp_server::ls_types::{Diagnostic, DiagnosticSeverity, NumberOrString, Position, Range};

/// Convert a core diagnostic to LSP diagnostic.
pub fn to_lsp_diagnostic(diagnostic: &CoreDiagnostic, source: &str) -> Diagnostic {
    let range = range_from_offsets(source, diagnostic.span.start, diagnostic.span.end);
    let mut message = diagnostic.message.clone();
    if let Some(help) = &diagnostic.help {
        message.push_str("\nhelp: ");
        message.push_str(help);
    }
    if let Some(note) = &diagnostic.note {
        message.push_str("\nnote: ");
        message.push_str(note);
    }

    Diagnostic {
        range,
        severity: Some(match diagnostic.severity {
            Severity::Error => DiagnosticSeverity::ERROR,
            Severity::Warning => DiagnosticSeverity::WARNING,
            Severity::Info => DiagnosticSeverity::INFORMATION,
        }),
        code: Some(NumberOrString::String(diagnostic.rule_id.clone())),
        source: Some("goodwrite".to_string()),
        message,
        ..Diagnostic::default()
    }
}

/// Convert byte offsets to LSP range.
pub fn range_from_offsets(source: &str, start: usize, end: usize) -> Range {
    Range {
        start: offset_to_position(source, start),
        end: offset_to_position(source, end),
    }
}

/// Convert a byte offset into an LSP position.
pub fn offset_to_position(source: &str, offset: usize) -> Position {
    let safe = offset.min(source.len());
    let mut line = 0u32;
    let mut col = 0u32;

    for (idx, ch) in source.char_indices() {
        if idx >= safe {
            break;
        }
        if ch == '\n' {
            line += 1;
            col = 0;
        } else {
            col += 1;
        }
    }

    Position::new(line, col)
}

/// Convert LSP position to byte offset.
pub fn position_to_offset(source: &str, position: Position) -> usize {
    let mut line = 0u32;
    let mut col = 0u32;

    for (idx, ch) in source.char_indices() {
        if line == position.line && col == position.character {
            return idx;
        }

        if ch == '\n' {
            if line == position.line {
                return idx;
            }
            line += 1;
            col = 0;
        } else {
            col += 1;
        }
    }

    source.len()
}
