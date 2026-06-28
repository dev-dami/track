# LSP Server

Track includes a language server for IDE support.

## Usage

```bash
track-lsp
```

## Features

- **Diagnostics** — Real-time error checking for `.trk` files and `track` code blocks in markdown
- **Auto-completion** — Keywords, types, macros, enum/union variants
- **Hover documentation** — Information on language constructs
- **Syntax highlighting** — TextMate grammar for GitHub and VS Code

## Supported Files

| File Type | Diagnostics | Completion |
|-----------|-------------|------------|
| `.trk` | Yes | Yes |
| `.md` (track blocks) | Yes | Yes |

## Installation

```bash
./install.sh
```

This installs both `track` and `track-lsp` to `/usr/local/bin`.

## VS Code Integration

Add to your `settings.json`:

```json
{
  "language-server.track": {
    "command": "track-lsp",
    "filePatterns": ["*.trk", "*.md"]
  }
}
```

## Syntax Highlighting

A TextMate grammar is provided in `grammars/track.tmLanguage.json`.

### GitHub

To add Track syntax highlighting to GitHub, submit a PR to [github-linguist/linguist](https://github.com/github-linguist/linguist).

### VS Code

```bash
mkdir -p ~/.vscode/extensions/track-syntax/syntaxes
cp grammars/track.tmLanguage.json ~/.vscode/extensions/track-syntax/syntaxes/
```
