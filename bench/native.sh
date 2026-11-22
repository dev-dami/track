#!/bin/sh
# Native scoreboard: each bench/progs program built by trackc, its output
# checked against tests/cases/native_runs/<prog>.out, then measured: user-mode
# cycles, wall ms, page faults and user-vs-total (user+kernel) cycles, so
# never-freed heap cost is visible. Ends with the self-compile time.
# Usage: bench/native.sh [trackc]   (default $BOOTSTRAP_DIR/stage3/trackc, else build/bootstrap/stage3/trackc)
# Needs perf for the cycle columns ("-" without it). Exit 1 on any output
# mismatch or build failure.
set -eu
root=$(cd "$(dirname "$0")/.." && pwd); cd "$root"
T=${1:-${BOOTSTRAP_DIR:-build/bootstrap}/stage3/trackc}
case "$T" in /*) ;; *) T="$root/$T" ;; esac
runs=${RACE_RUNS:-3}
o=$(mktemp -d); trap 'rm -rf "$o"' EXIT
fail=0
# probe <bin>: one perf run per sample -> median "ms faults Mucyc Mtotcyc"
# (wall ms, page faults, user-mode and user+kernel megacycles); perf columns
# are 0 without perf.
probe() {
  _i=0
  while [ $_i -lt "$runs" ]; do
    _s=$(date +%s%N)
    if command -v perf >/dev/null 2>&1; then
      _p=$(perf stat -x, -e page-faults,cycles:u,cycles "$@" 2>&1 >/dev/null || true)
    else
      "$@" >/dev/null 2>&1 || true; _p=
    fi
    _e=$(date +%s%N)
    echo "$(( (_e - _s) / 1000000 )) $(echo "$_p" | awk -F, '
      $3 ~ /^page-faults/ { f = $1 } $3 ~ /^cycles:u/ { u = $1 } $3 ~ /^cycles(:|$)/ && $3 !~ /:u/ { t = $1 }
      END { printf "%s %s %s", f + 0, int(u / 1e6), int(t / 1e6) }')"
    _i=$((_i + 1))
  done | awk '{ for (k = 1; k <= 4; k++) v[k, NR] = $k; n = NR }
    END { for (k = 1; k <= 4; k++) {
            for (i = 1; i <= n; i++) x[i] = v[k, i] + 0
            for (i = 1; i <= n; i++) for (j = i + 1; j <= n; j++) if (x[j] < x[i]) { t = x[i]; x[i] = x[j]; x[j] = t }
            printf "%s%s", x[int((n + 1) / 2)], (k < 4 ? " " : "\n") } }'
}
printf '%-11s %8s %9s %9s %10s %7s\n' prog "wall ms" faults "user Mcyc" "total Mcyc" "kern %"
for f in bench/progs/*.trk; do
  n=$(basename "$f" .trk)
  "$T" build "$f" -o "$o/$n" >/dev/null 2>&1 || { echo "$n: build failed"; fail=1; continue; }
  "$o/$n" >"$o/$n.out" 2>/dev/null || true
  if ! cmp -s "$o/$n.out" "tests/cases/native_runs/$n.out"; then echo "$n: OUTPUT MISMATCH"; fail=1; continue; fi
  probe "$o/$n" | awk -v n="$n" '{ k = ($3 > 0 && $4 >= $3) ? sprintf("%.0f", ($4 - $3) * 100 / $4) : "-"
    printf "%-11s %8s %9s %9s %10s %7s\n", n, $1, $2, $3, $4, k }'
done
# self-compile: median wall ms of 5
for i in 1 2 3 4 5; do
  s=$(date +%s%N); "$T" build compiler/src/main.trk -o "$o/self" >/dev/null; e=$(date +%s%N)
  echo $(( (e - s) / 1000000 ))
done | sort -n | sed -n 3p | xargs printf 'self-compile (main.trk): %s ms\n'
exit $fail
