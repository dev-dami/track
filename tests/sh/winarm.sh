# Windows on ARM (`--target aarch64-windows`, compiler/src/pe.trk with
# arm, ngen_a64.trk ax_import_call_win; docs/native.md "PE32+ layout"):
# structural checks only. Nothing here runs ARM64 Windows code: wine on an
# x86-64 host cannot load an ARM64 PE (CI's windows-arm job runs it).
# llvm-readobj and llvm-objdump must parse every image without a
# complaint. Commands run from the repository root.

need_llvm() {
    have llvm-readobj || skip "llvm-readobj not found"
    have llvm-objdump || skip "llvm-objdump not found"
}

t_winarm_hello_is_an_arm64_pe() {
    need_llvm
    in_repo "$TRACKC" build --target aarch64-windows examples/hello.trk -o "$T/hello.exe"
    rc_ok "build"
    eq "$(od -An -c -N 2 "$T/hello.exe" | tr -d ' ')" "MZ" "DOS magic"
    eq "$(od -An -tx1 -j 132 -N 2 "$T/hello.exe" | tr -d ' ')" "64aa" "COFF machine ARM64"
    cap llvm-readobj --file-headers --sections --coff-imports --coff-basereloc "$T/hello.exe"
    rc_ok "llvm-readobj"
    eq "$ERR" "" "llvm-readobj stderr"
    has "$OUT" "Format: COFF-ARM64" "format"
    has "$OUT" "IMAGE_FILE_MACHINE_ARM64 (0xAA64)" "machine"
    has "$OUT" "Magic: 0x20B" "PE32+"
    has "$OUT" "AddressOfEntryPoint: 0x1000" "entry at .text"
    has "$OUT" "SectionAlignment: 4096" "section alignment"
    has "$OUT" "IMAGE_DLL_CHARACTERISTICS_DYNAMIC_BASE" "dynamic base"
    has "$OUT" "IMAGE_SUBSYSTEM_WINDOWS_CUI" "console subsystem"
    has "$OUT" "Name: .reloc" "relocation section"
    has "$OUT" "Type: ABSOLUTE" "an empty relocation block"
    has "$OUT" "Name: kernel32.dll" "kernel32 imports"
    has "$OUT" "Symbol: WriteFile" "WriteFile imported"
    has "$OUT" "Symbol: ExitProcess" "ExitProcess imported"
    case "$OUT" in *RELOCS_STRIPPED*) fail "relocations marked stripped" ;; esac
}

t_winarm_examples_and_trackc_parse() {
    need_llvm
    local e s n=0
    for e in $(trk_files "$ROOT/examples") "$ROOT/compiler/src/main.trk"; do
        s=$(stem "$e")
        case "$s" in *_err) continue ;; esac
        in_repo "$TRACKC" build --target aarch64-windows "$e" -o "$T/$s.exe"
        rc_ok "$s: build"
        cap llvm-readobj --file-headers --sections --coff-imports "$T/$s.exe"
        rc_ok "$s: llvm-readobj"
        eq "$ERR" "" "$s: llvm-readobj stderr"
        has "$OUT" "IMAGE_FILE_MACHINE_ARM64" "$s: machine"
        cap llvm-objdump -d "$T/$s.exe"
        rc_ok "$s: llvm-objdump"
        eq "$ERR" "" "$s: llvm-objdump stderr"
        n=$((n + 1))
    done
    [ "$n" -ge 15 ] || fail "only $n images"
}

# Every import call is adrp x16 + ldr x16 + blr x16, and the slot the
# pair loads is one of the IAT's slots.
t_winarm_import_calls_load_iat_slots() {
    need_llvm
    have python3 || skip "python3 not found"
    in_repo "$TRACKC" build --target aarch64-windows examples/stdlib_demo.trk -o "$T/demo.exe"
    rc_ok "build"
    llvm-readobj --coff-imports "$T/demo.exe" >"$T/imp" || fail "llvm-readobj"
    llvm-objdump -d "$T/demo.exe" >"$T/dis" || fail "llvm-objdump"
    cap python3 -I - "$T/imp" "$T/dis" <<'PY'
import re, sys
imp = open(sys.argv[1]).read()
slots = {}
for block in imp.split("Import {")[1:]:
    iat = int(re.search(r"ImportAddressTableRVA: (0x[0-9A-Fa-f]+)", block).group(1), 16)
    for k, sym in enumerate(re.findall(r"Symbol: (\S+)", block)):
        slots[0x140000000 + iat + 8 * k] = sym
lines = open(sys.argv[2]).read().splitlines()
calls = 0
for i, l in enumerate(lines):
    if not re.search(r"\tblr\tx16$", l):
        continue
    a = re.search(r"\tadrp\tx16, (0x[0-9a-f]+)$", lines[i - 2])
    b = re.search(r"\tldr\tx16, \[x16, #(0x[0-9a-f]+)\]$", lines[i - 1])
    if not a or not b:
        sys.exit("blr x16 not after adrp + ldr: " + l)
    slot = int(a.group(1), 16) + int(b.group(1), 16)
    if slot not in slots:
        sys.exit("call through 0x%x, not an IAT slot" % slot)
    calls += 1
print("ok %d import calls" % calls)
PY
    rc_ok "$OUT"
    has "$OUT" "ok " "verdict"
}

t_winarm_never_touches_x18() {
    need_llvm
    in_repo "$TRACKC" build --target aarch64-windows compiler/src/main.trk -o "$T/trackc.exe"
    rc_ok "build"
    llvm-objdump -d "$T/trackc.exe" >"$T/dis" || fail "llvm-objdump"
    grep -qE '\b[xw]18\b' "$T/dis" && fail "x18 (the TEB) is used: $(grep -m3 -E '\b[xw]18\b' "$T/dis")"
    grep -q 'blr' "$T/dis" || fail "no import calls"
    return 0
}

t_winarm_build_is_deterministic() {
    in_repo "$TRACKC" build --target aarch64-windows examples/hello.trk -o "$T/1.exe"
    rc_ok "first build"
    in_repo "$TRACKC" build --target aarch64-windows examples/hello.trk -o "$T/2.exe"
    rc_ok "second build"
    same_file "$T/1.exe" "$T/2.exe" "two builds"
}
