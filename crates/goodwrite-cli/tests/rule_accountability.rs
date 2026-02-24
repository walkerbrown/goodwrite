use std::{
    collections::BTreeSet,
    fs,
    path::{Path, PathBuf},
    process::{Command, Output},
    sync::atomic::{AtomicU64, Ordering},
    time::{SystemTime, UNIX_EPOCH},
};

use goodwrite_core::RuleIndex;

#[test]
fn rule_index_matches_registered_rules() {
    let index = RuleIndex::load_embedded().expect("load embedded rule index");
    let indexed = index
        .rules
        .iter()
        .map(|entry| entry.id.clone())
        .collect::<BTreeSet<_>>();

    let mut registered = BTreeSet::new();
    registered.extend(
        goodwrite_asd_ste100::rules()
            .into_iter()
            .map(|rule| rule.id().to_string()),
    );
    registered.extend(
        goodwrite_ears::rules()
            .into_iter()
            .map(|rule| rule.id().to_string()),
    );
    registered.extend(
        goodwrite_glossary::rules()
            .into_iter()
            .map(|rule| rule.id().to_string()),
    );

    assert_eq!(indexed, registered);
}

#[test]
fn every_rule_has_linked_pass_and_fail_fixture() {
    // This is the production accountability gate:
    // each indexed rule must be absent in its pass fixture and present in its
    // fail fixture under the profile-specific config.
    let index = RuleIndex::load_embedded().expect("load embedded rule index");
    let repo_root = repo_root();
    let mut exercised = 0usize;

    for entry in &index.rules {
        let pass_src = fs::read_to_string(repo_root.join(&entry.test_pass))
            .unwrap_or_else(|_| panic!("missing pass fixture for {}", entry.id));
        let fail_src = fs::read_to_string(repo_root.join(&entry.test_fail))
            .unwrap_or_else(|_| panic!("missing fail fixture for {}", entry.id));
        let pass_ext = extension_for_fixture(&entry.test_pass);
        let fail_ext = extension_for_fixture(&entry.test_fail);

        let workspace = TempWorkspace::new();
        workspace.write("goodwrite.toml", &config_for_profile(&entry.profile));
        workspace.write("glossary.toml", USER_DICTIONARY_FIXTURE);

        let pass_path = workspace.write(&format!("case-pass.{pass_ext}"), &pass_src);
        let fail_path = workspace.write(&format!("case-fail.{fail_ext}"), &fail_src);

        let pass_output = run_goodwrite(
            &["--config", "goodwrite.toml", "check", as_utf8(&pass_path)],
            workspace.root(),
        );
        let pass_stderr = String::from_utf8_lossy(&pass_output.stderr);
        assert!(
            !pass_stderr.contains(&entry.id),
            "pass fixture for {} emitted target rule:\n{}",
            entry.id,
            pass_stderr
        );

        let fail_output = run_goodwrite(
            &["--config", "goodwrite.toml", "check", as_utf8(&fail_path)],
            workspace.root(),
        );
        let fail_stderr = String::from_utf8_lossy(&fail_output.stderr);
        assert!(
            fail_stderr.contains(&entry.id),
            "fail fixture for {} did not emit target rule:\n{}",
            entry.id,
            fail_stderr
        );

        exercised += 1;
    }

    println!("Rule linkage tested: {}/{}", exercised, index.rules.len());
}

fn config_for_profile(profile: &str) -> String {
    match profile {
        "asd-ste100" => r#"[profiles]
enable = ["asd-ste100", "glossary"]

[requirements]
active_rulesets = ["ears"]
default_ruleset = "ears"

[heuristics]
strict = false

[glossary]
path = "glossary.toml"
"#
        .to_string(),
        "ears" => r#"[profiles]
enable = ["ears"]

[requirements]
active_rulesets = ["ears"]
default_ruleset = "ears"

[heuristics]
strict = false
"#
        .to_string(),
        "glossary" => r#"[profiles]
enable = ["glossary"]

[heuristics]
strict = false

[glossary]
path = "glossary.toml"
"#
        .to_string(),
        other => panic!("unsupported profile in rule index: {other}"),
    }
}

fn repo_root() -> PathBuf {
    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    manifest_dir
        .parent()
        .and_then(Path::parent)
        .expect("workspace root")
        .to_path_buf()
}

fn extension_for_fixture(path: &str) -> &'static str {
    if path.ends_with(".typ") { "typ" } else { "md" }
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

struct TempWorkspace {
    root: PathBuf,
}

impl TempWorkspace {
    fn new() -> Self {
        let mut root = std::env::temp_dir();
        root.push(format!("goodwrite_rule_linkage_{}", unique_stamp()));
        fs::create_dir_all(&root).expect("create temp workspace");
        Self { root }
    }

    fn root(&self) -> &Path {
        &self.root
    }

    fn write(&self, relative: &str, contents: &str) -> PathBuf {
        let path = self.root.join(relative);
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).expect("create parent");
        }
        fs::write(&path, contents).expect("write fixture");
        path
    }
}

impl Drop for TempWorkspace {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.root);
    }
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

const USER_DICTIONARY_FIXTURE: &str = r#"[[approved]]
word = "actuator"
pos = "noun"

[[not_approved]]
word = "control unit"
pos = "noun"
alternatives = [{ word = "actuator" }]

[[approved]]
word = "gizmo"
pos = "noun"

[[not_approved]]
word = "controller"
pos = "noun"
alternatives = [{ word = "gizmo" }]

[[approved]]
word = "NASA"
pos = "noun"

[[approved]]
word = "Telemetry Control Bus"
pos = "noun"

[[not_approved]]
word = "TCB"
pos = "noun"
alternatives = [{ word = "Telemetry Control Bus" }]
"#;
