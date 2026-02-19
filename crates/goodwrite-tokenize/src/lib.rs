//! Sentence and word tokenization utilities.

pub mod pos;
pub mod sentence;
pub mod word;

use goodwrite_core::{ProseSpan, Sentence};

pub use pos::{
    AnalysisTraceStep, CandidateSource, DeterministicPosContext, DeterministicPosState,
    PosCandidate, PosClass, PosLexiconProvider, PosResolution, ResolutionKind, TokenAnalysis,
    analyze_tokens,
};
pub use sentence::split_sentences;
pub use word::{asd_ste100_word_count, tokenize_words};

/// Tokenize one extracted prose span into sentence units.
pub fn tokenize_span(span: &ProseSpan) -> Vec<Sentence> {
    split_sentences(&span.text)
        .into_iter()
        .map(|piece| {
            let absolute_start = span.range.start + piece.start;
            let absolute_end = span.range.start + piece.end;
            let tokens = tokenize_words(&piece.text, absolute_start);
            let ste_word_count = asd_ste100_word_count(&piece.text);
            Sentence {
                text: piece.text,
                range: goodwrite_core::SourceRange::new(absolute_start, absolute_end),
                tokens,
                ste_word_count,
            }
        })
        .collect()
}
