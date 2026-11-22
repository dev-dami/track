# LSP Server

Track's language server is part of the self-hosted compiler:

```bash
trackc lsp
```

It speaks JSON-RPC 2.0 over stdin/stdout with `Content-Length` framing
(the LSP base protocol) and is written in Track (`compiler/src/lsp.trk`).
The Rust `track-lsp` binary still builds but is deprecated; it will be
removed with the rest of the Rust code.

## Features

| Method | Behaviour |
|--------|-----------|
| `initialize` | Full text sync, completion (trigger `.` `:`), hover |
| `initialized` | Logs "Track LSP server initialized" (`window/logMessage`) |
| `textDocument/didOpen`, `didChange`, `didSave` (with text) | Publishes diagnostics |
| `textDocument/completion` | Keywords, types, built-ins, macros, and the file's own functions and enum/union variants |
| `textDocument/hover` | Docs for keywords, types, `print`/`read`, and macros |
| `shutdown`, `exit` | Exit status 0 after `shutdown`, 1 without it |

Any other request gets `-32601 Method not found`; requests before
`initialize` get `-32002`, requests after `shutdown` get `-32600`, and a
body that is not JSON gets `-32700`. Unknown notifications are ignored.

## Diagnostics

Diagnostics are exactly what `trackc check` reports: lexer, parser,
ownership and lifecycle errors, plus warnings (severity 2), each with its
line, column and code (`E0200`, `W0301`, ...). The buffer is checked under
its real path, so `import`s resolve as they do for `trackc check`; an
error inside an imported file is reported on line 1 with that file's
`path:line:col` in the message.

trackc stops at the first error by exiting, so the server never runs the
checker in-process: every analysis writes the buffer and its path to a
private scratch directory, `$TMPDIR/trackc-lsp-<tag>`, runs
`trackc lsp --check <tag>` as a child process (through
`/proc/$PPID/exe`, so Linux only) and reads the child's output. A checker
crash becomes a diagnostic, never a dead server.

- **Time limit.** The child runs under `timeout -k 1 <secs>` (exec'd, so
  no shell sits in between; a watchdog stands in when coreutils `timeout`
  is missing). `<secs>` is `$TRACKC_LSP_TIMEOUT`, default 10. A check that
  runs out of time publishes one diagnostic, `trackc: check timed out
  after <secs>s`, and the server keeps answering. If the server is killed
  mid-check, the orphaned child still dies at the limit.
- **Scratch directory.** `<tag>` is a fresh UUID from
  `/proc/sys/kernel/random/uuid`, and the directory is made with
  `mkdir -m 700`, which refuses any existing path (a planted symlink
  included). If that fails, the server keeps running but publishes no
  diagnostics and sends one `window/logMessage` saying so. The directory
  is removed on exit, and a reaper process removes it within about a
  second when the server dies any other way (SIGTERM, SIGKILL).
- **NUL bytes.** Messages are handled as NUL-terminated text: an escaped
  `\u0000` in a document is dropped, and a raw NUL byte (invalid JSON)
  cuts its message short, which gets a parse error reply.

## Supported Files

| File Type | Diagnostics | Completion |
|-----------|-------------|------------|
| `.trk` | Yes | Yes |
| `.md` / `.markdown` (```` ```track ```` / ```` ```trk ```` blocks) | Yes, per block | Keywords and built-ins |

## Editors

Neovim: `editor/nvim` launches `{ "trackc", "lsp" }` through
`nvim-lspconfig`; see `editor/nvim/README.md`.

Any LSP client: run the command `trackc` with the argument `lsp`, for
`*.trk` (and optionally `*.md`) files.

## Tests

```bash
tests/lsp/run.sh path/to/trackc
```

replays the recorded sessions in `tests/lsp/cases/` (no cargo needed).
`--update` re-records them.

## Syntax Highlighting

A TextMate grammar is provided in `grammars/track.tmLanguage.json`.

### GitHub

To add Track syntax highlighting to GitHub, submit a PR to [github-linguist/linguist](https://github.com/github-linguist/linguist).

### VS Code

```bash
mkdir -p ~/.vscode/extensions/track-syntax/syntaxes
cp grammars/track.tmLanguage.json ~/.vscode/extensions/track-syntax/syntaxes/
```
