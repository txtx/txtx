-- LSP configuration for txtx with workspace support
local M = {}
local utils = require("txtx.utils")

-- Store LSP client for later access
M.client = nil

function M.setup(config)
  config = config or {}
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
    -- Store client reference
    M.client = client

    -- Call user's on_attach if provided
    if config.on_attach then
      config.on_attach(client, bufnr)
    end

    -- Check if lspsaga is available
    local has_saga = pcall(require, "lspsaga")

    -- Setup buffer-local keymaps for LSP features
    local opts = { buffer = bufnr, silent = true, noremap = true }

    -- Use our custom go-to-definition that understands manifest/runbook relationships
    vim.keymap.set("n", "gd", function()
      local ok, navigation = pcall(require, "txtx.navigation")
      if ok and navigation.goto_definition() then
        return
      end
      -- Fallback to lspsaga or LSP go-to-definition
      if has_saga then
        vim.cmd("Lspsaga goto_definition")
      else
        vim.lsp.buf.definition()
      end
    end, opts)

    -- Peek definition (lspsaga only)
    if has_saga then
      vim.keymap.set("n", "gD", "<cmd>Lspsaga peek_definition<CR>", opts)
    end

    -- Hover documentation
    if has_saga then
      vim.keymap.set("n", "K", "<cmd>Lspsaga hover_doc<CR>", opts)
    else
      vim.keymap.set("n", "K", vim.lsp.buf.hover, opts)
    end

    -- Signature help
    if has_saga then
      vim.keymap.set("n", "<C-k>", "<cmd>Lspsaga signature_help<CR>", opts)
      vim.keymap.set("i", "<C-k>", "<cmd>Lspsaga signature_help<CR>", opts)
    else
      vim.keymap.set("n", "<C-k>", vim.lsp.buf.signature_help, opts)
    end

    -- Standard LSP mappings
    vim.keymap.set("n", "gi", vim.lsp.buf.implementation, opts)
    vim.keymap.set("n", "<leader>wa", vim.lsp.buf.add_workspace_folder, opts)
    vim.keymap.set("n", "<leader>wr", vim.lsp.buf.remove_workspace_folder, opts)
    vim.keymap.set("n", "<leader>wl", function()
      vim.notify(vim.inspect(vim.lsp.buf.list_workspace_folders()), vim.log.levels.INFO)
    end, opts)
    vim.keymap.set("n", "<leader>D", vim.lsp.buf.type_definition, opts)

    -- Rename
    if has_saga then
      vim.keymap.set("n", "<leader>rn", "<cmd>Lspsaga rename<CR>", opts)
      -- Also bind our smart rename for multi-file undo tracking
      vim.keymap.set("n", "<leader>rN", function()
        M.smart_rename()
      end, opts)
    else
      vim.keymap.set("n", "<leader>rn", function()
        M.smart_rename()
      end, opts)
    end

    -- Code actions
    if has_saga then
      vim.keymap.set({ "n", "v" }, "<leader>ca", "<cmd>Lspsaga code_action<CR>", opts)
    else
      vim.keymap.set("n", "<leader>ca", vim.lsp.buf.code_action, opts)
    end

    -- References
    if has_saga then
      vim.keymap.set("n", "gr", "<cmd>Lspsaga finder<CR>", opts)
    else
      vim.keymap.set("n", "gr", function()
        local ok, navigation = pcall(require, "txtx.navigation")
        if ok and navigation.find_references then
          navigation.find_references()
        else
          vim.lsp.buf.references()
        end
      end, opts)
    end

    -- Diagnostics navigation
    if has_saga then
      vim.keymap.set("n", "[d", "<cmd>Lspsaga diagnostic_jump_prev<CR>", opts)
      vim.keymap.set("n", "]d", "<cmd>Lspsaga diagnostic_jump_next<CR>", opts)
      vim.keymap.set("n", "<leader>d", "<cmd>Lspsaga show_line_diagnostics<CR>", opts)
      vim.keymap.set("n", "<leader>D", "<cmd>Lspsaga show_buf_diagnostics<CR>", opts)
    else
      vim.keymap.set("n", "[d", vim.diagnostic.goto_prev, opts)
      vim.keymap.set("n", "]d", vim.diagnostic.goto_next, opts)
      vim.keymap.set("n", "<leader>d", vim.diagnostic.open_float, opts)
    end

    -- Outline (lspsaga only)
    if has_saga then
      vim.keymap.set("n", "<leader>o", "<cmd>Lspsaga outline<CR>", opts)
    end

    -- Format
    vim.keymap.set("n", "<leader>f", function()
      vim.lsp.buf.format({ async = true })
    end, opts)
  end
  
  -- Helper to attach LSP client
  local function attach_lsp_client(args, root_dir)
    utils.init_workspace(args.file)

    local client_id = utils.start_lsp_client({
      cmd = config.cmd,
      root_dir = root_dir,
      capabilities = M.make_capabilities(config.capabilities),
      settings = config.settings,
    }, enhanced_on_attach)

    if client_id then
      vim.lsp.buf_attach_client(args.buf, client_id)
    end
  end

  -- Setup autocommands for LSP attachment
  vim.api.nvim_create_autocmd("FileType", {
    pattern = "txtx",
    callback = function(args)
      local root_files = vim.fs.find({ "txtx.yml", "txtx.yaml", ".git" }, {
        upward = true,
        path = vim.fs.dirname(args.file),
      })
      local root_dir = root_files[1] and vim.fs.dirname(root_files[1]) or vim.fn.getcwd()
      attach_lsp_client(args, root_dir)
    end,
  })

  -- Also attach LSP to txtx.yml/txtx.yaml files for validation
  vim.api.nvim_create_autocmd("BufRead", {
    pattern = { "txtx.yml", "txtx.yaml" },
    callback = function(args)
      local root_dir = vim.fs.dirname(args.file) or vim.fn.getcwd()
      attach_lsp_client(args, root_dir)
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

-- Request available environments from LSP server
function M.get_environments(callback)
  if not M.client then
    vim.notify("LSP client not available", vim.log.levels.ERROR)
    return
  end

  M.client.request("workspace/environments", {}, function(err, result)
    if err then
      vim.notify("Failed to get environments: " .. vim.inspect(err), vim.log.levels.ERROR)
      return
    end

    if callback then
      callback(result or {})
    end
  end)
end

-- Set environment via LSP notification
function M.set_environment(environment)
  if not M.client then
    vim.notify("LSP client not available", vim.log.levels.ERROR)
    return false
  end

  M.client.notify("workspace/setEnvironment", {
    environment = environment
  })

  return true
end

-- Smart rename with better undo support for multi-file changes
function M.smart_rename()
  if not M.client then
    vim.notify("LSP client not available", vim.log.levels.ERROR)
    return
  end

  -- Store original buffer states before rename
  local original_buffers = {}
  for _, buf in ipairs(vim.api.nvim_list_bufs()) do
    if vim.api.nvim_buf_is_loaded(buf) then
      original_buffers[buf] = {
        changedtick = vim.api.nvim_buf_get_changedtick(buf),
      }
    end
  end

  -- Perform the rename
  vim.lsp.buf.rename(nil, {
    -- Custom handler to track which buffers were modified
    handler = function(err, result, ctx, config)
      if err then
        vim.notify("Rename failed: " .. err.message, vim.log.levels.ERROR)
        return
      end

      -- Apply the workspace edit
      if result then
        vim.lsp.util.apply_workspace_edit(result, M.client.offset_encoding)

        -- Find all modified buffers
        local modified_buffers = {}
        for _, buf in ipairs(vim.api.nvim_list_bufs()) do
          if vim.api.nvim_buf_is_loaded(buf) then
            local old_tick = original_buffers[buf] and original_buffers[buf].changedtick or 0
            local new_tick = vim.api.nvim_buf_get_changedtick(buf)
            if new_tick ~= old_tick then
              table.insert(modified_buffers, buf)
            end
          end
        end

        -- Notify user of changes
        local file_count = #modified_buffers
        if file_count > 1 then
          local msg = string.format("Renamed in %d files. Use :TxtxUndoRename to undo all changes.", file_count)
          vim.notify(msg, vim.log.levels.INFO)

          -- Store the modified buffers for potential undo
          M._last_rename_buffers = modified_buffers
        end
      end
    end
  })
end

-- Undo the last multi-file rename operation
function M.undo_last_rename()
  if not M._last_rename_buffers or #M._last_rename_buffers == 0 then
    vim.notify("No recent rename to undo", vim.log.levels.WARN)
    return
  end

  local count = 0
  local current_buf = vim.api.nvim_get_current_buf()

  for _, buf in ipairs(M._last_rename_buffers) do
    if vim.api.nvim_buf_is_valid(buf) and vim.api.nvim_buf_is_loaded(buf) then
      -- Use vim.fn.bufwinid to check if buffer is displayed in a window
      local winid = vim.fn.bufwinid(buf)
      if winid ~= -1 then
        -- Buffer is visible, switch to it and undo
        local current_win = vim.api.nvim_get_current_win()
        vim.api.nvim_set_current_win(winid)
        vim.cmd("silent! undo")
        vim.api.nvim_set_current_win(current_win)
      else
        -- Buffer not visible, use nvim_buf_call for cleaner approach
        vim.api.nvim_buf_call(buf, function()
          vim.cmd("silent! undo")
        end)
      end
      count = count + 1
    end
  end

  -- Restore original buffer
  if vim.api.nvim_buf_is_valid(current_buf) then
    vim.api.nvim_set_current_buf(current_buf)
  end

  vim.notify(string.format("Undone rename in %d files", count), vim.log.levels.INFO)
  M._last_rename_buffers = nil
end

return M