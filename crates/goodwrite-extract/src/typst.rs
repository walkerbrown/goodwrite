use goodwrite_core::{
    ExtractResult, ProseSpan, SourceRange, SpanAnnotations, UnsafeAnnotation, WritingMode,
};

use crate::{
    ExtractError,
    spans::ensure_mode,
    suppressions::{parse_typst_metadata, parse_unsafe_annotation},
};

pub(crate) fn extract_typst(source: &str) -> Result<ExtractResult, ExtractError> {
    // Parse for syntax validation and to keep `typst-syntax` in the pipeline.
    let _ = typst_syntax::parse(source);

    // This extractor keeps a lightweight line-oriented state machine because
    // templates and annotation markers in this project are line-local.
    // We still parse with typst-syntax above so malformed input is surfaced.
    let mut spans = Vec::new();
    let mut has_mode_annotations = false;
    let mut used_mode_heuristic = false;
    let mut current = SpanAnnotations::default();
    let mut pending_unsafe: Vec<UnsafeAnnotation> = Vec::new();
    let mut in_mode_block = false;
    let mut in_requirement_block = false;

    let mut offset = 0usize;
    for line in source.lines() {
        let trimmed = line.trim();

        if let Some(annotation) =
            parse_unsafe_annotation(trimmed, SourceRange::new(offset, offset + line.len()))
        {
            pending_unsafe.push(annotation);
            offset += line.len() + 1;
            continue;
        }

        if let Some(annotation) = parse_typst_metadata(trimmed) {
            if annotation.starts_with("goodwrite:ears:") {
                return Err(ExtractError::LegacyAnnotation { annotation });
            }
            if current.apply_annotation(&annotation) && annotation.starts_with("goodwrite:mode:") {
                has_mode_annotations = true;
            }
            offset += line.len() + 1;
            continue;
        }

        if let Some(mode) = template_mode(trimmed) {
            current.writing_mode = Some(mode);
            in_mode_block = true;
            has_mode_annotations = true;
            offset += line.len() + 1;
            continue;
        }

        if let Some(requirement_type) = requirement_block_type(trimmed) {
            current.requirement = true;
            current.requirement_type = requirement_type;
            in_requirement_block = true;
            offset += line.len() + 1;
            continue;
        }

        if trimmed == "]" {
            if in_mode_block {
                current.writing_mode = None;
                in_mode_block = false;
            }
            if in_requirement_block {
                current.requirement = false;
                current.requirement_type = None;
                in_requirement_block = false;
            }
            offset += line.len() + 1;
            continue;
        }

        if trimmed.is_empty() || trimmed.starts_with("#import") || trimmed.starts_with("//") {
            offset += line.len() + 1;
            continue;
        }

        let text = normalize_typst_line(trimmed);
        if text.is_empty() {
            offset += line.len() + 1;
            continue;
        }

        let mut annotations = current.clone();
        used_mode_heuristic |= ensure_mode(&mut annotations, &text);

        let mut span = ProseSpan::new(
            text,
            SourceRange::new(offset, offset + line.len()),
            annotations,
        );
        span.unsafe_annotations = std::mem::take(&mut pending_unsafe);
        spans.push(span);

        offset += line.len() + 1;
    }

    Ok(ExtractResult {
        source: String::new(),
        spans,
        has_mode_annotations,
        used_mode_heuristic,
    })
}

fn template_mode(line: &str) -> Option<WritingMode> {
    if line.starts_with("#procedure[") {
        Some(WritingMode::Procedural)
    } else if line.starts_with("#description[") {
        Some(WritingMode::Descriptive)
    } else if line.starts_with("#warning[") || line.starts_with("#caution[") {
        Some(WritingMode::SafetyInstruction)
    } else if line.starts_with("#note[") {
        Some(WritingMode::Note)
    } else {
        None
    }
}

fn requirement_block_type(line: &str) -> Option<Option<String>> {
    // Untyped requirement block.
    if line.starts_with("#requirement[")
        || (line.starts_with("#requirement(") && line.contains('['))
    {
        return Some(None);
    }

    // Typed helper form: `#requirement_<type>[...]`.
    if !line.starts_with("#requirement_") {
        return None;
    }

    let after_prefix = &line["#requirement_".len()..];
    let terminator = after_prefix.find(['[', '('])?;
    let raw_type = &after_prefix[..terminator];
    let normalized = raw_type.trim().replace('_', "-").to_ascii_lowercase();
    if normalized.is_empty() {
        return None;
    }
    Some(Some(normalized))
}

fn normalize_typst_line(line: &str) -> String {
    let mut text = line.trim().trim_start_matches('+').trim().to_string();

    if let Some(rest) = strip_numbered_list_prefix(&text) {
        text = rest.to_string();
    }

    if text.starts_with('[') && text.ends_with(']') && text.len() > 2 {
        text = text[1..text.len() - 1].to_string();
    }

    text.replace("#", "").trim().to_string()
}

fn strip_numbered_list_prefix(text: &str) -> Option<&str> {
    let mut saw_digit = false;
    let mut split_at = None;

    for (idx, ch) in text.char_indices() {
        if ch.is_ascii_digit() {
            saw_digit = true;
            continue;
        }
        if saw_digit && ch == '.' {
            split_at = Some(idx + 1);
            break;
        }
        break;
    }

    let split = split_at?;
    Some(text[split..].trim_start())
}
