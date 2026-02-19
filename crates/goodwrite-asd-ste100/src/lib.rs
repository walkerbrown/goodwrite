//! ASD-STE100 profile rules.

pub mod compliance;
pub mod descriptive;
pub mod dict;
pub mod guidance;
pub mod nouns;
pub mod practices;
pub mod procedural;
pub mod punctuation;
pub mod safety;
pub mod sentences;
pub mod verbs;
pub mod words;

use std::sync::Arc;

use goodwrite_core::Rule;

/// Register implemented ASD-STE100 rules.
pub fn rules() -> Vec<Arc<dyn Rule>> {
    vec![
        Arc::new(words::UnapprovedWordRule),
        Arc::new(words::ApprovedWordPosMismatchRule),
        Arc::new(words::AmbiguousPosRule),
        Arc::new(words::AlternativePosMismatchRule),
        Arc::new(words::ApprovedMeaningRule),
        Arc::new(words::ApprovedFormRule),
        Arc::new(words::TechnicalNounCategoryRule),
        Arc::new(words::NonApprovedTechnicalNounRule),
        Arc::new(words::TechnicalNounAsVerbRule),
        Arc::new(words::CompanyApprovedTechnicalNounRule),
        Arc::new(words::TechnicalNounLengthRule),
        Arc::new(words::NoJargonTechnicalNounRule),
        Arc::new(words::ConsistentTechnicalNounRule),
        Arc::new(words::TechnicalVerbCategoryRule),
        Arc::new(words::TechnicalVerbAsNounRule),
        Arc::new(words::AmericanSpellingRule),
        Arc::new(nouns::MultiWordNounLengthRule),
        Arc::new(nouns::AbbreviationFirstUseRule),
        Arc::new(verbs::VerbFormRule),
        Arc::new(verbs::ApprovedTensesRule),
        Arc::new(verbs::PastParticipleAsAdjectiveRule),
        Arc::new(verbs::NoComplexAuxiliaryRule),
        Arc::new(verbs::IngFormRestrictionRule),
        Arc::new(verbs::PassiveVoiceRule),
        Arc::new(verbs::NominalizationRule),
        Arc::new(sentences::ContractionRule),
        Arc::new(sentences::UseVerticalListRule),
        Arc::new(sentences::ConnectingWordsRule),
        Arc::new(sentences::ArticlesBeforeNounsRule),
        Arc::new(procedural::SentenceLengthProceduralRule),
        Arc::new(procedural::OneInstructionPerSentenceRule),
        Arc::new(procedural::ImperativeProceduralRule),
        Arc::new(procedural::ConditionBeforeCommandRule),
        Arc::new(procedural::NoteNoImperativeRule),
        Arc::new(descriptive::SentenceLengthDescriptiveRule),
        Arc::new(descriptive::OneSubjectPerSentenceRule),
        Arc::new(descriptive::KeyWordsForStructureRule),
        Arc::new(descriptive::RelatedInfoParagraphRule),
        Arc::new(descriptive::OneTopicPerParagraphRule),
        Arc::new(descriptive::MaxSentencesPerParagraphRule),
        Arc::new(safety::SafetyRiskLevelRule),
        Arc::new(safety::SafetyCommandOrConditionRule),
        Arc::new(safety::SafetyRiskExplanationRule),
        Arc::new(punctuation::SemicolonRule),
        Arc::new(punctuation::HyphenRelatedWordsRule),
        Arc::new(punctuation::ParenthesesUsageRule),
        Arc::new(practices::ThatWhichRule),
        Arc::new(practices::AmbiguousWithRule),
        Arc::new(practices::PronounAntecedentRule),
        Arc::new(practices::ThisReferentRule),
        Arc::new(practices::FalseFriendsRule),
        Arc::new(practices::LatinAbbreviationRule),
        Arc::new(practices::InclusiveLanguageRule),
        Arc::new(practices::PossessiveFormRule),
    ]
}
