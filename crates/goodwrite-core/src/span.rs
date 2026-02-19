use crate::{SourceRange, SpanAnnotations};

/// Validation state for one `goodwrite:unsafe(...)` annotation.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum UnsafeAnnotationState {
    Pending,
    Consumed,
    Invalid(String),
}

/// One source-authored exemption entry attached to the next prose span.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UnsafeAnnotation {
    pub rule_id: String,
    pub reason: String,
    pub range: SourceRange,
    pub state: UnsafeAnnotationState,
}

impl UnsafeAnnotation {
    pub fn pending(
        rule_id: impl Into<String>,
        reason: impl Into<String>,
        range: SourceRange,
    ) -> Self {
        Self {
            rule_id: rule_id.into(),
            reason: reason.into(),
            range,
            state: UnsafeAnnotationState::Pending,
        }
    }

    pub fn invalid(
        rule_id: impl Into<String>,
        reason: impl Into<String>,
        range: SourceRange,
        message: impl Into<String>,
    ) -> Self {
        Self {
            rule_id: rule_id.into(),
            reason: reason.into(),
            range,
            state: UnsafeAnnotationState::Invalid(message.into()),
        }
    }
}

/// Format-agnostic extracted prose fragment.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProseSpan {
    pub text: String,
    pub range: SourceRange,
    pub annotations: SpanAnnotations,
    pub unsafe_annotations: Vec<UnsafeAnnotation>,
    pub heading: Option<String>,
}

impl ProseSpan {
    pub fn new(text: impl Into<String>, range: SourceRange, annotations: SpanAnnotations) -> Self {
        Self {
            text: text.into(),
            range,
            annotations,
            unsafe_annotations: Vec::new(),
            heading: None,
        }
    }
}

/// Extractor output for one source file.
#[derive(Debug, Clone)]
pub struct ExtractResult {
    pub source: String,
    pub spans: Vec<ProseSpan>,
    pub has_mode_annotations: bool,
    pub used_mode_heuristic: bool,
}
