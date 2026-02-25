mod fix;
mod format;
mod render;

use std::{
    collections::BTreeSet,
    fs,
    path::{Path, PathBuf},
    process,
};

use clap::{Parser, Subcommand, ValueEnum};
use goodwrite_core::{
    Applicability, CheckContext, Diagnostic, ExtractResult, GlossaryFileData, GoodwriteConfig,
    RuleInput, RuleSet, Severity,
};
use thiserror::Error;

#[derive(Debug, Parser)]
#[command(name = "goodwrite")]
#[command(version)]
#[command(
    about = "A linter for engineering requirements and Simplified Technical English (ASD-STE100).",
    long_about = "A linter for engineering requirements and Simplified Technical English (ASD-STE100).",
    override_usage = "goodwrite [OPTIONS] COMMAND [PATH]...",
    after_help = "Quick start:\n  goodwrite check docs/\n  goodwrite fix docs/manual.md\n  goodwrite init config\n  goodwrite init glossary\n\nhttps://goodwrite.dev\n",
    arg_required_else_help = true
)]
struct Cli {
    #[arg(short, long, default_value = "goodwrite.toml")]
    config: PathBuf,

    /// Control color output (auto, always, never).
    #[arg(long, visible_alias = "colors", value_enum, default_value = "auto")]
    color: ColorChoice,

    #[command(subcommand)]
    command: Command,
}

#[derive(Debug, Clone, Copy, ValueEnum)]
enum ColorChoice {
    Auto,
    Always,
    Never,
}

#[derive(Debug, Subcommand)]
enum Command {
    /// Check files and print diagnostics.
    Check {
        #[arg(value_name = "PATH")]
        files: Vec<PathBuf>,
        #[arg(long, value_enum, default_value = "terminal")]
        format: OutputFormat,
    },
    /// Apply machine-applicable suggestions (accepts `--dry-run`).
    Fix {
        #[arg(value_name = "PATH")]
        files: Vec<PathBuf>,
        /// Print planned edits as unified diffs without writing files.
        #[arg(long)]
        dry_run: bool,
    },
    /// Create a starter file (`config` or `glossary`).
    Init {
        #[arg(value_name = "TARGET", value_enum)]
        target: InitTarget,
    },
}

#[derive(Debug, Clone, Copy, ValueEnum)]
enum OutputFormat {
    Terminal,
    Json,
    Sarif,
}

#[derive(Debug, Clone, Copy, ValueEnum)]
enum InitTarget {
    Config,
    Glossary,
}

#[derive(Debug)]
struct FileDiagnostics {
    path: PathBuf,
    source: String,
    diagnostics: Vec<Diagnostic>,
}

fn main() {
    let cli = Cli::parse();
    let exit_code = run(cli);
    process::exit(exit_code);
}

fn run(cli: Cli) -> i32 {
    match execute(cli) {
        Ok(code) => code,
        Err(error) => {
            eprintln!("error: {error}");
            2
        }
    }
}

fn execute(cli: Cli) -> Result<i32, CliError> {
    let Cli {
        config: config_path,
        color: color_choice,
        command,
    } = cli;

    let color = match color_choice {
        ColorChoice::Always => true,
        ColorChoice::Never => false,
        ColorChoice::Auto => {
            std::env::var_os("NO_COLOR").is_none()
                && std::io::IsTerminal::is_terminal(&std::io::stderr())
        }
    };

    setup_miette(color);

    match command {
        Command::Init { target } => {
            let path = init_file(target)?;
            println!("created {path}");
            Ok(0)
        }
        Command::Check { files, format } => {
            let loaded = load_config(&config_path)?;

            let progress = matches!(format, OutputFormat::Terminal)
                .then(|| std::io::IsTerminal::is_terminal(&std::io::stderr()))
                .filter(|&is_tty| is_tty)
                .map(|_| {
                    let pb = indicatif::ProgressBar::new(0);
                    pb.set_draw_target(indicatif::ProgressDrawTarget::stderr());
                    pb.set_style(
                        indicatif::ProgressStyle::with_template("  Checking {pos}/{len} {msg}")
                            .unwrap(),
                    );
                    pb
                });

            let checks = analyze_files(&files, &loaded.config, &loaded.dir, progress.as_ref())?;

            match format {
                OutputFormat::Terminal => {
                    format::render_terminal(&checks);
                    format::render_summary(&checks, color);
                }
                OutputFormat::Json => format::render_json(&checks),
                OutputFormat::Sarif => format::render_sarif(&checks),
            }

            let has_error = checks.iter().any(|file| {
                file.diagnostics
                    .iter()
                    .any(|diag| diag.severity == Severity::Error)
            });

            Ok(if has_error { 1 } else { 0 })
        }
        Command::Fix { files, dry_run } => {
            let loaded = load_config(&config_path)?;
            let checks = analyze_files(&files, &loaded.config, &loaded.dir, None)?;
            let mut changed_files = 0usize;

            for item in checks {
                let suggestions = item
                    .diagnostics
                    .iter()
                    .flat_map(|diag| diag.suggestions.iter())
                    .filter(|suggestion| {
                        suggestion.applicability == Applicability::MachineApplicable
                    })
                    .cloned()
                    .collect::<Vec<_>>();

                if suggestions.is_empty() {
                    continue;
                }

                let (updated, applied) = fix::apply_suggestions(&item.source, &suggestions);
                if applied == 0 || updated == item.source {
                    continue;
                }

                changed_files += 1;
                if dry_run {
                    println!(
                        "{}",
                        fix::diff_output(&item.path, &item.source, &updated, color)
                    );
                } else {
                    fs::write(&item.path, updated).map_err(|source| CliError::Write {
                        path: item.path.display().to_string(),
                        source,
                    })?;
                    println!("fixed {} ({} edits)", item.path.display(), applied);
                }
            }

            if changed_files == 0 {
                println!("no machine-applicable fixes");
            }

            Ok(0)
        }
    }
}

fn setup_miette(color: bool) {
    miette::set_hook(Box::new(move |_| {
        if color {
            Box::new(miette::GraphicalReportHandler::new())
        } else {
            Box::new(miette::NarratableReportHandler::new())
        }
    }))
    .ok();
}

fn analyze_files(
    files: &[PathBuf],
    config: &GoodwriteConfig,
    config_dir: &Path,
    progress: Option<&indicatif::ProgressBar>,
) -> Result<Vec<FileDiagnostics>, CliError> {
    let files = expand_input_files(files, &config.check.exclude)?;
    let unsafe_ignore = compile_unsafe_ignore_patterns(&config.unsafe_.ignore);

    if let Some(pb) = progress {
        pb.set_length(files.len() as u64);
    }

    let ruleset = build_ruleset(config);
    let glossary_data = load_glossary(config)?;
    let glossary = config
        .profiles
        .enable
        .iter()
        .any(|profile| profile.eq_ignore_ascii_case("glossary"))
        .then(|| {
            glossary_data.as_ref().map(|loaded| {
                goodwrite_core::GlossaryData::new(
                    loaded.approved.clone(),
                    loaded.not_approved.clone(),
                )
            })
        })
        .flatten();

    let mut out = Vec::new();
    for path in &files {
        if let Some(pb) = progress {
            pb.set_message(
                path.file_name()
                    .unwrap_or_default()
                    .to_string_lossy()
                    .to_string(),
            );
        }

        let extract = goodwrite_extract::extract_path(path).map_err(CliError::Extract)?;
        let mode_notice_ignored = is_mode_notice_ignored(path, config_dir, &unsafe_ignore);
        let file = analyze_extract(
            path,
            extract,
            mode_notice_ignored,
            config,
            glossary.clone(),
            glossary_data.clone(),
            &ruleset,
        );
        out.push(file);

        if let Some(pb) = progress {
            pb.inc(1);
        }
    }

    if let Some(pb) = progress {
        pb.finish_and_clear();
    }

    Ok(out)
}

fn expand_input_files(files: &[PathBuf], exclude: &[String]) -> Result<Vec<PathBuf>, CliError> {
    if files.is_empty() {
        return Err(CliError::Usage("no input files provided".to_string()));
    }

    let exclude_patterns: Vec<glob::Pattern> = exclude
        .iter()
        .filter_map(|pat| glob::Pattern::new(pat).ok())
        .collect();

    let mut collected = BTreeSet::new();
    for path in files {
        let raw = path.to_string_lossy();
        if has_glob_pattern(&raw) {
            let entries = glob::glob(&raw).map_err(|error| CliError::UnsupportedGlobPattern {
                pattern: format!("{raw}: {error}"),
            })?;
            for entry in entries {
                let item = entry.map_err(|error| CliError::ReadDir {
                    path: raw.to_string(),
                    source: std::io::Error::other(error),
                })?;
                if item.is_file()
                    && is_supported_extension(&item)
                    && !is_excluded(&item, &exclude_patterns)
                {
                    collected.insert(item);
                }
            }
            continue;
        }

        if path.is_file() {
            if is_supported_extension(path) && !is_excluded(path, &exclude_patterns) {
                collected.insert(path.clone());
            }
            continue;
        }

        if path.is_dir() {
            let mut walked = Vec::new();
            collect_supported_files(path, &mut walked)?;
            for item in walked {
                if !is_excluded(&item, &exclude_patterns) {
                    collected.insert(item);
                }
            }
            continue;
        }

        return Err(CliError::MissingInputPath {
            path: path.display().to_string(),
        });
    }

    if collected.is_empty() {
        return Err(CliError::Usage(
            "no supported input files found (.md, .markdown, .typ)".to_string(),
        ));
    }

    Ok(collected.into_iter().collect())
}

fn is_excluded(path: &Path, patterns: &[glob::Pattern]) -> bool {
    let path_str = path.to_string_lossy();
    patterns.iter().any(|pat| pat.matches(&path_str))
}

fn compile_unsafe_ignore_patterns(patterns: &[String]) -> Vec<glob::Pattern> {
    patterns
        .iter()
        .filter_map(|value| glob::Pattern::new(value).ok())
        .collect()
}

fn is_mode_notice_ignored(path: &Path, config_dir: &Path, patterns: &[glob::Pattern]) -> bool {
    if patterns.is_empty() {
        return false;
    }

    let file_abs = absolute_from_cwd(path);
    let file_abs_candidate = normalize_glob_candidate(&file_abs);
    let file_rel_candidate = file_abs
        .strip_prefix(config_dir)
        .ok()
        .map(normalize_glob_candidate);

    patterns.iter().any(|pattern| {
        pattern.matches(&file_abs_candidate)
            || file_rel_candidate
                .as_ref()
                .is_some_and(|relative| pattern.matches(relative))
    })
}

fn absolute_from_cwd(path: &Path) -> PathBuf {
    if path.is_absolute() {
        path.to_path_buf()
    } else {
        std::env::current_dir()
            .unwrap_or_else(|_| PathBuf::from("."))
            .join(path)
    }
}

fn normalize_glob_candidate(path: &Path) -> String {
    path.to_string_lossy().replace('\\', "/")
}

fn collect_supported_files(path: &Path, out: &mut Vec<PathBuf>) -> Result<(), CliError> {
    if path.is_file() {
        if is_supported_extension(path) {
            out.push(path.to_path_buf());
        }
        return Ok(());
    }

    let entries = fs::read_dir(path).map_err(|source| CliError::ReadDir {
        path: path.display().to_string(),
        source,
    })?;

    for entry in entries {
        let entry = entry.map_err(|source| CliError::ReadDir {
            path: path.display().to_string(),
            source,
        })?;

        let candidate = entry.path();
        if candidate.is_dir() {
            collect_supported_files(&candidate, out)?;
            continue;
        }

        if candidate.is_file() && is_supported_extension(&candidate) {
            out.push(candidate);
        }
    }

    Ok(())
}

fn has_glob_pattern(value: &str) -> bool {
    value.contains('*') || value.contains('?') || value.contains('[')
}

fn is_supported_extension(path: &Path) -> bool {
    path.extension()
        .and_then(|ext| ext.to_str())
        .is_some_and(|ext| {
            ext.eq_ignore_ascii_case("md")
                || ext.eq_ignore_ascii_case("markdown")
                || ext.eq_ignore_ascii_case("typ")
        })
}

fn analyze_extract(
    path: &Path,
    extract: ExtractResult,
    mode_notice_ignored: bool,
    config: &GoodwriteConfig,
    glossary: Option<goodwrite_core::GlossaryData>,
    glossary_data: Option<GlossaryFileData>,
    ruleset: &RuleSet,
) -> FileDiagnostics {
    let mut diagnostics = Vec::new();
    let used_mode_inference = extract.used_mode_inference;

    let context = CheckContext {
        config: config.clone(),
        glossary,
        glossary_data,
        file_has_mode_annotations: extract.has_mode_annotations,
    };

    for span in extract.spans {
        let sentences = goodwrite_tokenize::tokenize_span(&span);
        let mut input = RuleInput {
            file_path: path.display().to_string(),
            span,
            sentences,
        };
        diagnostics.extend(ruleset.run(&mut input, &context));
    }

    if used_mode_inference && context.profile_enabled("asd-ste100") && !mode_notice_ignored {
        diagnostics.push(
            Diagnostic::new(
                "goodwrite/missing-mode-annotation",
                Severity::Info,
                "no file-level writing mode annotation was found",
                goodwrite_core::SourceRange::new(0, 1.min(extract.source.len())),
            )
            .with_help(
                "add explicit mode annotations (for example: <!-- goodwrite:mode:descriptive -->) or add a path glob to [unsafe].ignore in goodwrite.toml",
            ),
        );
    }

    diagnostics.sort_by_key(|diagnostic| diagnostic.span.start);

    FileDiagnostics {
        path: path.to_path_buf(),
        source: extract.source,
        diagnostics,
    }
}

fn load_glossary(config: &GoodwriteConfig) -> Result<Option<GlossaryFileData>, CliError> {
    let path = config
        .glossary
        .path
        .clone()
        .unwrap_or_else(|| "glossary.toml".to_string());

    if !Path::new(&path).exists() {
        return Ok(None);
    }

    let glossary_data =
        goodwrite_glossary::load_glossary_file_data(Path::new(&path)).map_err(|source| {
            CliError::Glossary {
                path: path.clone(),
                source,
            }
        })?;

    goodwrite_asd_ste100::dict::lookup::DictionaryLookup::validate_overlay_against_embedded(
        &glossary_data,
    )
    .map_err(CliError::GlossaryConflict)?;

    Ok(Some(glossary_data))
}

fn build_ruleset(config: &GoodwriteConfig) -> RuleSet {
    let mut rules = RuleSet::new();

    if config
        .profiles
        .enable
        .iter()
        .any(|profile| profile.eq_ignore_ascii_case("asd-ste100"))
    {
        rules.extend(goodwrite_asd_ste100::rules());
    }

    if config
        .profiles
        .enable
        .iter()
        .any(|profile| profile.eq_ignore_ascii_case("ears"))
    {
        rules.extend(goodwrite_ears::rules());
    }

    if config
        .profiles
        .enable
        .iter()
        .any(|profile| profile.eq_ignore_ascii_case("glossary"))
    {
        rules.extend(goodwrite_glossary::rules());
    }

    rules
}

struct LoadedConfig {
    config: GoodwriteConfig,
    dir: PathBuf,
}

fn load_config(path: &Path) -> Result<LoadedConfig, CliError> {
    let dir = resolve_config_dir(path)?;
    if !path.exists() {
        return Ok(LoadedConfig {
            config: GoodwriteConfig::default(),
            dir,
        });
    }
    let config = GoodwriteConfig::from_path(path).map_err(CliError::Config)?;
    Ok(LoadedConfig { config, dir })
}

fn resolve_config_dir(path: &Path) -> Result<PathBuf, CliError> {
    let cwd = std::env::current_dir().map_err(|source| CliError::CurrentDir { source })?;
    let absolute_path = if path.is_absolute() {
        path.to_path_buf()
    } else {
        cwd.join(path)
    };

    Ok(absolute_path
        .parent()
        .map(|parent| parent.to_path_buf())
        .unwrap_or(cwd))
}

fn init_file(target: InitTarget) -> Result<&'static str, CliError> {
    const CONFIG: &str = r#"# Optional: uncomment to customize enabled profiles.
# [profiles]
# enable = ["asd-ste100", "ears", "glossary"]

# Optional: suppress file-level mode annotation info for matched paths.
# Paths are evaluated as glob patterns relative to goodwrite.toml.
# [unsafe]
# ignore = ["docs/legacy/**/*.md"]

# Optional: customize glossary location if you do not use ./glossary.toml.
# [glossary]
# path = "glossary.toml"
"#;

    const GLOSSARY: &str = r#"# Optional: uncomment and adapt these examples.
# [[approved]]
# word = "actuator"
# pos = "noun"
#
# [[not_approved]]
# word = "control unit"
# pos = "noun"
# alternatives = [{ word = "actuator" }]
"#;

    let path = match target {
        InitTarget::Config => "goodwrite.toml",
        InitTarget::Glossary => "glossary.toml",
    };

    if Path::new(path).exists() {
        return Err(CliError::Usage(match target {
            InitTarget::Config => {
                "goodwrite.toml already exists; refusing to overwrite".to_string()
            }
            InitTarget::Glossary => {
                "glossary.toml already exists; refusing to overwrite".to_string()
            }
        }));
    }

    let contents = match target {
        InitTarget::Config => CONFIG,
        InitTarget::Glossary => GLOSSARY,
    };

    fs::write(path, contents).map_err(|source| CliError::Write {
        path: path.to_string(),
        source,
    })?;

    Ok(path)
}

#[derive(Debug, Error)]
enum CliError {
    #[error("{0}")]
    Usage(String),
    #[error("failed to resolve current directory")]
    CurrentDir {
        #[source]
        source: std::io::Error,
    },
    #[error("failed to load config")]
    Config(#[from] goodwrite_core::ConfigError),
    #[error("failed to extract file: {0}")]
    Extract(#[from] goodwrite_extract::ExtractError),
    #[error("input path does not exist: `{path}`")]
    MissingInputPath { path: String },
    #[error("glob pattern uses unsupported syntax: `{pattern}`")]
    UnsupportedGlobPattern { pattern: String },
    #[error("failed to read directory `{path}`")]
    ReadDir {
        path: String,
        #[source]
        source: std::io::Error,
    },
    #[error("failed to load glossary `{path}`")]
    Glossary {
        path: String,
        #[source]
        source: goodwrite_glossary::GlossaryError,
    },
    #[error("glossary conflicts with bundled STE dictionary: {0}")]
    GlossaryConflict(String),
    #[error("failed to write `{path}`")]
    Write {
        path: String,
        #[source]
        source: std::io::Error,
    },
}
