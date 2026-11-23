# macOS targets (`--target x86_64-macos` / `aarch64-macos`, compiler/src/
# macho.trk, rt/os/macos.trk; docs/native.md "Mach-O layout"): structural
# checks only, nothing here runs Mac code (CI's macos jobs do).
# tests/tools/macho_check.py (independent of macho.trk) re-parses every
# image and recomputes its code signature hashes; llvm-objdump, when
# present, must parse headers, binds and code without a complaint.
# Commands run from the repository root.

need_py() {
    have python3 || skip "python3 not found"
}

macho_check() {
    cap python3 "$ROOT/tests/tools/macho_check.py" "$@"
}

t_macos_hello_is_a_signed_macho() {
    need_py
    local a want
    for a in x86_64 aarch64; do
        in_repo "$TRACKC" build --target $a-macos examples/hello.trk -o "$T/hello_$a"
        rc_ok "$a: build"
        eq "$(od -An -tx1 -N 4 "$T/hello_$a" | tr -d ' ')" "cffaedfe" "$a: MH_MAGIC_64"
        want=07000001
        [ $a = aarch64 ] && want=0c000001
        eq "$(od -An -tx1 -j 4 -N 4 "$T/hello_$a" | tr -d ' ')" "$want" "$a: cputype"
        macho_check "$T/hello_$a"
        rc_ok "$a: macho_check"
        has "$OUT" "ok " "$a: checker verdict"
    done
}

t_macos_examples_and_trackc_pass_the_checker() {
    need_py
    local a e s n=0
    for a in x86_64 aarch64; do
        for e in $(trk_files "$ROOT/examples") "$ROOT/compiler/src/main.trk"; do
            s=$(stem "$e")
            case "$s" in *_err) continue ;; esac
            in_repo "$TRACKC" build --target $a-macos "$e" -o "$T/${a}_$s"
            rc_ok "$a $s: build"
            n=$((n + 1))
        done
    done
    macho_check "$T"/x86_64_* "$T"/aarch64_*
    rc_ok "macho_check: $OUT"
    [ "$n" -ge 30 ] || fail "only $n images"
}

t_macos_objdump_parses_headers_binds_and_code() {
    have llvm-objdump || skip "llvm-objdump not found"
    local a
    for a in x86_64 aarch64; do
        in_repo "$TRACKC" build --target $a-macos examples/stdlib_demo.trk -o "$T/demo_$a"
        rc_ok "$a: build"
        cap llvm-objdump --macho --private-headers --bind --lazy-bind -d "$T/demo_$a"
        rc_ok "$a: llvm-objdump"
        eq "$ERR" "" "$a: llvm-objdump stderr"
        has "$OUT" "LC_CODE_SIGNATURE" "$a: code signature command"
        has "$OUT" "/usr/lib/libSystem.B.dylib" "$a: libSystem"
        has "$OUT" "__got" "$a: GOT"
        has "$OUT" "libSystem        _write" "$a: write bound from libSystem"
        has "$OUT" "symbol address: _write" "$a: a call through the _write slot"
    done
}

t_macos_checker_catches_a_changed_page() {
    need_py
    in_repo "$TRACKC" build --target x86_64-macos examples/hello.trk -o "$T/h"
    rc_ok "build"
    printf '\220' | dd of="$T/h" bs=1 seek=20000 conv=notrunc 2>/dev/null
    macho_check "$T/h"
    rc_is 1 "a changed page fails"
    has "$OUT" "hash mismatch" "message"
}

t_macos_build_is_deterministic() {
    # the signature names the file, so both builds use the same name
    in_repo "$TRACKC" build --target aarch64-macos examples/hello.trk -o "$T/1/hello"
    rc_ok "first build"
    in_repo "$TRACKC" build --target aarch64-macos examples/hello.trk -o "$T/2/hello"
    rc_ok "second build"
    same_file "$T/1/hello" "$T/2/hello" "two builds"
}

t_macos_extern_binds_the_as_symbol() {
    need_py
    cat >"$T/ext.trk" <<'EOF2'
extern "libSystem.B.dylib" fn my_pid() -> i32 as "getpid";
fn main() -> i64 { print(my_pid() > 0); return 0; }
EOF2
    trackc check "$T/ext.trk"
    rc_ok "check"
    trackc build --target x86_64-macos "$T/ext.trk" -o "$T/ext"
    rc_ok "build"
    grep -q '_getpid' "$T/ext" || fail "no _getpid symbol"
    grep -q 'my_pid' "$T/ext" && fail "the Track name leaked into the image"
    macho_check "$T/ext"
    rc_ok "macho_check"
}
