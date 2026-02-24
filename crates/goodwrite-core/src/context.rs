use std::collections::HashSet;

use serde::{Deserialize, Serialize};

use crate::config::GoodwriteConfig;

/// One glossary alternative entry, mirroring glossary.toml shape.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct GlossaryAlternative {
    pub word: String,
    pub pos: Option<String>,
    pub context: Option<String>,
}

/// One approved glossary entry.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct GlossaryApprovedEntry {
    pub word: String,
    pub pos: String,
    #[serde(default)]
    pub forms: Vec<String>,
    #[serde(default)]
    pub approved_meaning: String,
    #[serde(default)]
    pub goodwrite_example: String,
    #[serde(default)]
    pub wrongwrite_example: String,
}

/// One non-approved glossary entry.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct GlossaryNotApprovedEntry {
    pub word: String,
    pub pos: String,
    #[serde(default)]
    pub alternatives: Vec<GlossaryAlternative>,
    #[serde(default)]
    pub approved_meaning: String,
    #[serde(default)]
    pub goodwrite_example: String,
    #[serde(default)]
    pub wrongwrite_example: String,
}

/// Parsed glossary payload loaded from `glossary.toml`.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct GlossaryFileData {
    #[serde(default)]
    pub approved: Vec<GlossaryApprovedEntry>,
    #[serde(default, rename = "not_approved")]
    pub not_approved: Vec<GlossaryNotApprovedEntry>,
}

/// In-memory glossary lookups shared with rules.
#[derive(Debug, Clone, Default)]
pub struct GlossaryData {
    approved: Vec<GlossaryApprovedEntry>,
    not_approved: Vec<GlossaryNotApprovedEntry>,
    canonicals: HashSet<String>,
}

impl GlossaryData {
    pub fn new(
        approved: Vec<GlossaryApprovedEntry>,
        not_approved: Vec<GlossaryNotApprovedEntry>,
    ) -> Self {
        let canonicals = approved
            .iter()
            .map(|entry| entry.word.to_ascii_lowercase())
            .collect::<HashSet<_>>();

        Self {
            approved,
            not_approved,
            canonicals,
        }
    }

    pub fn approved(&self) -> &[GlossaryApprovedEntry] {
        &self.approved
    }

    pub fn not_approved(&self) -> &[GlossaryNotApprovedEntry] {
        &self.not_approved
    }

    pub fn has_term(&self, word: &str) -> bool {
        self.canonicals.contains(&word.to_ascii_lowercase())
    }
}

/// Shared context passed to each rule evaluation.
#[derive(Debug, Clone)]
pub struct CheckContext {
    pub config: GoodwriteConfig,
    pub glossary: Option<GlossaryData>,
    pub glossary_data: Option<GlossaryFileData>,
    pub file_has_mode_annotations: bool,
}

impl CheckContext {
    pub fn profile_enabled(&self, name: &str) -> bool {
        self.config
            .profiles
            .enable
            .iter()
            .any(|profile| profile.eq_ignore_ascii_case(name))
    }
}
