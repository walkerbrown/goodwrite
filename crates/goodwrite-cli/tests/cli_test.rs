use std::{
    fs,
    path::{Path, PathBuf},
    process::{Command, Output},
    sync::atomic::{AtomicU64, Ordering},
    time::{SystemTime, UNIX_EPOCH},
};

#[test]
fn check_reports_unapproved_word() {
    let workspace = TempWorkspace::new();
    let config = workspace.write(
        "goodwrite.toml",
        r#"[profiles]
enable = ["asd-ste100"]
"#,
    );
    let input = workspace.write(
        "doc.md",
        "<!-- goodwrite:mode:descriptive -->\nThe operator must abandon the area.\n",
    );

    let output = run_goodwrite(
        &["--config", as_utf8(&config), "check", as_utf8(&input)],
        workspace.root(),
    );

    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(!output.status.success(), "{stderr}");
    assert!(stderr.contains("asd-ste100/unapproved-word"), "{stderr}");
}

#[test]
fn init_requires_explicit_target_argument() {
    let workspace = TempWorkspace::new();

    let output = run_goodwrite(&["init"], workspace.root());
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(!output.status.success(), "{stderr}");
    assert!(stderr.contains("Usage:"), "{stderr}");
    assert!(stderr.contains("init <TARGET>"), "{stderr}");
}

#[test]
fn init_config_writes_commented_default_profiles_template() {
    let workspace = TempWorkspace::new();

    let output = run_goodwrite(&["init", "config"], workspace.root());
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(output.status.success(), "{stderr}");

    let config_path = workspace.root().join("goodwrite.toml");
    assert!(config_path.exists(), "expected config file to exist");
    assert!(
        !workspace.root().join("glossary.toml").exists(),
        "did not expect glossary.toml to be created"
    );

    let config = fs::read_to_string(&config_path).expect("read generated config");
    assert!(config.contains("# [profiles]"), "{config}");
    assert!(
        config.contains("# enable = [\"asd-ste100\", \"ears\", \"glossary\"]"),
        "{config}"
    );
    assert!(config.contains("# [unsafe]"), "{config}");
    assert!(
        config.contains("# ignore = [\"docs/legacy/**/*.md\"]"),
        "{config}"
    );
}

#[test]
fn init_glossary_creates_glossary_template_only() {
    let workspace = TempWorkspace::new();

    let output = run_goodwrite(&["init", "glossary"], workspace.root());
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(output.status.success(), "{stderr}");

    let glossary_path = workspace.root().join("glossary.toml");
    assert!(glossary_path.exists(), "expected glossary file to exist");
    assert!(
        !workspace.root().join("goodwrite.toml").exists(),
        "did not expect goodwrite.toml to be created"
    );

    let glossary = fs::read_to_string(&glossary_path).expect("read generated glossary");
    assert!(glossary.contains("# [[approved]]"), "{glossary}");
    assert!(glossary.contains("# [[not_approved]]"), "{glossary}");
}

#[test]
fn init_glossary_fails_if_glossary_already_exists() {
    let workspace = TempWorkspace::new();
    workspace.write(
        "glossary.toml",
        "[[approved]]\nword = \"existing\"\npos = \"noun\"\n",
    );

    let output = run_goodwrite(&["init", "glossary"], workspace.root());
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(!output.status.success(), "{stderr}");
    assert!(stderr.contains("glossary.toml already exists"), "{stderr}");
}

#[test]
fn init_config_fails_if_config_already_exists() {
    let workspace = TempWorkspace::new();
    workspace.write("goodwrite.toml", "[profiles]\nenable = [\"asd-ste100\"]\n");

    let output = run_goodwrite(&["init", "config"], workspace.root());
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(!output.status.success(), "{stderr}");
    assert!(stderr.contains("goodwrite.toml already exists"), "{stderr}");
}

#[test]
fn default_profiles_include_ears_without_config_file() {
    let workspace = TempWorkspace::new();
    let input = workspace.write(
        "req.md",
        "<!-- goodwrite:requirement:auto -->\nWhen the pilot presses the button, while the aircraft is on ground, the system record the event.\n<!-- goodwrite:requirement:end -->\n",
    );

    let output = run_goodwrite(&["check", as_utf8(&input)], workspace.root());
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("ears/clause-order"), "{stderr}");
}

#[test]
fn fix_dry_run_prints_unified_diff() {
    let workspace = TempWorkspace::new();
    let config = workspace.write(
        "goodwrite.toml",
        r#"[profiles]
enable = ["asd-ste100"]
"#,
    );
    let original = "<!-- goodwrite:mode:descriptive -->\nDo not open valve; close valve.\n";
    let input = workspace.write("doc.md", original);

    let output = run_goodwrite(
        &[
            "--config",
            as_utf8(&config),
            "fix",
            "--dry-run",
            as_utf8(&input),
        ],
        workspace.root(),
    );

    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(output.status.success(), "{stderr}");
    assert!(stdout.contains("--- "), "{stdout}");
    assert!(stdout.contains("+++ "), "{stdout}");
    assert!(
        stdout.contains("-Do not open valve; close valve."),
        "{stdout}"
    );
    assert!(
        stdout.contains("+Do not open valve. Close valve."),
        "{stdout}"
    );

    let unchanged = fs::read_to_string(&input).expect("read source after dry-run");
    assert_eq!(unchanged, original);
}

#[test]
fn fix_dry_run_keeps_replacements_aligned_after_apostrophes() {
    let workspace = TempWorkspace::new();
    let config = workspace.write(
        "goodwrite.toml",
        r#"[profiles]
enable = ["asd-ste100"]
"#,
    );
    let input = workspace.write(
        "doc.md",
        "<!-- goodwrite:mode:descriptive -->\nThe operator can't review the startup trace, e.g. during bench validation.\n",
    );

    let output = run_goodwrite(
        &[
            "--config",
            as_utf8(&config),
            "fix",
            "--dry-run",
            as_utf8(&input),
        ],
        workspace.root(),
    );

    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(output.status.success(), "{stderr}");
    assert!(
        stdout.contains(
            "+The operator cannot review the startup trace, for example during bench validation."
        ),
        "{stdout}"
    );
    assert!(!stdout.contains("reinspectionhe"), "{stdout}");
    assert!(!stdout.contains("e.for exampleuring"), "{stdout}");
}

#[test]
fn fix_dry_run_honors_color_always() {
    let workspace = TempWorkspace::new();
    let config = workspace.write(
        "goodwrite.toml",
        r#"[profiles]
enable = ["asd-ste100"]
"#,
    );
    let input = workspace.write(
        "doc.md",
        "<!-- goodwrite:mode:descriptive -->\nDo not open valve; close valve.\n",
    );

    let output = run_goodwrite(
        &[
            "--config",
            as_utf8(&config),
            "--color",
            "always",
            "fix",
            "--dry-run",
            as_utf8(&input),
        ],
        workspace.root(),
    );

    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(output.status.success(), "{stderr}");
    assert!(stdout.contains("\u{1b}["), "{stdout}");
    assert!(
        stdout.contains("+Do not open valve. Close valve."),
        "{stdout}"
    );
}

#[test]
fn fix_dry_run_honors_color_never() {
    let workspace = TempWorkspace::new();
    let config = workspace.write(
        "goodwrite.toml",
        r#"[profiles]
enable = ["asd-ste100"]
"#,
    );
    let input = workspace.write(
        "doc.md",
        "<!-- goodwrite:mode:descriptive -->\nDo not open valve; close valve.\n",
    );

    let output = run_goodwrite(
        &[
            "--config",
            as_utf8(&config),
            "--color",
            "never",
            "fix",
            "--dry-run",
            as_utf8(&input),
        ],
        workspace.root(),
    );

    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(output.status.success(), "{stderr}");
    assert!(!stdout.contains("\u{1b}["), "{stdout}");
}

#[test]
fn cant_review_is_not_misclassified_as_noun_phrase() {
    let workspace = TempWorkspace::new();
    let config = workspace.write(
        "goodwrite.toml",
        r#"[profiles]
enable = ["asd-ste100"]
"#,
    );
    let input = workspace.write(
        "doc.md",
        "<!-- goodwrite:mode:descriptive -->\nThe operator can't review the startup trace.\n",
    );

    let output = run_goodwrite(
        &["--config", as_utf8(&config), "check", as_utf8(&input)],
        workspace.root(),
    );

    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(!output.status.success(), "{stderr}");
    assert!(stderr.contains("asd-ste100/contractions"), "{stderr}");
    assert!(stderr.contains("asd-ste100/unapproved-word"), "{stderr}");
    assert!(
        !stderr.contains("asd-ste100/articles-before-nouns"),
        "{stderr}"
    );
}

#[test]
fn config_override_can_disable_rule() {
    let workspace = TempWorkspace::new();
    let config = workspace.write(
        "goodwrite.toml",
        r#"[profiles]
enable = ["asd-ste100"]

[rules]
"asd-ste100/semicolons" = "off"
"#,
    );
    let input = workspace.write(
        "doc.md",
        "<!-- goodwrite:mode:descriptive -->\nThe system is active; the system is active.\n",
    );

    let output = run_goodwrite(
        &["--config", as_utf8(&config), "check", as_utf8(&input)],
        workspace.root(),
    );

    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(output.status.success(), "{stderr}");
    assert!(!stderr.contains("asd-ste100/semicolons"), "{stderr}");
}

#[test]
fn glossary_synonym_rule_loads_and_flags_terms() {
    let workspace = TempWorkspace::new();
    let config = workspace.write(
        "goodwrite.toml",
        r#"[profiles]
enable = ["glossary"]

[glossary]
path = "glossary.toml"
"#,
    );
    let _glossary = workspace.write(
        "glossary.toml",
        r#"[[approved]]
word = "actuator"
pos = "noun"

[[not_approved]]
word = "control unit"
pos = "noun"
alternatives = [{ word = "actuator" }]
"#,
    );
    let input = workspace.write(
        "requirements.md",
        "The control unit starts when requested.\n",
    );

    let output = run_goodwrite(
        &["--config", as_utf8(&config), "check", as_utf8(&input)],
        workspace.root(),
    );

    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(output.status.success(), "{stderr}");
    assert!(stderr.contains("glossary/synonym-enforce"), "{stderr}");
}

#[test]
fn glossary_phrase_synonym_suppresses_overlapping_asd_token_noise() {
    let workspace = TempWorkspace::new();
    let config = workspace.write(
        "goodwrite.toml",
        r#"[profiles]
enable = ["asd-ste100", "glossary"]

[glossary]
path = "glossary.toml"
"#,
    );
    let _glossary = workspace.write(
        "glossary.toml",
        r#"[[approved]]
word = "FluxDrive"
pos = "noun"

[[not_approved]]
word = "flux drive"
pos = "noun"
alternatives = [{ word = "FluxDrive" }]
"#,
    );
    let input = workspace.write(
        "doc.md",
        "<!-- goodwrite:mode:descriptive -->\nThe flux drive starts.\n",
    );

    let output = run_goodwrite(
        &["--config", as_utf8(&config), "check", as_utf8(&input)],
        workspace.root(),
    );

    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(output.status.success(), "{stderr}");
    assert!(stderr.contains("glossary/synonym-enforce"), "{stderr}");
    assert!(!stderr.contains("asd-ste100/unapproved-word"), "{stderr}");
    assert!(
        !stderr.contains("asd-ste100/non-approved-as-technical-noun"),
        "{stderr}"
    );
}

#[test]
fn glossary_undefined_term_rule_flags_unknown_acronym() {
    let workspace = TempWorkspace::new();
    let config = workspace.write(
        "goodwrite.toml",
        r#"[profiles]
enable = ["glossary"]

[glossary]
path = "glossary.toml"
"#,
    );
    let _glossary = workspace.write(
        "glossary.toml",
        r#"[[approved]]
word = "actuator"
pos = "noun"

[[not_approved]]
word = "control unit"
pos = "noun"
alternatives = [{ word = "actuator" }]
"#,
    );
    let input = workspace.write("requirements.md", "The MFMU starts when requested.\n");

    let output = run_goodwrite(
        &["--config", as_utf8(&config), "check", as_utf8(&input)],
        workspace.root(),
    );

    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(output.status.success(), "{stderr}");
    assert!(stderr.contains("glossary/undefined-term"), "{stderr}");
}

#[test]
fn approved_word_meaning_diagnostic_with_glossary() {
    let workspace = TempWorkspace::new();
    let config = workspace.write(
        "goodwrite.toml",
        r#"[profiles]
enable = ["asd-ste100", "glossary"]

[glossary]
path = "glossary.toml"
"#,
    );
    let _glossary = workspace.write(
        "glossary.toml",
        r#"[[approved]]
word = "actuator"
pos = "noun"
"#,
    );
    let input = workspace.write(
        "doc.md",
        "<!-- goodwrite:mode:procedural -->\nControl the valve.\n",
    );

    let output = run_goodwrite(
        &["--config", as_utf8(&config), "check", as_utf8(&input)],
        workspace.root(),
    );

    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(output.status.success(), "{stderr}");
    assert!(stderr.contains("asd-ste100/approved-meaning"), "{stderr}");
    assert!(
        !stderr.contains("asd-ste100/approved-word-pos-mismatch"),
        "{stderr}"
    );
}

#[test]
fn compliance_engine_reports_ambiguous_and_alternative_pos_mismatch() {
    let workspace = TempWorkspace::new();
    let config = workspace.write(
        "goodwrite.toml",
        r#"[profiles]
enable = ["asd-ste100", "glossary"]

[glossary]
path = "glossary.toml"
"#,
    );
    let _glossary = workspace.write(
        "glossary.toml",
        r#"[[approved]]
word = "actuator"
pos = "noun"
"#,
    );

    let ambiguous = workspace.write(
        "ambiguous.md",
        "<!-- goodwrite:mode:descriptive -->\nControl the valve.\n",
    );
    let mismatch = workspace.write(
        "mismatch.md",
        "<!-- goodwrite:mode:descriptive -->\nAbility systems are active.\n",
    );

    let output = run_goodwrite(
        &[
            "--config",
            as_utf8(&config),
            "check",
            as_utf8(&ambiguous),
            as_utf8(&mismatch),
        ],
        workspace.root(),
    );

    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(!output.status.success(), "{stderr}");
    assert!(stderr.contains("asd-ste100/ambiguous-pos"), "{stderr}");
    assert!(
        stderr.contains("asd-ste100/alternative-pos-mismatch"),
        "{stderr}"
    );
}

#[test]
fn user_dictionary_conflict_with_embedded_dictionary_fails() {
    let workspace = TempWorkspace::new();
    let config = workspace.write(
        "goodwrite.toml",
        r#"[profiles]
enable = ["asd-ste100"]

[glossary]
path = "glossary.toml"
"#,
    );
    let _user_dictionary = workspace.write(
        "glossary.toml",
        r#"[[approved]]
word = "use"
pos = "verb"
"#,
    );
    let input = workspace.write(
        "doc.md",
        "<!-- goodwrite:mode:descriptive -->\nUse this mode.\n",
    );

    let output = run_goodwrite(
        &["--config", as_utf8(&config), "check", as_utf8(&input)],
        workspace.root(),
    );
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(!output.status.success(), "{stderr}");
    assert!(
        stderr.contains("conflicts with bundled STE dictionary"),
        "{stderr}"
    );
}

#[test]
fn compliance_guidance_differs_by_writing_mode() {
    let workspace = TempWorkspace::new();
    let config = workspace.write(
        "goodwrite.toml",
        r#"[profiles]
enable = ["asd-ste100"]
"#,
    );
    let procedural = workspace.write(
        "procedural.md",
        "<!-- goodwrite:mode:procedural -->\nUtilize the handle.\n",
    );
    let descriptive = workspace.write(
        "descriptive.md",
        "<!-- goodwrite:mode:descriptive -->\nThe system shall utilize control.\n",
    );

    let output = run_goodwrite(
        &[
            "--config",
            as_utf8(&config),
            "check",
            as_utf8(&procedural),
            as_utf8(&descriptive),
        ],
        workspace.root(),
    );

    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(!output.status.success(), "{stderr}");
    assert!(
        stderr.contains("replace with an approved imperative verb or approved technical term"),
        "{stderr}"
    );
    assert!(
        stderr.contains("replace with an approved descriptive word that preserves sentence role"),
        "{stderr}"
    );
}

#[test]
fn requirement_span_routes_ears_rules() {
    let workspace = TempWorkspace::new();
    let config = workspace.write(
        "goodwrite.toml",
        r#"[profiles]
enable = ["ears"]

[requirements]
active_rulesets = ["ears"]
default_ruleset = "ears"
"#,
    );

    let unmarked = workspace.write(
        "unmarked.md",
        "When the pilot presses the button, while the aircraft is on ground, the system record the event.\n",
    );
    let marked = workspace.write(
        "marked.md",
        "<!-- goodwrite:requirement:auto -->\nWhen the pilot presses the button, while the aircraft is on ground, the system record the event.\n<!-- goodwrite:requirement:end -->\n",
    );

    let output = run_goodwrite(
        &[
            "--config",
            as_utf8(&config),
            "check",
            as_utf8(&unmarked),
            as_utf8(&marked),
        ],
        workspace.root(),
    );

    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(!stderr.contains("unmarked.md"), "{stderr}");
    assert!(stderr.contains("marked.md"), "{stderr}");
    assert!(stderr.contains("ears/clause-order"), "{stderr}");
}

#[test]
fn legacy_ears_annotations_are_rejected() {
    let workspace = TempWorkspace::new();
    let config = workspace.write(
        "goodwrite.toml",
        r#"[profiles]
enable = ["ears"]

[requirements]
active_rulesets = ["ears"]
default_ruleset = "ears"
"#,
    );

    let input = workspace.write(
        "legacy.md",
        "<!-- goodwrite:requirement -->\n<!-- goodwrite:ears:auto -->\nWhen the pilot presses the button, the system shall record the event.\n<!-- goodwrite:requirement:end -->\n",
    );

    let output = run_goodwrite(
        &["--config", as_utf8(&config), "check", as_utf8(&input)],
        workspace.root(),
    );

    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(!output.status.success(), "{stderr}");
    assert!(stderr.contains("unsupported source annotation"), "{stderr}");
}

#[test]
fn unknown_requirement_type_is_reported() {
    let workspace = TempWorkspace::new();
    let config = workspace.write(
        "goodwrite.toml",
        r#"[profiles]
enable = ["ears"]

[requirements]
active_rulesets = ["ears"]
default_ruleset = "ears"
"#,
    );

    let input = workspace.write(
        "unknown-type.md",
        "<!-- goodwrite:requirement:custom-type -->\nWhen the pilot presses the button, the system shall record the event.\n<!-- goodwrite:requirement:end -->\n",
    );

    let output = run_goodwrite(
        &["--config", as_utf8(&config), "check", as_utf8(&input)],
        workspace.root(),
    );

    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(output.status.success(), "{stderr}");
    assert!(stderr.contains("ears/invalid-requirement-type"), "{stderr}");
}

#[test]
fn requirement_ruleset_is_controlled_by_config_only() {
    let workspace = TempWorkspace::new();
    let disabled_config = workspace.write(
        "disabled.toml",
        r#"[profiles]
enable = ["ears"]

[requirements]
active_rulesets = ["internal-disabled"]
default_ruleset = "internal-disabled"
"#,
    );
    let enabled_config = workspace.write(
        "enabled.toml",
        r#"[profiles]
enable = ["ears"]

[requirements]
active_rulesets = ["ears"]
default_ruleset = "ears"
"#,
    );
    let input = workspace.write(
        "req.md",
        "<!-- goodwrite:requirement:auto -->\nWhen the pilot presses the button, while the aircraft is on ground, the system record the event.\n<!-- goodwrite:requirement:end -->\n",
    );

    let disabled_output = run_goodwrite(
        &[
            "--config",
            as_utf8(&disabled_config),
            "check",
            as_utf8(&input),
        ],
        workspace.root(),
    );
    let disabled_stderr = String::from_utf8_lossy(&disabled_output.stderr);
    assert!(disabled_output.status.success(), "{disabled_stderr}");
    assert!(
        !disabled_stderr.contains("ears/clause-order"),
        "{disabled_stderr}"
    );

    let enabled_output = run_goodwrite(
        &[
            "--config",
            as_utf8(&enabled_config),
            "check",
            as_utf8(&input),
        ],
        workspace.root(),
    );
    let enabled_stderr = String::from_utf8_lossy(&enabled_output.stderr);
    assert!(enabled_output.status.success(), "{enabled_stderr}");
    assert!(
        enabled_stderr.contains("ears/clause-order"),
        "{enabled_stderr}"
    );
}

#[test]
fn requirement_ruleset_cli_flag_is_rejected() {
    let workspace = TempWorkspace::new();
    let input = workspace.write(
        "req.md",
        "<!-- goodwrite:requirement -->\nThe system shall reset.\n<!-- goodwrite:requirement:end -->\n",
    );

    let output = run_goodwrite(
        &["--requirement-ruleset", "ears", "check", as_utf8(&input)],
        workspace.root(),
    );
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(!output.status.success(), "{stderr}");
    assert!(stderr.contains("--requirement-ruleset"), "{stderr}");
}

#[test]
fn missing_mode_annotation_emits_info_diagnostic() {
    let workspace = TempWorkspace::new();
    let config = workspace.write(
        "goodwrite.toml",
        r#"[profiles]
enable = ["asd-ste100"]
"#,
    );
    let input = workspace.write("fallback.md", "Open the valve.\n");

    let output = run_goodwrite(
        &["--config", as_utf8(&config), "check", as_utf8(&input)],
        workspace.root(),
    );
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(output.status.success(), "{stderr}");
    assert!(
        stderr.contains("goodwrite/missing-mode-annotation"),
        "{stderr}"
    );
}

#[test]
fn unsafe_ignore_suppresses_mode_annotation_info_relative_to_config() {
    let workspace = TempWorkspace::new();
    let config = workspace.write(
        "config/goodwrite.toml",
        r#"[profiles]
enable = ["asd-ste100"]

[unsafe]
ignore = ["docs/*.md"]
"#,
    );
    let input = workspace.write("config/docs/fallback.md", "Open the valve.\n");

    let output = run_goodwrite(
        &["--config", as_utf8(&config), "check", as_utf8(&input)],
        workspace.root(),
    );
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(output.status.success(), "{stderr}");
    assert!(
        !stderr.contains("goodwrite/missing-mode-annotation"),
        "{stderr}"
    );
}

#[test]
fn typst_requirement_typed_helper_is_supported() {
    let workspace = TempWorkspace::new();
    let config = workspace.write(
        "goodwrite.toml",
        r#"[profiles]
enable = ["ears"]

[requirements]
active_rulesets = ["ears"]
default_ruleset = "ears"
"#,
    );
    let input = workspace.write(
        "requirements.typ",
        "#requirement_event[\nWhen the pilot presses the button, the system shall record the event.\n]\n",
    );

    let output = run_goodwrite(
        &["--config", as_utf8(&config), "check", as_utf8(&input)],
        workspace.root(),
    );
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(output.status.success(), "{stderr}");
    assert!(
        !stderr.contains("ears/invalid-requirement-type"),
        "{stderr}"
    );
}

#[test]
fn ste_checks_apply_to_requirements() {
    let workspace = TempWorkspace::new();
    let config = workspace.write(
        "goodwrite.toml",
        r#"[profiles]
enable = ["asd-ste100"]

[requirements]
active_rulesets = ["ears"]
default_ruleset = "ears"
"#,
    );

    let input = workspace.write(
        "requirements.md",
        "<!-- goodwrite:mode:descriptive -->\n<!-- goodwrite:requirement -->\nUtilize this mode.\n<!-- goodwrite:requirement:end -->\n",
    );

    let output = run_goodwrite(
        &["--config", as_utf8(&config), "check", as_utf8(&input)],
        workspace.root(),
    );

    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(!output.status.success(), "{stderr}");
    assert!(stderr.contains("asd-ste100/unapproved-word"), "{stderr}");
}

#[test]
fn unsafe_annotation_exempts_target_rule() {
    let workspace = TempWorkspace::new();
    let config = workspace.write(
        "goodwrite.toml",
        r#"[profiles]
enable = ["asd-ste100"]
"#,
    );
    let input = workspace.write(
        "unsafe.md",
        "<!-- goodwrite:mode:descriptive -->\n<!-- goodwrite:unsafe(asd-ste100/unapproved-word): required wording from approved certification source -->\nUtilize this mode.\n",
    );

    let output = run_goodwrite(
        &["--config", as_utf8(&config), "check", as_utf8(&input)],
        workspace.root(),
    );

    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(output.status.success(), "{stderr}");
    // Check that the rule wasn't emitted as a diagnostic code (the rule ID
    // may appear in source snippets from the unsafe annotation text).
    assert!(
        !stderr.contains("diagnostic code: asd-ste100/unapproved-word"),
        "{stderr}"
    );
    assert!(!stderr.contains("goodwrite/unsafe-"), "{stderr}");
}

#[test]
fn unsafe_annotation_reports_unknown_rule() {
    let workspace = TempWorkspace::new();
    let config = workspace.write(
        "goodwrite.toml",
        r#"[profiles]
enable = ["asd-ste100"]
"#,
    );
    let input = workspace.write(
        "unsafe-unknown.md",
        "<!-- goodwrite:mode:descriptive -->\n<!-- goodwrite:unsafe(nonexistent/rule): required wording from source -->\nUtilize this mode.\n",
    );

    let output = run_goodwrite(
        &["--config", as_utf8(&config), "check", as_utf8(&input)],
        workspace.root(),
    );

    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(!output.status.success(), "{stderr}");
    assert!(stderr.contains("goodwrite/unsafe-unknown-rule"), "{stderr}");
}

#[test]
fn unsafe_annotation_reports_stale_entry() {
    let workspace = TempWorkspace::new();
    let config = workspace.write(
        "goodwrite.toml",
        r#"[profiles]
enable = ["asd-ste100"]
"#,
    );
    let input = workspace.write(
        "unsafe-stale.md",
        "<!-- goodwrite:mode:descriptive -->\n<!-- goodwrite:unsafe(asd-ste100/semicolons): required wording from source -->\nUse this mode.\n",
    );

    let output = run_goodwrite(
        &["--config", as_utf8(&config), "check", as_utf8(&input)],
        workspace.root(),
    );

    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(!output.status.success(), "{stderr}");
    assert!(stderr.contains("goodwrite/unsafe-stale"), "{stderr}");
}

#[test]
fn unsafe_annotation_requires_substantive_reason() {
    let workspace = TempWorkspace::new();
    let config = workspace.write(
        "goodwrite.toml",
        r#"[profiles]
enable = ["asd-ste100"]
"#,
    );
    let input = workspace.write(
        "unsafe-short-reason.md",
        "<!-- goodwrite:mode:descriptive -->\n<!-- goodwrite:unsafe(asd-ste100/unapproved-word): short -->\nUtilize this mode.\n",
    );

    let output = run_goodwrite(
        &["--config", as_utf8(&config), "check", as_utf8(&input)],
        workspace.root(),
    );

    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(!output.status.success(), "{stderr}");
    assert!(stderr.contains("goodwrite/unsafe-invalid"), "{stderr}");
}

#[test]
fn check_accepts_directory_and_glob_inputs() {
    let workspace = TempWorkspace::new();
    let config = workspace.write(
        "goodwrite.toml",
        r#"[profiles]
enable = ["asd-ste100"]
"#,
    );
    let _doc1 = workspace.write(
        "docs/a.md",
        "<!-- goodwrite:mode:descriptive -->\nThe operator must abandon the area.\n",
    );
    let _doc2 = workspace.write(
        "docs/b.md",
        "<!-- goodwrite:mode:descriptive -->\nThe operator must abandon the area.\n",
    );

    let output_dir = run_goodwrite(
        &[
            "--config",
            as_utf8(&config),
            "check",
            as_utf8(&workspace.root().join("docs")),
        ],
        workspace.root(),
    );
    let stderr_dir = String::from_utf8_lossy(&output_dir.stderr);
    assert!(!output_dir.status.success(), "{stderr_dir}");
    assert!(stderr_dir.contains("docs/a.md"), "{stderr_dir}");
    assert!(stderr_dir.contains("docs/b.md"), "{stderr_dir}");

    let output_glob = run_goodwrite(
        &["--config", as_utf8(&config), "check", "docs/*.md"],
        workspace.root(),
    );
    let stderr_glob = String::from_utf8_lossy(&output_glob.stderr);
    assert!(!output_glob.status.success(), "{stderr_glob}");
    assert!(stderr_glob.contains("docs/a.md"), "{stderr_glob}");
    assert!(stderr_glob.contains("docs/b.md"), "{stderr_glob}");
}

struct TempWorkspace {
    root: PathBuf,
}

impl TempWorkspace {
    fn new() -> Self {
        let mut root = std::env::temp_dir();
        root.push(format!("goodwrite_cli_test_{}", unique_stamp()));
        fs::create_dir_all(&root).expect("create temp workspace");
        Self { root }
    }

    fn root(&self) -> &Path {
        &self.root
    }

    fn write(&self, relative: &str, contents: &str) -> PathBuf {
        let path = self.root.join(relative);
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).expect("create parent directory");
        }
        fs::write(&path, contents).expect("write temp file");
        path
    }
}

impl Drop for TempWorkspace {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.root);
    }
}

fn run_goodwrite(args: &[&str], cwd: &Path) -> Output {
    Command::new(env!("CARGO_BIN_EXE_goodwrite"))
        .current_dir(cwd)
        .args(args)
        .output()
        .expect("run goodwrite")
}

fn as_utf8(path: &Path) -> &str {
    path.to_str().expect("utf8 path")
}

fn unique_stamp() -> u128 {
    static COUNTER: AtomicU64 = AtomicU64::new(0);

    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("clock")
        .as_nanos();
    let pid = u128::from(std::process::id());
    let seq = u128::from(COUNTER.fetch_add(1, Ordering::Relaxed));
    (now << 32) ^ (pid << 16) ^ seq
}
