#!/bin/sh
# Regenerate bootstrap/seed/ from the current compiler/src.
#
# The seed is the C that trackc emits for compiler/src/main.trk at its fixed
# point (trackc.c) plus the C runtime it writes beside it (trackc.rt.c).
# scripts/bootstrap.sh builds it with plain cc, so Track bootstraps with no
# Rust toolchain.
#
#   scripts/update_seed.sh          # rewrite bootstrap/seed/
#   scripts/update_seed.sh --check  # fail if bootstrap/seed/ is out of date
#
# The fixed point comes from scripts/bootstrap.sh --no-native (stage3, built
# from the current seed) unless TRACKC names a working trackc. If a change
# to compiler/src is beyond what the old seed can compile, use
# TRACKC=build/bootstrap/stage3/trackc from `scripts/bootstrap.sh
# --from-rust`, or any other trackc. Scratch goes to $BOOTSTRAP_DIR
# (default build/bootstrap).
set -eu

mode=write
case "${1:-}" in
    "") ;;
    --check) mode=check ;;
    *) echo "usage: $0 [--check]" >&2; exit 2 ;;
esac

root=$(cd "$(dirname "$0")/.." && pwd)
seed="$root/bootstrap/seed"
out=${BOOTSTRAP_DIR:-"$root/build/bootstrap"}
case "$out" in
    /*) ;;
    *) out="$(pwd)/$out" ;;
esac

if [ -n "${TRACKC:-}" ]; then
    case "$TRACKC" in
        /*) trackc=$TRACKC ;;
        *) trackc="$(pwd)/$TRACKC" ;;
    esac
    # Build twice so the result is the fixed point whatever trackc was given.
    fp="$out/seed-fp"
    rm -rf "$fp"
    mkdir -p "$fp/a" "$fp/b"
    TRACKC_CACHE="$fp/cache"
    export TRACKC_CACHE
    cd "$root"
    "$trackc" build compiler/src/main.trk -o "$fp/a/trackc" >/dev/null
    "$fp/a/trackc" build compiler/src/main.trk -o "$fp/b/trackc" >/dev/null
    if ! cmp -s "$fp/a/trackc.c" "$fp/b/trackc.c"; then
        echo "update_seed: compiler/src does not reach a fixed point from $TRACKC" >&2
        exit 1
    fi
    gen="$fp/b"
else
    BOOTSTRAP_DIR="$out" "$root/scripts/bootstrap.sh" --no-native >/dev/null
    gen="$out/stage3"
fi

stale=0
for f in trackc.c trackc.rt.c; do
    cmp -s "$gen/$f" "$seed/$f" || stale=1
done

if [ "$mode" = check ]; then
    if [ "$stale" -eq 0 ]; then
        echo "bootstrap/seed/ is up to date ($(wc -l < "$seed/trackc.c") lines)"
        exit 0
    fi
    echo "bootstrap/seed/ is stale; run scripts/update_seed.sh" >&2
    exit 1
fi
mkdir -p "$seed"
cp "$gen/trackc.c" "$gen/trackc.rt.c" "$seed/"
echo "wrote bootstrap/seed/trackc.c ($(wc -c < "$seed/trackc.c") bytes) and trackc.rt.c ($(wc -c < "$seed/trackc.rt.c") bytes)"
