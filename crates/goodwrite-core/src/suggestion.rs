use serde::{Deserialize, Serialize};

use crate::diagnostic::SourceRange;

/// Safety level for automated fixes.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum Applicability {
    MachineApplicable,
    MaybeIncorrect,
    HasPlaceholders,
}

/// Text replacement candidate for diagnostics.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Suggestion {
    pub span: SourceRange,
    pub replacement: String,
    pub applicability: Applicability,
    pub message: String,
}
