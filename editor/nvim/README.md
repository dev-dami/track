# Track — Neovim / Vim Support (flat, simple)

Minimal, dependency-light. One command to enable everything.

## Install

### lazy.nvim (recommended)

```lua
{
  "dev-dami/track",
  ft = "track",
  config = function()
    require("track").setup() -- auto filetype + syntax + LSP + devicons
  end,
}
-- optional deps:
-- { "neovim/nvim-lspconfig" }
-- { "nvim-tree/nvim-web-devicons" }
}
```

### Manual — Neovim `init.lua`

```lua
-- clone or symlink repo somewhere, then:
vim.opt.rtp:append("/path/to/track/editor/nvim")
require("track").setup()
```

Add this if you want the binary quickly:

```sh
curl -fsSL https://raw.githubusercontent.com/dev-dami/track/main/scripts/install.sh | bash
# or:  TRACK_INSTALL_DIR=~/.local/bin bash scripts/install.sh
```

### Vim 8 / legacy

```sh
mkdir -p ~/.vim/syntax ~/.vim/ftdetect
cp editor/nvim/track.vim ~/.vim/syntax/track.vim
echo 'augroup track_ft | au! | au BufNewFile,BufRead *.trk setf track | augroup END' > ~/.vim/ftdetect/track.vim
```

## Features

- **Filetype** `*.trk` → `track` (`vim.filetype.add` + autocmd fallback)
- **Flat syntax** keywords, types, macros (`@macro` / `@call`), functions, strings/escapes, numbers (`0x`/`0b`), operators (`->` `=>` `::` `..`), `//` + `/* */` comments with `TODO` highlight
- **LSP** `track-lsp` via `nvim-lspconfig` — hover/`gd`/`gr`/`K`/`<leader>ca`/`<leader>rn` on attach; warns if binary missing
- **Devicons** `nvim-web-devicons` → `trk` icon (`󱐌`, `#7aa2f7`) — *requires Nerd Font; falls back to `T` if missing*

## Why no SVG in Neovim?

Neovim is a **terminal UI** — it cannot render SVG/PNG images inline.  
`assets/track-icon.svg` and `editor/vscode/images/*.{svg,png}` are **for VS Code’s file-icon theme, GitHub, and docs** (graphical environments that can draw vectors).  
In the terminal, `nvim-web-devicons` uses a **font glyph** (`󱐌` from Nerd Fonts) instead. If you don’t have a Nerd Font, you’ll see `□` — install one (e.g. JetBrainsMono Nerd Font) or set `icon = "T"` in `editor/nvim/track.lua:89`.

## Commands

`:LspInfo` — verify `track_lsp` attached.  
`vim.lsp.buf.hover()` / `vim.lsp.buf.definition()` bound per-buffer on LSP attach.

## Other Editors

- **VS Code**: see `editor/vscode/` — `Track` extension uses `grammars/track.tmLanguage.json`.
- **Sublime / TextMate / GitHub Linguist**: `grammars/track.tmLanguage.json` + add `.trk` to `lib/linguist/languages.yml`.
