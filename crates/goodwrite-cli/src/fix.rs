use std::path::Path;

use goodwrite_core::Suggestion;
use owo_colors::OwoColorize;
use similar::{ChangeTag, TextDiff};

/// Apply non-overlapping machine-applicable suggestions.
pub fn apply_suggestions(source: &str, suggestions: &[Suggestion]) -> (String, usize) {
    if suggestions.is_empty() {
        return (source.to_string(), 0);
    }

    let mut edits = suggestions.to_vec();
    edits.sort_by(|a, b| b.span.start.cmp(&a.span.start));

    let mut updated = source.to_string();
    let mut applied = 0usize;
    let mut last_start = usize::MAX;

    for suggestion in edits {
        if suggestion.span.end > updated.len() || suggestion.span.start > suggestion.span.end {
            continue;
        }

        if suggestion.span.end > last_start {
            continue;
        }

        updated.replace_range(
            suggestion.span.start..suggestion.span.end,
            &suggestion.replacement,
        );
        last_start = suggestion.span.start;
        applied += 1;
    }

    (updated, applied)
}

/// Unified diff output for dry-run mode.
pub fn diff_output(path: &Path, before: &str, after: &str, color: bool) -> String {
    let diff = TextDiff::from_lines(before, after);
    let mut out = String::new();
    let before_header = format!("--- {}\n", path.display());
    let after_header = format!("+++ {}\n", path.display());
    if color {
        out.push_str(&before_header.red().bold().to_string());
        out.push_str(&after_header.green().bold().to_string());
    } else {
        out.push_str(&before_header);
        out.push_str(&after_header);
    }

    for change in diff.iter_all_changes() {
        let sign = match change.tag() {
            ChangeTag::Delete => '-',
            ChangeTag::Insert => '+',
            ChangeTag::Equal => ' ',
        };
        let line = format!("{sign}{}", change.as_str().unwrap_or_default());
        if color {
            match change.tag() {
                ChangeTag::Delete => out.push_str(&line.red().to_string()),
                ChangeTag::Insert => out.push_str(&line.green().to_string()),
                ChangeTag::Equal => out.push_str(&line),
            }
        } else {
            out.push_str(&line);
        }
    }

    out
}
