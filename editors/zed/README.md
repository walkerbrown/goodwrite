# goodwrite Zed Extension

A Zed extension that automatically downloads and runs the `goodwrite-lsp` Language Server for linting Simplified Technical English (ASD-STE100) and engineering requirement grammars in Markdown and Typst.

## Installation

Currently, to use this extension locally, you need to compile it and install it as a dev extension in Zed:

1. Add the Wasm target to your Rust toolchain:
   ```bash
   rustup target add wasm32-wasip1
   ```
2. Build the extension:
   ```bash
   cd editors/zed
   cargo build --target wasm32-wasip1
   ```
3. Open Zed, run the `zed: install dev extension` command from the command palette, and select the `editors/zed` directory.

Once activated, the extension will automatically download the correct binary for your platform from GitHub Releases and start the language server. Code Actions (quick fixes and inline suppressions) are fully supported.
