use goodwrite_core::WritingMode;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GuidanceScenario {
    PosMismatch,
    AmbiguousPos,
    AlternativePosMismatch,
    NonApprovedWord,
    GlossaryAction,
}

pub fn guidance_for(mode: WritingMode, scenario: GuidanceScenario) -> &'static str {
    match (mode, scenario) {
        (WritingMode::Procedural, GuidanceScenario::PosMismatch) => {
            "rewrite as imperative action with one approved verb at sentence start"
        }
        (WritingMode::Descriptive, GuidanceScenario::PosMismatch) => {
            "rewrite with explicit subject and approved descriptive verb form"
        }
        (WritingMode::SafetyInstruction, GuidanceScenario::PosMismatch) => {
            "rewrite as clear command/condition statement with approved verb usage"
        }
        (WritingMode::Note, GuidanceScenario::PosMismatch) => {
            "rewrite as descriptive information without imperative phrasing"
        }

        (WritingMode::Procedural, GuidanceScenario::AmbiguousPos) => {
            "replace with an explicit approved imperative verb to remove POS ambiguity"
        }
        (WritingMode::Descriptive, GuidanceScenario::AmbiguousPos) => {
            "replace with explicit noun/verb wording so part-of-speech is unambiguous"
        }
        (WritingMode::SafetyInstruction, GuidanceScenario::AmbiguousPos) => {
            "use explicit command and consequence wording to remove POS ambiguity"
        }
        (WritingMode::Note, GuidanceScenario::AmbiguousPos) => {
            "use explicit descriptive wording and avoid ambiguous shorthand"
        }

        (WritingMode::Procedural, GuidanceScenario::AlternativePosMismatch) => {
            "use a same-POS approved replacement or rewrite the instruction sentence"
        }
        (WritingMode::Descriptive, GuidanceScenario::AlternativePosMismatch) => {
            "use a same-POS approved replacement or restructure the sentence"
        }
        (WritingMode::SafetyInstruction, GuidanceScenario::AlternativePosMismatch) => {
            "use a same-POS approved replacement and keep command/condition clarity"
        }
        (WritingMode::Note, GuidanceScenario::AlternativePosMismatch) => {
            "use a same-POS approved replacement or descriptive rewrite"
        }

        (WritingMode::Procedural, GuidanceScenario::NonApprovedWord) => {
            "replace with an approved imperative verb or approved technical term"
        }
        (WritingMode::Descriptive, GuidanceScenario::NonApprovedWord) => {
            "replace with an approved descriptive word that preserves sentence role"
        }
        (WritingMode::SafetyInstruction, GuidanceScenario::NonApprovedWord) => {
            "replace with approved safety wording and keep the command explicit"
        }
        (WritingMode::Note, GuidanceScenario::NonApprovedWord) => {
            "replace with approved descriptive wording suitable for notes"
        }

        (_, GuidanceScenario::GlossaryAction) => {
            "add as technical noun/verb in glossary with category, or rewrite using approved dictionary terms"
        }
    }
}
