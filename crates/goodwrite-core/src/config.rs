use std::{collections::BTreeMap, fs, path::Path};

use serde::Deserialize;
use thiserror::Error;

/// Severity override loaded from configuration.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RuleLevel {
    Off,
    Info,
    Warn,
    Error,
}

impl RuleLevel {
    pub fn parse(value: &str) -> Option<Self> {
        match value.trim().to_ascii_lowercase().as_str() {
            "off" => Some(Self::Off),
            "info" => Some(Self::Info),
            "warn" | "warning" => Some(Self::Warn),
            "error" => Some(Self::Error),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, Deserialize)]
pub struct ProfilesSection {
    #[serde(default = "default_profiles")]
    pub enable: Vec<String>,
}

impl Default for ProfilesSection {
    fn default() -> Self {
        Self {
            enable: default_profiles(),
        }
    }
}

#[derive(Debug, Clone, Deserialize)]
pub struct RequirementsSection {
    #[serde(default = "default_active_rulesets")]
    pub active_rulesets: Vec<String>,
    #[serde(default = "default_default_ruleset")]
    pub default_ruleset: String,
}

impl Default for RequirementsSection {
    fn default() -> Self {
        Self {
            active_rulesets: default_active_rulesets(),
            default_ruleset: default_default_ruleset(),
        }
    }
}

#[derive(Debug, Clone, Deserialize, Default)]
pub struct GlossarySection {
    pub path: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(untagged)]
pub enum RuleOverride {
    Level(String),
    Full {
        level: Option<String>,
        max_words: Option<usize>,
    },
}

#[derive(Debug, Clone, Deserialize)]
pub struct FormatSection {
    #[serde(default = "default_true")]
    pub typst: bool,
    #[serde(default = "default_true")]
    pub markdown: bool,
}

impl Default for FormatSection {
    fn default() -> Self {
        Self {
            typst: true,
            markdown: true,
        }
    }
}

#[derive(Debug, Clone, Deserialize, Default)]
pub struct CheckSection {
    #[serde(default)]
    pub exclude: Vec<String>,
}

/// Controls behavior when goodwrite must infer metadata from heuristics.
#[derive(Debug, Clone, Default, Deserialize)]
pub struct HeuristicsSection {
    /// Escalate heuristic fallback diagnostics from warning to error.
    ///
    /// This applies when goodwrite must infer profile metadata (for example,
    /// writing mode) because explicit source annotations are missing.
    #[serde(default)]
    pub strict: bool,
}

/// Parsed `goodwrite.toml`.
#[derive(Debug, Clone, Deserialize)]
pub struct GoodwriteConfig {
    #[serde(default)]
    pub profiles: ProfilesSection,
    #[serde(default)]
    pub requirements: RequirementsSection,
    #[serde(default)]
    pub glossary: GlossarySection,
    #[serde(default)]
    pub check: CheckSection,
    #[serde(default)]
    pub rules: BTreeMap<String, RuleOverride>,
    #[serde(default)]
    pub format: FormatSection,
    #[serde(default)]
    pub heuristics: HeuristicsSection,
}

impl Default for GoodwriteConfig {
    fn default() -> Self {
        let mut config = Self {
            profiles: ProfilesSection {
                enable: default_profiles(),
            },
            requirements: RequirementsSection::default(),
            glossary: GlossarySection::default(),
            check: CheckSection::default(),
            rules: BTreeMap::new(),
            format: FormatSection::default(),
            heuristics: HeuristicsSection::default(),
        };
        config.normalize_requirement_rulesets();
        config
    }
}

impl GoodwriteConfig {
    pub fn from_toml(value: &str) -> Result<Self, ConfigError> {
        let mut parsed: Self = toml::from_str(value).map_err(ConfigError::Parse)?;
        parsed.normalize_requirement_rulesets();
        Ok(parsed)
    }

    pub fn from_path(path: impl AsRef<Path>) -> Result<Self, ConfigError> {
        let path = path.as_ref();
        let raw = fs::read_to_string(path).map_err(|source| ConfigError::Read {
            path: path.display().to_string(),
            source,
        })?;
        Self::from_toml(&raw)
    }

    pub fn rule_level(&self, id: &str) -> Option<RuleLevel> {
        let override_value = self.rules.get(id)?;
        match override_value {
            RuleOverride::Level(level) => RuleLevel::parse(level),
            RuleOverride::Full { level, .. } => level.as_deref().and_then(RuleLevel::parse),
        }
    }

    pub fn rule_max_words(&self, id: &str) -> Option<usize> {
        match self.rules.get(id) {
            Some(RuleOverride::Full { max_words, .. }) => *max_words,
            _ => None,
        }
    }

    pub fn requirement_ruleset_enabled(&self, ruleset: &str) -> bool {
        self.requirements
            .active_rulesets
            .iter()
            .any(|value| value.eq_ignore_ascii_case(ruleset))
    }

    fn normalize_requirement_rulesets(&mut self) {
        let mut normalized_active = Vec::new();
        for ruleset in &self.requirements.active_rulesets {
            let normalized = normalize_ruleset_name(ruleset);
            if normalized.is_empty() {
                continue;
            }
            if normalized_active
                .iter()
                .any(|item: &String| item.eq_ignore_ascii_case(&normalized))
            {
                continue;
            }
            normalized_active.push(normalized);
        }

        let mut normalized_default = normalize_ruleset_name(&self.requirements.default_ruleset);
        if normalized_default.is_empty() {
            normalized_default = default_default_ruleset();
        }

        if normalized_active.is_empty() {
            normalized_active.push(normalized_default.clone());
        } else if !normalized_active
            .iter()
            .any(|item| item.eq_ignore_ascii_case(&normalized_default))
        {
            // Keep one source of truth for routing requirement diagnostics.
            // If the configured default is not active, prefer the first active
            // ruleset so all runtimes (CLI/LSP) resolve spans the same way.
            normalized_default = normalized_active[0].clone();
        }

        self.requirements.active_rulesets = normalized_active;
        self.requirements.default_ruleset = normalized_default;
    }
}

#[derive(Debug, Error)]
pub enum ConfigError {
    #[error("failed to read config file `{path}`")]
    Read {
        path: String,
        #[source]
        source: std::io::Error,
    },
    #[error("failed to parse TOML config")]
    Parse(#[from] toml::de::Error),
}

fn default_profiles() -> Vec<String> {
    vec![
        "asd-ste100".to_string(),
        "ears".to_string(),
        "glossary".to_string(),
    ]
}

fn default_true() -> bool {
    true
}

fn default_active_rulesets() -> Vec<String> {
    vec!["ears".to_string()]
}

fn default_default_ruleset() -> String {
    "ears".to_string()
}

fn normalize_ruleset_name(value: &str) -> String {
    value.trim().to_ascii_lowercase()
}

#[cfg(test)]
mod tests {
    use super::GoodwriteConfig;

    #[test]
    fn empty_requirement_rulesets_fall_back_to_default() {
        let config = GoodwriteConfig::from_toml(
            r#"[requirements]
active_rulesets = []
default_ruleset = "ears"
"#,
        )
        .expect("parse config");

        assert_eq!(config.requirements.active_rulesets, vec!["ears"]);
        assert_eq!(config.requirements.default_ruleset, "ears");
    }

    #[test]
    fn default_ruleset_is_aligned_with_active_rulesets() {
        let config = GoodwriteConfig::from_toml(
            r#"[requirements]
active_rulesets = ["internal-disabled", "ears"]
default_ruleset = "custom"
"#,
        )
        .expect("parse config");

        assert_eq!(
            config.requirements.active_rulesets,
            vec!["internal-disabled", "ears"]
        );
        assert_eq!(config.requirements.default_ruleset, "internal-disabled");
    }
}
