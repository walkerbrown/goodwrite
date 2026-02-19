use serde::Deserialize;

/// Additional descriptive meaning metadata for approved words.
#[derive(Debug, Clone, Deserialize, Default)]
pub struct Meaning {
    pub text: String,
    pub context: Option<String>,
}

/// Dictionary entry for approved words.
#[derive(Debug, Clone, Deserialize)]
pub struct ApprovedWord {
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
    #[serde(default)]
    pub meanings: Vec<Meaning>,
}

/// Alternative mapping for not-approved words.
#[derive(Debug, Clone, Deserialize)]
#[serde(untagged)]
pub enum Alternative {
    Word(String),
    Detailed {
        word: String,
        pos: Option<String>,
        context: Option<String>,
    },
}

impl Alternative {
    pub fn word(&self) -> &str {
        match self {
            Alternative::Word(value) => value,
            Alternative::Detailed { word, .. } => word,
        }
    }
}

/// Dictionary entry for non-approved words.
#[derive(Debug, Clone, Deserialize)]
pub struct NotApprovedWord {
    pub word: String,
    pub pos: String,
    #[serde(default)]
    pub alternatives: Vec<Alternative>,
    #[serde(default)]
    pub approved_meaning: String,
    #[serde(default)]
    pub goodwrite_example: String,
    #[serde(default)]
    pub wrongwrite_example: String,
}

/// Embedded dictionary payload.
#[derive(Debug, Clone, Deserialize)]
pub struct Dictionary {
    pub notice: String,
    #[serde(default)]
    pub approved: Vec<ApprovedWord>,
    #[serde(default)]
    pub not_approved: Vec<NotApprovedWord>,
}

pub mod alternatives;
pub mod data;
pub mod lookup;
