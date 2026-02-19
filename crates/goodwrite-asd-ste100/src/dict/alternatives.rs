use super::lookup::{DictionaryLookup, LookupAlternative};

/// Get approved alternatives for a non-approved word.
pub fn alternatives(word: &str) -> Option<&'static [LookupAlternative]> {
    DictionaryLookup::global().alternatives_for_word(word)
}
