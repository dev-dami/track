#!/bin/sh
# Bootstrap the self-hosted Track compiler and verify the fixed point.
#
#   stage 0  the Rust compiler (`yard`) builds compiler/ -> stage1 trackc
#   stage 1  stage1 compiles compiler/src/main.trk     -> stage2 trackc
#   stage 2  stage2 compiles compiler/src/main.trk     -> stage3 trackc
#
# The gate: stage1 and stage2 emit byte-identical C, and the stage2 and
# stage3 executables are byte-identical. Artifacts go to build/bootstrap/
# (override with BOOTSTRAP_DIR). Set YARD to use a specific yard binary.
set -eu

root=$(cd "$(dirname "$0")/.." && pwd)
out=${BOOTSTRAP_DIR:-"$root/build/bootstrap"}
yard=${YARD:-}

if [ -z "$yard" ]; then
    (cd "$root" && cargo build --release --quiet)
    yard="$root/target/release/yard"
fi

"$root/scripts/embed_runtime.sh" --check

rm -rf "$out"
mkdir -p "$out/stage1" "$out/stage2" "$out/stage3"

echo "==> stage 0: yard builds compiler/ (Rust bootstrap compiler)"
(cd "$root/compiler" && "$yard" build >/dev/null)
cp "$root/compiler/target/trackc" "$out/stage1/trackc"

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
"$out/stage3/trackc" --version
exit $status
