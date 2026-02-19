/// Requirement type names supported by the built-in EARS ruleset.
pub const SUPPORTED_REQUIREMENT_TYPES: &[&str] = &[
    "auto",
    "ubiquitous",
    "event-driven",
    "state-driven",
    "unwanted",
    "optional",
    "complex",
];

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RequirementType {
    Auto,
    Ubiquitous,
    EventDriven,
    StateDriven,
    Unwanted,
    Optional,
    Complex,
}

impl RequirementType {
    pub fn from_annotation(value: &str) -> Option<Self> {
        match value.trim().replace('_', "-").to_ascii_lowercase().as_str() {
            "auto" => Some(Self::Auto),
            "ubiquitous" => Some(Self::Ubiquitous),
            "event-driven" | "event" => Some(Self::EventDriven),
            "state-driven" | "state" => Some(Self::StateDriven),
            "unwanted" => Some(Self::Unwanted),
            "optional" => Some(Self::Optional),
            "complex" => Some(Self::Complex),
            _ => None,
        }
    }

    pub const fn as_str(&self) -> &'static str {
        match self {
            Self::Auto => "auto",
            Self::Ubiquitous => "ubiquitous",
            Self::EventDriven => "event-driven",
            Self::StateDriven => "state-driven",
            Self::Unwanted => "unwanted",
            Self::Optional => "optional",
            Self::Complex => "complex",
        }
    }
}

/// Detect EARS pattern from sentence keywords.
pub fn detect_pattern(sentence: &str) -> Option<RequirementType> {
    let lower = sentence.to_ascii_lowercase();
    let has_where = starts_clause(&lower, "where ");
    let has_while = starts_clause(&lower, "while ");
    let has_when = starts_clause(&lower, "when ");
    let has_if_then =
        (lower.contains(" if ") || lower.starts_with("if ")) && lower.contains(" then ");
    let has_shall = contains_keyword(&lower, "shall");

    if !has_shall {
        return None;
    }

    let keyword_count = [has_where, has_while, has_when, has_if_then]
        .into_iter()
        .filter(|value| *value)
        .count();

    match keyword_count {
        0 => Some(RequirementType::Ubiquitous),
        1 if has_when => Some(RequirementType::EventDriven),
        1 if has_while => Some(RequirementType::StateDriven),
        1 if has_where => Some(RequirementType::Optional),
        1 if has_if_then => Some(RequirementType::Unwanted),
        _ => Some(RequirementType::Complex),
    }
}

pub fn count_keyword(sentence: &str, keyword: &str) -> usize {
    let lower = sentence.to_ascii_lowercase();
    lower
        .split(|ch: char| !ch.is_ascii_alphabetic())
        .filter(|word| *word == keyword)
        .count()
}

pub fn contains_keyword(sentence: &str, keyword: &str) -> bool {
    count_keyword(sentence, keyword) > 0
}

pub fn keyword_position(sentence: &str, keyword: &str) -> Option<usize> {
    let lower = sentence.to_ascii_lowercase();
    lower.find(keyword)
}

pub fn clause_after_keyword<'a>(sentence: &'a str, keyword: &str) -> Option<&'a str> {
    let lower = sentence.to_ascii_lowercase();
    let start = lower.find(keyword)? + keyword.len();
    let rest = &sentence[start..];
    let end = rest.find(',').unwrap_or(rest.len());
    Some(rest[..end].trim())
}

pub fn is_supported_requirement_type(value: &str) -> bool {
    RequirementType::from_annotation(value).is_some()
}

fn starts_clause(lower: &str, keyword_with_space: &str) -> bool {
    lower.starts_with(keyword_with_space) || lower.contains(&format!(", {keyword_with_space}"))
}
