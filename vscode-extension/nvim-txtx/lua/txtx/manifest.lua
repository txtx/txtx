-- Manifest parser and handler for txtx.yml files
local M = {}

-- Cache for parsed manifests (keyed by directory path)
M.cache = {}

-- Parse a txtx.yml/txtx.yaml manifest file
function M.parse(filepath)
  if not filepath or vim.fn.filereadable(filepath) == 0 then
    return nil
  end
  
  -- Check cache
  local dir = vim.fn.fnamemodify(filepath, ":h")
  if M.cache[dir] and M.cache[dir].mtime == vim.fn.getftime(filepath) then
    return M.cache[dir].data
  end
  
  -- Read and parse YAML
  local content = vim.fn.readfile(filepath)
  local ok, yaml = pcall(vim.fn.json_decode, vim.fn.system('yq -o json', content))
  
  if not ok then
    -- Fallback to basic parsing if yq is not available
    local manifest = M.parse_basic(content)
    if manifest then
      manifest.filepath = filepath
      manifest.dir = dir
      M.cache[dir] = {
        data = manifest,
        mtime = vim.fn.getftime(filepath)
      }
    end
    return manifest
  end
  
  -- Process parsed YAML
  local manifest = {
    filepath = filepath,
    dir = dir,
    name = yaml.name,
    id = yaml.id,
    runbooks = {},
    environments = yaml.environments or {}
  }
  
  -- Process runbooks
  if yaml.runbooks then
    for _, rb in ipairs(yaml.runbooks) do
      local runbook = {
        name = rb.name,
        id = rb.id,
        location = rb.location,
        description = rb.description,
        -- Resolve absolute path
        filepath = vim.fn.simplify(dir .. "/" .. rb.location)
      }
      table.insert(manifest.runbooks, runbook)
      -- Also index by id for quick lookup
      manifest.runbooks[rb.id] = runbook
    end
  end
  
  -- Cache the parsed manifest
  M.cache[dir] = {
    data = manifest,
    mtime = vim.fn.getftime(filepath)
  }
  
  return manifest
end

-- Basic YAML parser fallback (when yq is not available)
function M.parse_basic(lines)
  local manifest = {
    runbooks = {},
    environments = {}
  }
  
  local current_section = nil
  local current_runbook = nil
  local current_env = nil
  local indent_level = 0
  
  for _, line in ipairs(lines) do
    -- Skip empty lines and comments
    if line:match("^%s*$") or line:match("^%s*#") then
      goto continue
    end
    
    -- Calculate indent level
    local indent = #(line:match("^%s*") or "")
    
    -- Top-level keys
    if indent == 0 then
      local key, value = line:match("^(%w+):%s*(.*)$")
      if key then
        if key == "name" then
          manifest.name = value:gsub("^[\"']", ""):gsub("[\"']$", "")
        elseif key == "id" then
          manifest.id = value:gsub("^[\"']", ""):gsub("[\"']$", "")
        elseif key == "runbooks" then
          current_section = "runbooks"
        elseif key == "environments" then
          current_section = "environments"
        end
      end
    elseif current_section == "runbooks" then
      -- Parse runbook entries
      if line:match("^%s*%-%s*name:") then
        -- New runbook entry
        current_runbook = {}
        current_runbook.name = line:match("name:%s*(.+)$"):gsub("^[\"']", ""):gsub("[\"']$", "")
        table.insert(manifest.runbooks, current_runbook)
      elseif current_runbook and line:match("^%s+(%w+):") then
        local key, value = line:match("^%s+(%w+):%s*(.+)$")
        if key and value then
          value = value:gsub("^[\"']", ""):gsub("[\"']$", "")
          current_runbook[key] = value
        end
      end
    elseif current_section == "environments" then
      -- Parse environment entries
      if indent == 2 and line:match("^%s*%w+:") then
        -- New environment
        local env_name = line:match("^%s*(%w+):")
        current_env = {}
        manifest.environments[env_name] = current_env
      elseif current_env and indent == 4 then
        local key, value = line:match("^%s+(%w+):%s*(.+)$")
        if key and value then
          value = value:gsub("^[\"']", ""):gsub("[\"']$", "")
          current_env[key] = value
        end
      end
    end
    
    ::continue::
  end
  
  return manifest
end

-- Find manifest file starting from given path
function M.find_manifest(start_path)
  local path = vim.fn.fnamemodify(start_path, ":p:h")
  
  -- Search upward until we hit .git or root
  while path ~= "/" do
    -- Check for txtx.yml
    local yml_path = path .. "/txtx.yml"
    if vim.fn.filereadable(yml_path) == 1 then
      return yml_path
    end
    
    -- Check for txtx.yaml
    local yaml_path = path .. "/txtx.yaml"
    if vim.fn.filereadable(yaml_path) == 1 then
      return yaml_path
    end
    
    -- Stop at .git directory (workspace root)
    if vim.fn.isdirectory(path .. "/.git") == 1 then
      break
    end
    
    -- Move up one directory
    local parent = vim.fn.fnamemodify(path, ":h")
    if parent == path then
      break
    end
    path = parent
  end
  
  return nil
end

-- Get runbook info from manifest by location
function M.get_runbook_by_location(manifest, location)
  if not manifest or not manifest.runbooks then
    return nil
  end
  
  for _, runbook in ipairs(manifest.runbooks) do
    if runbook.location == location or runbook.filepath == location then
      return runbook
    end
  end
  
  return nil
end

-- Get environment variables for a specific environment
function M.get_environment_vars(manifest, env_name)
  if not manifest or not manifest.environments then
    return {}
  end
  
  return manifest.environments[env_name] or manifest.environments.default or {}
end

-- Clear cache for a specific directory
function M.clear_cache(dir)
  if dir then
    M.cache[dir] = nil
  else
    M.cache = {}
  end
end

-- Watch manifest file for changes
function M.watch(filepath)
  if not filepath then
    return
  end
  
  vim.api.nvim_create_autocmd("BufWritePost", {
    pattern = filepath,
    callback = function()
      local dir = vim.fn.fnamemodify(filepath, ":h")
      M.clear_cache(dir)
      -- Re-parse to update cache
      M.parse(filepath)
      -- Notify listeners
      vim.api.nvim_exec_autocmds("User", { pattern = "TxtxManifestChanged", data = { filepath = filepath } })
    end,
  })
end

return M