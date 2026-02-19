return {
  cmd = { "goodwrite-lsp" },
  filetypes = { "typst", "markdown" },
  root_dir = function(fname)
    return require("lspconfig.util").root_pattern("goodwrite.toml", ".git")(fname)
  end,
}
