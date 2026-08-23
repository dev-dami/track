-- Track — flat, simple Neovim plugin (init.lua / lazy.nvim)
-- Minimal, no external deps beyond nvim-lspconfig + optional nvim-web-devicons.
-- Usage:
--   require("track").setup()
-- Treesitter: add queries/track/highlights.scm for better capture if you use nvim-treesitter.

local M = {}

local function has(cmd) return vim.fn.executable(cmd) == 1 end

-- ── filetype ───────────────────────────────────────────────────────
M.setup_filetype = function()
  vim.filetype.add({
    extension = { trk = "track" },
  })
  -- fallback for older nvim
  vim.api.nvim_create_autocmd({ "BufRead", "BufNewFile" }, {
    pattern = "*.trk",
    callback = function() vim.bo.filetype = "track" end,
  })
end

-- ── highlights ─────────────────────────────────────────────────────
-- Executed on FileType=track so theme overrides don’t stick.
-- Note: terminal nvim cannot render SVG — it uses font glyphs (see setup_devicons).
M.setup_syntax = function()
  vim.api.nvim_create_autocmd("FileType", {
    pattern = "track",
    callback = function()
      vim.cmd([[syntax clear]])
      -- keywords
      vim.cmd([[syntax keyword trackKeyword import use fn return if else while for in let mut with struct enum union match const type as true false]])
      -- types
      vim.cmd([[syntax keyword trackType i8 u8 i32 u32 i64 u64 bool void ptr]])
      vim.cmd([[syntax match trackTypeName /\<[A-Z][a-zA-Z0-9_]*\>/]])
      -- macros
      vim.cmd([[syntax match trackMacro /@[a-zA-Z_][a-zA-Z0-9_]*\>/]])
      vim.cmd([[syntax keyword trackMacroDef @macro]])
      -- functions & calls
      vim.cmd([[syntax match trackFunction /\v<[a-z_][a-zA-Z0-9_]*\ze\s*\(/]])
      -- strings & escapes
      vim.cmd([[syntax region trackString start=/"/ skip=/\\./ end=/"/ contains=trackEscape]])
      vim.cmd([[syntax match trackEscape /\\[\"\\nrt0']/ contained]])
      vim.cmd([[syntax match trackEscape /\\x[0-9A-Fa-f]\{2\}/ contained]])
      -- comments (SVG not used here — font glyphs only)
      vim.cmd([[syntax match trackComment /\/\/.*$/ contains=trackTodo]])
      vim.cmd([[syntax region trackComment start=/\/\*/ end=/\*\// contains=trackTodo]])
      vim.cmd([[syntax keyword trackTodo TODO FIXME NOTE HACK contained]])
      -- numbers
      vim.cmd([[syntax match trackNumber /\<0[xX][0-9a-fA-F_]\+\>/]])
      vim.cmd([[syntax match trackNumber /\<0[bB][01_]\+\>/]])
      vim.cmd([[syntax match trackNumber /\<[0-9][0-9_]*\>/]])
      -- operators / punctuation
      vim.cmd([[syntax match trackOperator /->\|=>\|::\|\.\.\|&&\|||/]])
      vim.cmd([[syntax match trackOperator /[+\-*\/%<>=!&|^~]\+/]])
      vim.cmd([[syntax match trackDelimiter /[{}()\[\],;:\.]/]])
      vim.cmd([[syntax match trackLensArrow /->/]])
      -- links
      vim.cmd([[highlight default link trackKeyword Keyword]])
      vim.cmd([[highlight default link trackType Type]])
      vim.cmd([[highlight default link trackTypeName Structure]])
      vim.cmd([[highlight default link trackMacro Macro]])
      vim.cmd([[highlight default link trackMacroDef Define]])
      vim.cmd([[highlight default link trackFunction Function]])
      vim.cmd([[highlight default link trackString String]])
      vim.cmd([[highlight default link trackEscape SpecialChar]])
      vim.cmd([[highlight default link trackComment Comment]])
      vim.cmd([[highlight default link trackTodo Todo]])
      vim.cmd([[highlight default link trackNumber Number]])
      vim.cmd([[highlight default link trackOperator Operator]])
      vim.cmd([[highlight default link trackDelimiter Delimiter]])
      vim.cmd([[highlight default link trackLensArrow Operator]])
    end,
  })
end

-- ── devicons ────────────────────────────────────────────────────────
-- Terminal cannot render vector images — assets icon is for VS Code/docs only.
-- We use a Nerd Font glyph; fallback is plain "T" if font missing.
M.setup_devicons = function()
  local ok, devicons = pcall(require, "nvim-web-devicons")
  if ok then
    -- primary flat glyph (Nerd Font) — fallback icon = "T" if Nerd Font not installed
    devicons.set_icon({
      trk = {
        icon = "󱐌", -- alt: icon = "T"
        color = "#7aa2f7", -- flat, matches assets icon bg #0F172A; alt #ffffff
        cterm_color = "111",
        name = "Track",
      },
    })
  end
end

-- ── lsp ─────────────────────────────────────────────────────────────
M.lsp_config = {
  cmd = { "track-lsp" },
  filetypes = { "track" },
  root_dir = function(fname)
    local found = vim.fs.find({ "Track.toml", "yard.toml", ".git" }, { upward = true, path = vim.fs.dirname(fname) })[1]
    return found and vim.fs.dirname(found) or vim.fs.dirname(fname)
  end,
  settings = {},
}

M.setup_lsp = function(opts)
  opts = opts or {}
  local ok, lspconfig = pcall(require, "lspconfig")
  if not ok then
    vim.notify("[track] nvim-lspconfig not found — LSP disabled", vim.log.levels.WARN)
    return
  end
  local configs = require("lspconfig.configs")
  if not configs.track_lsp then
    configs.track_lsp = { default_config = M.lsp_config }
  end
  -- only autostart if binary exists
  if not has("track-lsp") then
    vim.notify("[track] track-lsp not on PATH — run scripts/install.sh", vim.log.levels.INFO)
    return
  end
  lspconfig.track_lsp.setup(vim.tbl_deep_extend("force", {
    on_attach = function(_, bufnr)
      local map = function(mode, lhs, rhs, desc)
        vim.keymap.set(mode, lhs, rhs, { buffer = bufnr, desc = desc, silent = true })
      end
      map("n", "gd", vim.lsp.buf.definition, "Track: goto definition")
      map("n", "gD", vim.lsp.buf.declaration, "Track: goto declaration")
      map("n", "K", vim.lsp.buf.hover, "Track: hover")
      map("n", "<leader>ca", vim.lsp.buf.code_action, "Track: code action")
      map("n", "<leader>rn", vim.lsp.buf.rename, "Track: rename")
      map("n", "gr", vim.lsp.buf.references, "Track: references")
    end,
  }, opts))
end

-- ── public setup ────────────────────────────────────────────────────
M.setup = function(opts)
  opts = opts or {}
  M.setup_filetype()
  M.setup_devicons()
  M.setup_syntax()
  if opts.lsp ~= false then M.setup_lsp(opts.lsp_opts) end
end

return M
