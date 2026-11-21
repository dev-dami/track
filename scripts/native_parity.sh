#!/bin/sh
# Compare `trackc build --native` against the C path on example programs.
#
#   scripts/native_parity.sh <trackc> [file.trk...]
#
# Default inputs: examples/*.trk (minus *_err) and tests/native/*.trk.
# Prints SAME / DIFF / BUILD / CBUILD per program (stdout, stderr and exit
# status must match) and exits non-zero on any mismatch or failed build.
# Each run may take up to $NATIVE_PARITY_TIMEOUT seconds (default 300) so a
# loaded machine does not turn slow programs into false DIFFs.
set -u
trackc=$1
shift
root=$(cd "$(dirname "$0")/.." && pwd)
work=${NATIVE_PARITY_DIR:-/tmp/native_parity}
limit=${NATIVE_PARITY_TIMEOUT:-300}
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
    if ! "$trackc" build "$f" -o "$work/c/$n" >"$work/c/$n.log" 2>&1; then
        echo "CBUILD $n: $(head -c 300 "$work/c/$n.log")"
        fail=1
        continue
    fi
    a=$(cd "$work" && timeout "$limit" "$work/c/$n" 2>&1; echo "rc=$?")
    b=$(cd "$work" && timeout "$limit" "$work/n/$n" 2>&1; echo "rc=$?")
    if [ "$a" = "$b" ]; then
        echo "SAME  $n"
    else
        echo "DIFF  $n"
        fail=1
    fi
done
exit $fail
