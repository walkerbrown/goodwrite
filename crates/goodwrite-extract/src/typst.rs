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
    let mut used_mode_inference = false;
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

        let Some((text, local_start, local_end)) = normalize_typst_line(line) else {
            offset += line.len() + 1;
            continue;
        };

        let mut annotations = current.clone();
        used_mode_inference |= ensure_mode(&mut annotations, &text);

        let mut span = ProseSpan::new(
            text,
            SourceRange::new(offset + local_start, offset + local_end),
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
        used_mode_inference,
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

fn normalize_typst_line(line: &str) -> Option<(String, usize, usize)> {
    let mut start = first_non_whitespace(line);
    let content_end = typst_inline_comment_start(line).unwrap_or(line.len());
    let mut end = trim_end_whitespace(line, content_end);

    if start >= end {
        return None;
    }

    // Typst `+` list markers are source syntax, not prose.
    while line[start..end].starts_with('+') {
        start += '+'.len_utf8();
        start = first_non_whitespace_from(line, start);
        if start >= end {
            return None;
        }
    }

    // Numbered list markers (`1. `) are syntax; keep the sentence text range aligned.
    if let Some(prefix_len) = numbered_list_prefix_len(&line[start..end]) {
        start += prefix_len;
        start = first_non_whitespace_from(line, start);
        if start >= end {
            return None;
        }
    }

    // Bracket wrappers are Typst container syntax around inline prose.
    if line[start..end].starts_with('[') && line[start..end].ends_with(']') && end - start > 2 {
        start += '['.len_utf8();
        end -= ']'.len_utf8();
        start = first_non_whitespace_from(line, start);
        end = trim_end_whitespace(line, end);
    }

    if start >= end {
        return None;
    }

    Some((line[start..end].to_string(), start, end))
}

fn numbered_list_prefix_len(text: &str) -> Option<usize> {
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

    split_at
}

fn typst_inline_comment_start(line: &str) -> Option<usize> {
    let mut in_string = false;
    let mut escaped = false;
    let mut previous = None;
    let mut iter = line.char_indices().peekable();

    while let Some((idx, ch)) = iter.next() {
        if in_string {
            if escaped {
                escaped = false;
            } else if ch == '\\' {
                escaped = true;
            } else if ch == '"' {
                in_string = false;
            }
            previous = Some(ch);
            continue;
        }

        if ch == '"' {
            in_string = true;
            previous = Some(ch);
            continue;
        }

        if ch == '/'
            && iter.peek().is_some_and(|(_, next)| *next == '/')
            && (idx == 0 || previous.is_some_and(char::is_whitespace))
        {
            return Some(idx);
        }

        previous = Some(ch);
    }

    None
}

fn first_non_whitespace(text: &str) -> usize {
    first_non_whitespace_from(text, 0)
}

fn first_non_whitespace_from(text: &str, mut idx: usize) -> usize {
    while idx < text.len() {
        let ch = text[idx..].chars().next().unwrap_or('\0');
        if !ch.is_whitespace() {
            return idx;
        }
        idx += ch.len_utf8();
    }
    idx
}

fn trim_end_whitespace(text: &str, mut end: usize) -> usize {
    while end > 0 {
        let ch = text[..end].chars().next_back().unwrap_or('\0');
        if !ch.is_whitespace() {
            return end;
        }
        end -= ch.len_utf8();
    }
    end
}

#[cfg(test)]
mod tests {
    use super::extract_typst;
    use goodwrite_core::ProseSpan;

    fn find_span_with_text<'a>(spans: &'a [ProseSpan], needle: &str) -> &'a ProseSpan {
        spans
            .iter()
            .find(|span| span.text.contains(needle))
            .expect("expected span containing target text")
    }

    fn assert_alignment(source: &str, span: &ProseSpan, needle: &str) {
        let local = span
            .text
            .find(needle)
            .expect("expected needle in extracted span text");
        let absolute = span.range.start + local;
        assert_eq!(&source[absolute..absolute + needle.len()], needle);
    }

    fn assert_semicolon_and_close_alignment(source: &str, span: &ProseSpan) {
        assert_alignment(source, span, ";");
        assert_alignment(source, span, "close");
    }

    #[test]
    fn numbered_list_span_offsets_match_original_source() {
        let source = "#procedure[\n1. Open the service panel; close the service panel.\n]\n";
        let extracted = extract_typst(source).expect("typst extraction should succeed");
        let span = find_span_with_text(
            &extracted.spans,
            "Open the service panel; close the service panel.",
        );
        assert_semicolon_and_close_alignment(source, span);
    }

    #[test]
    fn indented_plus_list_offsets_match_original_source() {
        let source = "#procedure[\n    + Open the service panel; close the service panel.\n]\n";
        let extracted = extract_typst(source).expect("typst extraction should succeed");
        let span = find_span_with_text(
            &extracted.spans,
            "Open the service panel; close the service panel.",
        );
        assert_semicolon_and_close_alignment(source, span);
    }

    #[test]
    fn typst_table_cell_offsets_match_original_source() {
        let source = "#description[\n#table(\n  columns: 1,\n  [Open the service panel; close the service panel.]\n)\n]\n";
        let extracted = extract_typst(source).expect("typst extraction should succeed");
        let span = find_span_with_text(
            &extracted.spans,
            "Open the service panel; close the service panel.",
        );
        assert_semicolon_and_close_alignment(source, span);
    }

    #[test]
    fn inline_typst_comment_text_is_excluded_from_span() {
        let source = "#description[\nUse this procedure. // utilize comment text\n]\n";
        let extracted = extract_typst(source).expect("typst extraction should succeed");
        let span = find_span_with_text(&extracted.spans, "Use this procedure.");
        assert!(!span.text.contains("utilize"));
        assert_alignment(source, span, "Use");
        assert_alignment(source, span, "procedure.");
    }

    #[test]
    fn inline_comment_after_numbered_list_keeps_offsets_aligned() {
        let source =
            "#procedure[\n1. Open the service panel; close the service panel. // comment\n]\n";
        let extracted = extract_typst(source).expect("typst extraction should succeed");
        let span = find_span_with_text(
            &extracted.spans,
            "Open the service panel; close the service panel.",
        );
        assert!(!span.text.contains("comment"));
        assert_semicolon_and_close_alignment(source, span);
    }
}
