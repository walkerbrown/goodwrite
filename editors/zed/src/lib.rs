//! Zed extension metadata for launching the `goodwrite-lsp` language server.

/// Language server command configured in `extension.toml`.
pub const LANGUAGE_SERVER_COMMAND: &str = "goodwrite-lsp";

/// Filetypes handled by the extension.
pub const SUPPORTED_LANGUAGES: &[&str] = &["Markdown", "Typst"];
