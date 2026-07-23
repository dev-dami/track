# Track Editor Support

## Neovim Setup

### Option 1: Manual (init.lua)

Add to your `~/.config/nvim/init.lua`:

```lua
-- Source the track plugin
vim.cmd("source /path/to/track/editor/nvim/track.vim")
```

### Option 2: lazy.nvim

```lua
{
  "dev-dami/track",
  ft = "track",
  config = function()
    vim.cmd("source " .. vim.fn.expand("~/.local/share/nvim/lazy/track/editor/nvim/track.vim"))
  end,
}
```

### Option 3: packer.nvim

```lua
use {
  "dev-dami/track",
  ft = "trk",
  config = function()
    vim.cmd("source " .. vim.fn.expand("~/.local/share/nvim/site/pack/packer/start/track/editor/nvim/track.vim"))
  end,
}
```

## Features

- Syntax highlighting for keywords, types, macros
- LSP support (diagnostics, completion, hover)
- Filetype detection for `.trk` files
- Keymaps: `gd` (definition), `K` (hover), `<leader>ca` (code action)

## Requirements

1. Install Track with `./install.sh`
2. Install [nvim-lspconfig](https://github.com/neovim/nvim-lspconfig)
3. Copy or source the plugin files

## Other Editors

### VS Code

Copy `grammars/track.tmLanguage.json` to your VS Code extensions directory.

### Sublime Text

Use the TextMate grammar in `grammars/track.tmLanguage.json`.
