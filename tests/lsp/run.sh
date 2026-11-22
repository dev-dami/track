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
# line) plus the exit status with NAME.out.
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
for case_in in "$here"/cases/*.in; do
    name=$(basename "$case_in" .in)
    expected="$here/cases/$name.out"
    actual="$scratch/$name.out"
    status=0
    (cd "$here/cases" && frame < "$case_in" | timeout 60 "$trackc" lsp > "$scratch/$name.raw") || status=$?
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
done
if [ "$update" = 0 ]; then
    echo "lsp sessions: $pass passed, $fail failed"
    [ "$fail" = 0 ]
fi
