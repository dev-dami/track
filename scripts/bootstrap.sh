#!/bin/sh
# Bootstrap the self-hosted Track compiler and verify the fixed point.
#
#   seed     bootstrap/seed/trackc, a committed static x86-64 Linux ELF
#            (the fixed-point trackc), compiles compiler/src/main.trk -> stage1
#   stage1   compiles compiler/src/main.trk                           -> stage2
#   stage2   compiles compiler/src/main.trk                           -> stage3
#
# Every stage runs under `env -i PATH=/nonexistent`: no C compiler,
# assembler, linker or any other program is reachable. The gate: stage2 and
# stage3 are byte-identical. Needs only POSIX sh, cmp and wc on x86-64 Linux.
#
#   scripts/bootstrap.sh
#   (--no-native and --from-seed are accepted for old callers; no-ops)
#
# Artifacts go to build/bootstrap/ (override with BOOTSTRAP_DIR); the
# result is build/bootstrap/stage3/trackc.
set -eu

for arg in "$@"; do
    case "$arg" in
        --from-seed|--no-native) ;;
        -h|--help) sed -n '2,17p' "$0" | sed 's/^# \{0,1\}//'; exit 0 ;;
        *) echo "usage: $0" >&2; exit 2 ;;
    esac
done

root=$(cd "$(dirname "$0")/.." && pwd)
out=${BOOTSTRAP_DIR:-"$root/build/bootstrap"}
case "$out" in
    /*) ;;
    *) out="$(pwd)/$out" ;;
esac
seed="$root/bootstrap/seed/trackc"

[ -f "$seed" ] || { echo "bootstrap: missing bootstrap/seed/trackc" >&2; exit 1; }
rm -rf "$out"
mkdir -p "$out/seed" "$out/stage1" "$out/stage2" "$out/stage3"
# A fresh, executable copy: git may check the seed out without its mode bit.
cp "$seed" "$out/seed/trackc"
chmod +x "$out/seed/trackc"

# Every stage compiles the same sources from the repository root, so the
# loader resolves identical module paths at every stage.
cd "$root"
prev="$out/seed/trackc"
for s in stage1 stage2 stage3; do
    echo "==> $s: $(basename "$(dirname "$prev")") compiles compiler/src/main.trk"
    env -i PATH=/nonexistent "$prev" build --native compiler/src/main.trk -o "$out/$s/trackc" >/dev/null
    prev="$out/$s/trackc"
done

status=0
if cmp -s "$out/stage2/trackc" "$out/stage3/trackc"; then
    echo "✓ stage2 and stage3 are byte-identical ($(wc -c < "$out/stage3/trackc") bytes)"
else
    echo "✗ stage2 and stage3 differ" >&2
    status=1
fi
# The seed only has to be able to build the current compiler; a stale seed
# is reported here and gated by scripts/update_seed.sh --check.
if cmp -s "$out/stage3/trackc" "$seed"; then
    echo "✓ bootstrap/seed/trackc matches the fixed point"
else
    echo "! bootstrap/seed/trackc differs from the fixed point; run scripts/update_seed.sh"
fi
"$out/stage3/trackc" --version
exit "$status"
