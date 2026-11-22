# Self-hosted compiler (compiler/), part 1. Port of tests/selfhost_tests.rs
# up to trackc_programs_print_the_expected_output. The Rust suite compared
# trackc with the Rust stage0 compiler; here trackc is checked alone, against
# the same expectations (stage0 behaviour of examples/ is frozen in
# tests/cases/examples/<stem>.out and .code). Case tables live in
# tests/cases/<table>/.

CASES="$ROOT/tests/cases"

t_build_creates_missing_output_directories() {
    write "$T/x.trk" "fn main() -> void { print(7); }"
    trackc build "$T/x.trk" -o "$T/some/new/dir/x"
    rc_ok "build into a missing directory"
    observe "$T/some/new/dir/x"
    eq "$OUT" 7 "output"
}

t_builds_stay_identical_in_any_directory() {
    write "$T/a.trk" "fn main() -> void { print(1); }"
    cap "$TRACKC" build "$T/a.trk" -o "$T/out1/a"
    rc_ok "build 1"
    cap_in "$T/out1" "$TRACKC" build "$T/a.trk" -o "$T/out2/a"
    rc_ok "build 2"
    same_file "$T/out1/a" "$T/out2/a" "builds"
    observe "$T/out2/a"
    eq "$OUT" 1 "output"
}

t_native_binaries_carry_a_symbol_table() {
    have nm || skip "nm not found"
    # recursive, so the inliner keeps it out of line
    write "$T/s.trk" 'fn sum_down(n: i64) -> i64 {
    if n <= 0 {
        return 0;
    }
    return n + sum_down(n - 1);
}
fn main() -> void { print(sum_down(100)); }
'
    trackc build -g "$T/s.trk" -o "$T/s"
    rc_ok "build -g"
    observe "$T/s"
    eq "$OUT" 5050 "output"
    # without -g: the same loaded image, no sections
    trackc build "$T/s.trk" -o "$T/plain"
    rc_ok "build"
    local lw lp
    lw=$(fsize "$T/s")
    lp=$(fsize "$T/plain")
    [ "$lw" -gt "$lp" ] || fail "-g image ($lw) not larger than plain ($lp)"
    cmp -s -i 64 -n 112 "$T/s" "$T/plain" || fail "program headers differ"
    cmp -s -i 176 -n $((lp - 176)) "$T/s" "$T/plain" || fail "loaded bytes differ"
    cap nm "$T/s"
    rc_ok "nm"
    local name
    for name in main sum_down _start; do
        printf '%s\n' "$OUT" | awk -v n="$name" 'NF == 3 && $2 == "T" && $3 == n { f = 1 } END { exit !f }' \
            || fail "nm lacks T $name:
$OUT"
    done
}

t_bootstrap_reaches_a_fixed_point() {
    mkdir -p "$T/stage2" "$T/stage3"
    in_repo "$TRACKC" build compiler/src/main.trk -o "$T/stage2/trackc"
    rc_ok "stage1"
    in_repo "$T/stage2/trackc" build compiler/src/main.trk -o "$T/stage3/trackc"
    rc_ok "stage2"
    same_file "$T/stage2/trackc" "$T/stage3/trackc" "stage2 and stage3 executables"
}

# Stage0 behaviour is frozen in tests/cases/examples (captured from the Rust
# compiler): trackc rejects every _err example and reproduces the rest.
t_examples_behave_the_same_under_both_compilers() {
    local p s hb n=0
    for p in $(trk_files "$ROOT/examples"); do
        s=$(stem "$p")
        n=$((n + 1))
        trackc build "$p" -o "$T/${s}_native"
        case "$s" in
            *_err)
                rc_fail "$s: trackc accepted a rejection test"
                continue
                ;;
        esac
        rc_ok "$s: trackc build"
        [ -f "$CASES/examples/$s.out" ] || fail "$s: no golden tests/cases/examples/$s.out"
        observe "$T/${s}_native"
        # @HOSTNAME_BYTES@ stands for the size of /etc/hostname (0 if it is
        # missing), which error_handling prints and which differs per machine.
        hb=0
        [ -f /etc/hostname ] && hb=$(wc -c </etc/hostname | tr -d ' ')
        sed "s/@HOSTNAME_BYTES@/$hb/" "$CASES/examples/$s.out" >"$T/.want_$s"
        out_is "$T/.want_$s" "$s: behaviour differs from stage0"
        rc_is "$(cat "$CASES/examples/$s.code")" "$s: exit code differs from stage0"
        rm -f "$T/${s}_native"
    done
    [ "$n" -gt 0 ] || fail "no examples"
}

# verdict_dir DIR: DIR/accept/*.trk must check clean, DIR/reject/*.trk not.
verdict_dir() {
    local f n=0
    for f in $(trk_files "$1/accept"); do
        n=$((n + 1))
        trackc check "$f"
        [ "$RC" = 0 ] || fail "$(stem "$f"): trackc rejected an accepted program
$(ctx)"
    done
    for f in $(trk_files "$1/reject"); do
        n=$((n + 1))
        trackc check "$f"
        [ "$RC" != 0 ] || fail "$(stem "$f"): trackc accepted a rejected program
$(ctx)"
    done
    echo "$n"
}

t_checkers_agree_on_accepted_and_rejected_programs() {
    local n
    n=$(verdict_dir "$CASES/verdicts") || exit 1
    eq "$n" 34 "verdict count"
}

# A constant `/` or `%` that would trap is E0200; trackc must not fold it.
t_const_divrem_traps_are_e0200_in_both_checkers() {
    local f s n=0
    for f in $(trk_files "$CASES/const_divrem"); do
        s=$(stem "$f")
        n=$((n + 1))
        trackc check "$f"
        if [ -f "$CASES/const_divrem/$s.want" ]; then
            rc_is 1 "$s"
            starts "$ERR" "$(cat "$CASES/const_divrem/$s.want")" "$s"
        else
            rc_ok "$s"
        fi
    done
    eq "$n" 19 "case count"
}

t_diagnostics_point_at_the_source() {
    write "$T/moved.trk" 'fn main() -> void {
    let s = str_concat("a", "b");
    let t = s;
    print(s);
}
'
    trackc_in "$T" check moved.trk
    rc_fail "check"
    starts "$ERR" "error[E0200]: Use-after-free! Resource 's' is already spent.
 --> moved.trk:4:11
" "diagnostic"
    has "$ERR" "4 |     print(s);" "source frame"
}

t_native_test_suite_passes_under_trackc() {
    local p rel n=0
    for p in $(trk_files "$ROOT/compiler/tests"); do
        rel=tests/${p##*/}
        n=$((n + 1))
        cap_in "$ROOT/compiler" "$TRACKC" run "$rel"
        rc_ok "$rel failed under trackc"
    done
    [ "$n" -gt 0 ] || fail "no compiler/tests"
}

t_program_without_main_runs() {
    write "$T/nomain.trk" 'let x = 41;
print(x + 1);
'
    build "$T/nomain.trk" "$T/nomain"
    observe "$T/nomain"
    stdout_is '42\n' "stdout"
    rc_ok "exit"
}

# Warnings channel: two unused bindings, sorted by position; only
# --deny-warnings fails.
t_warnings_print_sorted_and_only_deny_warnings_fails() {
    write "$T/w.trk" 'fn helper() -> i64 { let a = 1; return 2; }
fn main() -> void { let b = 3; print(helper()); }
'
    local path="$T/w.trk" args
    for args in "check $path" "build $path" "build --native $path"; do
        # shellcheck disable=SC2086
        trackc_in "$T" $args
        rc_ok "$args"
        eq "$(count "$ERR" "warning[W0301]")" 2 "$args: W0301 count"
        [ "$(pos "$ERR" "w.trk:1:")" -lt "$(pos "$ERR" "w.trk:2:")" ] \
            || fail "$args: warnings not sorted by position:
$ERR"
    done
    trackc_in "$T" check "$path"
    has "$OUT" "(2 warnings)" "check summary"

    trackc_in "$T" run "$path"
    rc_is 0 "run"
    eq "$OUT" 2 "run output"
    has "$ERR" "warning[W0301]" "run warnings"

    for args in "check --deny-warnings $path" "check $path --deny-warnings" \
        "build --native $path --deny-warnings" "run --deny-warnings $path"; do
        # shellcheck disable=SC2086
        trackc_in "$T" $args
        rc_is 1 "$args"
        has "$ERR" "warning[W0301]" "$args"
    done

    write "$T/quiet.trk" 'fn helper() -> i64 { let _a = 1; return 2; }
fn main() -> void { print(helper()); }
'
    trackc_in "$T" check --deny-warnings "$T/quiet.trk"
    rc_ok "quiet --deny-warnings"
    lacks "$ERR" "warning" "quiet"

    write "$T/bad.trk" 'fn first() -> i64 { let u = 1; return 0; }
fn main() -> void {
    print(nope);
}
'
    trackc_in "$T" check "$T/bad.trk"
    rc_is 1 "warning plus error"
}

# Lifecycle warnings W0301-W0305: each docs/lifecycles/wN example gives
# exactly its warning, exits 0, and fails under --deny-warnings.
t_lifecycle_warnings_match_the_catalogue() {
    local stem want rel f
    while IFS='|' read -r stem want; do
        rel=docs/lifecycles/$stem.trk
        in_repo "$TRACKC" check "$rel"
        rc_ok "$rel"
        has "$ERR" "$want" "$rel"
        eq "$(count "$ERR" "warning[")" 1 "$rel: warning count"
        in_repo "$TRACKC" check --deny-warnings "$rel"
        rc_is 1 "$rel --deny-warnings"
    done <<'CASES'
w1_fresh_unread|warning[W0301]: 'origin' is created and never read; remove it or name it '_origin'.
w2_write_only|warning[W0302]: write to 'p.y' is never read; 'p' ends at line 7.
w3_mutated_unread|warning[W0303]: 'bump' mutates 'c', but 'c' is never read afterwards.
w4_discarded_result|warning[W0304]: result of 'vadd' is discarded, and 'vadd' has no other effect (it only reads its parameters and returns a fresh value).
w5_consume_only_reads|warning[W0305]: 'norm1' takes 'p' by value but only reads it; take '&Point' so callers keep it.
CASES
    # Near misses stay warning-free.
    local n=0
    for f in $(trk_files "$CASES/life_warn_clean"); do
        n=$((n + 1))
        trackc check "$f"
        rc_ok "$(stem "$f")"
        lacks "$ERR" "warning[" "$(stem "$f")"
    done
    eq "$n" 6 "near-miss count"
}

# examples/ is warning-free except two intended demonstrations.
t_examples_warn_only_where_intended() {
    local p name want
    for p in $(trk_files "$ROOT/examples"); do
        name=${p##*/}
        case "$name" in
            linear_auto_free.trk) want="warning[W0301]: 'buf'" ;;
            structs_arrays.trk) want="warning[W0305]: 'quadrant'" ;;
            *) want= ;;
        esac
        in_repo "$TRACKC" check "examples/$name"
        if [ -n "$want" ]; then
            eq "$(count "$ERR" "warning[")" 1 "examples/$name: warning count"
            has "$ERR" "$want" "examples/$name"
        else
            eq "$(count "$ERR" "warning[")" 0 "examples/$name: warning count"
        fi
    done
}

t_compiler_sources_are_warning_clean() {
    in_repo "$TRACKC" check --deny-warnings compiler/src/main.trk
    rc_ok "check --deny-warnings compiler/src/main.trk"
}

# Holes where the Rust checker is also wrong (or differs): trackc alone.
t_trackc_closes_checker_holes() {
    local n
    n=$(verdict_dir "$CASES/trackc_verdicts") || exit 1
    eq "$n" 46 "verdict count"
}

# Lifecycle hard errors: exit 1 and the exact first diagnostic line. A case
# is NN.trk (inline source) or NN.path (a repo path), with NN.want.
t_lifecycle_hard_errors_print_the_catalogue_texts() {
    local w base src n=0
    for w in "$CASES"/life_errors/*.want; do
        base=${w%.want}
        if [ -f "$base.path" ]; then
            src="$ROOT/$(cat "$base.path")"
        else
            src="$base.trk"
        fi
        n=$((n + 1))
        trackc_in "$T" check "$src"
        rc_is 1 "$src"
        eq "$(head -n 1 "$T/.err")" "$(cat "$w")" "$src: first diagnostic line"
    done
    eq "$n" 19 "case count"
}

# Look-alikes of the lifecycle hard errors that are correct.
t_lifecycle_lookalikes_check_clean() {
    local f n=0
    for f in $(trk_files "$CASES/life_clean"); do
        n=$((n + 1))
        trackc_in "$T" check "$f"
        rc_ok "$(stem "$f") rejected"
    done
    eq "$n" 10 "case count"
}

# A tuple literal that never escapes lives in a stack slot: 20M of them in a
# loop allocate nothing, while one that is returned stays a heap cell.
t_non_escaping_tuples_use_stack_slots() {
    write "$T/t3.trk" 'fn pair(n: i64) -> (i64, i64) { let r = (n, n + 1); return r; } fn main() -> void { let mut i = 0; let mut acc = 0; while i < 20000000 { let p = (i, 1); acc = acc + p.1; i = i + 1; } print(acc); print(pair(4).1); }'
    trackc_in "$T" build "$T/t3.trk" -o "$T/t3"
    rc_ok "build"
    trackc_in "$T" life --place "$T/t3.trk"
    rc_ok "life --place"
    eq "$OUT" "fn pair
  L1 tuple heap ends -
fn main
  L1 tuple stack ends L1
  L1 call heap ends -" "placement: loop tuple on the stack, returned tuple on the heap"
    # The native runtime reserves its heap up front, so an address-space
    # limit cannot tell the two apart; the placement plan above is the check.
    observe "$T/t3"
    rc_ok "t3"
    stdout_is '20000000\n5\n' "t3"
}

t_trackc_programs_print_the_expected_output() {
    local f s n=0
    for f in $(trk_files "$CASES/run_outputs"); do
        s=$(stem "$f")
        n=$((n + 1))
        trackc_in "$T" run "$f"
        rc_ok "$s: trackc run"
        out_is "$CASES/run_outputs/$s.out" "$s: wrong output"
    done
    eq "$n" 10 "case count"
}
