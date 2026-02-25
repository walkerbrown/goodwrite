//! Source extraction for Typst and Markdown.

mod markdown;
mod spans;
mod suppressions;
mod typst;

use std::{fs, path::Path};

use goodwrite_core::ExtractResult;
use thiserror::Error;

pub use spans::mode_from_inference;

/// Extract prose spans from a source file.
pub fn extract_path(path: &Path) -> Result<ExtractResult, ExtractError> {
    let source = fs::read_to_string(path).map_err(|source| ExtractError::Read {
        path: path.display().to_string(),
        source,
    })?;
    extract_source(path, &source)
}

/// Extract prose spans from in-memory text using a path extension hint.
pub fn extract_source(path: &Path, source: &str) -> Result<ExtractResult, ExtractError> {
    let extension = path
        .extension()
        .and_then(|ext| ext.to_str())
        .map(|ext| ext.to_ascii_lowercase())
        .unwrap_or_default();

    let mut result = match extension.as_str() {
        "md" | "markdown" => markdown::extract_markdown(source),
        "typ" => typst::extract_typst(source),
        other => {
            return Err(ExtractError::UnsupportedExtension {
                extension: other.to_string(),
            });
        }
    }?;

    result.source = source.to_string();
    Ok(result)
}

#[derive(Debug, Error)]
pub enum ExtractError {
    #[error("failed to read `{path}`")]
    Read {
        path: String,
        #[source]
        source: std::io::Error,
    },
    #[error("unsupported file extension `{extension}`")]
    UnsupportedExtension { extension: String },
    #[error("failed to parse markdown")]
    Markdown,
    #[error("failed to parse typst")]
    Typst,
    #[error("unsupported source annotation `{annotation}`")]
    LegacyAnnotation { annotation: String },
}
