use std::collections::HashSet;

use serde::{Deserialize, Serialize};

use crate::config::GoodwriteConfig;

/// Glossary term from glossary profile.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct GlossaryTerm {
    pub canonical: String,
    pub synonyms: Vec<String>,
    pub pos: Option<String>,
    pub category: Option<String>,
}

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
    #[serde(default)]
    pub terms: Vec<GlossaryTerm>,
}

/// In-memory glossary lookups shared with rules.
#[derive(Debug, Clone, Default)]
pub struct GlossaryData {
    terms: Vec<GlossaryTerm>,
    canonicals: HashSet<String>,
    synonyms: Vec<(String, String)>,
}

impl GlossaryData {
    pub fn new(terms: Vec<GlossaryTerm>) -> Self {
        let canonicals = terms
            .iter()
            .map(|term| term.canonical.to_ascii_lowercase())
            .collect::<HashSet<_>>();

        let mut synonyms = Vec::new();
        for term in &terms {
            for synonym in &term.synonyms {
                synonyms.push((synonym.to_ascii_lowercase(), term.canonical.clone()));
            }
        }

        Self {
            terms,
            canonicals,
            synonyms,
        }
    }

    pub fn terms(&self) -> &[GlossaryTerm] {
        &self.terms
    }

    pub fn has_term(&self, word: &str) -> bool {
        self.canonicals.contains(&word.to_ascii_lowercase())
    }

    pub fn canonical_for_synonym(&self, value: &str) -> Option<&str> {
        let lower = value.to_ascii_lowercase();
        self.synonyms
            .iter()
            .find(|(synonym, _)| synonym == &lower)
            .map(|(_, canonical)| canonical.as_str())
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
