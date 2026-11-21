#!/bin/sh
# Native-race scoreboard: user-mode cycles of each bench/progs program built
# three ways (trackc --native, trackc via C/gcc -O2, the Rust compiler), with
# output checked identical, plus native self-compile time.
# Usage: bench/native.sh [trackc]   (default build/bootstrap/stage3/trackc)
# Exit 1 on any output mismatch or build failure.
set -eu
root=$(cd "$(dirname "$0")/.." && pwd); cd "$root"
T=${1:-build/bootstrap/stage3/trackc}
case "$T" in /*) ;; *) T="$root/$T" ;; esac
R=${RUST_TRACK:-$root/target/release/track}
runs=${RACE_RUNS:-3}
o=$(mktemp -d); trap 'rm -rf "$o"' EXIT
fail=0
cyc() { # median user cycles (millions) of $runs runs
  for i in $(seq "$runs"); do
    perf stat -x, -e cycles:u "$@" 2>&1 >/dev/null | grep cycles | cut -d, -f1
  done | sort -n | awk '{a[NR]=$1} END {printf "%.0f", a[int((NR+1)/2)]/1e6}'
}
printf '%-11s %9s %9s %9s %8s %8s\n' prog native C rust "nat/C" "nat/rust"
for f in bench/progs/*.trk; do
  n=$(basename "$f" .trk)
  "$T" build --native "$f" -o "$o/$n.nat" >/dev/null 2>&1 || { echo "$n: native build failed"; fail=1; continue; }
  "$T" build "$f" -o "$o/$n.c" >/dev/null 2>&1 || { echo "$n: C build failed"; fail=1; continue; }
  (cd "$o" && "$R" build "$root/$f" >/dev/null 2>&1 && mv "$n" "$n.rs") || { echo "$n: rust build failed"; fail=1; continue; }
  a=$("$o/$n.nat" 2>&1; echo "rc=$?"); b=$("$o/$n.c" 2>&1; echo "rc=$?"); c=$("$o/$n.rs" 2>&1; echo "rc=$?")
  if [ "$a" != "$b" ] || [ "$a" != "$c" ]; then echo "$n: OUTPUT MISMATCH"; fail=1; continue; fi
  cn=$(cyc "$o/$n.nat"); cc=$(cyc "$o/$n.c"); cr=$(cyc "$o/$n.rs")
  printf '%-11s %8sM %8sM %8sM %8s %8s\n' "$n" "$cn" "$cc" "$cr" \
    "$(awk "BEGIN{printf \"%.2f\", $cn/$cc}")" "$(awk "BEGIN{printf \"%.2f\", $cn/$cr}")"
done
# native self-compile: median wall ms of 5
for i in 1 2 3 4 5; do
  s=$(date +%s%N); "$T" build --native compiler/src/main.trk -o "$o/self" >/dev/null; e=$(date +%s%N)
  echo $(( (e - s) / 1000000 ))
done | sort -n | sed -n 3p | xargs printf 'native self-compile (main.trk): %s ms\n'
exit $fail
