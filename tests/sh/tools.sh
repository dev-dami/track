# Editor assets: the VS Code extension, the TextMate grammar and the
# Neovim plugin. Port of tests/editor_tests.rs and
# tests/headless_highlight_tests.rs. JSON is read with jq; grammar regexes
# run under `grep -P` (PCRE, close to the Oniguruma engine TextMate uses).
# The nvim and luac checks skip when the tool is missing, as the Rust
# tests did.

need_jq() { have jq || fail "jq is required to read the editor JSON files"; }

# json_ok FILE: FILE holds exactly one valid JSON value (as serde_json
# required). Call it before jqv, whose failures inside $(...) cannot end
# the test.
json_ok() {
    [ "$(jq -s length "$1" 2>"$T/.jqerr")" = 1 ] || fail "$1 is invalid JSON: $(cat "$T/.jqerr")"
}

# jqv FILE FILTER: one raw jq result.
jqv() {
    jq -r "$2" "$1" 2>"$T/.jqerr" || fail "$1: invalid JSON or bad filter: $(cat "$T/.jqerr")"
}

GRAMMAR=editor/vscode/syntaxes/track.tmLanguage.json

t_vscode_package_json() {
    need_jq
    local p="$ROOT/editor/vscode/package.json"
    [ -f "$p" ] || fail "package.json does not exist at editor/vscode/package.json"
    json_ok "$p"
    eq "$(jqv "$p" .name)" track-vscode "name"
    eq "$(jqv "$p" .publisher)" dev-dami "publisher"
    eq "$(jqv "$p" .icon)" images/track-icon.png "icon"
    eq "$(jqv "$p" '.contributes.languages | type')" array "languages array missing"
    ne "$(jqv "$p" '.contributes.languages | length')" 0 "languages empty"
    eq "$(jqv "$p" '.contributes.languages[0].id')" track "languages[0].id"
}

t_vscode_language_configuration() {
    need_jq
    local p="$ROOT/editor/vscode/language-configuration.json"
    [ -f "$p" ] || fail "language-configuration.json does not exist"
    json_ok "$p"
    eq "$(jqv "$p" .comments.lineComment)" "//" "lineComment"
    eq "$(jqv "$p" '.brackets | type')" array "brackets"
    eq "$(jqv "$p" '.autoClosingPairs | type')" array "autoClosingPairs"
}

t_vscode_textmate_grammar() {
    need_jq
    local p="$ROOT/$GRAMMAR" combined
    [ -f "$p" ] || fail "track.tmLanguage.json does not exist"
    json_ok "$p"
    eq "$(jqv "$p" .scopeName)" source.track "scopeName"
    eq "$(jqv "$p" .name)" Track "name"
    eq "$(jqv "$p" '.repository.keywords.patterns | type')" array "keywords patterns missing"
    combined="$(jqv "$p" '[.repository.keywords.patterns[] | .match // empty] | join(" ")') $(jqv "$p" '[(.repository.typeDefinitions.patterns // [])[] | .match // .begin // empty] | join(" ")')"
    has "$combined" import "Grammar keywords missing 'import'"
    has "$combined" fn "Grammar keywords missing 'fn'"
    has "$combined" struct "Grammar keywords missing 'struct'"
}

t_nvim_plugin_files() {
    local vim="$ROOT/editor/nvim/track.vim" lua="$ROOT/editor/nvim/track.lua" src
    [ -f "$vim" ] || fail "track.vim does not exist at editor/nvim/track.vim"
    [ -f "$lua" ] || fail "track.lua does not exist at editor/nvim/track.lua"
    src=$(cat "$lua")
    has "$src" import "track.lua syntax missing 'import'"
    case "$src" in
        *'icon = "T"'* | *'icon = "󱐌"'*) ;;
        *) fail "track.lua devicons missing icon" ;;
    esac
    case "$src" in
        *'color = "#ffffff"'* | *'color = "#7aa2f7"'*) ;;
        *) fail "track.lua devicons missing color" ;;
    esac
    if have luac; then
        cap luac -p "$lua"
        rc_ok "Lua syntax check failed for track.lua"
    fi
}

# Every major construct in a sample is matched by some grammar regex.
t_vscode_grammar_headless_tokenization() {
    need_jq
    local p="$ROOT/$GRAMMAR" pats n sample label text needles needle pat matched r
    json_ok "$p"
    pats="$T/patterns"
    jqv "$p" '.repository | to_entries[] | (.value.patterns // [])[] | (.match // empty), (.begin // empty)' >"$pats"
    n=$(wc -l <"$pats")
    [ "$n" -gt 10 ] || fail "too few patterns: $n"
    sample='import "std/io";
fn main() -> void {
  let mut x: i64 = 42;
  // TODO fix
  let s = "hello \"world\"";
  let n = 0xFF;
  let b = 0b1010;
  if x > 0 { print(s); }
  with u -> v { v.set(1); }
  @macro bit(n: u32) -> u32 { return 1 << n; }
  let p: ptr<u8> = alloc(8);
  match val { _ => print(0), }
}
'
    # label | text (S = the sample) | needles separated by TAB
    while IFS='|' read -r label text needles; do
        [ "$text" = S ] && text=$sample
        matched=0
        while IFS= read -r pat; do
            IFS=$'\t'
            for needle in $needles; do
                printf '%s' "$needle" | grep -qP -- "$pat" 2>/dev/null
                r=$?
                # r=2: not a valid PCRE pattern, skipped (as the Rust test skipped invalid regexes)
                if [ "$r" = 0 ]; then
                    case "$text" in *"$needle"*) matched=1 ;; esac
                fi
                [ "$matched" = 1 ] && break
            done
            unset IFS
            [ "$matched" = 1 ] && break
        done <"$pats"
        [ "$matched" = 1 ] || fail "headless VS Code: pattern for $label ($needles) not matched by any grammar regex"
    done <<CASES
keyword fn|S|fn
keyword import|S|import
keyword struct/enum|struct Foo { }|struct
type i64|S|i64
type ptr|S|ptr
string|S|"std/io"	"hello
comment|S|// TODO
hex number|S|0xFF
binary number|S|0b1010
macro|S|@macro
operator ->|S|->
operator ::|a::b|::
CASES
    eq "$(jqv "$p" .scopeName)" source.track "scopeName"
}

t_vscode_grammar_valid_json_headless() {
    need_jq
    local rel p bad
    for rel in grammars/track.tmLanguage.json "$GRAMMAR"; do
        p="$ROOT/$rel"
        json_ok "$p"
        eq "$(jqv "$p" .scopeName)" source.track "scopeName mismatch in $rel"
        eq "$(jqv "$p" '.repository | type')" object "repository missing in $rel"
        bad=$(jqv "$p" '.repository | to_entries[] | select((.value.patterns | type) != "array" or (.value.patterns | length) == 0) | .key')
        [ -z "$bad" ] || fail "$rel: repository entries without patterns: $bad"
    done
}

nvim_cmd() { # nvim_cmd EXTRA-COMMANDS...: nvim headless with the plugin loaded
    local args=(--headless -u NONE -n -c "set rtp+=$ROOT/editor/nvim" -c "lua require('track').setup()" -c "edit examples/hello.trk")
    local c
    for c in "$@"; do args+=(-c "$c"); done
    in_repo nvim "${args[@]}"
}

t_nvim_headless_filetype_and_syntax() {
    have nvim || skip "nvim not found"
    local grp
    nvim_cmd "lua print('FT='..vim.bo.filetype)" "qa!"
    has "$OUT $ERR" "FT=track" "nvim headless filetype not track"
    nvim_cmd "doautocmd FileType track" "redir => g:__syn | silent syn list | redir END | silent put =g:__syn | %print | qa!"
    for grp in trackKeyword trackType trackString trackComment trackNumber; do
        has "$OUT $ERR" "$grp" "nvim syntax group $grp missing in headless output"
    done
    nvim_cmd "doautocmd FileType track" "redir => g:__hl | silent hi trackKeyword | redir END | silent put =g:__hl | %print | qa!"
    has "$OUT $ERR" trackKeyword "highlight link for trackKeyword missing"
    has "$OUT $ERR" Keyword "highlight link for trackKeyword missing"
}

t_nvim_headless_lua_syntax_ok() {
    have nvim || skip "nvim not found"
    in_repo nvim --headless -u NONE -n -c "luafile $ROOT/editor/nvim/track.lua" -c "lua print('lua_ok')" -c "qa!"
    has "$OUT $ERR" lua_ok "nvim luafile failed"
}

# Terminal nvim cannot render SVG: the plugin documents that and its code
# never loads an .svg; the VS Code extension uses the PNG icon.
t_nvim_devicon_svg_not_needed_headless() {
    local lua="$ROOT/editor/nvim/track.lua" src code
    src=$(cat "$lua")
    case "$src" in
        *"Terminal cannot render"* | *"vector images"*) ;;
        *) fail "track.lua should document that SVG/vector is not for nvim" ;;
    esac
    code=$(grep -v '^[[:space:]]*--' "$lua")
    lacks "$code" ".svg" "nvim track.lua code should not reference .svg"
    grep -qF track-icon.png "$ROOT/editor/vscode/package.json" || fail "vscode package.json should reference PNG icon"
    [ -f "$ROOT/assets/track-icon.svg" ] || fail "assets/track-icon.svg missing"
}

t_nvim_syntax_highlight_sample_headless() {
    have nvim || skip "nvim not found"
    nvim_cmd "doautocmd FileType track" "let g:ids = [] | for lnum in range(1, line('\$')) | for col in range(1, strdisplaywidth(getline(lnum))) | call add(g:ids, synIDattr(synID(lnum, col, 1), 'name')) | endfor | endfor | echo join(g:ids, ',')" "qa!"
    case "$OUT $ERR" in
        *trackKeyword* | *trackFunction* | *trackString*) ;;
        *) fail "nvim headless synID produced no Track syntax: $OUT $ERR" ;;
    esac
}
