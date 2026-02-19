use crate::SourceRange;

/// Token with source range.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Token {
    pub text: String,
    pub range: SourceRange,
}

/// Sentence unit used by rules.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Sentence {
    pub text: String,
    pub range: SourceRange,
    pub tokens: Vec<Token>,
    pub ste_word_count: usize,
}
