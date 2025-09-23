-- Navigation support for txtx manifest and runbook files
local M = {}
local workspace = require("txtx.workspace")

-- Go to definition handler
function M.goto_definition()
  local line = vim.api.nvim_get_current_line()
  local col = vim.api.nvim_win_get_cursor(0)[2]
  local filetype = vim.bo.filetype
  
  if filetype == "yaml" then
    -- In manifest file, navigate to runbook
    return M.goto_runbook_from_manifest(line, col)
  elseif filetype == "txtx" then
    -- In runbook file, navigate to manifest input definition
    return M.goto_manifest_from_runbook(line, col)
  end
  
  return false
end

-- Navigate from manifest to runbook file
function M.goto_runbook_from_manifest(line, col)
  -- Check if we're on a location field
  local location_pattern = "location:%s*['\"]?([^'\"]+)['\"]?"
  local location = line:match(location_pattern)
  
  if not location then
    -- Try to find location in nearby lines (for multi-line YAML)
    local current_line = vim.fn.line(".")
    local lines = vim.api.nvim_buf_get_lines(0, math.max(0, current_line - 5), current_line + 5, false)
    
    for _, l in ipairs(lines) do
      location = l:match(location_pattern)
      if location then
        break
      end
    end
  end
  
  if location then
    local manifest = workspace.get_manifest()
    if manifest then
      -- Resolve relative to manifest directory
      local runbook_path = vim.fn.simplify(manifest.dir .. "/" .. location)
      
      if vim.fn.filereadable(runbook_path) == 1 then
        -- Open the runbook file
        vim.cmd("edit " .. vim.fn.fnameescape(runbook_path))
        return true
      else
        vim.notify("Runbook file not found: " .. runbook_path, vim.log.levels.WARN)
      end
    end
  end
  
  return false
end

-- Navigate from runbook to manifest input definition
function M.goto_manifest_from_runbook(line, col)
  -- Look for input references like ${input.varname} or input.varname
  local input_pattern = "input%.([%w_]+)"
  local var_name = line:match(input_pattern)
  
  if not var_name then
    -- Try environment variable pattern ${env.varname}
    local env_pattern = "env%.([%w_]+)"
    var_name = line:match(env_pattern)
    
    if var_name then
      return M.goto_environment_var(var_name)
    end
  end
  
  if var_name then
    local manifest = workspace.get_manifest()
    if manifest and manifest.filepath then
      -- Open manifest file
      vim.cmd("edit " .. vim.fn.fnameescape(manifest.filepath))
      
      -- Try to find the variable definition
      local lines = vim.api.nvim_buf_get_lines(0, 0, -1, false)
      for i, l in ipairs(lines) do
        if l:match(var_name .. ":") then
          vim.api.nvim_win_set_cursor(0, {i, 0})
          return true
        end
      end
    end
  end
  
  return false
end

-- Navigate to environment variable definition in manifest
function M.goto_environment_var(var_name)
  local manifest = workspace.get_manifest()
  if not manifest or not manifest.filepath then
    return false
  end
  
  -- Open manifest file
  vim.cmd("edit " .. vim.fn.fnameescape(manifest.filepath))
  
  -- Search for the variable in environments section
  local lines = vim.api.nvim_buf_get_lines(0, 0, -1, false)
  local in_environments = false
  
  for i, l in ipairs(lines) do
    if l:match("^environments:") then
      in_environments = true
    elseif in_environments and l:match("^%s+" .. var_name .. ":") then
      vim.api.nvim_win_set_cursor(0, {i, 0})
      return true
    end
  end
  
  return false
end

-- Rename handler for cross-file references
function M.rename()
  local old_name = vim.fn.expand("<cword>")
  local new_name = vim.fn.input("Rename '" .. old_name .. "' to: ", old_name)
  
  if new_name == "" or new_name == old_name then
    return
  end
  
  local filetype = vim.bo.filetype
  
  if filetype == "yaml" then
    M.rename_in_manifest(old_name, new_name)
  elseif filetype == "txtx" then
    M.rename_in_runbook(old_name, new_name)
  end
end

-- Rename references in manifest and related runbooks
function M.rename_in_manifest(old_name, new_name)
  local manifest = workspace.get_manifest()
  if not manifest then
    return
  end
  
  -- Collect all files to update
  local files_to_update = {manifest.filepath}
  
  -- Add all runbook files
  for _, runbook in ipairs(manifest.runbooks or {}) do
    if runbook.filepath and vim.fn.filereadable(runbook.filepath) == 1 then
      table.insert(files_to_update, runbook.filepath)
    end
  end
  
  -- Update each file
  local changes_made = 0
  for _, filepath in ipairs(files_to_update) do
    local bufnr = vim.fn.bufnr(filepath)
    local lines
    
    if bufnr ~= -1 and vim.api.nvim_buf_is_loaded(bufnr) then
      lines = vim.api.nvim_buf_get_lines(bufnr, 0, -1, false)
    else
      lines = vim.fn.readfile(filepath)
    end
    
    local modified = false
    for i, line in ipairs(lines) do
      -- Replace variable references
      local new_line = line:gsub("([^%w])" .. old_name .. "([^%w])", "%1" .. new_name .. "%2")
      if new_line ~= line then
        lines[i] = new_line
        modified = true
      end
    end
    
    if modified then
      if bufnr ~= -1 and vim.api.nvim_buf_is_loaded(bufnr) then
        vim.api.nvim_buf_set_lines(bufnr, 0, -1, false, lines)
      else
        vim.fn.writefile(lines, filepath)
      end
      changes_made = changes_made + 1
    end
  end
  
  vim.notify("Renamed '" .. old_name .. "' to '" .. new_name .. "' in " .. changes_made .. " files", vim.log.levels.INFO)
end

-- Rename references in runbook and manifest
function M.rename_in_runbook(old_name, new_name)
  -- Similar to rename_in_manifest but starts from runbook
  M.rename_in_manifest(old_name, new_name)
end

-- Find all references to a symbol
function M.find_references()
  local word = vim.fn.expand("<cword>")
  local manifest = workspace.get_manifest()
  
  if not manifest then
    vim.notify("No txtx manifest found", vim.log.levels.WARN)
    return
  end
  
  -- Collect all files to search
  local files = {manifest.filepath}
  for _, runbook in ipairs(manifest.runbooks or {}) do
    if runbook.filepath and vim.fn.filereadable(runbook.filepath) == 1 then
      table.insert(files, runbook.filepath)
    end
  end
  
  -- Use quickfix list to show results
  local qf_items = {}
  
  for _, filepath in ipairs(files) do
    local lines = vim.fn.readfile(filepath)
    for lnum, line in ipairs(lines) do
      if line:match(word) then
        table.insert(qf_items, {
          filename = filepath,
          lnum = lnum,
          text = line,
        })
      end
    end
  end
  
  if #qf_items > 0 then
    vim.fn.setqflist(qf_items)
    vim.cmd("copen")
  else
    vim.notify("No references found for '" .. word .. "'", vim.log.levels.INFO)
  end
end

-- Setup navigation keymaps and commands
function M.setup()
  -- Create autocommands for file-specific mappings
  vim.api.nvim_create_autocmd("FileType", {
    pattern = {"txtx", "yaml"},
    callback = function(args)
      local opts = { buffer = args.buf, silent = true }
      
      -- Check if it's a txtx-related file
      local is_txtx_file = false
      if vim.bo[args.buf].filetype == "txtx" then
        is_txtx_file = true
      elseif vim.bo[args.buf].filetype == "yaml" then
        local filename = vim.fn.expand("%:t")
        is_txtx_file = filename == "txtx.yml" or filename == "txtx.yaml"
      end
      
      if is_txtx_file then
        -- Go to definition
        vim.keymap.set("n", "gd", M.goto_definition, opts)
        vim.keymap.set("n", "<C-]>", M.goto_definition, opts)
        
        -- Find references
        vim.keymap.set("n", "gr", M.find_references, opts)
        
        -- Rename
        vim.keymap.set("n", "<leader>rn", M.rename, opts)
        vim.keymap.set("n", "<F2>", M.rename, opts)
      end
    end,
  })
end

return M