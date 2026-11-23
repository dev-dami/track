#!/bin/sh
# Regenerate every committed seed from the current compiler/src.
#
#   bootstrap/seed/trackc      the static x86-64 Linux trackc that
#                              compiler/src/main.trk builds at its fixed
#                              point; scripts/bootstrap.sh starts from it.
#   bootstrap/seed/trackc.exe  the x86_64-windows trackc, cross-compiled by
#                              that fixed point (--target x86_64-windows);
#                              scripts/bootstrap.ps1 starts from it. It is a
#                              fixed point too: trackc.exe builds itself
#                              byte for byte.
#
#   scripts/update_seed.sh          # rewrite both seeds
#   scripts/update_seed.sh --check  # fail if either seed is out of date
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
wseed="$root/bootstrap/seed/trackc.exe"
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

# The Windows seed: the fixed point cross-compiles compiler/src/main.trk.
(cd "$root" && env -i PATH=/nonexistent "$gen" build --target x86_64-windows compiler/src/main.trk -o "$out/trackc.exe" >/dev/null)
wgen="$out/trackc.exe"

if [ "$mode" = check ]; then
    status=0
    if cmp -s "$gen" "$seed"; then
        echo "bootstrap/seed/trackc is up to date ($(wc -c < "$seed") bytes)"
    else
        echo "bootstrap/seed/trackc is stale; run scripts/update_seed.sh" >&2
        status=1
    fi
    if cmp -s "$wgen" "$wseed"; then
        echo "bootstrap/seed/trackc.exe is up to date ($(wc -c < "$wseed") bytes)"
    else
        echo "bootstrap/seed/trackc.exe is stale; run scripts/update_seed.sh" >&2
        status=1
    fi
    exit "$status"
fi
mkdir -p "$(dirname "$seed")"
cp "$gen" "$seed"
chmod +x "$seed"
echo "wrote bootstrap/seed/trackc ($(wc -c < "$seed") bytes)"
cp "$wgen" "$wseed"
echo "wrote bootstrap/seed/trackc.exe ($(wc -c < "$wseed") bytes)"
