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
    syntax keyword trackKeyword fn return if else while let mut with struct enum union match const as true false
    syntax keyword trackType i8 i16 i32 i64 u8 u16 u32 u64 bool void ptr str
    syntax match trackMacro /@use/
    syntax match trackMacro /@macro/
    syntax match trackMacro /@bit/
    syntax match trackMacro /@pin/
    syntax match trackMacro /@register/
    syntax match trackMacro /@compile_error/
    syntax match trackMacro /@compile_warning/
    syntax match trackMacro /@now/
    syntax match trackMacro /@fib_comptime/
    syntax match trackMacro /@timer/
    syntax match trackMacro /@assert/
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
        icon = "󰜎",
        color = "#3b82f6",
        cterm_color = "39",
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
