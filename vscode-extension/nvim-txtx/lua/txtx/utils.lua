-- Utility functions for txtx plugin
local M = {}

--- Safely require a module with error handling
---@param module_name string
---@return boolean success, table|nil module
function M.safe_require(module_name)
  local ok, module = pcall(require, module_name)
  return ok, module
end

--- Get workspace module safely
---@return table|nil workspace
function M.get_workspace()
  local ok, workspace = M.safe_require("txtx.workspace")
  if not ok then
    vim.notify("Workspace module not available", vim.log.levels.ERROR)
    return nil
  end
  return workspace
end

--- Get manifest with error handling
---@return table|nil manifest
function M.get_manifest()
  local workspace = M.get_workspace()
  if not workspace then
    return nil
  end

  local manifest = workspace.get_manifest()
  if not manifest then
    vim.notify("No txtx manifest found", vim.log.levels.WARN)
    return nil
  end

  return manifest
end

--- Initialize workspace safely
---@param file string
function M.init_workspace(file)
  local ok, workspace = M.safe_require("txtx.workspace")
  if ok and workspace.init then
    pcall(workspace.init, file)
  end
end

--- Create a scratch buffer with options
---@param filetype? string
---@return integer bufnr
function M.create_scratch_buffer(filetype)
  local buf = vim.api.nvim_create_buf(false, true)
  vim.api.nvim_buf_set_option(buf, "buftype", "nofile")
  vim.api.nvim_buf_set_option(buf, "bufhidden", "wipe")
  vim.api.nvim_buf_set_option(buf, "swapfile", false)

  if filetype then
    vim.api.nvim_buf_set_option(buf, "filetype", filetype)
  end

  return buf
end

--- Run txtx CLI command on current file
---@param command string CLI subcommand (check, describe, etc.)
---@return boolean success, string output
function M.run_txtx_command(command)
  local file = vim.fn.expand("%:p")

  if vim.fn.filereadable(file) == 0 then
    vim.notify("No file to " .. command, vim.log.levels.ERROR)
    return false, ""
  end

  local cmd = string.format("txtx %s %s", command, vim.fn.shellescape(file))
  local output = vim.fn.system(cmd)
  local success = vim.v.shell_error == 0

  return success, output
end

--- Check if a module/feature is available
---@param module_name string
---@param display_name? string
---@return boolean available
function M.check_available(module_name, display_name)
  local ok = pcall(require, module_name)
  return ok
end

--- Get plugin path (directory containing lua/txtx)
---@return string|nil path
function M.get_plugin_path()
  local source = debug.getinfo(1, "S").source
  if not source then
    return nil
  end

  -- Remove @ prefix and extract path
  local path = source:sub(2):match("(.*)/lua/txtx/[^/]+%.lua$")
  return path
end

--- Get parser file extension for current OS
---@return string extension
function M.get_parser_extension()
  local uname = vim.loop.os_uname()
  return uname.sysname == "Darwin" and "dylib" or "so"
end

--- Start LSP client with common configuration
---@param config table LSP client config
---@param on_attach function Attach callback
---@return integer|nil client_id
function M.start_lsp_client(config, on_attach)
  local client_id = vim.lsp.start({
    name = "txtx_lsp",
    cmd = config.cmd or { "txtx", "lsp" },
    root_dir = config.root_dir,
    capabilities = config.capabilities,
    settings = config.settings or {},
    on_attach = on_attach,
  })

  return client_id
end

return M
