#!/bin/sh
# Race scoreboard: trackc (self-hosted) vs the Rust compiler on the same work.
# Usage: bench/race.sh [trackc]   (default: build/bootstrap/stage3/trackc)
# Env: RUST_TRACK, YARD override target/release/{track,yard}; RACE_SKIP_COMPILER=1
#      skips the compiler-build rows; RACE_RUNS sets the run count (default 5).
# Prints the median wall-clock ms; ratio = trackc/rust (below 1.00 = trackc wins).
# Run rows also show user-mode megacycles of one run (perf stat) and their
# ratio, cratio: the steadier number when the machine is loaded.
# Every bench/progs/*.trk program is built with both compilers and both
# binaries must print identical output, else the row says MISMATCH and the
# script exits non-zero at the end.
set -eu
root=$(cd "$(dirname "$0")/.." && pwd); cd "$root"
abs() { case "$1" in /*) echo "$1" ;; *) echo "$root/$1" ;; esac; }
T=$(abs "${1:-build/bootstrap/stage3/trackc}")
R=$(abs "${RUST_TRACK:-target/release/track}")
Y=$(abs "${YARD:-target/release/yard}")
N=${RACE_RUNS:-5}
med() { # med <cmd...> -> median ms of N runs
  i=0
  while [ $i -lt "$N" ]; do
    s=$(date +%s%N); "$@" >/dev/null 2>&1 || true; e=$(date +%s%N)
    echo $(( (e - s) / 1000000 )); i=$((i + 1))
  done | sort -n | sed -n "$(( (N + 1) / 2 ))p"
}
ratio() { # ratio <a> <b> -> a/b with two decimals
  case "$1$2" in *[!0-9]*|'') echo "-"; return ;; esac
  [ "$2" -gt 0 ] || { echo "-"; return; }
  r=$(( $1 * 100 / $2 )); printf '%d.%02d' $((r / 100)) $((r % 100))
}
cyc() { # cyc <bin> -> user-mode megacycles of one run (needs perf), else -
  command -v perf >/dev/null 2>&1 || { echo -; return; }
  perf stat -x, -e cycles:u "$1" 2>&1 >/dev/null |
    awk -F, '/cycles/ { c = $1 } END { if (c ~ /^[0-9]+$/) printf "%d", c / 1000000; else print "-" }'
}
row() { # row <task> <t ms> <r ms> [note] [t Mcyc] [r Mcyc]
  printf '%-30s %8s %8s %6s %8s %8s %6s  %s\n' "$1" "$2" "$3" "$(ratio "$2" "$3")" \
    "${5:--}" "${6:--}" "$(ratio "${5:--}" "${6:--}")" "${4:-}"
}
o=$(mktemp -d); trap 'rm -rf "$o"' EXIT
fail=0
printf '%-30s %8s %8s %6s %8s %8s %6s  %s\n' "task" "trackc" "rust" "ratio" "t Mcyc" "r Mcyc" "cratio" "output"
if [ "${RACE_SKIP_COMPILER:-0}" != 1 ]; then
  row "check compiler/src/main.trk" "$(med "$T" check compiler/src/main.trk)" "$(med "$R" check compiler/src/main.trk)"
  row "build compiler (end to end)" "$(med "$T" build compiler/src/main.trk -o "$o/t")" "$(cd compiler && med sh -c "rm -rf target && '$Y' build")"
fi
for f in bench/progs/*.trk; do
  [ -f "$f" ] || continue; n=$(basename "$f" .trk)
  "$T" build "$f" -o "$o/$n.tk" >/dev/null 2>&1 || { row "run $n" fail - "trackc build failed"; fail=1; continue; }
  # The Rust compiler writes ./<name> in the current directory.
  (cd "$o" && "$R" build "$root/$f" >/dev/null 2>&1 && mv "$n" "$n.rs") || { row "run $n" - fail "rust build failed"; fail=1; continue; }
  "$o/$n.tk" >"$o/$n.out.tk" 2>&1 || true
  "$o/$n.rs" >"$o/$n.out.rs" 2>&1 || true
  if cmp -s "$o/$n.out.tk" "$o/$n.out.rs"; then
    chk="same ($(wc -l <"$o/$n.out.tk" | tr -d ' ') lines)"
  else
    chk="MISMATCH"; fail=1
  fi
  row "run $n" "$(med "$o/$n.tk")" "$(med "$o/$n.rs")" "$chk" "$(cyc "$o/$n.tk")" "$(cyc "$o/$n.rs")"
done
exit $fail
