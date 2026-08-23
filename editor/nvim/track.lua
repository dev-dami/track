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
M.setup_syntax = function()
  vim.api.nvim_create_autocmd("FileType", {
    pattern = "track",
    callback = function()
      -- keywords
      vim.cmd([[syntax keyword trackKeyword import use fn return if else while for in let mut with struct enum union match const type as true false contained ]])
      -- types
      vim.cmd([[syntax keyword trackType i8 u8 i32 u32 i64 u64 bool void ptr contained ]])
      vim.cmd([[syntax match trackTypeName /\<[A-Z][a-zA-Z0-9_]*\>/ contained ]])
      -- macros
      vim.cmd([[syntax match trackMacro /@[a-zA-Z_][a-zA-Z0-9_]*\>/ contained ]])
      vim.cmd([[syntax keyword trackMacroDef @macro contained ]])
      -- functions & calls
      vim.cmd([[syntax match trackFunction /\v<[a-z_][a-zA-Z0-9_]*\ze\s*\(/ contained ]])
      -- strings & escapes
      vim.cmd([[syntax region trackString start=/"/ skip=/\\./ end=/"/ contained contains=trackEscape ]])
      vim.cmd([[syntax match trackEscape /\\[\"\\nrt0']/ contained ]])
      vim.cmd([[syntax match trackEscape /\\x[0-9A-Fa-f]\{2\}/ contained ]])
      -- comments
      vim.cmd([[syntax match trackComment /\/\/.*$/ contains=trackTodo contained ]])
      vim.cmd([[syntax region trackComment start=/\/\*/ end=/\*\// contains=trackTodo contained ]])
      vim.cmd([[syntax keyword trackTodo TODO FIXME NOTE HACK contained ]])
      -- numbers
      vim.cmd([[syntax match trackNumber /\<0[xX][0-9a-fA-F_]\+\>/ contained ]])
      vim.cmd([[syntax match trackNumber /\<0[bB][01_]\+\>/ contained ]])
      vim.cmd([[syntax match trackNumber /\<[0-9][0-9_]*\>/ contained ]])
      -- operators / punctuation
      vim.cmd([[syntax match trackOperator /->\|=>\|::\|\.\.\|&&\|||/ contained ]])
      vim.cmd([[syntax match trackOperator /[+\-*\/%<>=!&|^~]\+/ contained ]])
      vim.cmd([[syntax match trackDelimiter /[{}()\[\],;:\.]/ contained ]])
      vim.cmd([[syntax match trackLensArrow /->/ contained ]])

      -- cluster top-level groups so they actually appear:
      vim.cmd([[syntax cluster trackTop contains=trackKeyword,trackType,trackTypeName,trackMacro,trackMacroDef,trackFunction,trackString,trackEscape,trackComment,trackTodo,trackNumber,trackOperator,trackDelimiter,trackLensArrow ]])

      -- start parsing from top contains:
      -- (vim’s syntax sync needs a region; we define a trivial one)
      vim.cmd([[syntax region trackTop start=/^/ end=/$/ contains=@trackTop]])

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
M.setup_devicons = function()
  local ok, devicons = pcall(require, "nvim-web-devicons")
  if ok then
    devicons.set_icon({
      trk = {
        icon = "󱐌",
        color = "#7aa2f7",
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
