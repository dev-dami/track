#!/bin/sh
# Race scoreboard: the C-built trackc (stage3) vs the native trackc on the
# same work. The native trackc is trackc's own `--native` build of
# compiler/src/main.trk; programs are built through C by the first and with
# --native by the second.
# Usage: bench/race.sh [trackc]   (default: build/bootstrap/stage3/trackc)
# Env: NATIVE_TRACKC names a native trackc (default: built here from trackc);
#      RACE_SKIP_COMPILER=1 skips the compiler rows; RACE_RUNS sets the run
#      count (default 5).
# Prints the median wall-clock ms; ratio = trackc/native (below 1.00 = C-built wins).
# Run rows also show user-mode megacycles of one run (perf stat) and their
# ratio, cratio: the steadier number when the machine is loaded.
# A second table lists wall ms, page faults and user vs user+kernel megacycles.
# Every bench/progs/*.trk program is built both ways and both
# binaries must print identical output, else the row says MISMATCH and the
# script exits non-zero at the end.
set -eu
root=$(cd "$(dirname "$0")/.." && pwd); cd "$root"
abs() { case "$1" in /*) echo "$1" ;; *) echo "$root/$1" ;; esac; }
T=$(abs "${1:-build/bootstrap/stage3/trackc}")
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
# probe <bin>: one perf run per sample -> median "ms faults Mucyc Mtotcyc"
# (wall ms, page faults, user-mode and user+kernel megacycles); "-" without perf.
probe() {
  command -v perf >/dev/null 2>&1 || { echo "- - - -"; return; }
  _n=${RACE_RUNS:-5}; _i=0
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
o=$(mktemp -d); trap 'rm -rf "$o"' EXIT
if [ -n "${NATIVE_TRACKC:-}" ]; then
  R=$(abs "$NATIVE_TRACKC")
else
  R="$o/trackc-native"
  "$T" build --native compiler/src/main.trk -o "$R" >/dev/null
fi
fail=0
rows=
printf '%-30s %8s %8s %6s %8s %8s %6s  %s\n' "task" "trackc" "native" "ratio" "t Mcyc" "r Mcyc" "cratio" "output"
if [ "${RACE_SKIP_COMPILER:-0}" != 1 ]; then
  row "check compiler/src/main.trk" "$(med "$T" check compiler/src/main.trk)" "$(med "$R" check compiler/src/main.trk)"
  row "build compiler (end to end)" "$(med "$T" build compiler/src/main.trk -o "$o/t")" "$(med "$R" build --native compiler/src/main.trk -o "$o/n")"
fi
for f in bench/progs/*.trk; do
  [ -f "$f" ] || continue; n=$(basename "$f" .trk)
  "$T" build "$f" -o "$o/$n.tk" >/dev/null 2>&1 || { row "run $n" fail - "trackc build failed"; fail=1; continue; }
  "$R" build --native "$f" -o "$o/$n.nat" >/dev/null 2>&1 || { row "run $n" - fail "native build failed"; fail=1; continue; }
  "$o/$n.tk" >"$o/$n.out.tk" 2>&1 || true
  "$o/$n.nat" >"$o/$n.out.nat" 2>&1 || true
  if cmp -s "$o/$n.out.tk" "$o/$n.out.nat"; then
    chk="same ($(wc -l <"$o/$n.out.tk" | tr -d ' ') lines)"
  else
    chk="MISMATCH"; fail=1
  fi
  rows="$rows$(printf '%-11s %-7s %s' "$n" trackc "$(probe "$o/$n.tk")")
$(printf '%-11s %-7s %s' "$n" native "$(probe "$o/$n.nat")")
"
  row "run $n" "$(med "$o/$n.tk")" "$(med "$o/$n.nat")" "$chk" "$(cyc "$o/$n.tk")" "$(cyc "$o/$n.nat")"
done
# honest scoreboard: wall ms, page faults, user vs user+kernel megacycles (median of runs)
printf '\n%-11s %-7s %8s %9s %9s %10s %7s\n' prog build "wall ms" faults "user Mcyc" "total Mcyc" "kern %"
printf '%s' "$rows" | awk 'NF { k = ($5 > 0 && $6 >= $5) ? sprintf("%.0f", ($6 - $5) * 100 / $6) : "-"
  printf "%-11s %-7s %8s %9s %9s %10s %7s\n", $1, $2, $3, $4, $5, $6, k }'
exit $fail
