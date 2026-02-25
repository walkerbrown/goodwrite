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
    about = "A linter for Simplified Technical English (ASD-STE100)",
    long_about = "Checks technical documentation for compliance with the Simplified Technical English standard (ASD-STE100), requirement-style grammars, and your domain-specific technical glossary.",
    after_help = "Quick start:\n  goodwrite check docs/\n  goodwrite fix docs/manual.md\n  goodwrite init config\n  goodwrite init glossary\n\nRule explorer: https://goodwrite.dev/rules\n",
    arg_required_else_help = true
)]
struct Cli {
    #[arg(short, long, default_value = "goodwrite.toml")]
    config: PathBuf,

    /// Escalate heuristic fallback diagnostics from warning to error.
    #[arg(long)]
    strict: bool,

    /// Control color output (auto, always, never).
    #[arg(long, value_enum, default_value = "auto")]
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
    /// Apply machine-applicable suggestions.
    Fix {
        #[arg(value_name = "PATH")]
        files: Vec<PathBuf>,
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
        strict,
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
            let mut config = load_config(&config_path)?;
            apply_cli_overrides(&mut config, strict);

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

            let checks = analyze_files(&files, &config, progress.as_ref())?;

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
            let mut config = load_config(&config_path)?;
            apply_cli_overrides(&mut config, strict);
            let checks = analyze_files(&files, &config, None)?;
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
                    println!("{}", fix::diff_output(&item.path, &item.source, &updated));
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

fn apply_cli_overrides(config: &mut GoodwriteConfig, strict: bool) {
    if strict {
        config.heuristics.strict = true;
    }
}

fn analyze_files(
    files: &[PathBuf],
    config: &GoodwriteConfig,
    progress: Option<&indicatif::ProgressBar>,
) -> Result<Vec<FileDiagnostics>, CliError> {
    let files = expand_input_files(files, &config.check.exclude)?;

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
        let file = analyze_extract(
            path,
            extract,
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
    config: &GoodwriteConfig,
    glossary: Option<goodwrite_core::GlossaryData>,
    glossary_data: Option<GlossaryFileData>,
    ruleset: &RuleSet,
) -> FileDiagnostics {
    let mut diagnostics = Vec::new();
    let used_mode_heuristic = extract.used_mode_heuristic;

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

    if used_mode_heuristic && context.profile_enabled("asd-ste100") {
        // Heuristic fallback warnings are file-level signals: they indicate
        // metadata quality debt, not a sentence-local grammar violation.
        diagnostics.push(
            Diagnostic::new(
                "goodwrite/heuristic-fallback",
                if context.config.heuristics.strict {
                    Severity::Error
                } else {
                    Severity::Warning
                },
                "writing mode fallback inference was used",
                goodwrite_core::SourceRange::new(0, 1.min(extract.source.len())),
            )
            .with_help(
                "add explicit mode annotations (for example: <!-- goodwrite:mode:descriptive -->)",
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

fn load_config(path: &Path) -> Result<GoodwriteConfig, CliError> {
    if !path.exists() {
        return Ok(GoodwriteConfig::default());
    }
    GoodwriteConfig::from_path(path).map_err(CliError::Config)
}

fn init_file(target: InitTarget) -> Result<&'static str, CliError> {
    const CONFIG: &str = r#"# Optional: uncomment to customize enabled profiles.
# [profiles]
# enable = ["asd-ste100", "ears", "glossary"]

# Optional: treat heuristic fallback diagnostics as errors.
# [heuristics]
# strict = true

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
