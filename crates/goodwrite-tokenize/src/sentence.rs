/// Sentence split result with byte offsets relative to original span text.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SentencePiece {
    pub text: String,
    pub start: usize,
    pub end: usize,
}

/// Heuristic sentence splitting for technical prose.
pub fn split_sentences(text: &str) -> Vec<SentencePiece> {
    let mut out = Vec::new();
    let mut sentence_start = 0usize;

    for (idx, ch) in text.char_indices() {
        let boundary = is_boundary(text, idx, ch);
        if !boundary {
            continue;
        }

        let end = idx + ch.len_utf8();
        push_sentence(text, sentence_start, end, &mut out);
        sentence_start = next_non_whitespace(text, end);
    }

    if sentence_start < text.len() {
        push_sentence(text, sentence_start, text.len(), &mut out);
    }

    out
}

fn is_boundary(text: &str, idx: usize, ch: char) -> bool {
    if matches!(ch, '!' | '?') {
        return true;
    }

    if is_vertical_list_colon(text, idx, ch) {
        return true;
    }

    if ch != '.' {
        return false;
    }

    if is_decimal_point(text, idx)
        || is_known_abbreviation(text, idx)
        || is_acronym_dot(text, idx)
        || is_abbreviation_internal_dot(text, idx)
    {
        return false;
    }

    true
}

fn push_sentence(text: &str, start: usize, end: usize, out: &mut Vec<SentencePiece>) {
    if start >= end || end > text.len() {
        return;
    }

    let slice = &text[start..end];
    let trimmed = slice.trim();
    if trimmed.is_empty() {
        return;
    }

    let leading_ws = slice.len() - slice.trim_start().len();
    let trailing_ws = slice.len() - slice.trim_end().len();

    out.push(SentencePiece {
        text: trimmed.to_string(),
        start: start + leading_ws,
        end: end - trailing_ws,
    });
}

fn next_non_whitespace(text: &str, mut idx: usize) -> usize {
    while idx < text.len() {
        if !text[idx..].chars().next().is_some_and(char::is_whitespace) {
            return idx;
        }
        idx += text[idx..].chars().next().map(char::len_utf8).unwrap_or(1);
    }
    idx
}

fn is_vertical_list_colon(text: &str, idx: usize, ch: char) -> bool {
    if ch != ':' {
        return false;
    }

    let after = idx + ch.len_utf8();
    let suffix = &text[after..];
    suffix.starts_with('\n') || suffix.starts_with("\r\n")
}

fn is_decimal_point(text: &str, idx: usize) -> bool {
    let prev = text[..idx].chars().next_back();
    let next = text[idx + 1..].chars().next();
    prev.is_some_and(|ch| ch.is_ascii_digit()) && next.is_some_and(|ch| ch.is_ascii_digit())
}

fn is_acronym_dot(text: &str, idx: usize) -> bool {
    let prev = text[..idx].chars().next_back();
    let next = text[idx + 1..].chars().next();
    prev.is_some_and(|ch| ch.is_ascii_uppercase()) && next.is_some_and(|ch| ch.is_ascii_uppercase())
}

fn is_known_abbreviation(text: &str, idx: usize) -> bool {
    const ABBR: &[&str] = &[
        "e.g.", "i.e.", "etc.", "mr.", "mrs.", "dr.", "vs.", "fig.", "no.",
    ];
    let fragment = text[..=idx].to_ascii_lowercase();
    ABBR.iter().any(|abbr| fragment.ends_with(abbr))
}

fn is_abbreviation_internal_dot(text: &str, idx: usize) -> bool {
    let prev = text[..idx].chars().next_back();
    let next = text[idx + 1..].chars().next();
    let next_next = text[idx + 1..].chars().nth(1);

    prev.is_some_and(|ch| ch.is_ascii_alphabetic())
        && next.is_some_and(|ch| ch.is_ascii_alphabetic())
        && next_next.is_some_and(|ch| ch == '.')
}

#[cfg(test)]
mod tests {
    use super::split_sentences;

    #[test]
    fn splits_basic_sentences() {
        let parts = split_sentences("Open the valve. Remove the bolt.");
        assert_eq!(parts.len(), 2);
    }

    #[test]
    fn does_not_split_decimals_or_abbreviations() {
        let parts = split_sentences("Set pressure to 2.5 bar, e.g. for test. Continue.");
        assert_eq!(parts.len(), 2);
    }
}
