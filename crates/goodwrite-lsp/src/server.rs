use std::{
    collections::HashMap,
    path::{Path, PathBuf},
    sync::Arc,
};

use goodwrite_core::{
    CheckContext, Diagnostic as CoreDiagnostic, GlossaryFileData, GoodwriteConfig, RuleInput,
    RuleSet,
};
use tokio::{
    sync::{Mutex, RwLock},
    task::JoinHandle,
    time::{Duration, sleep},
};
use tower_lsp_server::{
    Client, LanguageServer,
    jsonrpc::Result,
    ls_types::{
        CodeActionParams, CodeActionResponse, DidChangeTextDocumentParams,
        DidCloseTextDocumentParams, DidOpenTextDocumentParams, DidSaveTextDocumentParams,
        InitializeParams, InitializeResult, InitializedParams, MessageType, ServerCapabilities,
        TextDocumentSyncCapability, TextDocumentSyncKind, Uri,
    },
};

use crate::{actions, diagnostics};

/// `tower-lsp-server` backend for goodwrite.
#[derive(Clone)]
pub struct GoodwriteServer {
    client: Client,
    config: Arc<RwLock<GoodwriteConfig>>,
    workspace_root: Arc<RwLock<Option<PathBuf>>>,
    open_documents: Arc<RwLock<HashMap<Uri, String>>>,
    pending_jobs: Arc<Mutex<HashMap<Uri, JoinHandle<()>>>>,
}

impl GoodwriteServer {
    pub fn new(client: Client) -> Self {
        Self {
            client,
            config: Arc::new(RwLock::new(GoodwriteConfig::default())),
            workspace_root: Arc::new(RwLock::new(None)),
            open_documents: Arc::new(RwLock::new(HashMap::new())),
            pending_jobs: Arc::new(Mutex::new(HashMap::new())),
        }
    }

    async fn schedule_publish(&self, uri: Uri, text: String) {
        let mut pending = self.pending_jobs.lock().await;
        if let Some(existing) = pending.remove(&uri) {
            existing.abort();
        }

        let server = self.clone();
        let uri_for_task = uri.clone();
        let handle = tokio::spawn(async move {
            sleep(Duration::from_millis(200)).await;
            let core = server.analyze_document(&uri_for_task, &text).await;
            let lsp = core
                .iter()
                .map(|diagnostic| diagnostics::to_lsp_diagnostic(diagnostic, &text))
                .collect::<Vec<_>>();
            server
                .client
                .publish_diagnostics(uri_for_task, lsp, None)
                .await;
        });

        pending.insert(uri, handle);
    }

    async fn analyze_document(&self, uri: &Uri, text: &str) -> Vec<CoreDiagnostic> {
        let Some(path) = uri.to_file_path() else {
            return Vec::new();
        };

        let extract = match goodwrite_extract::extract_source(path.as_ref(), text) {
            Ok(value) => value,
            Err(error) => {
                let message = format!("failed to parse document: {error}");
                return vec![CoreDiagnostic::new(
                    "goodwrite/extract",
                    goodwrite_core::Severity::Error,
                    message,
                    goodwrite_core::SourceRange::new(0, 1.min(text.len())),
                )];
            }
        };

        let config = self.config.read().await.clone();
        let root = self.workspace_root.read().await.clone();
        let glossary_data = match load_glossary(&config, root.as_deref()) {
            Ok(value) => value,
            Err(message) => {
                return vec![CoreDiagnostic::new(
                    "goodwrite/glossary",
                    goodwrite_core::Severity::Error,
                    message,
                    goodwrite_core::SourceRange::new(0, 1.min(text.len())),
                )];
            }
        };
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
        let ruleset = build_ruleset(&config);

        let context = CheckContext {
            config,
            glossary,
            glossary_data,
            file_has_mode_annotations: extract.has_mode_annotations,
        };

        let mut diagnostics = Vec::new();
        let used_mode_heuristic = extract.used_mode_heuristic;
        for span in extract.spans {
            let mut input = RuleInput {
                file_path: path.display().to_string(),
                sentences: goodwrite_tokenize::tokenize_span(&span),
                span,
            };
            diagnostics.extend(ruleset.run(&mut input, &context));
        }

        if used_mode_heuristic && context.profile_enabled("asd-ste100") {
            diagnostics.push(
                CoreDiagnostic::new(
                    "goodwrite/heuristic-fallback",
                    if context.config.heuristics.strict {
                        goodwrite_core::Severity::Error
                    } else {
                        goodwrite_core::Severity::Warning
                    },
                    "writing mode fallback inference was used",
                    goodwrite_core::SourceRange::new(0, 1.min(text.len())),
                )
                .with_help(
                    "add explicit mode annotations (for example: <!-- goodwrite:mode:descriptive -->)",
                ),
            );
        }

        diagnostics.sort_by_key(|diagnostic| diagnostic.span.start);
        diagnostics
    }

    async fn maybe_reload_config(&self, uri: &Uri) {
        let Some(path) = uri.to_file_path() else {
            return;
        };
        if path
            .file_name()
            .and_then(|name| name.to_str())
            .is_none_or(|name| name != "goodwrite.toml")
        {
            return;
        }

        let root = self.workspace_root.read().await.clone();
        let mut config = self.config.write().await;
        *config = load_config(root.as_deref());
        self.client
            .log_message(MessageType::INFO, "reloaded goodwrite.toml")
            .await;
    }

    async fn source_for_uri(&self, uri: &Uri) -> Option<String> {
        if let Some(source) = self.open_documents.read().await.get(uri).cloned() {
            return Some(source);
        }

        let path = uri.to_file_path()?;
        std::fs::read_to_string(path).ok()
    }

    fn extension_for_uri(uri: &Uri) -> Option<String> {
        let path = uri.to_file_path()?;
        path.extension()
            .and_then(|ext| ext.to_str())
            .map(|ext| ext.to_ascii_lowercase())
    }
}

impl LanguageServer for GoodwriteServer {
    async fn initialize(&self, params: InitializeParams) -> Result<InitializeResult> {
        if let Some(folders) = params.workspace_folders {
            if let Some(first) = folders.first() {
                if let Some(root_path) = first.uri.to_file_path() {
                    *self.workspace_root.write().await = Some(root_path.to_path_buf());
                }
            }
        }

        let root = self.workspace_root.read().await.clone();
        *self.config.write().await = load_config(root.as_deref());

        Ok(InitializeResult {
            server_info: None,
            capabilities: ServerCapabilities {
                text_document_sync: Some(TextDocumentSyncCapability::Kind(
                    TextDocumentSyncKind::FULL,
                )),
                code_action_provider: Some(
                    tower_lsp_server::ls_types::CodeActionProviderCapability::Options(
                        tower_lsp_server::ls_types::CodeActionOptions {
                            code_action_kinds: Some(vec![
                                tower_lsp_server::ls_types::CodeActionKind::QUICKFIX,
                            ]),
                            work_done_progress_options: Default::default(),
                            resolve_provider: Some(false),
                        },
                    ),
                ),
                ..ServerCapabilities::default()
            },
        })
    }

    async fn initialized(&self, _: InitializedParams) {
        self.client
            .log_message(MessageType::INFO, "goodwrite-lsp initialized")
            .await;
    }

    async fn shutdown(&self) -> Result<()> {
        Ok(())
    }

    async fn did_open(&self, params: DidOpenTextDocumentParams) {
        let uri = params.text_document.uri;
        let text = params.text_document.text;
        self.open_documents
            .write()
            .await
            .insert(uri.clone(), text.clone());
        self.schedule_publish(uri, text).await;
    }

    async fn did_change(&self, params: DidChangeTextDocumentParams) {
        let uri = params.text_document.uri;
        let Some(change) = params.content_changes.into_iter().last() else {
            return;
        };

        let text = change.text;
        self.open_documents
            .write()
            .await
            .insert(uri.clone(), text.clone());
        self.schedule_publish(uri, text).await;
    }

    async fn did_save(&self, params: DidSaveTextDocumentParams) {
        let uri = params.text_document.uri;
        self.maybe_reload_config(&uri).await;

        let text = if let Some(text) = params.text {
            text
        } else {
            self.source_for_uri(&uri).await.unwrap_or_default()
        };

        if !text.is_empty() {
            self.open_documents
                .write()
                .await
                .insert(uri.clone(), text.clone());
            self.schedule_publish(uri, text).await;
        }
    }

    async fn did_close(&self, params: DidCloseTextDocumentParams) {
        let uri = params.text_document.uri;
        self.open_documents.write().await.remove(&uri);
        self.client.publish_diagnostics(uri, Vec::new(), None).await;
    }

    async fn code_action(&self, params: CodeActionParams) -> Result<Option<CodeActionResponse>> {
        let uri = params.text_document.uri;
        let Some(source) = self.source_for_uri(&uri).await else {
            return Ok(None);
        };

        let core = self.analyze_document(&uri, &source).await;
        let lsp_diagnostics = core
            .iter()
            .map(|diagnostic| diagnostics::to_lsp_diagnostic(diagnostic, &source))
            .collect::<Vec<_>>();

        let extension = Self::extension_for_uri(&uri);
        let mut out = Vec::new();

        for (lsp_diag, core_diag) in lsp_diagnostics.iter().zip(core.iter()) {
            if !actions::overlaps(params.range, lsp_diag) {
                continue;
            }
            out.extend(actions::actions_for_diagnostic(
                &uri,
                &source,
                extension.as_deref(),
                lsp_diag,
                core_diag,
            ));
        }

        Ok(Some(out))
    }
}

fn load_config(workspace_root: Option<&Path>) -> GoodwriteConfig {
    let config_path = match workspace_root {
        Some(root) => root.join("goodwrite.toml"),
        None => PathBuf::from("goodwrite.toml"),
    };

    GoodwriteConfig::from_path(&config_path).unwrap_or_default()
}

fn load_glossary(
    config: &GoodwriteConfig,
    workspace_root: Option<&Path>,
) -> std::result::Result<Option<GlossaryFileData>, String> {
    let path = config
        .glossary
        .path
        .clone()
        .unwrap_or_else(|| "glossary.toml".to_string());

    let resolved = match workspace_root {
        Some(root) => root.join(&path),
        None => PathBuf::from(&path),
    };

    if !resolved.exists() {
        return Ok(None);
    }

    let loaded = goodwrite_glossary::load_glossary_file_data(&resolved)
        .map_err(|error| format!("failed to load glossary: {error}"))?;
    goodwrite_asd_ste100::dict::lookup::DictionaryLookup::validate_overlay_against_embedded(
        &loaded,
    )
    .map_err(|error| format!("glossary conflicts with bundled STE dictionary: {error}"))?;
    Ok(Some(loaded))
}

fn build_ruleset(config: &GoodwriteConfig) -> RuleSet {
    let mut set = RuleSet::new();

    if config
        .profiles
        .enable
        .iter()
        .any(|name| name.eq_ignore_ascii_case("asd-ste100"))
    {
        set.extend(goodwrite_asd_ste100::rules());
    }

    if config
        .profiles
        .enable
        .iter()
        .any(|name| name.eq_ignore_ascii_case("ears"))
    {
        set.extend(goodwrite_ears::rules());
    }

    if config
        .profiles
        .enable
        .iter()
        .any(|name| name.eq_ignore_ascii_case("glossary"))
    {
        set.extend(goodwrite_glossary::rules());
    }

    set
}
