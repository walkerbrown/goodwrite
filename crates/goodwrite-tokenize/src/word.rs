use regex::Regex;

use goodwrite_core::{SourceRange, Token};

/// Tokenize words with source offsets.
pub fn tokenize_words(text: &str, absolute_start: usize) -> Vec<Token> {
    static RE: std::sync::LazyLock<Regex> = std::sync::LazyLock::new(|| {
        Regex::new(r#"\([^)]*\)|"[^"]+"|[A-Za-z0-9]+(?:-[A-Za-z0-9]+)*"#)
            .expect("valid token regex")
    });

    RE.find_iter(text)
        .map(|m| Token {
            text: m.as_str().to_string(),
            range: SourceRange::new(absolute_start + m.start(), absolute_start + m.end()),
        })
        .collect()
}

/// ASD-STE100 counting rules 8.4-8.7 (best-effort implementation).
pub fn asd_ste100_word_count(sentence: &str) -> usize {
    static RE: std::sync::LazyLock<Regex> = std::sync::LazyLock::new(|| {
        Regex::new(r#"\([^)]*\)|"[^"]+"|[A-Za-z0-9]+(?:-[A-Za-z0-9]+)*"#)
            .expect("valid count regex")
    });

    let tokens = RE
        .find_iter(sentence)
        .map(|m| m.as_str().to_string())
        .collect::<Vec<_>>();

    let mut count = 0usize;
    let mut idx = 0usize;
    while idx < tokens.len() {
        let token = tokens[idx].as_str();

        // Rule 8.5: parenthesized text counts as one word.
        if token.starts_with('(') && token.ends_with(')') {
            count += 1;
            idx += 1;
            continue;
        }

        // Rule 8.6: quoted text counts as one word.
        if token.starts_with('"') && token.ends_with('"') {
            count += 1;
            idx += 1;
            continue;
        }

        // Rule 8.6: number + unit counts as one.
        if is_number(token) && idx + 1 < tokens.len() && looks_like_unit(tokens[idx + 1].as_str()) {
            count += 1;
            idx += 2;
            continue;
        }

        // Rule 8.6: organization/proper-noun names count as one word.
        if is_capitalized_word(token) {
            let mut end = idx + 1;
            while end < tokens.len() && is_capitalized_word(tokens[end].as_str()) {
                end += 1;
            }
            count += 1;
            idx = end;
            continue;
        }

        // Rule 8.7: hyphenated words are already one token via regex.
        count += 1;
        idx += 1;
    }

    count
}

fn is_number(token: &str) -> bool {
    !token.is_empty()
        && token
            .chars()
            .all(|ch| ch.is_ascii_digit() || ch == '.' || ch == ',')
}

fn looks_like_unit(token: &str) -> bool {
    token.len() <= 6
        && token
            .chars()
            .all(|ch| ch.is_ascii_alphabetic() || ch == '%' || ch == '°')
}

fn is_capitalized_word(token: &str) -> bool {
    let mut chars = token.chars();
    chars.next().is_some_and(|ch| ch.is_ascii_uppercase())
        && chars.all(|ch| ch.is_ascii_lowercase())
}

#[cfg(test)]
mod tests {
    use super::asd_ste100_word_count;

    #[test]
    fn parenthesized_counts_as_one() {
        let count = asd_ste100_word_count("Open the panel (with a special tool) now.");
        assert_eq!(count, 5);
    }

    #[test]
    fn number_plus_unit_counts_as_one() {
        let count = asd_ste100_word_count("Set pressure to 25 psi.");
        assert_eq!(count, 4);
    }

    #[test]
    fn proper_noun_sequence_counts_as_one() {
        let count = asd_ste100_word_count("Send report to Federal Aviation Administration office.");
        assert_eq!(count, 5);
    }
}
