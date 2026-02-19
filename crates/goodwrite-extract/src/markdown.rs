use std::ops::Range;

use goodwrite_core::{ExtractResult, ProseSpan, SourceRange, SpanAnnotations, UnsafeAnnotation};
use pulldown_cmark::{Event, Options, Parser, Tag, TagEnd};

use crate::{
    ExtractError,
    spans::ensure_mode,
    suppressions::{parse_html_annotations, parse_unsafe_annotation_content},
};

pub(crate) fn extract_markdown(source: &str) -> Result<ExtractResult, ExtractError> {
    // The markdown extractor is intentionally event-driven so we can keep
    // source offsets from pulldown-cmark and avoid a second pass over the file.
    let parser = Parser::new_ext(source, Options::all()).into_offset_iter();

    let mut spans = Vec::new();
    let mut has_mode_annotations = false;
    let mut used_mode_heuristic = false;

    let mut default_annotations = SpanAnnotations::default();
    let mut current_annotations = SpanAnnotations::default();
    let mut pending_unsafe = Vec::new();
    let mut saw_prose = false;

    let mut in_code_block = false;
    let mut in_heading = false;
    let mut heading_text = String::new();
    let mut current_heading: Option<String> = None;

    let mut buffer = String::new();
    let mut buffer_start = None;
    let mut buffer_end = 0usize;

    for (event, range) in parser {
        match event {
            Event::Start(Tag::CodeBlock(_)) => {
                flush_span(
                    &mut spans,
                    &mut buffer,
                    &mut buffer_start,
                    &mut buffer_end,
                    &current_annotations,
                    &mut pending_unsafe,
                    current_heading.clone(),
                    &mut used_mode_heuristic,
                );
                in_code_block = true;
            }
            Event::End(TagEnd::CodeBlock) => {
                in_code_block = false;
            }
            _ if in_code_block => {}
            Event::Code(_) => {}
            Event::Start(Tag::Heading { .. }) => {
                flush_span(
                    &mut spans,
                    &mut buffer,
                    &mut buffer_start,
                    &mut buffer_end,
                    &current_annotations,
                    &mut pending_unsafe,
                    current_heading.clone(),
                    &mut used_mode_heuristic,
                );
                in_heading = true;
                heading_text.clear();
            }
            Event::End(TagEnd::Heading(_)) => {
                if !heading_text.trim().is_empty() {
                    current_heading = Some(heading_text.trim().to_string());
                }
                in_heading = false;
            }
            Event::Html(html) => {
                flush_span(
                    &mut spans,
                    &mut buffer,
                    &mut buffer_start,
                    &mut buffer_end,
                    &current_annotations,
                    &mut pending_unsafe,
                    current_heading.clone(),
                    &mut used_mode_heuristic,
                );

                let annotations = parse_html_annotations(html.as_ref());
                for annotation in annotations {
                    if let Some(unsafe_annotation) = parse_unsafe_annotation_content(
                        &annotation,
                        SourceRange::new(range.start, range.end),
                    ) {
                        pending_unsafe.push(unsafe_annotation);
                        continue;
                    }

                    if annotation.starts_with("goodwrite:ears:") {
                        return Err(ExtractError::LegacyAnnotation { annotation });
                    }
                    if annotation.starts_with("goodwrite:mode:") {
                        has_mode_annotations = true;
                    }
                    apply_markdown_annotation(
                        &annotation,
                        &mut default_annotations,
                        &mut current_annotations,
                        saw_prose,
                    );
                }
            }
            Event::Text(text) => {
                saw_prose = true;
                if in_heading {
                    heading_text.push_str(text.as_ref());
                }
                append_text(
                    &mut buffer,
                    &mut buffer_start,
                    &mut buffer_end,
                    text.as_ref(),
                    range,
                );
            }
            Event::SoftBreak => {
                append_text(&mut buffer, &mut buffer_start, &mut buffer_end, " ", range);
            }
            Event::HardBreak => {
                append_text(&mut buffer, &mut buffer_start, &mut buffer_end, ". ", range);
            }
            Event::End(TagEnd::Paragraph | TagEnd::Item | TagEnd::BlockQuote) => {
                flush_span(
                    &mut spans,
                    &mut buffer,
                    &mut buffer_start,
                    &mut buffer_end,
                    &current_annotations,
                    &mut pending_unsafe,
                    current_heading.clone(),
                    &mut used_mode_heuristic,
                );
            }
            _ => {}
        }
    }

    flush_span(
        &mut spans,
        &mut buffer,
        &mut buffer_start,
        &mut buffer_end,
        &current_annotations,
        &mut pending_unsafe,
        current_heading,
        &mut used_mode_heuristic,
    );

    if !has_mode_annotations {
        for span in &mut spans {
            used_mode_heuristic |= ensure_mode(&mut span.annotations, &span.text);
        }
    }

    Ok(ExtractResult {
        source: String::new(),
        spans,
        has_mode_annotations,
        used_mode_heuristic,
    })
}

fn append_text(
    buffer: &mut String,
    buffer_start: &mut Option<usize>,
    buffer_end: &mut usize,
    text: &str,
    range: Range<usize>,
) {
    if buffer_start.is_none() {
        *buffer_start = Some(range.start);
    }
    *buffer_end = range.end;
    buffer.push_str(text);
}

#[allow(clippy::too_many_arguments)]
fn flush_span(
    spans: &mut Vec<ProseSpan>,
    buffer: &mut String,
    buffer_start: &mut Option<usize>,
    buffer_end: &mut usize,
    annotations: &SpanAnnotations,
    pending_unsafe: &mut Vec<UnsafeAnnotation>,
    heading: Option<String>,
    used_mode_heuristic: &mut bool,
) {
    if buffer.trim().is_empty() {
        buffer.clear();
        *buffer_start = None;
        *buffer_end = 0;
        return;
    }

    let leading_trim = buffer.len() - buffer.trim_start().len();
    let trailing_trim = buffer.len() - buffer.trim_end().len();
    let trimmed_end = buffer.len().saturating_sub(trailing_trim);
    let trimmed_text = &buffer[leading_trim..trimmed_end];
    let absolute_start = buffer_start.unwrap_or(0) + leading_trim;
    let absolute_end = buffer_end.saturating_sub(trailing_trim);

    let mut span_annotations = annotations.clone();
    *used_mode_heuristic |= ensure_mode(&mut span_annotations, trimmed_text);

    let mut span = ProseSpan::new(
        trimmed_text.to_string(),
        SourceRange::new(absolute_start, absolute_end),
        span_annotations,
    );
    span.unsafe_annotations = std::mem::take(pending_unsafe);
    span.heading = heading;
    spans.push(span);

    buffer.clear();
    *buffer_start = None;
    *buffer_end = 0;
}

fn apply_markdown_annotation(
    annotation: &str,
    default_annotations: &mut SpanAnnotations,
    current_annotations: &mut SpanAnnotations,
    saw_prose: bool,
) {
    // Requirement markers always apply to the active block and are never
    // promoted to file-level defaults.
    if annotation.eq_ignore_ascii_case("goodwrite:requirement")
        || annotation.starts_with("goodwrite:requirement:")
    {
        let _ = current_annotations.apply_annotation(annotation);
        return;
    }

    if annotation.eq_ignore_ascii_case("goodwrite:mode:end") {
        current_annotations.writing_mode = default_annotations.writing_mode;
        return;
    }

    if !saw_prose {
        // Leading metadata in markdown documents is treated as file defaults.
        // Once prose has been seen, annotations become local state changes only.
        let mut updated = default_annotations.clone();
        if updated.apply_annotation(annotation) {
            *default_annotations = updated.clone();
            *current_annotations = updated;
        }
        return;
    }

    let _ = current_annotations.apply_annotation(annotation);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn attaches_unsafe_annotation_from_html_comments_to_next_span() {
        let source = "<!-- goodwrite:mode:descriptive -->\n<!-- goodwrite:unsafe(asd-ste100/unapproved-word): required wording from certification source -->\nUtilize this mode.\n";
        let extracted = extract_markdown(source).expect("markdown extraction should succeed");
        assert_eq!(extracted.spans.len(), 1);
        let span = &extracted.spans[0];
        assert_eq!(span.unsafe_annotations.len(), 1);
        assert_eq!(
            span.unsafe_annotations[0].rule_id,
            "asd-ste100/unapproved-word"
        );
        assert_eq!(
            span.unsafe_annotations[0].reason,
            "required wording from certification source"
        );
    }
}
