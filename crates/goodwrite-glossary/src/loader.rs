use std::{fs, path::Path};

use std::collections::HashSet;

use goodwrite_core::{
    GlossaryAlternative, GlossaryApprovedEntry, GlossaryData, GlossaryFileData,
    GlossaryNotApprovedEntry, GlossaryTerm,
};
use serde::Deserialize;
use thiserror::Error;

#[derive(Debug, Deserialize, Default)]
struct GlossaryFile {
    #[serde(default)]
    approved: Vec<RawApprovedEntry>,
    #[serde(default, rename = "not_approved")]
    not_approved: Vec<RawNotApprovedEntry>,
    #[serde(default)]
    terms: Vec<RawTerm>,
}

#[derive(Debug, Deserialize)]
struct RawApprovedEntry {
    word: String,
    pos: String,
    #[serde(default)]
    forms: Vec<String>,
    #[serde(default)]
    approved_meaning: String,
    #[serde(default)]
    goodwrite_example: String,
    #[serde(default)]
    wrongwrite_example: String,
}

#[derive(Debug, Deserialize)]
struct RawNotApprovedEntry {
    word: String,
    pos: String,
    #[serde(default)]
    alternatives: Vec<RawAlternative>,
    #[serde(default)]
    approved_meaning: String,
    #[serde(default)]
    goodwrite_example: String,
    #[serde(default)]
    wrongwrite_example: String,
}

#[derive(Debug, Deserialize)]
#[serde(untagged)]
enum RawAlternative {
    Word(String),
    Detailed {
        word: String,
        pos: Option<String>,
        context: Option<String>,
    },
}

#[derive(Debug, Deserialize)]
struct RawTerm {
    canonical: String,
    #[serde(default)]
    synonyms: Vec<String>,
    pos: Option<String>,
    category: Option<String>,
}

pub fn load_glossary_file(path: &Path) -> Result<GlossaryFileData, GlossaryError> {
    let source = fs::read_to_string(path).map_err(|source| GlossaryError::Read {
        path: path.display().to_string(),
        source,
    })?;

    let parsed: GlossaryFile = toml::from_str(&source).map_err(GlossaryError::Parse)?;

    let approved = parsed
        .approved
        .into_iter()
        .map(|entry| GlossaryApprovedEntry {
            word: entry.word,
            pos: entry.pos,
            forms: entry.forms,
            approved_meaning: entry.approved_meaning,
            goodwrite_example: entry.goodwrite_example,
            wrongwrite_example: entry.wrongwrite_example,
        })
        .collect::<Vec<_>>();

    let not_approved = parsed
        .not_approved
        .into_iter()
        .map(|entry| GlossaryNotApprovedEntry {
            word: entry.word,
            pos: entry.pos,
            alternatives: entry
                .alternatives
                .into_iter()
                .map(|alternative| match alternative {
                    RawAlternative::Word(word) => GlossaryAlternative {
                        word,
                        pos: None,
                        context: None,
                    },
                    RawAlternative::Detailed { word, pos, context } => {
                        GlossaryAlternative { word, pos, context }
                    }
                })
                .collect(),
            approved_meaning: entry.approved_meaning,
            goodwrite_example: entry.goodwrite_example,
            wrongwrite_example: entry.wrongwrite_example,
        })
        .collect::<Vec<_>>();

    let terms = parsed
        .terms
        .into_iter()
        .map(|term| GlossaryTerm {
            canonical: term.canonical,
            synonyms: term.synonyms,
            pos: term.pos,
            category: term.category,
        })
        .collect::<Vec<_>>();

    validate_glossary_entries(&approved, &not_approved)?;

    Ok(GlossaryFileData {
        approved,
        not_approved,
        terms,
    })
}

pub fn load_glossary(path: &Path) -> Result<GlossaryData, GlossaryError> {
    let parsed = load_glossary_file(path)?;
    Ok(GlossaryData::new(parsed.terms))
}

fn validate_glossary_entries(
    approved: &[GlossaryApprovedEntry],
    not_approved: &[GlossaryNotApprovedEntry],
) -> Result<(), GlossaryError> {
    let mut approved_keys = HashSet::new();
    for entry in approved {
        let key = (
            entry.word.trim().to_ascii_lowercase(),
            entry.pos.trim().to_ascii_lowercase(),
        );
        if key.0.is_empty() || key.1.is_empty() {
            return Err(GlossaryError::Validation(
                "approved entry must include non-empty word and pos".to_string(),
            ));
        }
        if !approved_keys.insert(key) {
            return Err(GlossaryError::Validation(
                "duplicate approved entry in glossary".to_string(),
            ));
        }
    }

    let mut not_approved_keys = HashSet::new();
    for entry in not_approved {
        let key = (
            entry.word.trim().to_ascii_lowercase(),
            entry.pos.trim().to_ascii_lowercase(),
        );
        if key.0.is_empty() || key.1.is_empty() {
            return Err(GlossaryError::Validation(
                "not_approved entry must include non-empty word and pos".to_string(),
            ));
        }
        if !not_approved_keys.insert(key) {
            return Err(GlossaryError::Validation(
                "duplicate not_approved entry in glossary".to_string(),
            ));
        }
    }

    for (word, _) in &approved_keys {
        if not_approved_keys
            .iter()
            .any(|(candidate, _)| candidate == word)
        {
            return Err(GlossaryError::Validation(
                "entry cannot appear in both approved and not_approved sections".to_string(),
            ));
        }
    }

    Ok(())
}

#[derive(Debug, Error)]
pub enum GlossaryError {
    #[error("failed to read glossary `{path}`")]
    Read {
        path: String,
        #[source]
        source: std::io::Error,
    },
    #[error("failed to parse glossary TOML")]
    Parse(#[from] toml::de::Error),
    #[error("{0}")]
    Validation(String),
}
