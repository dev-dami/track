#!/bin/sh
# Compare `trackc build --native` against the C path on example programs.
#
#   scripts/native_parity.sh <trackc> [file.trk...]
#
# Default inputs: examples/*.trk (minus *_err) and tests/native/*.trk.
# Prints SAME / DIFF / BUILD per program (stdout, stderr and exit status
# must match) and exits non-zero on any mismatch.
set -u
trackc=$1
shift
root=$(cd "$(dirname "$0")/.." && pwd)
work=${NATIVE_PARITY_DIR:-/tmp/native_parity}
mkdir -p "$work/c" "$work/n"
cd "$root"
if [ $# -eq 0 ]; then
    set -- examples/*.trk tests/native/*.trk
fi
fail=0
for f in "$@"; do
    n=$(basename "$f" .trk)
    case $n in *_err) continue ;; esac
    if ! "$trackc" build --native "$f" -o "$work/n/$n" >"$work/n/$n.log" 2>&1; then
        echo "BUILD $n: $(head -c 300 "$work/n/$n.log")"
        fail=1
        continue
    fi
    "$trackc" build "$f" -o "$work/c/$n" >/dev/null 2>&1 || { echo "CBUILD $n"; continue; }
    a=$(cd "$work" && timeout 10 "$work/c/$n" 2>&1; echo "rc=$?")
    b=$(cd "$work" && timeout 10 "$work/n/$n" 2>&1; echo "rc=$?")
    if [ "$a" = "$b" ]; then
        echo "SAME  $n"
    else
        echo "DIFF  $n"
        fail=1
    fi
done
exit $fail
