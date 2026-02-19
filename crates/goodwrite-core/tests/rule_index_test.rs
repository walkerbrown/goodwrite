use goodwrite_core::RuleIndex;

#[test]
fn rule_index_has_no_duplicate_ids() {
    let index = RuleIndex::load_embedded().expect("load embedded rule index");
    let duplicates = index.duplicate_ids();
    assert!(
        duplicates.is_empty(),
        "duplicate rule ids in index: {duplicates:?}"
    );
}

#[test]
fn rule_index_entries_are_fully_populated() {
    let index = RuleIndex::load_embedded().expect("load embedded rule index");
    assert!(!index.rules.is_empty(), "rule index is empty");

    for entry in index.rules {
        assert!(!entry.id.trim().is_empty(), "entry with empty id");
        assert!(
            !entry.profile.trim().is_empty(),
            "{} missing profile",
            entry.id
        );
        assert!(!entry.title.trim().is_empty(), "{} missing title", entry.id);
        assert!(
            !entry.standard.trim().is_empty(),
            "{} missing standard",
            entry.id
        );
        assert!(!entry.part.trim().is_empty(), "{} missing part", entry.id);
        assert!(
            !entry.section_number.trim().is_empty(),
            "{} missing section_number",
            entry.id
        );
        assert!(
            !entry.section_name.trim().is_empty(),
            "{} missing section_name",
            entry.id
        );
        assert!(
            !entry.rule_number.trim().is_empty(),
            "{} missing rule_number",
            entry.id
        );
        assert!(
            !entry.citation.trim().is_empty(),
            "{} missing citation",
            entry.id
        );
        assert!(
            !entry.test_pass.trim().is_empty(),
            "{} missing test_pass",
            entry.id
        );
        assert!(
            !entry.test_fail.trim().is_empty(),
            "{} missing test_fail",
            entry.id
        );
    }
}
