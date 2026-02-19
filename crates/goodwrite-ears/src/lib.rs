//! EARS profile checks.

pub mod clause_order;
pub mod patterns;
pub mod structure;
pub mod vagueness;

use std::sync::Arc;

use goodwrite_core::Rule;

pub fn rules() -> Vec<Arc<dyn Rule>> {
    vec![
        Arc::new(structure::InvalidRequirementTypeRule),
        Arc::new(structure::MissingShallRule),
        Arc::new(structure::MultipleShallRule),
        Arc::new(structure::MissingSystemNameRule),
        Arc::new(clause_order::ClauseOrderRule),
        Arc::new(structure::MissingPatternRule),
        Arc::new(structure::MissingConditionKeywordRule),
        Arc::new(structure::PassiveShallRule),
        Arc::new(vagueness::UntestableResponseRule),
        Arc::new(vagueness::AmbiguousTriggerRule),
        Arc::new(vagueness::VaguePreconditionRule),
    ]
}
