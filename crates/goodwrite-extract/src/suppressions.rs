use goodwrite_core::{SourceRange, UnsafeAnnotation};

/// Parse `goodwrite:unsafe(<rule-id>): <reason>` from Typst or Markdown comments.
pub fn parse_unsafe_annotation(text: &str, range: SourceRange) -> Option<UnsafeAnnotation> {
    let content = strip_comment_wrappers(text)?;
    parse_unsafe_annotation_content(content, range)
}

/// Parse `goodwrite:unsafe(<rule-id>): <reason>` from already-unwrapped annotation text.
pub fn parse_unsafe_annotation_content(
    annotation: &str,
    range: SourceRange,
) -> Option<UnsafeAnnotation> {
    let annotation = annotation.trim();
    let rest = annotation.strip_prefix("goodwrite:unsafe")?;

    let rest = rest.trim_start();
    if !rest.starts_with('(') {
        return Some(UnsafeAnnotation::invalid(
            "",
            "",
            range,
            "missing `(<rule-id>)` segment",
        ));
    }

    let Some(close_idx) = rest.find(')') else {
        return Some(UnsafeAnnotation::invalid(
            "",
            "",
            range,
            "missing closing `)` after rule id",
        ));
    };

    let rule_id = rest[1..close_idx].trim();
    let after = rest[close_idx + 1..].trim_start();
    if !after.starts_with(':') {
        return Some(UnsafeAnnotation::invalid(
            rule_id,
            "",
            range,
            "missing `:` before explanation",
        ));
    }

    let reason = after[1..].trim();
    if rule_id.is_empty() {
        return Some(UnsafeAnnotation::invalid(
            "",
            reason,
            range,
            "rule id cannot be empty",
        ));
    }

    if reason.len() < 8 {
        return Some(UnsafeAnnotation::invalid(
            rule_id,
            reason,
            range,
            "explanation is too short; provide a brief technical reason",
        ));
    }

    Some(UnsafeAnnotation::pending(rule_id, reason, range))
}

/// Parse all `goodwrite:*` annotations from HTML in Markdown.
///
/// Pulldown-cmark can emit multiple contiguous HTML comments in one `Event::Html`.
/// We scan the full event payload and return each `<!-- ... -->` annotation.
pub fn parse_html_annotations(text: &str) -> Vec<String> {
    let mut annotations = Vec::new();
    let mut offset = 0usize;

    while let Some(start_rel) = text[offset..].find("<!--") {
        let start = offset + start_rel;
        let Some(end_rel) = text[start..].find("-->") else {
            break;
        };
        let end = start + end_rel + 3;

        let content = text[start + 4..end - 3].trim();
        if content.starts_with("goodwrite:") {
            annotations.push(content.to_string());
        }

        offset = end;
    }

    annotations
}

/// Parse `metadata("goodwrite:...")` from Typst source lines.
pub fn parse_typst_metadata(text: &str) -> Option<String> {
    let marker = "metadata(\"";
    let start = text.find(marker)? + marker.len();
    let rest = &text[start..];
    let end = rest.find("\"")?;
    let candidate = &rest[..end];
    if candidate.starts_with("goodwrite:") {
        Some(candidate.to_string())
    } else {
        None
    }
}

fn strip_comment_wrappers(text: &str) -> Option<&str> {
    let trimmed = text.trim();
    if let Some(content) = trimmed
        .strip_prefix("<!--")
        .and_then(|v| v.strip_suffix("-->"))
    {
        return Some(content.trim());
    }

    if let Some(content) = trimmed.strip_prefix("//") {
        return Some(content.trim());
    }

    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_multiple_goodwrite_annotations_from_html_block() {
        let html = "<!-- goodwrite:mode:descriptive -->\n<!-- goodwrite:unsafe(asd-ste100/unapproved-word): required phrasing from certification input -->";
        let annotations = parse_html_annotations(html);
        assert_eq!(
            annotations,
            vec![
                "goodwrite:mode:descriptive".to_string(),
                "goodwrite:unsafe(asd-ste100/unapproved-word): required phrasing from certification input".to_string(),
            ]
        );
    }

    #[test]
    fn parses_unsafe_from_unwrapped_annotation() {
        let annotation = "goodwrite:unsafe(asd-ste100/unapproved-word): required phrasing from certification input";
        let parsed =
            parse_unsafe_annotation_content(annotation, SourceRange::new(0, annotation.len()))
                .expect("unsafe annotation should parse");
        assert_eq!(parsed.rule_id, "asd-ste100/unapproved-word");
        assert_eq!(parsed.reason, "required phrasing from certification input");
    }
}
