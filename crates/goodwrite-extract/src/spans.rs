use goodwrite_core::{SpanAnnotations, WritingMode};

/// Best-effort writing mode inference when explicit annotations are absent.
pub fn mode_from_heuristic(text: &str) -> WritingMode {
    let trimmed = text.trim_start();
    let lower = trimmed.to_ascii_lowercase();

    if lower.starts_with("warning:") || lower.starts_with("caution:") {
        return WritingMode::SafetyInstruction;
    }

    if lower.starts_with("note:") || lower.starts_with("> **note:**") {
        return WritingMode::Note;
    }

    if looks_imperative(trimmed) {
        return WritingMode::Procedural;
    }

    WritingMode::Descriptive
}

/// Ensure spans always carry a mode.
///
/// Returns `true` when this function had to infer the mode heuristically.
pub fn ensure_mode(annotations: &mut SpanAnnotations, text: &str) -> bool {
    if annotations.writing_mode.is_none() {
        annotations.writing_mode = Some(mode_from_heuristic(text));
        return true;
    }
    false
}

fn looks_imperative(text: &str) -> bool {
    const IMPERATIVE_VERBS: &[&str] = &[
        "open",
        "remove",
        "install",
        "set",
        "press",
        "push",
        "turn",
        "disconnect",
        "connect",
        "tighten",
        "loosen",
        "apply",
        "verify",
        "check",
        "close",
        "start",
        "stop",
        "attach",
        "detach",
        "fill",
        "drain",
    ];

    let first = text
        .split(|c: char| !c.is_ascii_alphabetic())
        .find(|chunk| !chunk.is_empty())
        .unwrap_or_default()
        .to_ascii_lowercase();

    IMPERATIVE_VERBS.iter().any(|verb| *verb == first)
}
