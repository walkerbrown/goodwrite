//! Shared types and rule infrastructure for goodwrite.

pub mod annotations;
pub mod config;
pub mod context;
pub mod diagnostic;
pub mod profile;
pub mod rule;
pub mod rule_index;
pub mod span;
pub mod suggestion;
pub mod text;

pub use annotations::{SpanAnnotations, WritingMode};
pub use config::{
    CheckSection, ConfigError, GlossarySection, GoodwriteConfig, RequirementsSection, RuleLevel,
    UnsafeSection,
};
pub use context::{
    CheckContext, GlossaryAlternative, GlossaryApprovedEntry, GlossaryData, GlossaryFileData,
    GlossaryNotApprovedEntry,
};
pub use diagnostic::{Diagnostic, Severity, SourceRange};
pub use rule::{Rule, RuleInput, RuleSet};
pub use rule_index::{RuleIndex, RuleIndexEntry, RuleIndexError};
pub use span::{ExtractResult, ProseSpan, UnsafeAnnotation, UnsafeAnnotationState};
pub use suggestion::{Applicability, Suggestion};
pub use text::{Sentence, Token};
