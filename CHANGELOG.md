# Changelog

## 0.1.0 (Unreleased)

Initial release of goodwrite.

### Features

- **ASD-STE100 profile**: 24+ word-level rules, verb rules, sentence-length checks, and compliance engine with deterministic POS resolution
- **EARS requirement profile**: clause-order, shall-keyword, untestable-response, and requirement-type validation
- **Glossary profile**: undefined-term, synonym-enforce, and casing rules driven by `glossary.toml`
- **CLI** (`goodwrite check`, `goodwrite fix`, `goodwrite init`): terminal, JSON, and SARIF output formats with colored diagnostics and source snippets
- **Language server** (`goodwrite-lsp`): real-time diagnostics, quick-fix code actions, and suppression comment insertion for VS Code, Neovim, and Zed
- **Rule accountability**: every shipped rule indexed with citation and linked pass/fail fixtures, CI-gated
- **Format support**: Typst and Markdown with source-mapped prose extraction
- **Unsafe annotations**: per-line suppression with stale/unknown/invalid detection

### Supported Platforms

- `x86_64-unknown-linux-gnu`
- `x86_64-apple-darwin`
- `aarch64-apple-darwin`
- `x86_64-pc-windows-msvc`

### Known Limitations

- Dictionary regeneration requires local access to ASD-STE100 PDF (not distributed)
- No incremental analysis (full file re-check on each change)
- VS Code extension not yet published to marketplace
