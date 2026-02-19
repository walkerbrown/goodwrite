use std::collections::HashMap;

use goodwrite_core::{Applicability, Diagnostic as CoreDiagnostic};
use tower_lsp_server::ls_types::{
    CodeAction, CodeActionKind, CodeActionOrCommand, Diagnostic, Range, TextEdit, Uri,
    WorkspaceEdit,
};

use crate::diagnostics::{position_to_offset, range_from_offsets};

/// Build quick fixes and unsafe-annotation actions for one core diagnostic.
pub fn actions_for_diagnostic(
    uri: &Uri,
    source: &str,
    extension: Option<&str>,
    source_diagnostic: &Diagnostic,
    core: &CoreDiagnostic,
) -> Vec<CodeActionOrCommand> {
    let mut actions = Vec::new();

    for suggestion in &core.suggestions {
        if suggestion.applicability != Applicability::MachineApplicable {
            continue;
        }

        let edit = TextEdit {
            range: range_from_offsets(source, suggestion.span.start, suggestion.span.end),
            new_text: suggestion.replacement.clone(),
        };

        let mut changes = HashMap::new();
        changes.insert(uri.clone(), vec![edit]);

        actions.push(CodeActionOrCommand::CodeAction(CodeAction {
            title: suggestion.message.clone(),
            kind: Some(CodeActionKind::QUICKFIX),
            diagnostics: Some(vec![source_diagnostic.clone()]),
            edit: Some(WorkspaceEdit {
                changes: Some(changes),
                ..WorkspaceEdit::default()
            }),
            is_preferred: Some(true),
            ..CodeAction::default()
        }));
    }

    let insertion = suppression_text(extension, &core.rule_id);
    let line_start = line_start_offset(source, core.span.start);
    let edit = TextEdit {
        range: range_from_offsets(source, line_start, line_start),
        new_text: insertion,
    };
    let mut changes = HashMap::new();
    changes.insert(uri.clone(), vec![edit]);

    actions.push(CodeActionOrCommand::CodeAction(CodeAction {
        title: format!("Add unsafe exemption for {}", core.rule_id),
        kind: Some(CodeActionKind::QUICKFIX),
        diagnostics: Some(vec![source_diagnostic.clone()]),
        edit: Some(WorkspaceEdit {
            changes: Some(changes),
            ..WorkspaceEdit::default()
        }),
        is_preferred: Some(false),
        ..CodeAction::default()
    }));

    actions
}

/// Keep only diagnostics overlapping selected range.
pub fn overlaps(selection: Range, diagnostic: &Diagnostic) -> bool {
    !(diagnostic.range.end.line < selection.start.line
        || diagnostic.range.start.line > selection.end.line)
}

fn suppression_text(extension: Option<&str>, rule_id: &str) -> String {
    match extension.map(|value| value.to_ascii_lowercase()) {
        Some(ext) if ext == "md" || ext == "markdown" => {
            format!(
                "<!-- goodwrite:unsafe({rule_id}): explain why this exception is necessary -->\n"
            )
        }
        Some(ext) if ext == "typ" => {
            format!("// goodwrite:unsafe({rule_id}): explain why this exception is necessary\n")
        }
        _ => String::new(),
    }
}

fn line_start_offset(source: &str, offset: usize) -> usize {
    let pos = crate::diagnostics::offset_to_position(source, offset);
    position_to_offset(
        source,
        tower_lsp_server::ls_types::Position::new(pos.line, 0),
    )
}
