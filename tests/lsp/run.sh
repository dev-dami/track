#!/bin/sh
# Recorded JSON-RPC sessions against `trackc lsp`; no cargo needed.
#
#   tests/lsp/run.sh [path/to/trackc]      compare every session
#   tests/lsp/run.sh --update [trackc]     re-record the expected output
#
# Each tests/lsp/cases/NAME.in holds one JSON message per line (`#` lines
# are comments). The runner frames every message with a Content-Length
# header, pipes the session to `trackc lsp`, checks that every reply is
# framed with the right byte count, and compares the reply bodies (one per
# line) plus the exit status with NAME.out. Sessions generated below (a
# slow check, planted links, no scratch dir, a NUL byte) record the same way.
set -eu
LC_ALL=C
export LC_ALL
here=$(cd "$(dirname "$0")" && pwd)
root=$(cd "$here/../.." && pwd)
update=0
if [ "${1:-}" = "--update" ]; then
    update=1
    shift
fi
trackc=${1:-${TRACKC:-$root/build/bootstrap/stage3/trackc}}
case "$trackc" in
    /*) ;;
    *) trackc="$(pwd)/$trackc" ;;
esac
if [ ! -x "$trackc" ]; then
    echo "error: no trackc at $trackc (pass one as the first argument)" >&2
    exit 2
fi
scratch=$(mktemp -d)
trap 'rm -rf "$scratch"' EXIT
TMPDIR=$scratch
export TMPDIR

frame() {
    while IFS= read -r line || [ -n "$line" ]; do
        case "$line" in
            ''|'#'*) continue ;;
        esac
        printf 'Content-Length: %d\r\n\r\n%s' "$(printf %s "$line" | wc -c)" "$line"
    done
}

# Split framed output into one body per line; flag bad framing.
unframe() {
    awk 'BEGIN { RS = "\001" }
    {
        s = $0
        while (length(s) > 0) {
            if (match(s, /^Content-Length: [0-9]+\r\n\r\n/) == 0) {
                print "BAD FRAMING: " substr(s, 1, 60)
                exit
            }
            n = substr(s, 17, RLENGTH - 20) + 0
            body = substr(s, RLENGTH + 1, n)
            if (length(body) != n) { print "SHORT BODY: " body; exit }
            print body
            s = substr(s, RLENGTH + n + 1)
        }
    }'
}

pass=0
fail=0

# Run the framed session in file $2 as case $1 (from tests/lsp/cases, with
# the optional env assignment $3) and compare (or record) the reply bodies
# plus the exit status.
session() {
    name=$1
    expected="$here/cases/$name.out"
    actual="$scratch/$name.out"
    status=0
    (cd "$here/cases" && timeout 60 env ${3:-} "$trackc" lsp < "$2" > "$scratch/$name.raw") || status=$?
    { unframe < "$scratch/$name.raw"; echo "exit=$status"; } > "$actual"
    if [ "$update" = 1 ]; then
        cp "$actual" "$expected"
        echo "recorded $name"
    elif cmp -s "$expected" "$actual"; then
        pass=$((pass + 1))
        echo "ok   $name"
    else
        fail=$((fail + 1))
        echo "FAIL $name"
        diff "$expected" "$actual" | head -n 20 || true
    fi
}

# A shell condition $2 that must hold, named $1.
check() {
    if [ "$update" = 1 ]; then
        return 0
    fi
    if eval "$2"; then
        pass=$((pass + 1))
        echo "ok   $1"
    else
        fail=$((fail + 1))
        echo "FAIL $1"
    fi
}

for case_in in "$here"/cases/*.in; do
    name=$(basename "$case_in" .in)
    frame < "$case_in" > "$scratch/$name.framed"
    session "$name" "$scratch/$name.framed"
done

# Generated sessions. `opened URI TEXT` opens file:///URI with TEXT (JSON
# escaped), hovers at its start, then shuts down.
opened() {
    printf '%s\n' '{"jsonrpc":"2.0","id":1,"method":"initialize","params":{"capabilities":{}}}'
    printf '{"jsonrpc":"2.0","method":"textDocument/didOpen","params":{"textDocument":{"uri":"file:///%s","languageId":"track","version":1,"text":"%s"}}}\n' "$1" "$2"
    printf '{"jsonrpc":"2.0","id":2,"method":"textDocument/hover","params":{"textDocument":{"uri":"file:///%s"},"position":{"line":0,"character":1}}}\n' "$1"
    printf '%s\n' '{"jsonrpc":"2.0","id":3,"method":"shutdown"}' '{"jsonrpc":"2.0","method":"exit"}'
}

# A check the checker needs many seconds for (12000 lets in one fn hit a
# quadratic in the checker) times out after TRACKC_LSP_TIMEOUT seconds
# with one diagnostic, and the hover after it is still answered.
slow=$(awk 'BEGIN { printf "fn main() -> i64 {\\n"; for (i = 0; i < 12000; i++) printf "    let a: i64 = 1;\\n"; printf "    return 0;\\n}\\n" }')
opened slow.trk "$slow" | frame > "$scratch/slow.framed"
started=$(date +%s)
session timeout "$scratch/slow.framed" TRACKC_LSP_TIMEOUT=1
check timeout_bounded '[ $(( $(date +%s) - started )) -lt 15 ]'

# The scratch dir is no longer the predictable trackc-lsp-<ms>-215583
# (monotonic clock_ms, about /proc/uptime, plus the unseeded math_random):
# directories linked in at every such name for the seconds around the
# session never receive the buffer.
victim="$scratch/victim"
mkdir "$victim"
now=$(awk '{ printf "%d", $1 * 1000 }' /proc/uptime)
awk -v n="$now" 'BEGIN { for (i = -5000; i < 10000; i++) print "trackc-lsp-" n + i "-215583" }' > "$scratch/names"
(cd "$victim" && xargs mkdir < "$scratch/names")
sed "s|^|$victim/|" "$scratch/names" | xargs ln -s -t "$TMPDIR"
opened planted.trk 'fn main() -> i64 { return 0; }' | frame > "$scratch/planted.framed"
session planted "$scratch/planted.framed"
check planted_not_followed '[ -z "$(find "$victim" -type f)" ]'
find "$TMPDIR" -maxdepth 1 -name 'trackc-lsp-*-215583' -type l -delete
rm -rf "$victim"
check scratch_removed '[ -z "$(find "$TMPDIR" -maxdepth 1 -name "trackc-lsp-*")" ]'

# No private directory can be made: one logMessage, no diagnostics, and
# requests are still answered.
opened closed.trk 'fn main() -> i64 { return 0; }' | frame > "$scratch/closed.framed"
session fail_closed "$scratch/closed.framed" TMPDIR="$scratch/missing"

# A raw NUL byte (invalid JSON) cuts its body short: a parse error, and the
# session goes on. An escaped \u0000 in a text is dropped.
nul='{"jsonrpc":"2.0","method":"textDocument/didOpen","params":{"textDocument":{"uri":"file:///nul.trk","languageId":"track","version":1,"text":"fn main() -> i64 { return 0; }'
{
    opened esc.trk 'fn main() -> i64 {\u0000 return 0; }' | sed '$d' | sed '$d' | frame
    printf 'Content-Length: %d\r\n\r\n%s\000"}}}' $((${#nul} + 5)) "$nul"
    printf '%s\n' '{"jsonrpc":"2.0","id":4,"method":"textDocument/hover","params":{"textDocument":{"uri":"file:///esc.trk"},"position":{"line":0,"character":1}}}' \
        '{"jsonrpc":"2.0","id":3,"method":"shutdown"}' '{"jsonrpc":"2.0","method":"exit"}' | frame
} > "$scratch/nul.framed"
session nul_byte "$scratch/nul.framed"

if [ "$update" = 0 ]; then
    echo "lsp sessions: $pass passed, $fail failed"
    [ "$fail" = 0 ]
fi
