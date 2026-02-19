//! Glossary profile support.

mod loader;
mod rules;

use std::{path::Path, sync::Arc};

use goodwrite_core::{GlossaryData, GlossaryFileData, Rule};

pub use loader::{GlossaryError, load_glossary, load_glossary_file};

/// Load glossary file into shared core representation.
pub fn load_glossary_data(path: &Path) -> Result<GlossaryData, GlossaryError> {
    loader::load_glossary(path)
}

/// Load complete glossary payload.
pub fn load_glossary_file_data(path: &Path) -> Result<GlossaryFileData, GlossaryError> {
    loader::load_glossary_file(path)
}

/// Register glossary profile rules.
pub fn rules() -> Vec<Arc<dyn Rule>> {
    vec![
        Arc::new(rules::UndefinedTermRule),
        Arc::new(rules::SynonymRule),
        Arc::new(rules::CasingRule),
    ]
}
