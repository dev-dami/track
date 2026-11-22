#!/bin/sh
# C-path parity for card L8: build every tests/**/*.trk, bench/progs and
# examples program with a given trackc and record build status, program
# output and exit code, one file per program.
#   tests/stack/parity.sh <trackc> <outdir>
# Then compare two runs with `diff -r <outdir-a> <outdir-b>`.
set -u
tc=$1
out=$2
root=$(cd "$(dirname "$0")/../.." && pwd)
mkdir -p "$out/bin"
cd "$root"
find tests bench/progs examples -name '*.trk' | sort | while read -r f; do
    key=$(echo "${f%.trk}" | tr '/' '_')
    exe="$out/bin/$key"
    {
        if "$tc" build "$f" -o "$exe" >/dev/null 2>"$out/bin/$key.err"; then
            echo "build ok"
            timeout 20 "$exe" </dev/null 2>&1
            echo "exit $?"
        else
            echo "build failed"
            cat "$out/bin/$key.err"
        fi
    } >"$out/$key.txt"
    rm -f "$exe" "$exe.c" "$out/bin/$key.err"
done
ls "$out"/*.txt | wc -l
