use std::collections::{BTreeMap, BTreeSet};

use serde::{Deserialize, Serialize};
use thiserror::Error;

/// Canonical rule index used by docs, CI accountability checks, and website rendering.
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct RuleIndex {
    #[serde(default)]
    pub rules: Vec<RuleIndexEntry>,
}

impl RuleIndex {
    pub fn from_toml(value: &str) -> Result<Self, RuleIndexError> {
        toml::from_str(value).map_err(RuleIndexError::Parse)
    }

    pub fn load_embedded() -> Result<Self, RuleIndexError> {
        Self::from_toml(include_str!("../data/rule_index.toml"))
    }

    pub fn by_id(&self) -> BTreeMap<&str, &RuleIndexEntry> {
        self.rules
            .iter()
            .map(|entry| (entry.id.as_str(), entry))
            .collect()
    }

    pub fn duplicate_ids(&self) -> BTreeSet<&str> {
        let mut seen = BTreeSet::new();
        let mut duplicates = BTreeSet::new();
        for entry in &self.rules {
            if !seen.insert(entry.id.as_str()) {
                duplicates.insert(entry.id.as_str());
            }
        }
        duplicates
    }
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct RuleIndexEntry {
    pub id: String,
    pub profile: String,
    pub title: String,
    pub standard: String,
    pub part: String,
    pub section_number: String,
    pub section_name: String,
    pub rule_number: String,
    pub citation: String,
    pub test_pass: String,
    pub test_fail: String,
}

#[derive(Debug, Error)]
pub enum RuleIndexError {
    #[error("failed to parse rule index TOML")]
    Parse(#[from] toml::de::Error),
}
