use goodwrite_core::{Diagnostic, Severity};
use owo_colors::OwoColorize;
use serde_json::json;

use crate::FileDiagnostics;
use crate::render::{RenderDiagnostic, SharedSource};

pub fn render_terminal(files: &[FileDiagnostics]) {
    for file in files {
        if file.diagnostics.is_empty() {
            continue;
        }
        let source = SharedSource::new(file.path.display().to_string(), file.source.clone());
        for diagnostic in &file.diagnostics {
            let rd = RenderDiagnostic::new(diagnostic, source.clone());
            eprintln!("{:?}", miette::Report::new(rd));
        }
    }
}

pub fn render_summary(files: &[FileDiagnostics], color: bool) {
    let mut errors = 0usize;
    let mut warnings = 0usize;
    let mut infos = 0usize;
    let mut files_with_diagnostics = 0usize;

    for file in files {
        if !file.diagnostics.is_empty() {
            files_with_diagnostics += 1;
        }
        for diag in &file.diagnostics {
            match diag.severity {
                Severity::Error => errors += 1,
                Severity::Warning => warnings += 1,
                Severity::Info => infos += 1,
            }
        }
    }

    let total = errors + warnings + infos;
    if total == 0 {
        return;
    }

    let mut parts = Vec::new();
    if errors > 0 {
        parts.push(if color {
            format!("{} {}", errors, "error(s)".red().bold())
        } else {
            format!("{errors} error(s)")
        });
    }
    if warnings > 0 {
        parts.push(if color {
            format!("{} {}", warnings, "warning(s)".yellow().bold())
        } else {
            format!("{warnings} warning(s)")
        });
    }
    if infos > 0 {
        parts.push(if color {
            format!("{} {}", infos, "info(s)".cyan().bold())
        } else {
            format!("{infos} info(s)")
        });
    }

    eprintln!(
        "Found {} in {files_with_diagnostics} file(s)",
        parts.join(" and ")
    );
}

pub fn render_json(files: &[FileDiagnostics]) {
    let output = files
        .iter()
        .map(|file| {
            json!({
                "path": file.path,
                "diagnostics": file.diagnostics,
            })
        })
        .collect::<Vec<_>>();

    println!(
        "{}",
        serde_json::to_string_pretty(&output).expect("serializable diagnostics")
    );
}

pub fn render_sarif(files: &[FileDiagnostics]) {
    let mut results = Vec::new();
    let mut rule_ids = std::collections::BTreeSet::new();

    for file in files {
        for diagnostic in &file.diagnostics {
            let (start_line, start_column) = line_col(&file.source, diagnostic.span.start);
            let (end_line, end_column) = line_col(&file.source, diagnostic.span.end);

            rule_ids.insert(diagnostic.rule_id.clone());

            // Build enriched message with help and note
            let mut message_text = diagnostic.message.clone();
            if let Some(help) = &diagnostic.help {
                message_text.push_str("\nhelp: ");
                message_text.push_str(help);
            }
            if let Some(note) = &diagnostic.note {
                message_text.push_str("\nnote: ");
                message_text.push_str(note);
            }

            // Build SARIF fixes from MachineApplicable suggestions
            let fixes: Vec<serde_json::Value> = diagnostic
                .suggestions
                .iter()
                .filter(|s| s.applicability == goodwrite_core::Applicability::MachineApplicable)
                .map(|s| {
                    let (del_start_line, del_start_col) = line_col(&file.source, s.span.start);
                    let (del_end_line, del_end_col) = line_col(&file.source, s.span.end);
                    json!({
                        "description": { "text": s.message },
                        "artifactChanges": [{
                            "artifactLocation": { "uri": file.path.display().to_string() },
                            "replacements": [{
                                "deletedRegion": {
                                    "startLine": del_start_line,
                                    "startColumn": del_start_col,
                                    "endLine": del_end_line,
                                    "endColumn": del_end_col,
                                },
                                "insertedContent": { "text": s.replacement }
                            }]
                        }]
                    })
                })
                .collect();

            let mut result = json!({
                "ruleId": diagnostic.rule_id,
                "level": sarif_level(diagnostic),
                "message": { "text": message_text },
                "locations": [{
                    "physicalLocation": {
                        "artifactLocation": { "uri": file.path.display().to_string() },
                        "region": {
                            "startLine": start_line,
                            "startColumn": start_column,
                            "endLine": end_line,
                            "endColumn": end_column,
                        }
                    }
                }]
            });

            if !fixes.is_empty() {
                result["fixes"] = json!(fixes);
            }

            results.push(result);
        }
    }

    let rules: Vec<serde_json::Value> = rule_ids
        .iter()
        .map(|id| {
            json!({
                "id": id,
                "helpUri": format!("https://goodwrite.dev/rules#{id}"),
            })
        })
        .collect();

    let document = json!({
        "$schema": "https://json.schemastore.org/sarif-2.1.0.json",
        "version": "2.1.0",
        "runs": [{
            "tool": {
                "driver": {
                    "name": "goodwrite",
                    "informationUri": "https://goodwrite.dev",
                    "version": env!("CARGO_PKG_VERSION"),
                    "rules": rules,
                }
            },
            "results": results,
        }]
    });

    println!(
        "{}",
        serde_json::to_string_pretty(&document).expect("serializable SARIF")
    );
}

fn sarif_level(diagnostic: &Diagnostic) -> &'static str {
    match diagnostic.severity {
        Severity::Error => "error",
        Severity::Warning => "warning",
        Severity::Info => "note",
    }
}

fn line_col(source: &str, offset: usize) -> (usize, usize) {
    let safe_offset = offset.min(source.len());
    let mut line = 1usize;
    let mut col = 1usize;

    for (idx, ch) in source.char_indices() {
        if idx >= safe_offset {
            break;
        }
        if ch == '\n' {
            line += 1;
            col = 1;
        } else {
            col += 1;
        }
    }

    (line, col)
}
