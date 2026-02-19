use std::sync::Arc;

use goodwrite_core::{CheckContext, GlossaryData, Sentence, Token, WritingMode};
use goodwrite_tokenize::{
    CandidateSource, DeterministicPosContext, PosCandidate, PosClass, PosLexiconProvider,
    PosResolution, TokenAnalysis, analyze_tokens,
};

use crate::dict::lookup::{DictionaryLookup, LookupAlternative};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ComplianceState {
    Compliant,
    NeedsRewrite,
    NeedsGlossaryAction,
    Ambiguous,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ComplianceAction {
    UseWord,
    UseAlternative { word: String },
    DoNotUseWord,
    RewriteSentence { reason: String },
    AddToGlossary { term: String, category: String },
    ConfirmMeaning,
    AlternativePosMismatch,
    ClarifyPartOfSpeech,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ComplianceStep {
    pub node: &'static str,
    pub outcome: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct ComplianceTrace {
    pub steps: Vec<ComplianceStep>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ComplianceDecision {
    pub token: Token,
    pub normalized: String,
    pub analysis: TokenAnalysis,
    pub state: ComplianceState,
    pub actions: Vec<ComplianceAction>,
    pub trace: ComplianceTrace,
}

pub struct ComplianceEngine<'a> {
    ctx: &'a CheckContext,
    dictionary: Arc<DictionaryLookup>,
}

impl<'a> ComplianceEngine<'a> {
    pub fn new(ctx: &'a CheckContext) -> Self {
        Self {
            ctx,
            dictionary: DictionaryLookup::for_overlay(ctx.glossary_data.as_ref()),
        }
    }

    pub fn analyze_sentence(
        &self,
        sentence: &Sentence,
        mode: WritingMode,
    ) -> Vec<ComplianceDecision> {
        let analyses = analyze_sentence_tokens(sentence, mode, self.ctx);
        analyses
            .into_iter()
            .map(|analysis| self.evaluate_token(mode, analysis))
            .collect()
    }

    fn evaluate_token(&self, mode: WritingMode, analysis: TokenAnalysis) -> ComplianceDecision {
        let token = analysis.token.clone();
        let lower = token.text.to_ascii_lowercase();
        let mut trace = ComplianceTrace::default();

        if !is_word_token(&token.text) {
            trace.steps.push(ComplianceStep {
                node: "lexical-token",
                outcome: "non-word token bypassed".to_string(),
            });
            return ComplianceDecision {
                token,
                normalized: lower,
                analysis,
                state: ComplianceState::Compliant,
                actions: vec![ComplianceAction::UseWord],
                trace,
            };
        }

        let in_dictionary = self.dictionary.is_known_word(&lower);
        trace.steps.push(ComplianceStep {
            node: "in_dictionary",
            outcome: in_dictionary.to_string(),
        });

        let approved = self.dictionary.is_approved_form(&lower);
        trace.steps.push(ComplianceStep {
            node: "approved",
            outcome: approved.to_string(),
        });

        if approved {
            return self.evaluate_approved(mode, analysis, trace, lower, token);
        }

        if in_dictionary {
            return self.evaluate_not_approved(mode, analysis, trace, lower, token);
        }

        self.evaluate_not_in_dictionary(mode, analysis, trace, lower, token)
    }

    fn evaluate_approved(
        &self,
        _mode: WritingMode,
        analysis: TokenAnalysis,
        mut trace: ComplianceTrace,
        lower: String,
        token: Token,
    ) -> ComplianceDecision {
        let same_pos =
            resolved_pos_matches_dictionary(&self.dictionary, &lower, &analysis.resolution);
        trace.steps.push(ComplianceStep {
            node: "same_pos",
            outcome: match same_pos {
                Some(value) => value.to_string(),
                None => "ambiguous".to_string(),
            },
        });

        match same_pos {
            Some(true) => {
                let mut actions = vec![ComplianceAction::UseWord];
                if meaning_confirmation_required(&lower) {
                    trace.steps.push(ComplianceStep {
                        node: "meaning_confirmation_required",
                        outcome: "true".to_string(),
                    });
                    actions.push(ComplianceAction::ConfirmMeaning);
                }

                ComplianceDecision {
                    token,
                    normalized: lower,
                    analysis,
                    state: ComplianceState::Compliant,
                    actions,
                    trace,
                }
            }
            Some(false) => ComplianceDecision {
                token,
                normalized: lower,
                analysis,
                state: ComplianceState::NeedsRewrite,
                actions: vec![
                    ComplianceAction::ClarifyPartOfSpeech,
                    ComplianceAction::RewriteSentence {
                        reason: "approved word is used with non-approved part-of-speech"
                            .to_string(),
                    },
                ],
                trace,
            },
            None => ComplianceDecision {
                token,
                normalized: lower,
                analysis,
                state: ComplianceState::Ambiguous,
                actions: vec![
                    ComplianceAction::ClarifyPartOfSpeech,
                    ComplianceAction::RewriteSentence {
                        reason: "part-of-speech cannot be deterministically resolved".to_string(),
                    },
                ],
                trace,
            },
        }
    }

    fn evaluate_not_approved(
        &self,
        _mode: WritingMode,
        analysis: TokenAnalysis,
        mut trace: ComplianceTrace,
        lower: String,
        token: Token,
    ) -> ComplianceDecision {
        let alternatives = self.dictionary.alternatives_for_word(&lower).unwrap_or(&[]);
        trace.steps.push(ComplianceStep {
            node: "alternatives_present",
            outcome: (!alternatives.is_empty()).to_string(),
        });

        let resolved = resolved_pos_class(&analysis.resolution);
        trace.steps.push(ComplianceStep {
            node: "resolved_pos",
            outcome: resolved
                .map(|pos| pos.as_str().to_string())
                .unwrap_or_else(|| "ambiguous".to_string()),
        });

        if let Some(observed_pos) = resolved {
            let alternatives_for_pos = self
                .dictionary
                .alternatives_for_word_by_pos(&lower, observed_pos.as_str())
                .unwrap_or(alternatives);

            if let Some(alternative) =
                first_same_pos_alternative(&self.dictionary, alternatives_for_pos, observed_pos)
            {
                trace.steps.push(ComplianceStep {
                    node: "alternative_same_pos",
                    outcome: "true".to_string(),
                });

                return ComplianceDecision {
                    token,
                    normalized: lower,
                    analysis,
                    state: ComplianceState::NeedsRewrite,
                    actions: vec![ComplianceAction::UseAlternative {
                        word: alternative.word.clone(),
                    }],
                    trace,
                };
            }

            trace.steps.push(ComplianceStep {
                node: "alternative_same_pos",
                outcome: "false".to_string(),
            });

            return ComplianceDecision {
                token,
                normalized: lower,
                analysis,
                state: ComplianceState::NeedsRewrite,
                actions: vec![
                    ComplianceAction::AlternativePosMismatch,
                    ComplianceAction::RewriteSentence {
                        reason: "no same-POS approved alternative is available".to_string(),
                    },
                ],
                trace,
            };
        }

        trace.steps.push(ComplianceStep {
            node: "alternative_same_pos",
            outcome: "ambiguous".to_string(),
        });

        ComplianceDecision {
            token,
            normalized: lower,
            analysis,
            state: ComplianceState::Ambiguous,
            actions: vec![
                ComplianceAction::ClarifyPartOfSpeech,
                ComplianceAction::RewriteSentence {
                    reason: "part-of-speech is ambiguous for non-approved word".to_string(),
                },
            ],
            trace,
        }
    }

    fn evaluate_not_in_dictionary(
        &self,
        _mode: WritingMode,
        analysis: TokenAnalysis,
        mut trace: ComplianceTrace,
        lower: String,
        token: Token,
    ) -> ComplianceDecision {
        let is_technical = technical_noun_or_verb(self.ctx.glossary.as_ref(), &lower);
        trace.steps.push(ComplianceStep {
            node: "technical_noun_or_verb",
            outcome: is_technical.to_string(),
        });

        if is_technical {
            return ComplianceDecision {
                token,
                normalized: lower,
                analysis,
                state: ComplianceState::Compliant,
                actions: vec![ComplianceAction::UseWord],
                trace,
            };
        }

        if looks_glossary_candidate(&token.text) {
            return ComplianceDecision {
                token,
                normalized: lower.clone(),
                analysis,
                state: ComplianceState::NeedsGlossaryAction,
                actions: vec![ComplianceAction::AddToGlossary {
                    term: lower,
                    category: "technical-noun-or-technical-verb".to_string(),
                }],
                trace,
            };
        }

        ComplianceDecision {
            token,
            normalized: lower,
            analysis,
            state: ComplianceState::NeedsRewrite,
            actions: vec![
                ComplianceAction::DoNotUseWord,
                ComplianceAction::RewriteSentence {
                    reason: "word is outside dictionary and not an approved technical term"
                        .to_string(),
                },
            ],
            trace,
        }
    }
}

pub fn analyze_sentence_tokens(
    sentence: &Sentence,
    mode: WritingMode,
    ctx: &CheckContext,
) -> Vec<TokenAnalysis> {
    let lexicon = ComplianceLexicon {
        dictionary: DictionaryLookup::for_overlay(ctx.glossary_data.as_ref()),
        glossary: ctx.glossary.as_ref(),
    };
    let analyze_context = DeterministicPosContext {
        lexicon: &lexicon,
        mode: Some(mode),
    };
    analyze_tokens(&sentence.tokens, &analyze_context)
}

fn resolved_pos_matches_dictionary(
    dictionary: &DictionaryLookup,
    word: &str,
    resolution: &PosResolution,
) -> Option<bool> {
    let observed = resolved_pos_class(resolution)?;
    let allowed = dictionary.allowed_pos(word)?;
    Some(allowed.contains(observed.as_str()))
}

fn resolved_pos_class(resolution: &PosResolution) -> Option<PosClass> {
    match resolution {
        PosResolution::Resolved(candidate) => Some(candidate.pos),
        PosResolution::Ambiguous(_) | PosResolution::Unresolved => None,
    }
}

fn first_same_pos_alternative<'a>(
    dictionary: &DictionaryLookup,
    alternatives: &'a [LookupAlternative],
    observed_pos: PosClass,
) -> Option<&'a LookupAlternative> {
    for alternative in alternatives {
        if let Some(pos) = &alternative.pos {
            if PosClass::from_dictionary_tag(pos).is_some_and(|candidate| candidate == observed_pos)
            {
                return Some(alternative);
            }
            continue;
        }

        if dictionary
            .allowed_pos(&alternative.word)
            .is_some_and(|allowed| allowed.contains(observed_pos.as_str()))
        {
            return Some(alternative);
        }
    }

    None
}

fn technical_noun_or_verb(glossary: Option<&GlossaryData>, lower: &str) -> bool {
    glossary
        .and_then(|loaded| {
            loaded
                .terms()
                .iter()
                .find(|term| term.canonical.eq_ignore_ascii_case(lower))
        })
        .is_some_and(|term| {
            matches!(
                term.pos
                    .as_deref()
                    .unwrap_or_default()
                    .to_ascii_lowercase()
                    .as_str(),
                "noun" | "verb"
            )
        })
}

fn looks_glossary_candidate(token: &str) -> bool {
    token.len() >= 3
        && token
            .chars()
            .all(|ch| ch.is_ascii_alphanumeric() || ch == '-')
        && token
            .chars()
            .any(|ch| ch.is_ascii_uppercase() || ch.is_ascii_digit())
}

fn meaning_confirmation_required(word: &str) -> bool {
    matches!(word, "set" | "control" | "support" | "clear" | "normal")
}

fn is_word_token(token: &str) -> bool {
    token
        .chars()
        .all(|ch| ch.is_ascii_alphanumeric() || ch == '-' || ch == '\'')
}

struct ComplianceLexicon<'a> {
    dictionary: Arc<DictionaryLookup>,
    glossary: Option<&'a GlossaryData>,
}

impl PosLexiconProvider for ComplianceLexicon<'_> {
    fn candidates_for(&self, surface: &str) -> Vec<PosCandidate> {
        let lower = surface.to_ascii_lowercase();
        let mut candidates = Vec::new();

        if let Some(entries) = self.dictionary.pos_candidates_for(&lower) {
            for entry in entries {
                let Some(pos) = PosClass::from_dictionary_tag(&entry.pos) else {
                    continue;
                };

                candidates.push(PosCandidate {
                    lemma: entry.lemma.clone(),
                    pos,
                    source: CandidateSource::Lexicon,
                });
            }
        }

        if let Some(glossary) = self.glossary {
            for term in glossary.terms() {
                if !term.canonical.eq_ignore_ascii_case(&lower) {
                    continue;
                }

                let pos = term
                    .pos
                    .as_deref()
                    .and_then(PosClass::from_dictionary_tag)
                    .unwrap_or(PosClass::Noun);

                candidates.push(PosCandidate {
                    lemma: lower.clone(),
                    pos,
                    source: CandidateSource::Lexicon,
                });
            }
        }

        candidates
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use goodwrite_core::{GlossaryData, GlossaryTerm, GoodwriteConfig, SourceRange};

    fn token(text: &str, start: usize) -> Token {
        Token {
            text: text.to_string(),
            range: SourceRange::new(start, start + text.len()),
        }
    }

    fn sentence(words: &[&str]) -> Sentence {
        let mut tokens = Vec::new();
        let mut cursor = 0usize;
        for word in words {
            tokens.push(token(word, cursor));
            cursor += word.len() + 1;
        }

        Sentence {
            text: words.join(" "),
            range: SourceRange::new(0, cursor.saturating_sub(1)),
            tokens,
            ste_word_count: words.len(),
        }
    }

    fn context() -> CheckContext {
        CheckContext {
            config: GoodwriteConfig::default(),
            glossary: None,
            glossary_data: None,
            file_has_mode_annotations: true,
        }
    }

    fn context_with_glossary(terms: Vec<GlossaryTerm>) -> CheckContext {
        CheckContext {
            config: GoodwriteConfig::default(),
            glossary: Some(GlossaryData::new(terms)),
            glossary_data: None,
            file_has_mode_annotations: true,
        }
    }

    #[test]
    fn approved_word_with_matching_pos_is_compliant() {
        let ctx = context();
        let engine = ComplianceEngine::new(&ctx);
        let sent = sentence(&["the", "system"]);
        let decisions = engine.analyze_sentence(&sent, WritingMode::Descriptive);
        assert!(decisions.iter().all(|decision| {
            decision.normalized == "the" || matches!(decision.state, ComplianceState::Compliant)
        }));
    }

    #[test]
    fn non_approved_word_with_same_pos_alternative_is_replacement() {
        let ctx = context();
        let engine = ComplianceEngine::new(&ctx);
        let sent = sentence(&["shall", "utilize"]);
        let decisions = engine.analyze_sentence(&sent, WritingMode::Procedural);
        let utilize = decisions
            .iter()
            .find(|decision| decision.normalized == "utilize")
            .expect("decision exists");

        assert!(matches!(utilize.state, ComplianceState::NeedsRewrite));
        assert!(
            utilize
                .actions
                .iter()
                .any(|action| matches!(action, ComplianceAction::UseAlternative { .. }))
        );
    }

    #[test]
    fn approved_word_with_mismatched_pos_requires_rewrite() {
        let ctx = context_with_glossary(vec![GlossaryTerm {
            canonical: "system".to_string(),
            synonyms: Vec::new(),
            pos: Some("verb".to_string()),
            category: Some("technical-verb".to_string()),
        }]);
        let engine = ComplianceEngine::new(&ctx);
        let sent = sentence(&["shall", "system"]);
        let decisions = engine.analyze_sentence(&sent, WritingMode::Procedural);
        let system = decisions
            .iter()
            .find(|decision| decision.normalized == "system")
            .expect("decision exists");

        assert!(matches!(system.state, ComplianceState::NeedsRewrite));
        assert!(
            system
                .actions
                .iter()
                .any(|action| matches!(action, ComplianceAction::ClarifyPartOfSpeech))
        );
        assert!(
            system
                .trace
                .steps
                .iter()
                .any(|step| step.node == "same_pos")
        );
    }

    #[test]
    fn non_approved_word_without_same_pos_alternative_flags_mismatch() {
        let ctx = context();
        let engine = ComplianceEngine::new(&ctx);
        let sent = sentence(&["ability", "systems", "are", "active"]);
        let decisions = engine.analyze_sentence(&sent, WritingMode::Descriptive);
        let ability = decisions
            .iter()
            .find(|decision| decision.normalized == "ability")
            .expect("decision exists");

        assert!(matches!(ability.state, ComplianceState::NeedsRewrite));
        assert!(
            ability
                .actions
                .iter()
                .any(|action| matches!(action, ComplianceAction::AlternativePosMismatch))
        );
        assert!(
            ability
                .trace
                .steps
                .iter()
                .any(|step| { step.node == "alternative_same_pos" && step.outcome == "false" })
        );
    }

    #[test]
    fn unresolved_pos_for_approved_word_is_ambiguous() {
        let ctx = context();
        let engine = ComplianceEngine::new(&ctx);
        let sent = sentence(&["shall", "control"]);
        let decisions = engine.analyze_sentence(&sent, WritingMode::Descriptive);
        let control = decisions
            .iter()
            .find(|decision| decision.normalized == "control")
            .expect("decision exists");

        assert!(matches!(control.state, ComplianceState::Ambiguous));
        assert!(
            control
                .actions
                .iter()
                .any(|action| matches!(action, ComplianceAction::ClarifyPartOfSpeech))
        );
        assert!(
            control
                .trace
                .steps
                .iter()
                .any(|step| { step.node == "same_pos" && step.outcome == "ambiguous" })
        );
    }
}
