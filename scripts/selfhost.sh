#!/bin/sh
# Track builds Track from .trk alone.
#
#   seed     any trackc (default: the bootstrap's stage3 under $BOOTSTRAP_DIR, built once via
#            scripts/bootstrap.sh) compiles compiler/src/main.trk with
#            --native -> n1. This is the only step that uses a prior binary.
#   n1..n4   each native trackc compiles compiler/src/main.trk --native
#            under `env -i PATH=/nonexistent`: no C compiler, assembler,
#            linker, or any other program is reachable.
#
# The gate: n2, n3 and n4 are byte-identical static ELF executables.
# Usage: scripts/selfhost.sh [seed-trackc]   (artifacts in build/selfhost/)
set -eu

root=$(cd "$(dirname "$0")/.." && pwd)
cd "$root"
bs=${BOOTSTRAP_DIR:-build/bootstrap}
seed=${1:-$bs/stage3/trackc}
out=${SELFHOST_DIR:-"$root/build/selfhost"}

if [ ! -x "$seed" ]; then
    echo "==> no seed trackc at $seed; running scripts/bootstrap.sh once"
    scripts/bootstrap.sh >/dev/null
    seed=$bs/stage3/trackc
fi

rm -rf "$out"
mkdir -p "$out"
ms() { echo $(( ($2 - $1) / 1000000 )); }

printf '%-6s %-42s %9s %8s  %s\n' stage "built by" bytes ms sha256
row() {
    printf '%-6s %-42s %9s %8s  %s\n' "$1" "$2" "$(wc -c < "$out/$1")" "$3" \
        "$(sha256sum "$out/$1" | cut -c1-16)"
}

t0=$(date +%s%N)
"$seed" build --native compiler/src/main.trk -o "$out/n1" >/dev/null
t1=$(date +%s%N)
row n1 "seed ($seed)" "$(ms "$t0" "$t1")"

prev=n1
for s in n2 n3 n4; do
    t0=$(date +%s%N)
    env -i PATH=/nonexistent "$out/$prev" build --native compiler/src/main.trk -o "$out/$s" >/dev/null
    t1=$(date +%s%N)
    row "$s" "$prev, env -i PATH=/nonexistent" "$(ms "$t0" "$t1")"
    prev=$s
done

status=0
if cmp -s "$out/n2" "$out/n3" && cmp -s "$out/n3" "$out/n4"; then
    echo "✓ n2 = n3 = n4: Track reproduces itself from .trk with no C toolchain"
else
    echo "✗ native stages differ" >&2
    status=1
fi
if command -v strace >/dev/null 2>&1; then
    n=$(strace -f -e trace=execve,fork,vfork,clone -o "$out/trace" \
        env -i PATH=/nonexistent "$out/n4" build --native compiler/src/main.trk -o "$out/n5" >/dev/null 2>&1;
        grep -c -E 'execve|fork|clone' "$out/trace" || true)
    echo "✓ processes started by n4 while compiling itself: $((n - 2)) (strace)"
fi
if command -v file >/dev/null 2>&1; then
    echo "  $(file -b "$out/n4" | cut -d, -f1-3)"
fi
exit $status
