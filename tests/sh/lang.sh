# Language-level tests ported from the Rust stage0 suites
# (integration_tests.rs, codegen_semantics_tests.rs, regression_tests.rs,
# soundness_tests.rs, logic_and_soundness_strict_tests.rs, checker_tests.rs,
# lexer_tests.rs, parser_tests.rs, performance_tests.rs). The Rust tests
# drove stage0's library; these drive trackc with the same sources and the
# same (or stricter) expectations. Data lives in tests/cases/lang/.

LANG_CASES="$ROOT/tests/cases/lang"

# verdict_table FILE: every row `name<TAB>rc<TAB>prefix<TAB>source` must get
# exit `rc` from `trackc check`; prefix "-" means stderr has no error line,
# else stderr's first line starts with prefix. With PARSE=1, stderr must
# also hold no lexer (E0001) or parser (E01xx) diagnostic.
verdict_table() {
    local name rc prefix src n=0
    while IFS=$'\t' read -r name rc prefix src; do
        case "$name" in '#'* | '') continue ;; esac
        n=$((n + 1))
        printf '%s' "$src" >"$T/$name.trk"
        trackc_in "$T" check "$name.trk"
        rc_is "$rc" "$name"
        if [ "$prefix" = - ]; then
            lacks "$ERR" "error[" "$name"
        else
            starts "$ERR" "$prefix" "$name"
        fi
        if [ "${PARSE:-0}" = 1 ]; then
            lacks "$ERR" "error[E0001]" "$name: lexer error"
            lacks "$ERR" "error[E01" "$name: parse error"
        fi
    done <"$1"
    [ "$n" -gt 0 ] || fail "no rows in $1"
    note "$n verdicts"
}

t_checker_verdicts() { verdict_table "$LANG_CASES/verdicts.tsv"; }

t_parser_sources_parse() { PARSE=1 verdict_table "$LANG_CASES/parses.tsv"; }

# parse_bitwise_vs_logical_precedence: `a | b || c & d` is
# `(a | b) || (c & d)`; with a=2, b=c=d=0 only that grouping prints 1
# (`a | ((b || c) & d)` gives 2, `((a | b) || c) & d` gives 0).
t_bitwise_vs_logical_or_precedence() {
    write "$T/p.trk" 'fn main() -> void { let a = 2; let b = 0; let c = 0; let d = 0; let res = a | b || c & d; print(res); }'
    build "$T/p.trk" "$T/p"
    observe "$T/p"
    rc_ok "run"
    stdout_is '1\n' "precedence"
}

# lexer_tests.rs: `trackc tokens` dumps match goldens that list the same
# token kinds the Rust tests asserted.
t_lexer_token_dumps() {
    local s
    for s in lex_keywords lex_types lex_literals lex_operators lex_comments; do
        trackc tokens "$LANG_CASES/$s.trk"
        rc_ok "$s"
        out_is "$LANG_CASES/$s.tokens" "$s"
    done
}

t_lexer_invalid_character() {
    write "$T/e.trk" 'let x = #;'
    trackc_in "$T" tokens e.trk
    rc_is 1 "tokens"
    starts "$ERR" "error[E0001]: unexpected character
 --> e.trk:1:9" "diagnostic"
    trackc_in "$T" check e.trk
    rc_is 1 "check"
    starts "$ERR" "error[E0001]: unexpected character" "check diagnostic"
}

# integration_tests.rs ------------------------------------------------------

t_valid_examples_compile_and_run() {
    local e
    for e in hello arithmetic borrow linear_auto_free macro_test union_enum_test use_test; do
        [ -f "$ROOT/examples/$e.trk" ] || fail "example not found: $e"
        build "$ROOT/examples/$e.trk" "$T/$e"
        cap_in "$ROOT/examples" "$T/$e"
        rc_ok "$e: run"
    done
}

t_memory_boundary_exceeded_crash() {
    build "$LANG_CASES/mem_test.trk" "$T/m"
    observe "$T/m"
    rc_fail "memory boundary exceeded program should fail"
    has "$ERR" "Process memory boundary limit exceeded" "stderr"
}

t_invalid_examples_rejection() {
    local e want
    while read -r e want; do
        trackc check "$ROOT/examples/$e.trk"
        rc_is 1 "$e"
        has "$ERR" "$want" "$e"
    done <<'CASES'
borrow_lock_err frozen
escape_err escapes
manual_free_err managed automatically
CASES
}

t_for_in_loop_execution() {
    build "$LANG_CASES/for_test.trk" "$T/f"
    observe "$T/f"
    rc_ok "for loop"
    stdout_is '0\n1\n2\n' "for loop"
}

t_explicit_error_handling_convention() {
    build "$LANG_CASES/error_handling.trk" "$T/e"
    observe "$T/e"
    rc_ok "run"
    local first second
    first=$(sed -n 1p "$T/.out")
    second=$(sed -n 2p "$T/.out")
    if [ -e /etc/hostname ]; then
        [ -n "$first" ] && [ "$first" != -1 ] || fail "want a real size for /etc/hostname, got [$first]"
    fi
    eq "$second" 7 "fallback for the missing file"
    has "$ERR" "missing input" "stderr"
}

t_logical_not_operator() {
    build "$LANG_CASES/lnot.trk" "$T/l"
    observe "$T/l"
    rc_ok "run"
    if [ -e /etc/hostname ]; then stdout_is '42\n' "logical not"; fi
}

t_const_resolution_across_functions() {
    build "$LANG_CASES/consts.trk" "$T/c"
    observe "$T/c"
    rc_ok "run"
    stdout_is '100\n123\n' "consts"
}

t_error_predicates_disambiguate_sentinels() {
    build "$LANG_CASES/pred.trk" "$T/p"
    observe "$T/p"
    rc_ok "run"
    stdout_is '42\n111\n222\n333\n' "predicates"
}

t_generics_monomorphization() {
    build "$LANG_CASES/gen.trk" "$T/g"
    observe "$T/g"
    rc_ok "run"
    stdout_is '5\n10\n1\n2\n1\n2\n' "generics"
}

# codegen_semantics_tests.rs ------------------------------------------------

t_executables_are_position_independent() {
    write "$T/pie.trk" 'fn main() -> i64 { print(25988585); return 0; }
'
    build "$T/pie.trk" "$T/pie"
    eq "$(u16_at "$T/pie" 16)" 3 "ELF type must be ET_DYN (PIE)"
    observe "$T/pie"
    stdout_is '25988585\n' "stdout"
}

t_for_in_binds_array_elements() {
    build "$ROOT/tests/native/for_in.trk" "$T/f"
    observe "$T/f"
    rc_ok "run"
    stdout_is '10\n20\n30\n3\n2\n3\n4\n10\n20\n30\n110\n120\n130\nab\ncde\n60\n11\n21\n31\n' "for_in"
}

t_program_arguments_reach_os_args() {
    build "$LANG_CASES/argv.trk" "$T/a"
    observe "$T/a" hello
    rc_ok "run"
    stdout_is '2\nhello\n' "argv"
}

# sem_case STEM WANT: build tests/cases/lang/STEM.trk, run, exit 0, stdout.
sem_case() {
    build "$LANG_CASES/$1.trk" "$T/$1"
    observe "$T/$1"
    rc_ok "$1: run"
    stdout_is "$2" "$1"
}

t_logical_operators_short_circuit_and_normalise() { sem_case logic '10\n0\n3\n13\n1\n'; }
t_return_inside_match_while_and_nested_if() { sem_case returns '10\n20\n8\n7\n8\n9\n5\n6\n'; }
t_if_and_match_produce_values() { sem_case values '-1\n0\n1\n200\n300\n'; }
t_returned_tuple_variables_do_not_dangle() { sem_case tuples '5\n6\n9\n'; }
t_string_patterns_compare_contents() { sem_case strpat '1\n'; }
t_union_payloads_and_enum_discriminants() { sem_case unions '42\n1001\n0\n5\n6\n10\n'; }
t_constants_fold_comparisons_and_variants() { sem_case constfold '1\n1\n2\n12\n1\n7\n'; }
t_negative_runtime_results_keep_their_sign() { sem_case signs '-1\n98\n'; }
t_block_bindings_do_not_leak_into_the_enclosing_scope() { sem_case scopes '20\n10\n1\n10\n'; }
t_structs_and_arrays_have_runtime_values() { sem_case structs '3\n4\n18\n4\n2\n40\n9\n'; }

t_array_index_out_of_bounds_aborts() {
    write "$T/b.trk" 'fn main() -> void { let a = [1, 2]; print(a[2]); }'
    build "$T/b.trk" "$T/b"
    observe "$T/b"
    rc_is 134 "abort status"
    has "$ERR" "array index out of bounds" "stderr"
}

# sigfpe_case NAME SOURCE WANT: stdout is flushed, then SIGFPE (136).
sigfpe_case() {
    write "$T/$1.trk" "$2"
    build "$T/$1.trk" "$T/$1"
    observe "$T/$1"
    stdout_is "$3" "$1"
    rc_is 136 "$1: expected SIGFPE"
}

t_division_by_zero_flushes_and_raises_sigfpe() {
    sigfpe_case divz 'fn main()->i64{ print(1); let mut i=3; while i>=0 { print(100/i); i=i-1; } return 0; }' '1\n33\n50\n100\n'
    sigfpe_case modz 'fn main()->i64{ print(7); let z = 0; print(5 % z); return 0; }' '7\n'
    sigfpe_case constz 'fn main()->i64{ print(2); print(9 / 0); return 0; }' '2\n'
    sigfpe_case shadowz 'const TWO = 2; const ONE = 1 < 2; fn d(TWO: i64) -> i64 { return 7 / TWO; } fn e(ONE: i64) -> i64 { return 7 % (ONE + 1); } fn main()->i64{ print(3); print(e(1)); print(d(0)); return 0; }' '3\n1\n'
}

t_int64_min_div_minus_one_flushes_and_raises_sigfpe() {
    write "$T/negone.trk" 'fn main()->i64{ let m = -1; print(7 / m); print(7 % m); print(-9223372036854775807 / m); return 0; }'
    build "$T/negone.trk" "$T/negone"
    observe "$T/negone"
    rc_ok "negone"
    stdout_is '-7\n0\n9223372036854775807\n' "negone"
    sigfpe_case mindiv 'fn main()->i64{ print(3); let m = -1; let a = -9223372036854775807 - 1; print(a / m); return 0; }' '3\n'
    sigfpe_case minmod 'fn main()->i64{ print(3); let m = -1; let a = -9223372036854775807 - 1; print(a % m); return 0; }' '3\n'
}

# performance_tests.rs: same inputs and the same 5 s ceilings, on trackc.

now_ms() { echo $(($(date +%s%N) / 1000000)); }

t_performance_lexer_throughput() {
    local i start ms ntok
    for i in $(seq 5000); do echo 'let mut x: i64 = 100 + 200 * 300 / 400;'; done >"$T/big.trk"
    start=$(now_ms)
    trackc tokens "$T/big.trk"
    ms=$(($(now_ms) - start))
    rc_ok "tokens"
    ntok=$(($(wc -l <"$T/.out") - 1))
    note "lexed $ntok tokens in ${ms}ms"
    [ "$ntok" -ge 60000 ] || fail "only $ntok tokens"
    [ "$ms" -lt 5000 ] || fail "lexer took ${ms}ms"
}

# Rust parsed 1000 copies of one fn; trackc check also resolves names, so
# the copies get distinct names (a repeated fn is E0201).
t_performance_parser_throughput() {
    local i start ms
    for i in $(seq 0 999); do echo "fn add$i(a: i32, b: i32) -> i32 { return a + b; }"; done >"$T/p.trk"
    start=$(now_ms)
    trackc check "$T/p.trk"
    ms=$(($(now_ms) - start))
    rc_ok "check"
    note "checked 1000 functions in ${ms}ms"
    [ "$ms" -lt 5000 ] || fail "parser took ${ms}ms"
}

t_performance_full_pipeline_throughput() {
    local i start ms
    for i in $(seq 0 499); do echo "fn compute$i(x: i32) -> i32 { let y = x + 1; return y * 2; }"; done >"$T/c.trk"
    start=$(now_ms)
    trackc check "$T/c.trk"
    ms=$(($(now_ms) - start))
    rc_ok "check"
    note "checked 500 functions in ${ms}ms"
    [ "$ms" -lt 5000 ] || fail "pipeline took ${ms}ms"
}
