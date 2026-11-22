#!/bin/sh
# Bootstrap the self-hosted Track compiler and verify the fixed point.
#
#   stage 0  cc builds the committed C seed (bootstrap/seed/) -> stage1 trackc
#            (or, with --from-rust / BOOTSTRAP_FROM=rust, the Rust compiler
#            `yard` builds compiler/ -> stage1 trackc)
#   stage 1  stage1 compiles compiler/src/main.trk     -> stage2 trackc
#   stage 2  stage2 compiles compiler/src/main.trk     -> stage3 trackc
#   native   scripts/selfhost.sh with stage3 as its seed: n2 = n3 = n4
#
# The gate: stage2 and stage3 C are byte-identical, the stage2 and stage3
# executables are byte-identical, and the native stages reproduce
# themselves. The default path needs only a C compiler ($CC, default cc)
# and POSIX tools: no Rust, no cargo.
#
#   scripts/bootstrap.sh               # from the C seed
#   scripts/bootstrap.sh --from-rust   # from the Rust stage0 (cargo / $YARD)
#   scripts/bootstrap.sh --no-native   # skip the native fixed point
#
# Artifacts go to build/bootstrap/ (override with BOOTSTRAP_DIR); the native
# stages go to $BOOTSTRAP_DIR/selfhost unless SELFHOST_DIR is set. Set YARD
# to use a specific yard binary on the Rust path.
set -eu

from=${BOOTSTRAP_FROM:-seed}
native=1
for arg in "$@"; do
    case "$arg" in
        --from-rust) from=rust ;;
        --from-seed) from=seed ;;
        --no-native) native=0 ;;
        -h|--help) sed -n '2,23p' "$0" | sed 's/^# \{0,1\}//'; exit 0 ;;
        *) echo "usage: $0 [--from-rust | --from-seed] [--no-native]" >&2; exit 2 ;;
    esac
done
case "$from" in
    seed|rust) ;;
    *) echo "bootstrap: BOOTSTRAP_FROM must be 'seed' or 'rust', not '$from'" >&2; exit 2 ;;
esac

root=$(cd "$(dirname "$0")/.." && pwd)
out=${BOOTSTRAP_DIR:-"$root/build/bootstrap"}
case "$out" in
    /*) ;;
    *) out="$(pwd)/$out" ;;
esac
seed="$root/bootstrap/seed"
cc=${CC:-cc}
# The flags trackc itself passes to the C compiler (cc_flags in main.trk).
cflags="-std=gnu99 -O2 -fwrapv -fno-strict-aliasing -fno-builtin -w"

if [ "$from" = rust ]; then
    yard=${YARD:-}
    if [ -z "$yard" ]; then
        (cd "$root" && cargo build --release --quiet)
        yard="${CARGO_TARGET_DIR:-$root/target}/release/yard"
    fi
    # The stages run from other directories, so pin an absolute path.
    case "$yard" in
        /*) ;;
        *) yard="$(pwd)/$yard" ;;
    esac
fi

"$root/scripts/embed_runtime.sh" --check
"$root/scripts/embed_rt.sh" --check

rm -rf "$out"
mkdir -p "$out/stage1" "$out/stage2" "$out/stage3"
# A private runtime-object cache per run: every stage links the runtime the
# same way, and concurrent bootstraps cannot race on a shared cache.
TRACKC_CACHE="$out/cache"
export TRACKC_CACHE

if [ "$from" = seed ]; then
    for f in trackc.c trackc.rt.c; do
        [ -f "$seed/$f" ] || { echo "bootstrap: missing bootstrap/seed/$f" >&2; exit 1; }
    done
    command -v "$cc" >/dev/null 2>&1 || { echo "bootstrap: C compiler '$cc' not found" >&2; exit 1; }
    echo "==> stage 0: $cc builds the C seed (bootstrap/seed/, $(wc -l < "$seed/trackc.c") lines)"
    cp "$seed/trackc.c" "$seed/trackc.rt.c" "$out/stage1/"
    # Compile inside the stage directory with trackc's own base names, the
    # same way trackc links its output.
    (cd "$out/stage1" && $cc $cflags -o trackc trackc.c trackc.rt.c)
else
    echo "==> stage 0: yard builds compiler/ (Rust bootstrap compiler)"
    (cd "$root/compiler" && "$yard" build >/dev/null)
    cp "$root/compiler/target/trackc" "$out/stage1/trackc"
fi

# Every stage compiles the same sources from the repository root, so the
# loader resolves identical module paths at every stage.
cd "$root"
echo "==> stage 1: stage1 compiles compiler/src/main.trk"
"$out/stage1/trackc" build compiler/src/main.trk -o "$out/stage2/trackc" >/dev/null
echo "==> stage 2: stage2 compiles compiler/src/main.trk"
"$out/stage2/trackc" build compiler/src/main.trk -o "$out/stage3/trackc" >/dev/null

status=0
if cmp -s "$out/stage2/trackc.c" "$out/stage3/trackc.c"; then
    echo "✓ stage1 and stage2 emit identical C ($(wc -l < "$out/stage2/trackc.c") lines)"
else
    echo "✗ stage1 and stage2 emitted different C" >&2
    status=1
fi
if cmp -s "$out/stage2/trackc" "$out/stage3/trackc"; then
    echo "✓ stage2 and stage3 executables are byte-identical ($(wc -c < "$out/stage2/trackc") bytes)"
else
    echo "✗ stage2 and stage3 executables differ" >&2
    status=1
fi
# The seed only has to be able to build the current compiler; a stale seed
# is reported here and gated by scripts/update_seed.sh --check.
if cmp -s "$out/stage3/trackc.c" "$seed/trackc.c" && cmp -s "$out/stage3/trackc.rt.c" "$seed/trackc.rt.c"; then
    echo "✓ bootstrap/seed/ matches the fixed-point C"
else
    echo "! bootstrap/seed/ differs from the fixed-point C; run scripts/update_seed.sh"
fi
"$out/stage3/trackc" --version
[ "$status" -eq 0 ] || exit "$status"

if [ "$native" -eq 1 ]; then
    echo "==> native: scripts/selfhost.sh, seeded by stage3"
    SELFHOST_DIR=${SELFHOST_DIR:-"$out/selfhost"} BOOTSTRAP_DIR="$out" \
        "$root/scripts/selfhost.sh" "$out/stage3/trackc"
fi
