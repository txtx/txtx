-- Tree-sitter configuration for txtx
local M = {}

local function get_parser_path()
  -- Get the path to this plugin
  local plugin_path = debug.getinfo(1, "S").source:sub(2):match("(.*)/lua/txtx/treesitter.lua$")
  
  -- Determine the correct extension based on OS
  local uname = vim.loop.os_uname()
  local ext = uname.sysname == "Darwin" and "dylib" or "so"
  
  return plugin_path .. "/parser/txtx." .. ext
end

function M.setup(config)
  -- Check if tree-sitter is available
  local ok, parsers = pcall(require, "nvim-treesitter.parsers")
  if not ok then
    vim.notify("nvim-treesitter not found. Tree-sitter highlighting will not be available.", vim.log.levels.WARN)
    return
  end
  
  local parser_path = get_parser_path()
  
  -- Check if parser exists
  if vim.fn.filereadable(parser_path) == 0 then
    vim.notify("txtx parser not found at " .. parser_path .. ". Run :TxtxBuildParser to build it.", vim.log.levels.WARN)
    return
  end
  
  -- Register the parser
  local parser_config = parsers.get_parser_configs()
  parser_config.txtx = {
    install_info = {
      url = "https://github.com/txtx/txtx",
      files = { "src/parser.c" },
      branch = "main",
    },
    filetype = "txtx",
  }
  
  -- Add the parser path to vim's runtimepath
  vim.opt.runtimepath:append(vim.fn.fnamemodify(parser_path, ":h:h"))
  
  -- Load highlights
  M.load_highlights()
end

function M.load_highlights()
  -- Create highlight groups for txtx
  local highlights = {
    -- Keywords
    ["@keyword.txtx"] = { link = "Keyword" },
    ["@keyword.function.txtx"] = { link = "Keyword" },
    
    -- Types
    ["@type.txtx"] = { link = "Type" },
    ["@type.builtin.txtx"] = { link = "Type" },
    
    -- Strings
    ["@string.txtx"] = { link = "String" },
    ["@string.escape.txtx"] = { link = "SpecialChar" },
    
    -- Comments
    ["@comment.txtx"] = { link = "Comment" },
    ["@comment.documentation.txtx"] = { link = "SpecialComment" },
    
    -- Functions
    ["@function.txtx"] = { link = "Function" },
    ["@function.builtin.txtx"] = { link = "Special" },
    ["@function.call.txtx"] = { link = "Function" },
    
    -- Variables
    ["@variable.txtx"] = { link = "Identifier" },
    ["@variable.builtin.txtx"] = { link = "Special" },
    ["@variable.parameter.txtx"] = { link = "Parameter" },
    
    -- Properties/Fields
    ["@property.txtx"] = { link = "Property" },
    ["@field.txtx"] = { link = "Field" },
    
    -- Operators
    ["@operator.txtx"] = { link = "Operator" },
    
    -- Punctuation
    ["@punctuation.bracket.txtx"] = { link = "Delimiter" },
    ["@punctuation.delimiter.txtx"] = { link = "Delimiter" },
    
    -- Constants
    ["@constant.txtx"] = { link = "Constant" },
    ["@constant.builtin.txtx"] = { link = "Special" },
    ["@boolean.txtx"] = { link = "Boolean" },
    ["@number.txtx"] = { link = "Number" },
  }
  
  for group, opts in pairs(highlights) do
    vim.api.nvim_set_hl(0, group, opts)
  end
end

return M