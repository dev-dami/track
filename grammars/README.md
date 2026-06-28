# Track Syntax Highlighting

## GitHub Markdown

To add Track syntax highlighting to GitHub markdown code blocks, submit a PR to [github-linguist/linguist](https://github.com/github-linguist/linguist) with:

1. Add `.trk` to `lib/linguist/languages.yml`
2. Add the TextMate grammar from `track.tmLanguage.json`

This enables:

````markdown
```track
fn main() -> void {
    print("hello");
}
```
````

## VS Code

Copy `track.tmLanguage.json` to your VS Code extensions directory or use the extension manifest.

## Installation

```bash
# Copy grammar to VS Code
mkdir -p ~/.vscode/extensions/track-syntax/syntaxes
cp track.tmLanguage.json ~/.vscode/extensions/track-syntax/syntaxes/
```
