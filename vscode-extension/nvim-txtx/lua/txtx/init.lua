-- nvim-txtx main module
local M = {}

M.config = {
  lsp = {
    enabled = true,
    cmd = { "txtx", "lsp" },
    settings = {},
    capabilities = nil,
    on_attach = nil,
  },
  treesitter = {
    enabled = true,
  },
  workspace = {
    enabled = true,
  },
  navigation = {
    enabled = true,
  }
}

function M.setup(opts)
  M.config = vim.tbl_deep_extend("force", M.config, opts or {})
  
  -- Setup filetype detection
  -- Only .tx files are txtx runbooks
  -- txtx.yml/txtx.yaml are YAML manifest files
  vim.filetype.add({
    extension = {
      tx = "txtx",
    },
  })
  
  -- Setup workspace discovery
  if M.config.workspace.enabled then
    require("txtx.workspace").setup()
  end
  
  -- Setup navigation features
  if M.config.navigation.enabled then
    require("txtx.navigation").setup()
  end
  
  -- Setup Tree-sitter if enabled
  if M.config.treesitter.enabled then
    local ok, treesitter = pcall(require, "txtx.treesitter")
    if ok then
      treesitter.setup(M.config.treesitter)
    end
  end
  
  -- Setup LSP if enabled
  if M.config.lsp.enabled then
    local ok, lsp = pcall(require, "txtx.lsp")
    if ok then
      lsp.setup(M.config.lsp)
    end
  end
  
  -- Setup commands
  local ok, commands = pcall(require, "txtx.commands")
  if ok then
    commands.setup()
  end
end

return M