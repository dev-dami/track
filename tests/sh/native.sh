# Native x86-64 backend (`trackc build --native`, compiler/src/ngen.trk).
# Port of tests/native_tests.rs: native executables need no C toolchain,
# behave like the C backend's, and the natively built compiler reaches a
# byte-identical fixed point. Commands run from the repository root.

t_native_build_needs_no_c_toolchain() {
    in_repo env PATH=/nonexistent "$TRACKC" build --native tests/native_raw/hello_syscall.trk -o "$T/hello"
    rc_ok "native build with no PATH"
    eq "$(head -c 4 "$T/hello" | od -An -c | tr -d ' ')" '177ELF' "ELF magic"
    observe "$T/hello"
    stdout_is 'Hello, native world!\n' "stdout"
    rc_is 42 "exit code"
}

t_native_programs_match_the_c_backend() {
    local n=0 p s
    for p in $(trk_files "$ROOT/examples") $(trk_files "$ROOT/tests/native") $(trk_files "$ROOT/bench/progs"); do
        s=$(stem "$p")
        case "$s" in *_err) continue ;; esac
        n=$((n + 1))
        in_repo "$TRACKC" build "$p" -o "$T/${s}_c"
        rc_ok "$s: C build"
        in_repo "$TRACKC" build --native "$p" -o "$T/${s}_n"
        rc_ok "$s: native build"
        is_exe "$T/${s}_c" "$s: C build"
        is_exe "$T/${s}_n" "$s: native build"
        in_repo "$T/${s}_c"; save_run c
        in_repo "$T/${s}_n"; save_run n
        same_run c n "$s: behaviour differs"
        rm -f "$T/${s}_c" "$T/${s}_n"
    done
    [ "$n" -gt 10 ] || fail "only $n inputs"
}

t_native_compiler_reaches_a_fixed_point() {
    in_repo "$TRACKC" build --native compiler/src/main.trk -o "$T/trackc2"
    rc_ok "n2 build"
    in_repo env PATH=/nonexistent "$T/trackc2" build --native compiler/src/main.trk -o "$T/trackc3"
    rc_ok "n3 build with no PATH"
    same_file "$T/trackc2" "$T/trackc3" "native stage2 and stage3"
}

# Every program `trackc check` rejects must be rejected by the native build
# and by `--rt=track` too.
t_native_and_track_rt_reject_what_check_rejects() {
    local n=0 p
    for p in $(trk_files "$ROOT/tests/native_reject") $(trk_files "$ROOT/examples" | grep '_err\.trk$'); do
        n=$((n + 1))
        in_repo "$TRACKC" check "$p"
        rc_fail "$p: check accepted it"
        in_repo "$TRACKC" build --native "$p" -o "$T/x"
        rc_fail "$p: --native accepted it"
        in_repo "$TRACKC" build --rt=track "$p" -o "$T/x"
        rc_fail "$p: --rt=track accepted it"
    done
    [ "$n" -ge 12 ] || fail "only $n inputs"
}

# Tuples stored in a struct, array, union or tuple, returned, or built in a
# loop outlive the frame that made them.
t_native_tuples_outlive_their_frame() {
    local name want
    while IFS='|' read -r name want; do
        in_repo "$TRACKC" build --native "tests/native_tuples/$name.trk" -o "$T/$name"
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
    in_repo "$TRACKC" build --native tests/native/strlit_eq.trk -o "$T/s"
    rc_ok "build"
    observe "$T/s"
    starts "$OUT
" "1
" "stdout"
}

t_native_program_without_main_builds() {
    local name
    for name in nomain nomain_lib; do
        in_repo "$TRACKC" build --native "tests/native/$name.trk" -o "$T/$name"
        rc_ok "$name: build"
        observe "$T/$name"
        rc_is 0 "$name"
        eq "$(fsize "$T/.out"):$(fsize "$T/.err")" "0:0" "$name: output"
    done
}

# `--native` after the file and after `-o` too; with no C compiler on PATH
# only the native backend can succeed.
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
    in_repo "$TRACKC" build --native tests/native_raw/hello_syscall.trk -o "$T/h"
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

# K3b/K3c: programs both backends once got wrong the same way. Each prints
# the expected stdout and ends with the status (136 = SIGFPE) on both.
t_native_and_c_print_the_expected_output() {
    local stem want code
    while IFS='|' read -r stem want code; do
        in_repo "$TRACKC" build "tests/native/$stem.trk" -o "$T/${stem}_c"
        rc_ok "$stem c: build"
        in_repo "$TRACKC" build --native "tests/native/$stem.trk" -o "$T/${stem}_native"
        rc_ok "$stem native: build"
        for exe in "$T/${stem}_c" "$T/${stem}_native"; do
            observe "$exe"
            stdout_is "$want" "${exe##*/}"
            rc_is "$code" "${exe##*/}: status"
        done
    done <<'CASES'
for_in|10\n20\n30\n3\n2\n3\n4\n10\n20\n30\n110\n120\n130\nab\ncde\n60\n11\n21\n31\n|0
exit_flush|1\n|3
divzero_flush|1\n|136
divmin_flush|-7\n0\n-9223372036854775807\n|136
divmin_mod_flush|0\n|136
CASES
}
