# aarch64 Linux output (`trackc build --arch aarch64`, compiler/src/ngen_a64.trk
# and a64.trk): executables are cross-compiled here and run under
# qemu-aarch64 user emulation; every test skips when qemu is absent.
# Commands run from the repository root.

qemu_a64() {
    if have qemu-aarch64-static; then
        QEMU=$(command -v qemu-aarch64-static)
    elif have qemu-aarch64; then
        QEMU=$(command -v qemu-aarch64)
    else
        skip "no qemu-aarch64"
    fi
}

t_arm64_hello_runs_under_qemu() {
    qemu_a64
    in_repo "$TRACKC" build --arch aarch64 examples/hello.trk -o "$T/hello"
    rc_ok "aarch64 build"
    eq "$(u16_at "$T/hello" 18)" 183 "e_machine EM_AARCH64"
    in_repo "$QEMU" "$T/hello"
    out_is "$ROOT/tests/cases/examples/hello.out" "hello output"
    rc_is 0 "exit code"
}

t_arm64_programs_print_their_goldens() {
    qemu_a64
    local n=0 p s g hb=0
    [ -f /etc/hostname ] && hb=$(wc -c </etc/hostname | tr -d ' ')
    for p in $(trk_files "$ROOT/examples") $(trk_files "$ROOT/tests/native") $(trk_files "$ROOT/bench/progs"); do
        s=$(stem "$p")
        case "$s" in
            *_err) continue ;;
            rt_lcache) continue ;; # minutes under emulation; same code paths as rt_large
        esac
        g="$ROOT/tests/cases/native_runs"
        case "$p" in "$ROOT/examples/"*) g="$ROOT/tests/cases/examples" ;; esac
        n=$((n + 1))
        in_repo "$TRACKC" build --arch aarch64 "$p" -o "$T/$s"
        rc_ok "$s: aarch64 build"
        in_repo "$QEMU" "$T/$s"
        sed "s/@HOSTNAME_BYTES@/$hb/" "$g/$s.out" >"$T/.want_$s"
        out_is "$T/.want_$s" "$s: output"
        rc_is "$(cat "$g/$s.code")" "$s: exit code"
        rm -f "$T/$s"
    done
    [ "$n" -gt 50 ] || fail "only $n inputs"
}

t_arm64_compiler_reaches_a_fixed_point_under_qemu() {
    qemu_a64
    in_repo "$TRACKC" build --arch aarch64 compiler/src/main.trk -o "$T/trackc2"
    rc_ok "aarch64 stage2 build"
    in_repo env PATH=/nonexistent "$QEMU" "$T/trackc2" build --arch aarch64 compiler/src/main.trk -o "$T/trackc3"
    rc_ok "stage3 build under qemu"
    same_file "$T/trackc2" "$T/trackc3" "aarch64 stage2 and stage3"
    in_repo "$QEMU" "$T/trackc2" build compiler/src/main.trk -o "$T/trackc_x86"
    rc_ok "x86-64 build under qemu"
    in_repo "$TRACKC" build compiler/src/main.trk -o "$T/trackc_ref"
    same_file "$T/trackc_x86" "$T/trackc_ref" "x86-64 output of the aarch64 compiler"
}

t_arm64_x86_64_is_the_default_arch() {
    in_repo "$TRACKC" build examples/hello.trk -o "$T/a"
    rc_ok "default build"
    in_repo "$TRACKC" build --arch x86_64 examples/hello.trk -o "$T/b"
    rc_ok "--arch x86_64 build"
    same_file "$T/a" "$T/b" "--arch x86_64 output"
    eq "$(u16_at "$T/a" 18)" 62 "e_machine EM_X86_64"
}

t_arm64_unknown_arch_is_rejected() {
    in_repo "$TRACKC" build --arch sparc examples/hello.trk -o "$T/x"
    rc_is 2 "unknown --arch exit code"
    [ ! -e "$T/x" ] || fail "an executable was written for an unknown arch"
}
