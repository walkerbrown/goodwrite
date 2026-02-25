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
        stdout.contains("+Do not open valve. close valve."),
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
            "+The operator cannot inspection the startup trace, for example during bench validation."
        ),
        "{stdout}"
    );
    assert!(!stdout.contains("reinspectionhe"), "{stdout}");
    assert!(!stdout.contains("e.for exampleuring"), "{stdout}");
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
fn requirement_ruleset_is_controlled_by_cli_or_config_only() {
    let workspace = TempWorkspace::new();
    let config = workspace.write(
        "goodwrite.toml",
        r#"[profiles]
enable = ["ears"]

[requirements]
active_rulesets = ["internal-disabled"]
default_ruleset = "internal-disabled"
"#,
    );
    let input = workspace.write(
        "req.md",
        "<!-- goodwrite:requirement:auto -->\nWhen the pilot presses the button, while the aircraft is on ground, the system record the event.\n<!-- goodwrite:requirement:end -->\n",
    );

    let no_override = run_goodwrite(
        &["--config", as_utf8(&config), "check", as_utf8(&input)],
        workspace.root(),
    );
    let no_override_stderr = String::from_utf8_lossy(&no_override.stderr);
    assert!(no_override.status.success(), "{no_override_stderr}");
    assert!(
        !no_override_stderr.contains("ears/clause-order"),
        "{no_override_stderr}"
    );

    let with_override = run_goodwrite(
        &[
            "--config",
            as_utf8(&config),
            "--requirement-ruleset",
            "ears",
            "check",
            as_utf8(&input),
        ],
        workspace.root(),
    );
    let with_override_stderr = String::from_utf8_lossy(&with_override.stderr);
    assert!(with_override.status.success(), "{with_override_stderr}");
    assert!(
        with_override_stderr.contains("ears/clause-order"),
        "{with_override_stderr}"
    );
}

#[test]
fn heuristic_fallback_emits_warning_and_can_be_strict() {
    let workspace = TempWorkspace::new();
    let warning_config = workspace.write(
        "warning.toml",
        r#"[profiles]
enable = ["asd-ste100"]

[heuristics]
strict = false
"#,
    );
    let strict_config = workspace.write(
        "strict.toml",
        r#"[profiles]
enable = ["asd-ste100"]

[heuristics]
strict = true
"#,
    );
    let input = workspace.write("fallback.md", "Open the valve.\n");

    let warning_output = run_goodwrite(
        &[
            "--config",
            as_utf8(&warning_config),
            "check",
            as_utf8(&input),
        ],
        workspace.root(),
    );
    let warning_stderr = String::from_utf8_lossy(&warning_output.stderr);
    assert!(warning_output.status.success(), "{warning_stderr}");
    assert!(
        warning_stderr.contains("goodwrite/heuristic-fallback"),
        "{warning_stderr}"
    );

    let strict_output = run_goodwrite(
        &[
            "--config",
            as_utf8(&strict_config),
            "check",
            as_utf8(&input),
        ],
        workspace.root(),
    );
    let strict_stderr = String::from_utf8_lossy(&strict_output.stderr);
    assert!(!strict_output.status.success(), "{strict_stderr}");
    assert!(
        strict_stderr.contains("goodwrite/heuristic-fallback"),
        "{strict_stderr}"
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
