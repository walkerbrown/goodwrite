pub const DICTIONARY_TOML: &str = include_str!("../../data/dictionary.toml");

#[cfg(test)]
mod tests {
    use super::*;
    use crate::dict::Dictionary;

    #[test]
    fn embedded_dictionary_parses_and_matches_specification_counts() {
        let dictionary: Dictionary =
            toml::from_str(DICTIONARY_TOML).expect("parse dictionary.toml");

        // Assert exactly on the expected counts of words parsed from ASD-STE100 Issue 9
        // This validates that our PDF parsing logic hasn't regressed or dropped columns.
        assert_eq!(dictionary.approved.len(), 876, "approved word count");
        assert_eq!(
            dictionary.not_approved.len(),
            1316,
            "not-approved word count"
        );

        // Ensure no empty words leaked in
        assert!(
            !dictionary
                .approved
                .iter()
                .any(|entry| entry.word.trim().is_empty())
        );
        assert!(
            !dictionary
                .not_approved
                .iter()
                .any(|entry| entry.word.trim().is_empty())
        );

        // Ensure words have a part of speech
        assert!(
            !dictionary
                .approved
                .iter()
                .any(|entry| entry.pos.trim().is_empty())
        );
        assert!(
            !dictionary
                .not_approved
                .iter()
                .any(|entry| entry.pos.trim().is_empty())
        );
    }
}
