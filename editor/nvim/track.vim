-- Track language support for Neovim
-- Usage: Add this to your init.lua or source it

-- 1. Filetype detection
vim.filetype.add({
  extension = {
    trk = "track",
  },
})

-- 2. Syntax highlighting
vim.cmd([[
  syntax keyword trackKeyword import fn return if else while let mut with struct enum union match const as true false
  syntax keyword trackType i8 i16 i32 i64 u8 u16 u32 u64 bool void ptr str
  syntax match trackMacro /@macro/

  syntax match trackMacro /@bit/
  syntax match trackMacro /@pin/
  syntax match trackMacro /@register/
  syntax match trackMacro /@compile_error/
  syntax match trackMacro /@now/
  syntax match trackMacro /@fib_comptime/
  syntax match trackMacro /@timer/
  syntax match trackMacro /@assert/
  syntax match trackNamespace /::/
  syntax match trackOperator /->/
  syntax match trackOperator /=>/
  syntax region trackString start=/"/ end=/"/
  syntax match trackComment /\/\/.*$/

  highlight default link trackKeyword Keyword
  highlight default link trackType Type
  highlight default link trackMacro Macro
  highlight default link trackNamespace Structure
  highlight default link trackOperator Operator
  highlight default link trackString String
  highlight default link trackComment Comment
]])

-- 3. LSP configuration
local ok, lspconfig = pcall(require, "lspconfig")
if ok then
  local configs = require("lspconfig.configs")
  if not configs.track_lsp then
    configs.track_lsp = {
      default_config = {
        cmd = { "track-lsp" },
        filetypes = { "track", "trk" },
        root_dir = vim.fs.dirname(vim.fs.find({ "Track.toml", ".git" }, { upward = true })[1]),
      },
    }
  end

  lspconfig.track_lsp.setup({
    on_attach = function(client, bufnr)
      vim.notify("Track LSP attached", vim.log.levels.INFO)
    end,
  })
else
  vim.notify("lspconfig not found. Install neovim/nvim-lspconfig", vim.log.levels.WARN)
end

-- 4. Keymaps (optional)
vim.api.nvim_create_autocmd("FileType", {
  pattern = "track",
  callback = function()
    vim.keymap.set("n", "gd", vim.lsp.buf.definition, { buffer = true, desc = "Track: Go to definition" })
    vim.keymap.set("n", "K", vim.lsp.buf.hover, { buffer = true, desc = "Track: Hover" })
    vim.keymap.set("n", "<leader>ca", vim.lsp.buf.code_action, { buffer = true, desc = "Track: Code action" })
  end,
})
