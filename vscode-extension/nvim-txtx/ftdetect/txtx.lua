-- Filetype detection for txtx files
vim.api.nvim_create_autocmd({ "BufRead", "BufNewFile" }, {
  pattern = "*.tx",
  callback = function()
    vim.bo.filetype = "txtx"
  end,
})

-- Detect txtx manifest files by content and set to hcl
vim.api.nvim_create_autocmd({ "BufRead", "BufNewFile" }, {
  pattern = { "*.yml", "*.yaml" },
  callback = function()
    -- Read first 100 lines to check for txtx manifest markers
    local lines = vim.api.nvim_buf_get_lines(0, 0, 100, false)
    local content = table.concat(lines, "\n")

    -- Check if file has txtx manifest structure
    local has_id = content:match("^id:%s") or content:match("\nid:%s")
    local has_environments = content:match("^environments:%s") or content:match("\nenviroments:%s")
    local has_runbooks = content:match("^runbooks:%s") or content:match("\nrunbooks:%s")

    if has_id and has_environments and has_runbooks then
      vim.bo.filetype = "yaml.txtx"
    end
  end,
})
