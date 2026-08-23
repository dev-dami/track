# Track — TextMate Grammar

Flat, simple, accurate highlighting for `.trk`.

## Coverage

`grammars/track.tmLanguage.json` (`scopeName: source.track`) covers:

- **Comments** `//` + `/* */` (with `TODO`/`FIXME` sub-scope)
- **Strings** `"..."` with escapes `\" \\ \n \r \t \0` and `\xFF`
- **Keywords** `import use as fn return if else while for in let mut with match const type struct enum union` + lens `with`
- **Types** `i8 u8 i32 u32 i64 u64 bool void ptr` + `[]T`/`[T; N]` and generic `Array<T>`
- **Functions** `fn name<T,U>(a: T) -> (T,U) {}` — name as `entity.name.function`
- **Macros** `@macro` definitions and `@call` sites
- **Generics** `<T, U>`
- **Numbers** `42`, `0xFF` with `_`, `0b1010`
- **Operators** `== != <= >= -> => :: .. && || & | ^ << >> + - * / %`
- **Punctuation** `{} () [] , : ; .`
- **Stdlib** `print` `vec_push` `alloc` `str_find` … → `support.function.stdlib`
- **Constants** `true/false` + `SCREAMING_SNAKE`

Folding: `\{` / `^\s*\}`; brackets for VS Code match the pairs in `editor/vscode/language-configuration.json`.

## Use

### VS Code

Use the bundled extension:

```bash
cd editor/vscode && vsce package && code --install-extension track-vscode-*.vsix
# or dev: open editor/vscode/ and press F5
```

Or copy the grammar manually:

```bash
mkdir -p ~/.vscode/extensions/track-syntax/syntaxes
cp grammars/track.tmLanguage.json ~/.vscode/extensions/track-syntax/syntaxes/
```

### Neovim

See `editor/nvim/README.md` — `require("track").setup()` gives tree-sitter-less `syntax/track.vim` + optional LSP.

### Sublime / TextMate / GitHub Linguist

- **Sublime/TextMate:** add `track.tmLanguage.json` to `Packages/`.
- **Linguist:** PR to `github-linguist/linguist`:
  1. Add `.trk` to `lib/linguist/languages.yml` (language `Track`, `type: programming`)
  2. Vendor this JSON under `vendor/grammars/`

```markdown
```track
fn main() -> void {
    print("hello");
}
```
```

### Validate

```bash
python3 -m json.tool grammars/track.tmLanguage.json >/dev/null && echo ok
```

Icon: `assets/track-icon.svg` — flat 2-rail + 3-tie rounded square (`#0F172A` / `#FFFFFF`).
