# Lifecycle dumps, placement plans, auto-free and the K3 regressions.
# Port of tests/selfhost_tests.rs from life_dumps_match_goldens through
# int64_min_div_minus_one_flushes_on_every_backend. trackc runs from the
# repository root and built programs run from it too, as under cargo.
# UPDATE_LIFE=1 rewrites the .life and .place goldens.

# golden GOT_FILE GOLDEN MSG: GOT_FILE equals GOLDEN byte for byte
# (UPDATE_LIFE=1 first copies GOT_FILE over GOLDEN).
golden() {
    if [ -n "${UPDATE_LIFE+x}" ]; then
        mkdir -p "$(dirname "$2")"
        cp "$1" "$2"
    fi
    [ -f "$2" ] || fail "missing golden $2"
    cmp -s "$1" "$2" || fail "$3: differs from $2
$(diff "$2" "$1" | head -n 40)"
}

# run_want EXE WANT_FILE MSG: EXE (run from the repo root) prints exactly
# WANT_FILE and exits 0.
run_want() {
    in_repo "$1"
    out_is "$2" "$3"
    rc_is 0 "$3"
}

# plan REL: `trackc life --place REL` into $T/plan.got; appended to
# $T/plans as "<stem>:\n<plan>" (the Rust `plans` string).
plan() {
    in_repo "$TRACKC" life --place "$1"
    cp "$T/.out" "$T/plan.got"
    { printf '%s:\n' "$(stem "$1")"; cat "$T/plan.got"; } >>"$T/plans"
}

# plans_text: the accumulated plans, trailing newline kept.
plans_text() {
    local s
    s=$(cat "$T/plans"; echo .)
    printf '%s' "${s%.}"
}

# block PLANS PROG: the text after the first "PROG:\n", cut at the next ":\n".
block() {
    local b
    case "$1" in
        *"$2:"$'\n'*) b=${1#*"$2:"$'\n'} ;;
        *) b= ;;
    esac
    printf '%s' "${b%%:$'\n'*}"
}

# need_lines PLANS: each stdin line "LINE|WHY" must occur in PLANS.
need_lines() {
    local line why
    while IFS='|' read -r line why; do
        has "$1" "$line" "$why"
    done
}

# no_line_starting BLOCK PREFIX WORD MSG: no line of BLOCK, left-trimmed,
# starts with PREFIX (right-trimmed) and contains WORD.
no_line_starting() {
    local l t p=${2%"${2##*[! ]}"}
    while IFS= read -r l; do
        t=${l#"${l%%[! ]*}"}
        case "$t" in
            "$p"*) case "$l" in *"$3"*) fail "$4: [$l] in:
$1" ;; esac ;;
        esac
    done <<<"$1"
}

# asan_cc: gcc builds an AddressSanitizer program (the Rust probe: compile
# only, from stdin).
asan_cc() {
    printf 'int main(void){return 0;}' | gcc -fsanitize=address -x c - -o "$T/probe" >/dev/null 2>&1
}

# asan_run REL LEAKS NEEDLE: build REL with CC="gcc -fsanitize=address -g"
# (private cache), run it with ASAN_OPTIONS=LEAKS; it must exit 0 and its
# stderr must not contain NEEDLE.
asan_run() {
    local rel=$1 exe
    exe="$T/$(stem "$rel")_asan"
    in_repo env CC="gcc -fsanitize=address -g" TRACKC_CACHE="$T/asan-cache" "$TRACKC" build "$rel" -o "$exe"
    rc_ok "$rel: ASan build failed"
    in_repo env ASAN_OPTIONS="$2" "$exe"
    rc_ok "$rel: ASan run ($2)"
    lacks "$ERR" "$3" "$rel: ASan reports an error or a leak"
}

# c_and_native DIR STEM: build tests/DIR/STEM.trk with both backends; each
# prints tests/DIR/STEM.out and exits 0.
c_and_native() {
    local rel="tests/$1/$2.trk" want="$ROOT/tests/$1/$2.out"
    in_repo "$TRACKC" build "$rel" -o "$T/$2"
    rc_ok "$rel: build failed"
    run_want "$T/$2" "$want" "$rel: C output differs"
    in_repo "$TRACKC" build --native "$rel" -o "$T/$2_nat"
    rc_ok "$rel: native build failed"
    run_want "$T/$2_nat" "$want" "$rel: native output differs"
}

t_life_dumps_match_goldens() {
    local checked=0 sub f rel
    for sub in docs/lifecycles bench/progs tests/life/progs; do
        for f in $(trk_files "$ROOT/$sub"); do
            rel=${f#"$ROOT/"}
            in_repo "$TRACKC" life "$rel"
            rc_ok "$rel: trackc life failed"
            golden "$T/.out" "$ROOT/tests/life/$(stem "$f").life" "$rel: lifecycle dump"
            checked=$((checked + 1))
        done
    done
    [ "$checked" -ge 22 ] || fail "expected at least 22 programs, saw $checked"
}

t_life_place_dumps_match_goldens() {
    local f rel n=0
    for f in $(trk_files "$ROOT/tests/life/place"); do
        rel=${f#"$ROOT/"}
        n=$((n + 1))
        in_repo "$TRACKC" life --place "$rel"
        rc_ok "$rel: trackc life --place failed"
        golden "$T/.out" "${f%.trk}.place" "$rel: placement plan"
    done
    [ "$n" -gt 0 ] || fail "no programs in tests/life/place"
    in_repo "$TRACKC" life --place tests/life/place/place.trk
    need_lines "$OUT" <<'LINES'
L20 tuple stack ends L21|a tuple only destructured lives on the stack
L8 tuple ret-slot ends L8|a returned tuple goes to the caller's slot
L24 struct heap ends -|a value stored into a collection is heap
L26 call heap ends -|a value passed to an esc param is heap
L28 str heap-autofree ends L29|a fresh Str that dies in this frame is auto-freed (L13)
LINES
}

t_stack_cells_run_and_plan() {
    local s rel
    for s in escapes mutate read_only union_match; do
        rel="tests/stack/$s.trk"
        in_repo "$TRACKC" build "$rel" -o "$T/$s"
        rc_ok "$rel: build failed"
        run_want "$T/$s" "$ROOT/tests/stack/$s.out" "$rel: output differs"
        if [ "$s" != escapes ]; then
            grep -qF 'tk_w s0[' "$T/$s.c" || fail "$rel: no stack slot in emitted C"
        fi
        plan "$rel"
        golden "$T/plan.got" "$ROOT/tests/stack/$s.place" "$rel: placement plan"
    done
    need_lines "$(plans_text)" <<'LINES'
L23 struct stack ends L25|a struct only lent to a reading callee is stack
L24 struct stack ends L25|a struct only read through its fields is stack
L29 array stack ends L31|an array only indexed and lent is stack
L8 struct heap ends -|a returned struct is heap
L17 struct heap ends -|a struct stored into another literal is heap
L27 struct heap ends -|a struct passed to a `ret` parameter is heap
L29 struct heap ends -|a struct inside an array literal is heap
L17 struct stack ends L23|a struct mutated through the slot is stack
L21 array stack ends L24|an array written by index is stack
L13 union stack ends L14|a payload union only matched is stack
LINES
}

t_autofree_frees_heap_cells() {
    local stems="die_param early_return moved_field nested returned" s rel
    for s in $stems; do
        rel="tests/autofree/$s.trk"
        in_repo "$TRACKC" build "$rel" -o "$T/$s"
        rc_ok "$rel: build failed"
        run_want "$T/$s" "$ROOT/tests/autofree/$s.out" "$rel: output differs"
        grep -qF 'dealloc(l' "$T/$s.c" || fail "$rel: no free in emitted C"
        plan "$rel"
        golden "$T/plan.got" "$ROOT/tests/autofree/$s.place" "$rel: placement plan"
    done
    need_lines "$(plans_text)" <<'LINES'
L23 call heap-autofree deep ends L24|a fresh cell passed to a `die` parameter is freed by the caller
L21 call stack ends L22|a ret-slot result passed to a `die` parameter is a stack slot (L10)
L19 struct stack ends L20|a stack cell passed to a `die` parameter stays a stack cell
L16 call heap-autofree ends L17|a cell whose child is moved out frees only itself
L22 struct heap-autofree deep ends L27|a nested literal reassigned fresh cells runs drop glue
L7 struct heap ends -|a returned cell is never freed
L13 struct heap ends -|a cell stored into a returned cell is never freed
LINES
    asan_cc || skip "gcc has no AddressSanitizer"
    for s in $stems; do
        asan_run "tests/autofree/$s.trk" detect_leaks=0 AddressSanitizer
    done
}

t_handles_free_fresh_values() {
    local stems="builder coll_kept coll_local coll_move int_loop str_kept str_local" s rel prog free kept plans moved
    for s in $stems; do
        rel="tests/handles/$s.trk"
        c_and_native handles "$s"
        plan "$rel"
        golden "$T/plan.got" "$ROOT/tests/handles/$s.place" "$rel: placement plan"
        while read -r prog free; do
            if [ "$prog" = "$s" ]; then
                grep -qF "$free" "$T/$s.c" || fail "$rel: no \`$free\` in emitted C"
            fi
        done <<'FREES'
str_local track_str_free(l
int_loop track_str_free(l
builder track_sb_free(l
builder track_str_free(l
coll_local track_i64_collection_free(l
coll_local vec_free(l
FREES
    done
    plans=$(plans_text)
    # coll_move (card L15): only `rows` moves a fresh row into `prev` as
    # its last mention; the looped and the used-after moves keep both.
    moved=
    case "$plans" in *coll_move:*) moved=${plans#*coll_move:} ;; esac
    moved=${moved%%int_loop:*}
    eq "$moved" $'\nfn rows\n  L6 collection heap-autofree ends L15\n' "coll_move: moves planned"
    need_lines "$plans" <<'LINES'
L4 str heap-autofree ends L6|a str_concat result that dies locally is freed
L14 str heap-autofree ends L17|a Str reassigned fresh strings is freed
L7 str heap-autofree ends L10|an int_to_str in a loop is freed each iteration
L4 builder heap-autofree ends L12|a builder finished into a Str is freed
L12 str heap-autofree ends L13|the finished Str is freed
L3 collection heap-autofree ends L12|a collection that dies locally is freed
L19 vec heap-autofree ends L27|a Vec that dies locally is freed
LINES
    while IFS='|' read -r prog kept; do
        no_line_starting "$(block "$plans" "$prog")" "$kept" autofree \
            "$prog: a returned, stored or aliased value at \`$kept\` must not be freed"
    done <<'KEPT'
str_kept|L4
str_kept|L12 str
str_kept|L14 str
coll_kept|L7
coll_kept|L11
KEPT
    asan_cc || skip "gcc has no AddressSanitizer"
    for s in $stems; do
        asan_run "tests/handles/$s.trk" detect_leaks=0 AddressSanitizer
    done
}

t_temps_free_fresh_temporaries() {
    local stems="alias discard kept nested print_loop" s rel plans prog line n why kept leaks
    for s in $stems; do
        rel="tests/temps/$s.trk"
        c_and_native temps "$s"
        plan "$rel"
        golden "$T/plan.got" "$ROOT/tests/temps/$s.place" "$rel: placement plan"
    done
    plans=$(plans_text)
    while IFS='|' read -r prog line n why; do
        eq "$(count "$(block "$plans" "$prog")" "$line")" "$n" "$why: \`$line\` x$n"
    done <<'COUNTS'
print_loop|str temp-autofree|2|int_to_str passed to print and str_len is freed
nested|str temp-autofree|6|every nested fresh argument is freed
discard|str temp-autofree|3|discarded results and their fresh arguments are freed
kept|L12 str temp-autofree ends L12|1|int_to_str lent to str_concat is freed
COUNTS
    while IFS='|' read -r prog kept; do
        no_line_starting "$(block "$plans" "$prog")" "$kept" temp-autofree \
            "$prog: a temporary that is kept or aliased (\`$kept\`) must not be freed"
    done <<'KEPT'
kept|L11
alias|temp-autofree
KEPT
    asan_cc || skip "gcc has no AddressSanitizer"
    for s in $stems; do
        case "$s" in discard | nested | print_loop) leaks=detect_leaks=1 ;; *) leaks=detect_leaks=0 ;; esac
        asan_run "tests/temps/$s.trk" "$leaks" Sanitizer
    done
}

t_temps_free_fresh_cell_temporaries() {
    local stems="accloop accpass accret alias branch discard kept nested print_loop str_alias str_cell tailr"
    local s rel plans b leaks
    for s in $stems; do
        rel="tests/temps_cell/$s.trk"
        c_and_native temps_cell "$s"
        plan "$rel"
        golden "$T/plan.got" "$ROOT/tests/temps_cell/$s.place" "$rel: placement plan"
    done
    plans=$(plans_text)
    need_lines "$plans" <<'LINES'
L33 call stack ends L33|print_loop
L34 struct stack ends L34|print_loop
L35 call temp-autofree deep ends L35|print_loop
L36 struct temp-autofree deep ends L36|print_loop
L37 tuple stack ends L37|print_loop
L26 call stack ends L26|nested
L18 struct stack ends L18|str_cell
L17 call temp-autofree deep ends L17|str_cell
L23 call temp-autofree deep ends L23|discard
L24 call temp-autofree deep ends L24|discard
L25 call temp-autofree deep ends L25|discard
L19 call temp-autofree deep ends L19|str_alias
L21 call temp-autofree deep ends L21|str_alias
LINES
    # Card F3: accumulator recursion with temporary arguments stays a loop
    # natively; a million levels deep would overflow as real calls.
    in_repo "$TRACKC" build --native tests/temps_cell/accdeep.trk -o "$T/accdeep_nat"
    rc_ok "accdeep: native build failed"
    run_want "$T/accdeep_nat" "$ROOT/tests/temps_cell/accdeep.out" "accdeep: native output"
    eq "$(count "$(block "$plans" nested)" "call temp-autofree deep")" 3 "every nested fresh cell is freed"
    for s in kept alias; do
        b=$(block "$plans" "$s")
        lacks "$b" temp-autofree "$s: a cell that is kept or aliased must stay on the heap"
        lacks "$b" stack "$s: a cell that is kept or aliased must stay on the heap"
    done
    asan_cc || skip "gcc has no AddressSanitizer"
    for s in $stems; do
        # str_alias keeps `keep.name` alive to the end of main and drop glue
        # never frees Str, so it is checked for errors, not leaks.
        case "$s" in
            nested | print_loop | branch | discard | str_cell | tailr | accret | accloop) leaks=detect_leaks=1 ;;
            *) leaks=detect_leaks=0 ;;
        esac
        asan_run "tests/temps_cell/$s.trk" "$leaks" Sanitizer
    done
}

t_strfield_frees_owned_str_fields() {
    local stems="alias keep literal local moved movelocal param result temp" s rel got leaks
    for s in $stems; do
        rel="tests/strfield/$s.trk"
        c_and_native strfield "$s"
        in_repo "$TRACKC" life --place "$rel"
        cp "$T/.out" "$T/plan.got"
        golden "$T/plan.got" "$ROOT/tests/strfield/$s.place" "$rel: placement plan"
        got=$(cat "$T/plan.got")
        case "$s" in
            temp | local | result | movelocal) has "$got" " strs " "$s: no owned Str field" ;;
            param)
                # Only main's `keep` owns its string.
                lacks "${got%%fn main*}" " strs " "param: callee frees a parameter's Str" ;;
            *) lacks "$got" " strs " "$s: a Str field it does not own is freed" ;;
        esac
    done
    asan_cc || skip "gcc has no AddressSanitizer"
    for s in $stems; do
        case "$s" in temp | local | literal | result | movelocal) leaks=detect_leaks=1 ;; *) leaks=detect_leaks=0 ;; esac
        asan_run "tests/strfield/$s.trk" "$leaks" Sanitizer
    done
}

t_returns_hidden_in_expressions_are_seen() {
    local s banned rel
    local cases='str_let|str temp-autofree
str_hidden|str temp-autofree
sfres_cond|deep strs
sfres_hidden|deep strs
glue_hidden|temp-autofree
retslot_hidden|ret-slot'
    while IFS='|' read -r s banned; do
        rel="tests/returns/$s.trk"
        c_and_native returns "$s"
        in_repo "$TRACKC" life --place "$rel"
        rc_ok "$rel: life --place failed"
        [ -s "$T/.out" ] || fail "$rel: life --place printed nothing"
        lacks "$OUT" "$banned" "$rel: a hidden return rules out \`$banned\`"
    done <<<"$cases"
    asan_cc || skip "gcc has no AddressSanitizer"
    while IFS='|' read -r s banned; do
        asan_run "tests/returns/$s.trk" detect_leaks=0 Sanitizer
    done <<<"$cases"
}

t_ret_slots_write_caller_cells() {
    local stems="alias_ref alias_val both collection nested recursion" s rel
    for s in $stems; do
        rel="tests/retslot/$s.trk"
        in_repo "$TRACKC" build "$rel" -o "$T/$s"
        rc_ok "$rel: build failed"
        run_want "$T/$s" "$ROOT/tests/retslot/$s.out" "$rel: output differs"
        grep -qF '__rs(' "$T/$s.c" || fail "$rel: no ret-slot entry in emitted C"
        plan "$rel"
        golden "$T/plan.got" "$ROOT/tests/retslot/$s.place" "$rel: placement plan"
    done
    need_lines "$(plans_text)" <<'LINES'
L12 struct stack ends L22|a local reassigned only ret-slot results is a stack cell
L16 call stack ends L22|`acc = f(&acc)` writes acc's own cell
L14 call stack ends L20|`acc = f(acc)` writes acc's own cell
L13 call heap ends -|a ret-slot result stored into a collection is heap
L20 call stack ends L22|a ret-slot result in a stack slot
L21 call heap-autofree deep ends L26|a local also reassigned a heap-only cell stays heap-autofree
L25 call heap-autofree ends L26|a ret-slot result written into an autofree local's cell
L12 call heap ends -|a ret-slot result returned onward through a local is heap
L10 call stack ends L11|a recursive ret-slot call writes a slot of its caller's frame
L18 call ret-slot ends L18|a `return f(..)` passes the caller's slot through
L18 call stack ends L19|the outer call of `f(g(x))` writes a stack slot
LINES
    asan_cc || skip "gcc has no AddressSanitizer"
    for s in $stems; do
        asan_run "tests/retslot/$s.trk" detect_leaks=0 AddressSanitizer
    done
}

# K3a: a user program may define any name a runtime uses internally.
t_user_names_never_clash_with_runtime_names() {
    local s modes mode exe
    while IFS='|' read -r s modes; do
        for mode in $modes; do
            [ "$mode" = c ] && mode=
            exe="$T/$(printf '%s' "$s" | tr / _)$mode"
            # shellcheck disable=SC2086
            in_repo "$TRACKC" build $mode "$s.trk" -o "$exe"
            rc_ok "$s.trk $mode: build failed"
            run_want "$exe" "$ROOT/$s.out" "$s.trk $mode: output differs"
        done
    done <<'CASES'
tests/names/rt_clash|c --rt=track --native
tests/stack/read_only|--native
tests/autofree/die_param|--native
CASES
}

# A user `RT_BASE` (and friends) under --native is E0201, not a segfault.
t_runtime_layout_consts_stay_reserved() {
    local name
    for name in RT_BASE RS_USED RS_HEAP RS_LIMIT RS_HEAP_END RS_FREE; do
        printf 'const %s = 5;\nstruct P { x: i64 }\nfn mk(i: i64) -> P { return P { x: i }; }\nfn main() -> i64 { let p = mk(3); print(p.x); return 0; }\n' "$name" >"$T/$name.trk"
        trackc build --native "$name.trk" -o "$T/$name"
        rc_fail "$name: user const accepted under --native"
        has "$ERR" E0201 "$name: user const accepted under --native"
    done
}

t_duplicate_functions_are_rejected() {
    local file mode
    write "$T/other.trk" $'fn place_root() -> i64 { return 7; }\n'
    write "$T/one.trk" $'fn place_root() -> i64 { return 1; }\nfn place_root() -> i64 { return 2; }\nfn main() -> i64 { print(place_root()); return 0; }\n'
    write "$T/two.trk" $'import "other";\nfn place_root() -> i64 { return 2; }\nfn main() -> i64 { print(place_root()); return 0; }\n'
    for file in one.trk two.trk; do
        for mode in check build; do
            trackc "$mode" "$file"
            rc_fail "$mode $file: duplicate accepted"
            has "$ERR" "error[E0201]: duplicate definition of function 'place_root'" "$mode $file: duplicate accepted"
        done
    done
}

# K3a: `a + b` on two strings would add the pointers, so trackc rejects it.
t_string_plus_string_is_a_type_error() {
    local i=0 src
    while IFS= read -r src; do
        printf '%s\n' "$src" >"$T/bad$i.trk"
        trackc check "bad$i.trk"
        rc_fail "$src: string + string accepted"
        has "$ERR" str_concat "$src: string + string accepted"
        i=$((i + 1))
    done <<'BAD'
fn main() -> i64 { let a = "ab"; let b = "cd"; let c = a + b; print(c); return 0; }
fn main() -> i64 { print("ab" + "cd"); return 0; }
fn main() -> i64 { let a = "ab"; let s = str_concat(&a, &a); let t = s + a; print(t); return 0; }
BAD
    [ "$i" = 3 ] || fail "ran $i bad cases"
    # Pointer arithmetic (string + integer, string - string) stays legal.
    write "$T/ok.trk" $'fn main() -> i64 { let a = "abcd"; let n = 2; let p = a + n; print(p - a); return 0; }\n'
    trackc check ok.trk
    rc_ok "string + int rejected"
}

# build_both_rt STEM SRC: build SRC with the C and the Track runtime into
# $T/STEM_crt and $T/STEM_trt (trackc runs in $T, as run_under_both_runtimes).
build_both_rt() {
    trackc build "$2" -o "$T/$1_crt"
    rc_ok "$1: build failed"
    trackc build --rt=track "$2" -o "$T/$1_trt"
    rc_ok "$1: --rt=track build failed"
}

# K3b: for-in, exit(3) and a division trap keep what was printed on both
# runtimes; a scalar iterable and a range value are type errors.
t_for_in_exit_and_div_trap_on_both_runtimes() {
    local exe
    build_both_rt for_in "$ROOT/tests/native/for_in.trk"
    for exe in for_in_crt for_in_trt; do
        observe "$T/$exe"
        stdout_is '10\n20\n30\n3\n2\n3\n4\n10\n20\n30\n110\n120\n130\nab\ncde\n60\n11\n21\n31\n' "$exe"
        rc_is 0 "$exe"
    done
    build_both_rt exit_flush "$ROOT/tests/native/exit_flush.trk"
    for exe in exit_flush_crt exit_flush_trt; do
        observe "$T/$exe"
        stdout_is '1\n' "$exe"
        rc_is 3 "$exe"
    done
    build_both_rt divzero_flush "$ROOT/tests/native/divzero_flush.trk"
    for exe in divzero_flush_crt divzero_flush_trt; do
        observe "$T/$exe"
        stdout_is '1\n' "$exe: output lost on SIGFPE"
        rc_is 136 "$exe: SIGFPE"
    done
    write "$T/forint.trk" $'fn main() -> i64 { let n = 3; for i in n { print(i); } return 0; }\n'
    trackc check forint.trk
    rc_fail "for over an integer accepted"
    has "$ERR" '`for` iterates over a range or an array' "for over an integer accepted"
    # F3: a range bound as a value segfaulted on both backends; it is E0200.
    write "$T/rangeval.trk" $'fn main() -> void { let r = (0-1)..3; for i in r { print(i); } }\n'
    trackc check rangeval.trk
    rc_fail "range bound as a value accepted"
    has "$ERR" 'A range `a..b` is not a value' "range bound as a value accepted"
}

# K3c: INT64_MIN / -1 (and % -1) flushes, then SIGFPE, on every backend.
t_int64_min_div_minus_one_flushes_on_every_backend() {
    local s want mode exe
    while IFS='|' read -r s want; do
        for mode in c --rt=track --native; do
            exe="$T/$s$(printf '%s' "$mode" | tr -- '-=' '__')"
            if [ "$mode" = c ]; then
                trackc build "$ROOT/tests/native/$s.trk" -o "$exe"
            else
                trackc build "$mode" "$ROOT/tests/native/$s.trk" -o "$exe"
            fi
            rc_ok "$s $mode: build failed"
            observe "$exe"
            stdout_is "$want" "$s $mode: output lost"
            rc_is 136 "$s $mode: wrong signal"
        done
    done <<'CASES'
divmin_flush|-7\n0\n-9223372036854775807\n
divmin_mod_flush|0\n
CASES
}
