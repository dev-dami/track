# trackc CLI behaviour, ported from tests/cli_tests.rs (which drove the
# Rust `track` binary; trackc has the same commands and messages).

t_cli_help() {
    trackc --help
    rc_ok "--help"
    has "$OUT$ERR" "USAGE:" "help text"
}

t_cli_check_command() {
    write "$T/test.trk" 'fn main() -> void { print(42); }'
    trackc_in "$T" check test.trk
    rc_ok "check"
    has "$OUT" "Check passed" "stdout"
}

t_cli_invalid_command() {
    trackc nonexistent_command
    rc_fail "unknown command"
    has "$ERR" "Unknown command" "stderr"
}

t_cli_version_reports_1_0_0() {
    trackc --version
    rc_ok "--version"
    has "$OUT$ERR" "1.0.0" "version"
}

t_cli_duplicate_function_is_a_diagnostic() {
    write "$T/dup.trk" 'fn place_root() -> i64 { return 1; }
fn place_root() -> i64 { return 2; }
fn main() -> i64 { print(place_root()); return 0; }
'
    local mode
    for mode in check build; do
        trackc_in "$T" "$mode" dup.trk
        rc_is 1 "$mode"
        has "$ERR" "error[E0201]: duplicate definition of function 'place_root'" "$mode"
        lacks "$ERR" "panicked" "$mode"
    done
}

t_cli_duplicate_function_through_import_is_a_diagnostic() {
    write "$T/other.trk" 'fn place_root() -> i64 { return 1; }
'
    write "$T/main.trk" 'import "other";
fn place_root() -> i64 { return 2; }
fn main() -> i64 { print(place_root()); return 0; }
'
    local mode
    for mode in check build; do
        trackc_in "$T" "$mode" main.trk
        rc_is 1 "$mode"
        has "$ERR" "error[E0201]: duplicate definition of function 'place_root'" "$mode"
    done
}

# Build with no -o: the executable is written next to the source's stem.
t_cli_build_and_run_generics() {
    write "$T/gen.trk" 'fn identity<T>(x: T) -> T { return x; } fn main() -> void { let a: i32 = 7; print(identity(a)); }'
    trackc_in "$T" build gen.trk
    rc_ok "build generics"
    [ -x "$T/gen" ] || fail "no executable gen"
    observe "$T/gen"
    stdout_is '7\n' "run"
}
