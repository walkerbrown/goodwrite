use std::collections::HashSet;

use goodwrite_core::{Token, WritingMode};

/// Deterministic POS classes used by ASD compliance logic.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum PosClass {
    Noun,
    Verb,
    Adjective,
    Adverb,
    Determiner,
    Modal,
    Preposition,
    Conjunction,
    Pronoun,
    Participle,
    Number,
    Unknown,
}

impl PosClass {
    pub fn as_str(self) -> &'static str {
        match self {
            PosClass::Noun => "noun",
            PosClass::Verb => "verb",
            PosClass::Adjective => "adjective",
            PosClass::Adverb => "adverb",
            PosClass::Determiner => "determiner",
            PosClass::Modal => "modal",
            PosClass::Preposition => "preposition",
            PosClass::Conjunction => "conjunction",
            PosClass::Pronoun => "pronoun",
            PosClass::Participle => "participle",
            PosClass::Number => "number",
            PosClass::Unknown => "unknown",
        }
    }

    pub fn from_dictionary_tag(value: &str) -> Option<Self> {
        match value.trim().to_ascii_lowercase().as_str() {
            "n" | "nn" | "noun" => Some(Self::Noun),
            "v" | "vb" | "verb" => Some(Self::Verb),
            "adj" | "jj" | "adjective" => Some(Self::Adjective),
            "adv" | "rb" | "adverb" => Some(Self::Adverb),
            "det" | "article" | "determiner" => Some(Self::Determiner),
            "modal" | "md" => Some(Self::Modal),
            "prep" | "preposition" => Some(Self::Preposition),
            "conj" | "conjunction" => Some(Self::Conjunction),
            "pron" | "pronoun" => Some(Self::Pronoun),
            "participle" | "ptcp" => Some(Self::Participle),
            "num" | "number" => Some(Self::Number),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum CandidateSource {
    Lexicon,
    Morphology,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PosCandidate {
    pub lemma: String,
    pub pos: PosClass,
    pub source: CandidateSource,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PosResolution {
    Resolved(PosCandidate),
    Ambiguous(Vec<PosCandidate>),
    Unresolved,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DeterministicPosState {
    ClauseStart,
    AfterDeterminer,
    AfterModal,
    AfterPreposition,
    AfterCopula,
    AfterConjunction,
    AfterPunctuation,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ResolutionKind {
    Resolved,
    Ambiguous,
    Unresolved,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AnalysisTraceStep {
    EnterState(DeterministicPosState),
    GeneratedCandidates {
        count: usize,
    },
    AppliedStateConstraint {
        state: DeterministicPosState,
        before: usize,
        after: usize,
    },
    PreferLexicon {
        before: usize,
        after: usize,
    },
    Resolution(ResolutionKind),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TokenAnalysis {
    pub token: Token,
    pub candidates: Vec<PosCandidate>,
    pub resolution: PosResolution,
    pub state_trace: Vec<AnalysisTraceStep>,
}

pub trait PosLexiconProvider {
    fn candidates_for(&self, surface: &str) -> Vec<PosCandidate>;
}

pub struct DeterministicPosContext<'a, L: PosLexiconProvider + ?Sized> {
    pub lexicon: &'a L,
    pub mode: Option<WritingMode>,
}

pub fn analyze_tokens<L: PosLexiconProvider + ?Sized>(
    tokens: &[Token],
    ctx: &DeterministicPosContext<'_, L>,
) -> Vec<TokenAnalysis> {
    let mut state = DeterministicPosState::ClauseStart;
    let mut analyses = Vec::with_capacity(tokens.len());

    for token in tokens {
        let mut trace = vec![AnalysisTraceStep::EnterState(state)];
        let lower = token.text.to_ascii_lowercase();

        let generated = generate_candidates(&lower, ctx.lexicon);
        trace.push(AnalysisTraceStep::GeneratedCandidates {
            count: generated.len(),
        });

        let constrained = apply_state_constraint(&generated, state, ctx.mode, &lower);
        trace.push(AnalysisTraceStep::AppliedStateConstraint {
            state,
            before: generated.len(),
            after: constrained.len(),
        });

        let preferred = prefer_lexicon(constrained);
        trace.push(AnalysisTraceStep::PreferLexicon {
            before: generated.len(),
            after: preferred.len(),
        });

        let resolution = match preferred.len() {
            0 => {
                trace.push(AnalysisTraceStep::Resolution(ResolutionKind::Unresolved));
                PosResolution::Unresolved
            }
            1 => {
                trace.push(AnalysisTraceStep::Resolution(ResolutionKind::Resolved));
                PosResolution::Resolved(preferred[0].clone())
            }
            _ => {
                trace.push(AnalysisTraceStep::Resolution(ResolutionKind::Ambiguous));
                PosResolution::Ambiguous(preferred.clone())
            }
        };

        let analysis = TokenAnalysis {
            token: token.clone(),
            candidates: preferred,
            resolution,
            state_trace: trace,
        };

        state = next_state(&analysis, &lower);
        analyses.push(analysis);
    }

    analyses
}

fn generate_candidates<L: PosLexiconProvider + ?Sized>(
    lower: &str,
    lexicon: &L,
) -> Vec<PosCandidate> {
    let mut candidates = lexicon.candidates_for(lower);

    for morphology in morphology_candidates(lower) {
        candidates.push(morphology);
    }

    dedupe_candidates(candidates)
}

fn morphology_candidates(lower: &str) -> Vec<PosCandidate> {
    let mut out = Vec::new();

    if lower.is_empty() {
        return out;
    }

    if lower.chars().all(|ch| ch.is_ascii_digit()) {
        out.push(PosCandidate {
            lemma: lower.to_string(),
            pos: PosClass::Number,
            source: CandidateSource::Morphology,
        });
        return out;
    }

    if matches!(lower, "the" | "a" | "an") {
        out.push(PosCandidate {
            lemma: lower.to_string(),
            pos: PosClass::Determiner,
            source: CandidateSource::Morphology,
        });
    }

    if matches!(lower, "shall" | "must" | "can" | "may" | "will") {
        out.push(PosCandidate {
            lemma: lower.to_string(),
            pos: PosClass::Modal,
            source: CandidateSource::Morphology,
        });
    }

    if matches!(
        lower,
        "if" | "when" | "while" | "where" | "then" | "and" | "or" | "but"
    ) {
        out.push(PosCandidate {
            lemma: lower.to_string(),
            pos: PosClass::Conjunction,
            source: CandidateSource::Morphology,
        });
    }

    if matches!(
        lower,
        "to" | "of" | "in" | "on" | "at" | "by" | "with" | "from" | "for"
    ) {
        out.push(PosCandidate {
            lemma: lower.to_string(),
            pos: PosClass::Preposition,
            source: CandidateSource::Morphology,
        });
    }

    if matches!(
        lower,
        "it" | "they" | "them" | "this" | "that" | "these" | "those" | "he" | "she"
    ) {
        out.push(PosCandidate {
            lemma: lower.to_string(),
            pos: PosClass::Pronoun,
            source: CandidateSource::Morphology,
        });
    }

    if lower.ends_with("ly") {
        out.push(PosCandidate {
            lemma: lower.trim_end_matches("ly").to_string(),
            pos: PosClass::Adverb,
            source: CandidateSource::Morphology,
        });
    }

    if lower.ends_with("tion") || lower.ends_with("ment") {
        out.push(PosCandidate {
            lemma: lower.to_string(),
            pos: PosClass::Noun,
            source: CandidateSource::Morphology,
        });
    }

    if lower.ends_with("ive") || lower.ends_with("ous") || lower.ends_with("al") {
        out.push(PosCandidate {
            lemma: lower.to_string(),
            pos: PosClass::Adjective,
            source: CandidateSource::Morphology,
        });
    }

    if lower.ends_with("ize") || lower.ends_with("ise") {
        out.push(PosCandidate {
            lemma: lower.to_string(),
            pos: PosClass::Verb,
            source: CandidateSource::Morphology,
        });
    }

    if lower.ends_with("ing") {
        let stem = lower.trim_end_matches("ing");
        if !stem.is_empty() {
            out.push(PosCandidate {
                lemma: stem.to_string(),
                pos: PosClass::Verb,
                source: CandidateSource::Morphology,
            });
            out.push(PosCandidate {
                lemma: stem.to_string(),
                pos: PosClass::Participle,
                source: CandidateSource::Morphology,
            });
        }
    }

    if lower.ends_with("ed") {
        let stem = lower.trim_end_matches("ed");
        if !stem.is_empty() {
            out.push(PosCandidate {
                lemma: stem.to_string(),
                pos: PosClass::Verb,
                source: CandidateSource::Morphology,
            });
            out.push(PosCandidate {
                lemma: stem.to_string(),
                pos: PosClass::Participle,
                source: CandidateSource::Morphology,
            });
        }
    }

    if out.is_empty() {
        out.push(PosCandidate {
            lemma: lower.to_string(),
            pos: PosClass::Unknown,
            source: CandidateSource::Morphology,
        });
    }

    out
}

fn apply_state_constraint(
    candidates: &[PosCandidate],
    state: DeterministicPosState,
    mode: Option<WritingMode>,
    lower: &str,
) -> Vec<PosCandidate> {
    let filtered = candidates
        .iter()
        .filter(|candidate| candidate_allowed_in_state(candidate.pos, state))
        .cloned()
        .collect::<Vec<_>>();

    let mut state_filtered = filtered;

    if matches!(
        mode,
        Some(WritingMode::Procedural | WritingMode::SafetyInstruction)
    ) && matches!(state, DeterministicPosState::ClauseStart)
        && lower
            .chars()
            .all(|ch| ch.is_ascii_alphabetic() || ch == '-')
    {
        let imperative = state_filtered
            .iter()
            .filter(|candidate| matches!(candidate.pos, PosClass::Verb))
            .cloned()
            .collect::<Vec<_>>();
        if !imperative.is_empty() {
            state_filtered = imperative;
        }
    }

    dedupe_candidates(state_filtered)
}

fn candidate_allowed_in_state(pos: PosClass, state: DeterministicPosState) -> bool {
    match state {
        DeterministicPosState::ClauseStart | DeterministicPosState::AfterPunctuation => true,
        DeterministicPosState::AfterDeterminer => {
            matches!(pos, PosClass::Noun | PosClass::Adjective | PosClass::Number)
        }
        DeterministicPosState::AfterModal => matches!(pos, PosClass::Verb),
        DeterministicPosState::AfterPreposition => matches!(
            pos,
            PosClass::Determiner
                | PosClass::Adjective
                | PosClass::Noun
                | PosClass::Pronoun
                | PosClass::Number
        ),
        DeterministicPosState::AfterCopula => {
            matches!(
                pos,
                PosClass::Adjective | PosClass::Noun | PosClass::Participle
            )
        }
        DeterministicPosState::AfterConjunction => true,
    }
}

fn prefer_lexicon(candidates: Vec<PosCandidate>) -> Vec<PosCandidate> {
    let lexicon = candidates
        .iter()
        .filter(|candidate| matches!(candidate.source, CandidateSource::Lexicon))
        .cloned()
        .collect::<Vec<_>>();
    if lexicon.is_empty() {
        dedupe_candidates(candidates)
    } else {
        dedupe_candidates(lexicon)
    }
}

fn next_state(analysis: &TokenAnalysis, lower: &str) -> DeterministicPosState {
    if matches!(lower, "and" | "or" | "but") {
        return DeterministicPosState::AfterConjunction;
    }

    if matches!(lower, "if" | "when" | "while" | "where" | "then") {
        return DeterministicPosState::ClauseStart;
    }

    match &analysis.resolution {
        PosResolution::Resolved(candidate) => match candidate.pos {
            PosClass::Determiner => DeterministicPosState::AfterDeterminer,
            PosClass::Modal => DeterministicPosState::AfterModal,
            PosClass::Preposition => DeterministicPosState::AfterPreposition,
            PosClass::Conjunction => DeterministicPosState::AfterConjunction,
            PosClass::Verb if matches!(lower, "is" | "are" | "was" | "were" | "be") => {
                DeterministicPosState::AfterCopula
            }
            _ => DeterministicPosState::ClauseStart,
        },
        _ => DeterministicPosState::ClauseStart,
    }
}

fn dedupe_candidates(candidates: Vec<PosCandidate>) -> Vec<PosCandidate> {
    let mut seen = HashSet::new();
    let mut out = Vec::new();

    for candidate in candidates {
        if seen.insert((candidate.lemma.clone(), candidate.pos, candidate.source)) {
            out.push(candidate);
        }
    }

    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;

    struct TestLexicon {
        entries: HashMap<String, Vec<PosCandidate>>,
    }

    impl PosLexiconProvider for TestLexicon {
        fn candidates_for(&self, surface: &str) -> Vec<PosCandidate> {
            self.entries.get(surface).cloned().unwrap_or_default()
        }
    }

    fn token(text: &str, start: usize) -> Token {
        Token {
            text: text.to_string(),
            range: goodwrite_core::SourceRange::new(start, start + text.len()),
        }
    }

    #[test]
    fn after_determiner_prefers_noun() {
        let mut entries = HashMap::new();
        entries.insert(
            "control".to_string(),
            vec![
                PosCandidate {
                    lemma: "control".to_string(),
                    pos: PosClass::Noun,
                    source: CandidateSource::Lexicon,
                },
                PosCandidate {
                    lemma: "control".to_string(),
                    pos: PosClass::Verb,
                    source: CandidateSource::Lexicon,
                },
            ],
        );

        let lexicon = TestLexicon { entries };
        let ctx = DeterministicPosContext {
            lexicon: &lexicon,
            mode: Some(WritingMode::Descriptive),
        };

        let analyses = analyze_tokens(&[token("the", 0), token("control", 4)], &ctx);
        let second = &analyses[1];
        assert!(matches!(
            second.resolution,
            PosResolution::Resolved(PosCandidate {
                pos: PosClass::Noun,
                ..
            })
        ));
    }

    #[test]
    fn after_modal_requires_verb() {
        let mut entries = HashMap::new();
        entries.insert(
            "open".to_string(),
            vec![PosCandidate {
                lemma: "open".to_string(),
                pos: PosClass::Verb,
                source: CandidateSource::Lexicon,
            }],
        );

        let lexicon = TestLexicon { entries };
        let ctx = DeterministicPosContext {
            lexicon: &lexicon,
            mode: Some(WritingMode::Procedural),
        };

        let analyses = analyze_tokens(&[token("shall", 0), token("open", 6)], &ctx);
        let second = &analyses[1];
        assert!(matches!(
            second.resolution,
            PosResolution::Resolved(PosCandidate {
                pos: PosClass::Verb,
                ..
            })
        ));
    }

    #[test]
    fn unresolved_when_no_valid_candidate() {
        let mut entries = HashMap::new();
        entries.insert(
            "panel".to_string(),
            vec![PosCandidate {
                lemma: "panel".to_string(),
                pos: PosClass::Noun,
                source: CandidateSource::Lexicon,
            }],
        );

        let lexicon = TestLexicon { entries };
        let ctx = DeterministicPosContext {
            lexicon: &lexicon,
            mode: Some(WritingMode::Procedural),
        };

        let analyses = analyze_tokens(&[token("shall", 0), token("panel", 6)], &ctx);
        assert!(matches!(analyses[1].resolution, PosResolution::Unresolved));
    }

    #[test]
    fn unresolved_when_ambiguous_without_constraints() {
        let mut entries = HashMap::new();
        entries.insert(
            "set".to_string(),
            vec![
                PosCandidate {
                    lemma: "set".to_string(),
                    pos: PosClass::Noun,
                    source: CandidateSource::Lexicon,
                },
                PosCandidate {
                    lemma: "set".to_string(),
                    pos: PosClass::Verb,
                    source: CandidateSource::Lexicon,
                },
            ],
        );

        let lexicon = TestLexicon { entries };
        let ctx = DeterministicPosContext {
            lexicon: &lexicon,
            mode: Some(WritingMode::Descriptive),
        };

        let analyses = analyze_tokens(&[token("set", 0)], &ctx);
        assert!(matches!(
            analyses[0].resolution,
            PosResolution::Ambiguous(_)
        ));
    }
}
