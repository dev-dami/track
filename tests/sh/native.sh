# Native x86-64 backend (`trackc build`, compiler/src/ngen.trk). Port of
# tests/native_tests.rs: executables need no C toolchain, print their
# goldens (tests/cases/native_runs/, recorded from the retired C backend),
# and the compiler reaches a byte-identical fixed point. Commands run from
# the repository root.

t_native_build_needs_no_c_toolchain() {
    in_repo env PATH=/nonexistent "$TRACKC" build tests/native_raw/hello_syscall.trk -o "$T/hello"
    rc_ok "native build with no PATH"
    eq "$(head -c 4 "$T/hello" | od -An -c | tr -d ' ')" '177ELF' "ELF magic"
    observe "$T/hello"
    stdout_is 'Hello, native world!\n' "stdout"
    rc_is 42 "exit code"
}

t_native_programs_print_their_goldens() {
    local n=0 p s g="$ROOT/tests/cases/native_runs"
    for p in $(trk_files "$ROOT/tests/native") $(trk_files "$ROOT/bench/progs") "$ROOT/compiler/rt/tests/rt_smoke.trk"; do
        s=$(stem "$p")
        n=$((n + 1))
        [ -f "$g/$s.out" ] || fail "$s: no golden tests/cases/native_runs/$s.out"
        in_repo "$TRACKC" build "$p" -o "$T/$s"
        rc_ok "$s: build"
        is_exe "$T/$s" "$s: build"
        in_repo "$T/$s"
        out_is "$g/$s.out" "$s: output"
        rc_is "$(cat "$g/$s.code")" "$s: exit code"
        rm -f "$T/$s"
    done
    [ "$n" -gt 50 ] || fail "only $n inputs"
}

t_native_compiler_reaches_a_fixed_point() {
    in_repo "$TRACKC" build compiler/src/main.trk -o "$T/trackc2"
    rc_ok "n2 build"
    in_repo env PATH=/nonexistent "$T/trackc2" build compiler/src/main.trk -o "$T/trackc3"
    rc_ok "n3 build with no PATH"
    same_file "$T/trackc2" "$T/trackc3" "native stage2 and stage3"
}

# Every program `trackc check` rejects must be rejected by the build too.
t_native_build_rejects_what_check_rejects() {
    local n=0 p
    for p in $(trk_files "$ROOT/tests/native_reject") $(trk_files "$ROOT/examples" | grep '_err\.trk$'); do
        n=$((n + 1))
        in_repo "$TRACKC" check "$p"
        rc_fail "$p: check accepted it"
        in_repo "$TRACKC" build "$p" -o "$T/x"
        rc_fail "$p: build accepted it"
    done
    [ "$n" -ge 12 ] || fail "only $n inputs"
}

# Tuples stored in a struct, array, union or tuple, returned, or built in a
# loop outlive the frame that made them.
t_native_tuples_outlive_their_frame() {
    local name want
    while IFS='|' read -r name want; do
        in_repo "$TRACKC" build "tests/native_tuples/$name.trk" -o "$T/$name"
        rc_ok "$name: build"
        observe "$T/$name"
        rc_is 0 "$name"
        stdout_is "$want" "$name"
    done <<'CASES'
tup_arr|200\n5\n
tup_loop|0\n20\n
tup_nested|200\n6\n
tup_struct|200\n8\n
tup_struct2|30\nxy\n8\n
tup_union|200\n6\n
CASES
}

t_native_identical_string_literals_compare_equal() {
    in_repo "$TRACKC" build tests/native/strlit_eq.trk -o "$T/s"
    rc_ok "build"
    observe "$T/s"
    starts "$OUT
" "1
" "stdout"
}

t_native_program_without_main_builds() {
    local name
    for name in nomain nomain_lib; do
        in_repo "$TRACKC" build "tests/native/$name.trk" -o "$T/$name"
        rc_ok "$name: build"
        observe "$T/$name"
        rc_is 0 "$name"
        eq "$(fsize "$T/.out"):$(fsize "$T/.err")" "0:0" "$name: output"
    done
}

# `--native` (an accepted no-op) after the file and after `-o` too, with
# nothing on PATH.
t_native_flag_is_honoured_in_any_position() {
    local src=tests/native_raw/hello_syscall.trk e="$T/h" order
    for order in "build $src --native -o $e" "build $src -o $e --native" "build -o $e --native $src"; do
        rm -f "$e"
        # shellcheck disable=SC2086
        in_repo env PATH=/nonexistent "$TRACKC" $order
        rc_ok "$order"
        observe "$e"
        rc_is 42 "$order"
    done
}

# PT_GNU_STACK present with flags RW (6), not executable.
t_native_executables_have_a_non_executable_stack() {
    in_repo "$TRACKC" build tests/native_raw/hello_syscall.trk -o "$T/h"
    rc_ok "build"
    local img="$T/h" phoff entsize count i h found=
    phoff=$(u64_at "$img" 32)
    entsize=$(u16_at "$img" 54)
    count=$(u16_at "$img" 56)
    for i in $(seq 0 $((count - 1))); do
        h=$((phoff + i * entsize))
        if [ "$(u32_at "$img" "$h")" = $((0x6474e551)) ]; then
            found=$h
            break
        fi
    done
    [ -n "$found" ] || fail "no PT_GNU_STACK program header"
    eq "$(u32_at "$img" $((found + 4)))" 6 "PT_GNU_STACK flags must be RW"
}

# K3b/K3c: programs that once lost their output on a trap. Each prints the
# expected stdout and ends with the status (136 = SIGFPE).
t_native_traps_print_the_expected_output() {
    local stem want code
    while IFS='|' read -r stem want code; do
        in_repo "$TRACKC" build "tests/native/$stem.trk" -o "$T/$stem"
        rc_ok "$stem: build"
        observe "$T/$stem"
        stdout_is "$want" "$stem"
        rc_is "$code" "$stem: status"
    done <<'CASES'
for_in|10\n20\n30\n3\n2\n3\n4\n10\n20\n30\n110\n120\n130\nab\ncde\n60\n11\n21\n31\n|0
exit_flush|1\n|3
divzero_flush|1\n|136
divmin_flush|-7\n0\n-9223372036854775807\n|136
divmin_mod_flush|0\n|136
CASES
}
