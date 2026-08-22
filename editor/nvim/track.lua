-- Track language support for Neovim
-- Add to your init.lua or load as plugin

local M = {}

-- LSP configuration
M.lsp_config = {
  default_config = {
    cmd = { "track-lsp" },
    filetypes = { "track", "trk" },
    root_dir = vim.fs.dirname(vim.fs.find({ "Track.toml", ".git" }, { upward = true })[1]),
  },
  docs = {
    description = "Track language server",
  },
}

-- Syntax highlighting via TreeSitter or vim syntax
M.setup_syntax = function()
  -- Define syntax keywords
  vim.cmd([[
    syntax keyword trackKeyword import use fn return if else while for in let mut with struct enum union match const type as true false
    syntax keyword trackType i8 u8 i32 u32 i64 u64 bool void ptr
    syntax match trackMacro /@[a-zA-Z_][a-zA-Z0-9_]*/
    syntax match trackNamespace /::/
    syntax match trackOperator /->/
    syntax match trackOperator /=>/
    syntax match trackComment /\/\/.*$/

    highlight default link trackKeyword Keyword
    highlight default link trackType Type
    highlight default link trackMacro Macro
    highlight default link trackNamespace Structure
    highlight default link trackOperator Operator
    highlight default link trackComment Comment
  ]])
end

-- Filetype detection
M.setup_filetype = function()
  vim.filetype.add({
    extension = {
      trk = "track",
    },
    pattern = {
      ["*.trk"] = "track",
    },
  })
end

-- Register LSP with nvim-lspconfig
M.setup_lsp = function()
  local ok, lspconfig = pcall(require, "lspconfig")
  if not ok then
    vim.notify("lspconfig not found. Install neovim/nvim-lspconfig", vim.log.levels.WARN)
    return
  end

  local configs = require("lspconfig.configs")
  if not configs.track_lsp then
    configs.track_lsp = {
      default_config = M.lsp_config.default_config,
    }
  end

  lspconfig.track_lsp.setup({
    on_attach = function(client, bufnr)
      vim.notify("Track LSP attached", vim.log.levels.INFO)
    end,
  })
end

-- Filetype devicon registration for Neovim file explorers / statuslines
M.setup_devicons = function()
  local ok, devicons = pcall(require, "nvim-web-devicons")
  if ok then
    devicons.set_icon({
      trk = {
        icon = "T",
        color = "#ffffff",
        cterm_color = "15",
        name = "Track"
      }
    })
  end
end



-- Full setup
M.setup = function()
  M.setup_filetype()
  M.setup_devicons()
  M.setup_syntax()
  M.setup_lsp()
end


return M
