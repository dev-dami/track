#!/bin/sh
# Native-race scoreboard: user-mode cycles of each bench/progs program built
# three ways (trackc --native, trackc via C/gcc -O2, the Rust compiler), with
# output checked identical, plus native self-compile time. A second table adds wall ms, page faults and
# user-vs-total (user+kernel) cycles so never-freed heap cost is visible.
# Usage: bench/native.sh [trackc]   (default $BOOTSTRAP_DIR/stage3/trackc, else build/bootstrap/stage3/trackc)
# Exit 1 on any output mismatch or build failure.
set -eu
root=$(cd "$(dirname "$0")/.." && pwd); cd "$root"
T=${1:-${BOOTSTRAP_DIR:-build/bootstrap}/stage3/trackc}
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
# probe <bin>: one perf run per sample -> median "ms faults Mucyc Mtotcyc"
# (wall ms, page faults, user-mode and user+kernel megacycles); "-" without perf.
probe() {
  command -v perf >/dev/null 2>&1 || { echo "- - - -"; return; }
  _n=${RACE_RUNS:-3}; _i=0
  while [ $_i -lt "$_n" ]; do
    _s=$(date +%s%N)
    _p=$(perf stat -x, -e page-faults,cycles:u,cycles "$@" 2>&1 >/dev/null || true)
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
rows=
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
  for k in native C rust; do
    case $k in native) b=nat ;; C) b=c ;; rust) b=rs ;; esac
    rows="$rows$(printf '%-11s %-7s %s' "$n" "$k" "$(probe "$o/$n.$b")")
"
  done
done
# honest scoreboard: wall ms, page faults, user vs user+kernel megacycles (median of runs)
printf '\n%-11s %-7s %8s %9s %9s %10s %7s\n' prog build "wall ms" faults "user Mcyc" "total Mcyc" "kern %"
printf '%s' "$rows" | awk 'NF { k = ($5 > 0 && $6 >= $5) ? sprintf("%.0f", ($6 - $5) * 100 / $6) : "-"
  printf "%-11s %-7s %8s %9s %9s %10s %7s\n", $1, $2, $3, $4, $5, $6, k }'
# native self-compile: median wall ms of 5
for i in 1 2 3 4 5; do
  s=$(date +%s%N); "$T" build --native compiler/src/main.trk -o "$o/self" >/dev/null; e=$(date +%s%N)
  echo $(( (e - s) / 1000000 ))
done | sort -n | sed -n 3p | xargs printf 'native self-compile (main.trk): %s ms\n'
exit $fail
