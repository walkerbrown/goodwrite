use serde::{Deserialize, Serialize};

/// Writing mode used by ASD-STE100 checks.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum WritingMode {
    Procedural,
    #[default]
    Descriptive,
    SafetyInstruction,
    Note,
}

impl WritingMode {
    /// Parse a `goodwrite:mode:*` annotation value.
    pub fn from_annotation(value: &str) -> Option<Self> {
        match value.trim().to_ascii_lowercase().as_str() {
            "procedural" => Some(Self::Procedural),
            "descriptive" => Some(Self::Descriptive),
            "safety" | "safety-instruction" => Some(Self::SafetyInstruction),
            "note" => Some(Self::Note),
            _ => None,
        }
    }
}

/// Flat profile metadata attached to extracted prose spans.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct SpanAnnotations {
    pub writing_mode: Option<WritingMode>,
    pub requirement: bool,
    pub requirement_type: Option<String>,
}

impl SpanAnnotations {
    /// Apply one `goodwrite:*` annotation token.
    pub fn apply_annotation(&mut self, raw: &str) -> bool {
        let token = raw.trim().trim_matches('"').trim();
        if let Some(value) = token.strip_prefix("goodwrite:mode:") {
            if value.eq_ignore_ascii_case("end") {
                self.writing_mode = None;
                return true;
            }
            self.writing_mode = WritingMode::from_annotation(value);
            return self.writing_mode.is_some();
        }

        if token.eq_ignore_ascii_case("goodwrite:requirement") {
            self.requirement = true;
            self.requirement_type = None;
            return true;
        }

        if let Some(value) = token.strip_prefix("goodwrite:requirement:") {
            if value.eq_ignore_ascii_case("end") {
                self.requirement = false;
                self.requirement_type = None;
                return true;
            }

            // Requirement type syntax is source-facing and ruleset agnostic.
            // We normalize to kebab-case so Markdown and Typst helpers map to
            // the same canonical value.
            let normalized = normalize_requirement_type(value);
            if normalized.is_empty() {
                return false;
            }

            self.requirement = true;
            self.requirement_type = Some(normalized);
            return self.requirement_type.is_some();
        }

        false
    }

    /// Resolve the effective ASD-STE100 mode.
    pub fn effective_mode(&self) -> WritingMode {
        self.writing_mode.unwrap_or(WritingMode::Descriptive)
    }

    /// Resolve the requirement ruleset for this span.
    pub fn effective_requirement_ruleset<'a>(
        &'a self,
        default_ruleset: &'a str,
    ) -> Option<&'a str> {
        if !self.requirement {
            return None;
        }

        Some(default_ruleset)
    }
}

fn normalize_requirement_type(value: &str) -> String {
    value.trim().replace('_', "-").to_ascii_lowercase()
}
