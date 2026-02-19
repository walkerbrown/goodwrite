use std::path::Path;

use goodwrite_core::Suggestion;
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
pub fn diff_output(path: &Path, before: &str, after: &str) -> String {
    let diff = TextDiff::from_lines(before, after);
    let mut out = String::new();
    out.push_str(&format!("--- {}\n+++ {}\n", path.display(), path.display()));

    for change in diff.iter_all_changes() {
        let sign = match change.tag() {
            ChangeTag::Delete => '-',
            ChangeTag::Insert => '+',
            ChangeTag::Equal => ' ',
        };
        out.push(sign);
        out.push_str(change.as_str().unwrap_or_default());
    }

    out
}
