-- LSP configuration for txtx with workspace support
local M = {}
local workspace = require("txtx.workspace")

function M.setup(config)
  -- Check if lspconfig is available
  local ok, lspconfig = pcall(require, "lspconfig")
  if not ok then
    vim.notify("nvim-lspconfig not found. LSP support will not be available.", vim.log.levels.WARN)
    return
  end
  
  -- Check if txtx is installed
  if vim.fn.executable("txtx") == 0 then
    vim.notify("txtx CLI not found. Please install it with: cargo install --path crates/txtx-cli", vim.log.levels.WARN)
    return
  end
  
  -- Create custom LSP configuration
  local configs = require("lspconfig.configs")
  
  if not configs.txtx_lsp then
    configs.txtx_lsp = {
      default_config = {
        cmd = config.cmd or { "txtx", "lsp" },
        filetypes = { "txtx" },
        root_dir = function(fname)
          return lspconfig.util.find_git_ancestor(fname)
            or lspconfig.util.root_pattern("txtx.yml", "txtx.yaml", ".txtx")(fname)
            or vim.fn.getcwd()
        end,
        settings = config.settings or {},
        init_options = {
          provideFormatter = true,
        },
      },
    }
  end
  
  -- Enhanced on_attach function with workspace awareness
  local function enhanced_on_attach(client, bufnr)
    -- Call user's on_attach if provided
    if config.on_attach then
      config.on_attach(client, bufnr)
    end
    
    -- Add workspace-aware completions
    local manifest = workspace.get_manifest()
    if manifest then
      -- Provide environment variables as completion items
      local env_vars = workspace.get_environment_vars()
      -- This would require custom completion source or LSP server support
    end
    
    -- Setup buffer-local keymaps for LSP features
    local opts = { buffer = bufnr, silent = true }
    
    -- Use our custom go-to-definition that understands manifest/runbook relationships
    vim.keymap.set("n", "gd", function()
      local navigation = require("txtx.navigation")
      if not navigation.goto_definition() then
        -- Fallback to LSP go-to-definition
        vim.lsp.buf.definition()
      end
    end, opts)
    
    -- Standard LSP mappings
    vim.keymap.set("n", "K", vim.lsp.buf.hover, opts)
    vim.keymap.set("n", "gi", vim.lsp.buf.implementation, opts)
    vim.keymap.set("n", "<C-k>", vim.lsp.buf.signature_help, opts)
    vim.keymap.set("n", "<leader>wa", vim.lsp.buf.add_workspace_folder, opts)
    vim.keymap.set("n", "<leader>wr", vim.lsp.buf.remove_workspace_folder, opts)
    vim.keymap.set("n", "<leader>wl", function()
      print(vim.inspect(vim.lsp.buf.list_workspace_folders()))
    end, opts)
    vim.keymap.set("n", "<leader>D", vim.lsp.buf.type_definition, opts)
    
    -- Use our custom rename that handles cross-file references
    vim.keymap.set("n", "<leader>rn", function()
      local navigation = require("txtx.navigation")
      navigation.rename()
    end, opts)
    
    vim.keymap.set("n", "<leader>ca", vim.lsp.buf.code_action, opts)
    
    -- Use our custom references finder
    vim.keymap.set("n", "gr", function()
      local navigation = require("txtx.navigation")
      navigation.find_references()
    end, opts)
    
    vim.keymap.set("n", "<leader>f", function()
      vim.lsp.buf.format { async = true }
    end, opts)
  end
  
  -- Setup autocommands for LSP attachment
  vim.api.nvim_create_autocmd("FileType", {
    pattern = "txtx",
    callback = function(args)
      -- Initialize workspace first
      workspace.init(args.file)
      
      local client_id = vim.lsp.start({
        name = "txtx_lsp",
        cmd = config.cmd or { "txtx", "lsp" },
        root_dir = vim.fs.dirname(vim.fs.find({ "txtx.yml", "txtx.yaml", ".git" }, {
          upward = true,
          path = vim.fs.dirname(args.file),
        })[1]) or vim.fn.getcwd(),
        capabilities = M.make_capabilities(config.capabilities),
        settings = config.settings or {},
        on_attach = enhanced_on_attach,
      })
      
      if client_id then
        vim.lsp.buf_attach_client(args.buf, client_id)
      end
    end,
  })
  
  -- Also attach LSP to txtx.yml/txtx.yaml files for validation
  vim.api.nvim_create_autocmd("BufRead", {
    pattern = { "txtx.yml", "txtx.yaml" },
    callback = function(args)
      -- Initialize workspace
      workspace.init(args.file)
      
      local client_id = vim.lsp.start({
        name = "txtx_lsp",
        cmd = config.cmd or { "txtx", "lsp" },
        root_dir = vim.fs.dirname(args.file) or vim.fn.getcwd(),
        capabilities = M.make_capabilities(config.capabilities),
        settings = config.settings or {},
        on_attach = enhanced_on_attach,
      })
      
      if client_id then
        vim.lsp.buf_attach_client(args.buf, client_id)
      end
    end,
  })
end

function M.make_capabilities(custom_capabilities)
  local capabilities = vim.lsp.protocol.make_client_capabilities()
  
  -- Add completion capabilities if cmp_nvim_lsp is available
  local ok, cmp_nvim_lsp = pcall(require, "cmp_nvim_lsp")
  if ok then
    capabilities = cmp_nvim_lsp.default_capabilities(capabilities)
  end
  
  -- Merge with custom capabilities if provided
  if custom_capabilities then
    capabilities = vim.tbl_deep_extend("force", capabilities, custom_capabilities)
  end
  
  return capabilities
end

return M