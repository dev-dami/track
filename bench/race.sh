#!/bin/sh
# Race scoreboard: trackc (self-hosted) vs the Rust compiler on the same work.
# Usage: bench/race.sh [trackc]   (default: build/bootstrap/stage3/trackc)
# Needs target/release/{track,yard}. Prints median of 5 wall-clock runs (ms).
set -eu
root=$(cd "$(dirname "$0")/.." && pwd); cd "$root"
T=${1:-build/bootstrap/stage3/trackc}
R=target/release/track; Y=target/release/yard
med() { # med <cmd...> -> median ms of 5 runs
  for i in 1 2 3 4 5; do
    s=$(date +%s%N); "$@" >/dev/null 2>&1 || true; e=$(date +%s%N)
    echo $(( (e - s) / 1000000 ))
  done | sort -n | sed -n 3p
}
o=$(mktemp -d)
printf '%-34s %8s %8s\n' "task" "trackc" "rust"
printf '%-34s %8s %8s\n' "check compiler/src/main.trk" "$(med $T check compiler/src/main.trk)" "$(med $R check compiler/src/main.trk)"
printf '%-34s %8s %8s\n' "build compiler (end to end)" "$(med $T build compiler/src/main.trk -o $o/t)" "$(cd compiler && med sh -c "rm -rf target && ../$Y build")"
for f in bench/progs/*.trk; do
  [ -f "$f" ] || continue; n=$(basename "$f" .trk)
  $T build "$f" -o "$o/$n.tk" >/dev/null 2>&1 || { echo "$n: trackc build failed"; continue; }
  (cd "$o" && "$root/$R" build "$root/$f" >/dev/null 2>&1 && mv "$n" "$n.rs") || true
  printf '%-34s %8s %8s\n' "run $n" "$(med $o/$n.tk)" "$( [ -x $o/$n.rs ] && med $o/$n.rs || echo n/a)"
done
rm -rf "$o"
