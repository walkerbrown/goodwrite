# goodwrite Neovim setup

Use with `nvim-lspconfig` and the provided Lua config.

## Installation

First, install the `goodwrite-lsp` binary. You can do this quickly using the install script:

```bash
curl -fsSL https://raw.githubusercontent.com/walkerbrown/goodwrite/main/scripts/install.sh | bash
```

Make sure `~/.goodwrite/bin` is in your `$PATH`.

## Configuration

In your Neovim configuration, require the `goodwrite` Lua module to set up the LSP. 
For example, if you are using `nvim-lspconfig`:

```lua
-- Save the `lua/goodwrite.lua` file from this directory somewhere in your `lua/` path
-- or use this snippet directly in your config:
local lspconfig = require("lspconfig")
local configs = require("lspconfig.configs")

if not configs.goodwrite then
  configs.goodwrite = {
    default_config = {
      cmd = { "goodwrite-lsp" },
      filetypes = { "typst", "markdown" },
      root_dir = function(fname)
        return require("lspconfig.util").root_pattern("goodwrite.toml", ".git")(fname)
      end,
    },
  }
end

lspconfig.goodwrite.setup({})
```

Code Actions (quick-fixes and inline suppressions) are fully supported.
