use crate::{RuleLevel, Severity, config::GoodwriteConfig};

/// Resolve final severity after configuration overrides.
pub fn effective_severity(
    config: &GoodwriteConfig,
    rule_id: &str,
    default: Severity,
) -> Option<Severity> {
    match config.rule_level(rule_id) {
        Some(RuleLevel::Off) => None,
        Some(RuleLevel::Info) => Some(Severity::Info),
        Some(RuleLevel::Warn) => Some(Severity::Warning),
        Some(RuleLevel::Error) => Some(Severity::Error),
        None => Some(default),
    }
}
