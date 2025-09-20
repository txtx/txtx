-- Commands for txtx plugin with workspace support
local M = {}
local workspace = require("txtx.workspace")
local navigation = require("txtx.navigation")

function M.setup()
  -- Environment management commands
  vim.api.nvim_create_user_command("TxtxSelectEnvironment", function()
    M.select_environment()
  end, {
    desc = "Select active txtx environment",
  })
  
  vim.api.nvim_create_user_command("TxtxShowManifest", function()
    M.show_manifest()
  end, {
    desc = "Show parsed txtx manifest structure",
  })
  
  vim.api.nvim_create_user_command("TxtxListRunbooks", function()
    M.list_runbooks()
  end, {
    desc = "List all runbooks in workspace",
  })
  
  vim.api.nvim_create_user_command("TxtxOpenRunbook", function()
    M.open_runbook()
  end, {
    desc = "Open a runbook from the manifest",
  })
  
  vim.api.nvim_create_user_command("TxtxGotoManifest", function()
    M.goto_manifest()
  end, {
    desc = "Go to workspace manifest file",
  })
  
  vim.api.nvim_create_user_command("TxtxValidateWorkspace", function()
    M.validate_workspace()
  end, {
    desc = "Validate manifest and runbook consistency",
  })
  
  -- Existing commands
  vim.api.nvim_create_user_command("TxtxBuildParser", function()
    M.build_parser()
  end, {
    desc = "Build the txtx Tree-sitter parser",
  })
  
  vim.api.nvim_create_user_command("TxtxInfo", function()
    M.show_info()
  end, {
    desc = "Show txtx plugin information",
  })
  
  vim.api.nvim_create_user_command("TxtxCheck", function()
    M.check_current_file()
  end, {
    desc = "Run txtx check on current file",
  })
  
  vim.api.nvim_create_user_command("TxtxDescribe", function()
    M.describe_current_file()
  end, {
    desc = "Run txtx describe on current file",
  })
end

-- Select active environment
function M.select_environment()
  local envs = workspace.list_environments()
  
  if #envs == 0 then
    vim.notify("No environments defined in manifest", vim.log.levels.WARN)
    return
  end
  
  vim.ui.select(envs, {
    prompt = "Select environment:",
    format_item = function(env)
      if env == workspace.state.active_environment then
        return env .. " (active)"
      end
      return env
    end,
  }, function(choice)
    if choice then
      if workspace.set_environment(choice) then
        vim.notify("Switched to environment: " .. choice, vim.log.levels.INFO)
      else
        vim.notify("Failed to switch environment", vim.log.levels.ERROR)
      end
    end
  end)
end

-- Show parsed manifest structure
function M.show_manifest()
  local manifest = workspace.get_manifest()
  
  if not manifest then
    vim.notify("No txtx manifest found", vim.log.levels.WARN)
    return
  end
  
  -- Create a new buffer for display
  vim.cmd("new")
  vim.bo.buftype = "nofile"
  vim.bo.bufhidden = "wipe"
  vim.bo.swapfile = false
  vim.bo.filetype = "yaml"
  
  local lines = {
    "# txtx Manifest",
    "Path: " .. manifest.filepath,
    "",
    "## Project",
    "Name: " .. (manifest.name or "N/A"),
    "ID: " .. (manifest.id or "N/A"),
    "",
    "## Runbooks",
  }
  
  for _, runbook in ipairs(manifest.runbooks or {}) do
    table.insert(lines, string.format("- %s (%s)", runbook.name or "unnamed", runbook.location or "no location"))
    if runbook.description then
      table.insert(lines, "  " .. runbook.description)
    end
  end
  
  table.insert(lines, "")
  table.insert(lines, "## Environments")
  
  for env_name, env_vars in pairs(manifest.environments or {}) do
    table.insert(lines, "### " .. env_name)
    for key, value in pairs(env_vars) do
      table.insert(lines, string.format("  %s: %s", key, value))
    end
  end
  
  vim.api.nvim_buf_set_lines(0, 0, -1, false, lines)
  vim.api.nvim_buf_set_name(0, "txtx manifest")
  vim.bo.modifiable = false
end

-- List all runbooks
function M.list_runbooks()
  local runbooks = workspace.list_runbooks()
  
  if #runbooks == 0 then
    vim.notify("No runbooks found in workspace", vim.log.levels.INFO)
    return
  end
  
  local items = {}
  for _, runbook in ipairs(runbooks) do
    table.insert(items, {
      text = string.format("%s - %s", runbook.name or "unnamed", runbook.description or ""),
      filename = runbook.filepath,
    })
  end
  
  vim.ui.select(items, {
    prompt = "Select runbook to open:",
    format_item = function(item)
      return item.text
    end,
  }, function(choice)
    if choice and choice.filename then
      vim.cmd("edit " .. vim.fn.fnameescape(choice.filename))
    end
  end)
end

-- Open a runbook
function M.open_runbook()
  M.list_runbooks()
end

-- Go to manifest file
function M.goto_manifest()
  local manifest = workspace.get_manifest()
  
  if not manifest or not manifest.filepath then
    vim.notify("No txtx manifest found", vim.log.levels.WARN)
    return
  end
  
  vim.cmd("edit " .. vim.fn.fnameescape(manifest.filepath))
end

-- Validate workspace consistency
function M.validate_workspace()
  local manifest = workspace.get_manifest()
  
  if not manifest then
    vim.notify("No txtx manifest found", vim.log.levels.WARN)
    return
  end
  
  local issues = {}
  
  -- Check runbook files exist
  for _, runbook in ipairs(manifest.runbooks or {}) do
    if runbook.filepath then
      if vim.fn.filereadable(runbook.filepath) == 0 then
        table.insert(issues, string.format("Runbook file not found: %s (%s)", runbook.name, runbook.location))
      end
    else
      table.insert(issues, string.format("Runbook has no location: %s", runbook.name))
    end
  end
  
  -- Check for duplicate runbook IDs
  local seen_ids = {}
  for _, runbook in ipairs(manifest.runbooks or {}) do
    if runbook.id then
      if seen_ids[runbook.id] then
        table.insert(issues, string.format("Duplicate runbook ID: %s", runbook.id))
      end
      seen_ids[runbook.id] = true
    end
  end
  
  -- Display results
  if #issues == 0 then
    vim.notify("✓ Workspace validation passed", vim.log.levels.INFO)
  else
    local msg = "Workspace validation issues:\n" .. table.concat(issues, "\n")
    vim.notify(msg, vim.log.levels.WARN)
  end
end

-- Build Tree-sitter parser
function M.build_parser()
  local plugin_path = debug.getinfo(1, "S").source:sub(2):match("(.*)/lua/txtx/commands.lua$")
  local build_script = plugin_path .. "/scripts/build.sh"
  
  if vim.fn.filereadable(build_script) == 0 then
    vim.notify("Build script not found at " .. build_script, vim.log.levels.ERROR)
    return
  end
  
  vim.notify("Building txtx Tree-sitter parser...", vim.log.levels.INFO)
  
  local output = vim.fn.system("cd " .. vim.fn.shellescape(plugin_path) .. " && ./scripts/build.sh")
  
  if vim.v.shell_error == 0 then
    vim.notify("Parser built successfully! Restart Neovim to use it.", vim.log.levels.INFO)
  else
    vim.notify("Failed to build parser:\n" .. output, vim.log.levels.ERROR)
  end
end

-- Show plugin info
function M.show_info()
  local info = {}
  
  -- Check txtx CLI
  if vim.fn.executable("txtx") == 1 then
    local version = vim.fn.system("txtx --version 2>/dev/null"):gsub("\n", "")
    table.insert(info, "✓ txtx CLI: " .. version)
  else
    table.insert(info, "✗ txtx CLI: Not found")
  end
  
  -- Check Tree-sitter
  local ts_ok = pcall(require, "nvim-treesitter")
  if ts_ok then
    table.insert(info, "✓ nvim-treesitter: Installed")
  else
    table.insert(info, "✗ nvim-treesitter: Not found")
  end
  
  -- Check parser
  local plugin_path = debug.getinfo(1, "S").source:sub(2):match("(.*)/lua/txtx/commands.lua$")
  local uname = vim.loop.os_uname()
  local ext = uname.sysname == "Darwin" and "dylib" or "so"
  local parser_path = plugin_path .. "/parser/txtx." .. ext
  
  if vim.fn.filereadable(parser_path) == 1 then
    table.insert(info, "✓ Tree-sitter parser: " .. parser_path)
  else
    table.insert(info, "✗ Tree-sitter parser: Not found (run :TxtxBuildParser)")
  end
  
  -- Check LSP
  local lsp_ok = pcall(require, "lspconfig")
  if lsp_ok then
    table.insert(info, "✓ nvim-lspconfig: Installed")
  else
    table.insert(info, "✗ nvim-lspconfig: Not found")
  end
  
  -- Check workspace
  local manifest = workspace.get_manifest()
  if manifest then
    table.insert(info, "✓ Workspace: " .. manifest.name .. " (" .. manifest.filepath .. ")")
    table.insert(info, "  Active environment: " .. workspace.state.active_environment)
    table.insert(info, "  Runbooks: " .. #(manifest.runbooks or {}))
  else
    table.insert(info, "✗ Workspace: No manifest found")
  end
  
  -- Check current file
  local ft = vim.bo.filetype
  if ft == "txtx" then
    table.insert(info, "✓ Current file: txtx runbook")
    
    local runbook = workspace.get_current_runbook()
    if runbook then
      table.insert(info, "  Runbook: " .. runbook.name)
    end
  elseif ft == "yaml" then
    local filename = vim.fn.expand("%:t")
    if filename == "txtx.yml" or filename == "txtx.yaml" then
      table.insert(info, "✓ Current file: txtx manifest")
    end
  else
    table.insert(info, "Current file: Not a txtx file")
  end
  
  -- Display info
  vim.notify(table.concat(info, "\n"), vim.log.levels.INFO)
end

-- Check current file
function M.check_current_file()
  local file = vim.fn.expand("%:p")
  if vim.fn.filereadable(file) == 0 then
    vim.notify("No file to check", vim.log.levels.ERROR)
    return
  end
  
  local output = vim.fn.system("txtx check " .. vim.fn.shellescape(file))
  
  if vim.v.shell_error == 0 then
    vim.notify("✓ txtx check passed\n" .. output, vim.log.levels.INFO)
  else
    vim.notify("✗ txtx check failed\n" .. output, vim.log.levels.ERROR)
  end
end

-- Describe current file
function M.describe_current_file()
  local file = vim.fn.expand("%:p")
  if vim.fn.filereadable(file) == 0 then
    vim.notify("No file to describe", vim.log.levels.ERROR)
    return
  end
  
  local output = vim.fn.system("txtx describe " .. vim.fn.shellescape(file))
  
  if vim.v.shell_error == 0 then
    -- Open output in a new buffer
    vim.cmd("new")
    vim.bo.buftype = "nofile"
    vim.bo.bufhidden = "wipe"
    vim.bo.swapfile = false
    vim.bo.filetype = "markdown"
    
    local lines = vim.split(output, "\n")
    vim.api.nvim_buf_set_lines(0, 0, -1, false, lines)
    vim.api.nvim_buf_set_name(0, "txtx describe: " .. vim.fn.fnamemodify(file, ":t"))
  else
    vim.notify("Failed to describe file:\n" .. output, vim.log.levels.ERROR)
  end
end

return M