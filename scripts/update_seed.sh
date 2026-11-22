#!/bin/sh
# Regenerate bootstrap/seed/trackc from the current compiler/src.
#
# The seed is the static x86-64 Linux trackc that compiler/src/main.trk
# builds at its fixed point. scripts/bootstrap.sh starts from it.
#
#   scripts/update_seed.sh          # rewrite bootstrap/seed/trackc
#   scripts/update_seed.sh --check  # fail if bootstrap/seed/trackc is out of date
#
# The fixed point comes from scripts/bootstrap.sh (stage3, built from the
# current seed) unless TRACKC names a working trackc. If a change to
# compiler/src is beyond what the old seed can compile, build it in steps:
# point TRACKC at a trackc built from an intermediate compiler/src that the
# old seed can compile (e.g. an earlier commit's stage3). Scratch goes to
# $BOOTSTRAP_DIR (default build/bootstrap).
set -eu

mode=write
case "${1:-}" in
    "") ;;
    --check) mode=check ;;
    *) echo "usage: $0 [--check]" >&2; exit 2 ;;
esac

root=$(cd "$(dirname "$0")/.." && pwd)
seed="$root/bootstrap/seed/trackc"
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
    # Build twice more so the result is the fixed point whatever trackc was given.
    fp="$out/seed-fp"
    rm -rf "$fp"
    mkdir -p "$fp"
    cd "$root"
    env -i PATH=/nonexistent "$trackc" build --native compiler/src/main.trk -o "$fp/a" >/dev/null
    env -i PATH=/nonexistent "$fp/a" build --native compiler/src/main.trk -o "$fp/b" >/dev/null
    env -i PATH=/nonexistent "$fp/b" build --native compiler/src/main.trk -o "$fp/c" >/dev/null
    if ! cmp -s "$fp/b" "$fp/c"; then
        echo "update_seed: compiler/src does not reach a fixed point from $TRACKC" >&2
        exit 1
    fi
    gen="$fp/c"
else
    BOOTSTRAP_DIR="$out" "$root/scripts/bootstrap.sh" >/dev/null
    gen="$out/stage3/trackc"
fi

if [ "$mode" = check ]; then
    if cmp -s "$gen" "$seed"; then
        echo "bootstrap/seed/trackc is up to date ($(wc -c < "$seed") bytes)"
        exit 0
    fi
    echo "bootstrap/seed/trackc is stale; run scripts/update_seed.sh" >&2
    exit 1
fi
mkdir -p "$(dirname "$seed")"
cp "$gen" "$seed"
chmod +x "$seed"
echo "wrote bootstrap/seed/trackc ($(wc -c < "$seed") bytes)"
