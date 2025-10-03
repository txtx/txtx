-- Workspace context provider for txtx projects
local M = {}
local manifest = require("txtx.manifest")

-- Current workspace state
M.state = {
  manifest = nil,
  manifest_path = nil,
  active_environment = "default",
  runbook_map = {},  -- Maps runbook paths to manifest entries
}

-- Initialize workspace from a file path
function M.init(filepath)
  -- Find manifest file
  local manifest_path = manifest.find_manifest(filepath)
  
  if not manifest_path then
    -- No manifest found, clear state
    M.state.manifest = nil
    M.state.manifest_path = nil
    M.state.runbook_map = {}
    return false
  end
  
  -- Parse manifest if it's different from current
  if manifest_path ~= M.state.manifest_path then
    local parsed = manifest.parse(manifest_path)
    if parsed then
      M.state.manifest = parsed
      M.state.manifest_path = manifest_path
      
      -- Build runbook map
      M.state.runbook_map = {}
      for _, runbook in ipairs(parsed.runbooks or {}) do
        if runbook.filepath then
          M.state.runbook_map[runbook.filepath] = runbook
        end
      end
      
      -- Watch manifest for changes
      manifest.watch(manifest_path)
      
      -- Notify listeners
      vim.api.nvim_exec_autocmds("User", { 
        pattern = "TxtxWorkspaceInitialized", 
        data = { manifest = parsed } 
      })
      
      return true
    end
  end
  
  return M.state.manifest ~= nil
end

-- Get current workspace manifest
function M.get_manifest()
  return M.state.manifest
end

-- Get runbook info for current file
function M.get_current_runbook()
  local filepath = vim.fn.expand("%:p")
  return M.state.runbook_map[filepath]
end

-- Set active environment
function M.set_environment(env_name)
  if M.state.manifest and M.state.manifest.environments[env_name] then
    M.state.active_environment = env_name
    vim.api.nvim_exec_autocmds("User", { 
      pattern = "TxtxEnvironmentChanged", 
      data = { environment = env_name } 
    })
    return true
  end
  return false
end

-- Get active environment variables
function M.get_environment_vars()
  if not M.state.manifest then
    return {}
  end
  return manifest.get_environment_vars(M.state.manifest, M.state.active_environment)
end

-- Find runbook file by reference (id or name)
function M.find_runbook(reference)
  if not M.state.manifest or not M.state.manifest.runbooks then
    return nil
  end
  
  for _, runbook in ipairs(M.state.manifest.runbooks) do
    if runbook.id == reference or runbook.name == reference then
      return runbook.filepath
    end
  end
  
  return nil
end

-- Get all runbooks in workspace
function M.list_runbooks()
  if not M.state.manifest then
    return {}
  end
  return M.state.manifest.runbooks or {}
end

-- Get available environments
function M.list_environments()
  if not M.state.manifest then
    return {}
  end
  
  local envs = {}
  for name, _ in pairs(M.state.manifest.environments or {}) do
    table.insert(envs, name)
  end
  return envs
end

-- Clear workspace state
function M.clear()
  M.state = {
    manifest = nil,
    manifest_path = nil,
    active_environment = "default",
    runbook_map = {}
  }
end

-- Setup autocommands for workspace discovery
function M.setup()
  -- Initialize workspace when opening files
  vim.api.nvim_create_autocmd({"BufRead", "BufNewFile"}, {
    pattern = {"*.tx", "txtx.yml", "txtx.yaml"},
    callback = function(args)
      M.init(args.file)
    end,
  })
  
  -- Re-initialize on directory change
  vim.api.nvim_create_autocmd("DirChanged", {
    callback = function()
      local current_file = vim.fn.expand("%:p")
      if current_file ~= "" then
        M.init(current_file)
      end
    end,
  })
end

return M