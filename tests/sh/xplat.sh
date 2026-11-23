# Cross-platform targets (`--target`, docs/native.md "Targets"): target
# parsing, x86_64-linux output unchanged by the target machinery, and
# x86_64-windows PE executables run under wine (skipped without wine).
# Commands run from the repository root.

# wine with a quiet log; WINEPREFIX may point at a scratch prefix.
win_run() {
    (cd "$1" && shift && WINEDEBUG=-all timeout 120 wine "$@") >"$T/.out" 2>"$T/.err"
    RC=$?
    OUT=$(cat "$T/.out")
    ERR=$(cat "$T/.err")
    return 0
}

need_wine() {
    have wine || skip "wine not found"
}

t_xplat_unknown_target_is_a_clear_error() {
    in_repo "$TRACKC" build --target sparc-plan9 examples/hello.trk -o "$T/x"
    rc_is 2 "exit code"
    has "$ERR" "unknown target 'sparc-plan9'" "message"
    has "$ERR" "x86_64 or aarch64" "hint"
    in_repo env TRACKC_TARGET=x86_64-beos "$TRACKC" check examples/hello.trk
    rc_is 2 "TRACKC_TARGET is read"
}

t_xplat_unsupported_target_says_so() {
    in_repo "$TRACKC" build --target aarch64-windows examples/hello.trk -o "$T/x"
    rc_is 2 "exit code"
    has "$ERR" "not supported yet" "message"
    in_repo "$TRACKC" check --target aarch64-windows examples/hello.trk
    rc_ok "check works for any known target"
}

t_xplat_linux_target_is_the_default_output() {
    in_repo "$TRACKC" build examples/hello.trk -o "$T/a"
    rc_ok "default build"
    in_repo "$TRACKC" build --target x86_64-linux examples/hello.trk -o "$T/b"
    rc_ok "--target build"
    in_repo env TRACKC_TARGET=x86_64-linux "$TRACKC" build examples/hello.trk -o "$T/c"
    rc_ok "TRACKC_TARGET build"
    same_file "$T/a" "$T/b" "--target x86_64-linux"
    same_file "$T/a" "$T/c" "TRACKC_TARGET=x86_64-linux"
}

t_xplat_windows_build_is_a_pe32plus_image() {
    in_repo "$TRACKC" build --target x86_64-windows examples/hello.trk -o "$T/hello.exe"
    rc_ok "build"
    eq "$(head -c 2 "$T/hello.exe")" "MZ" "DOS magic"
    eq "$(dd if="$T/hello.exe" bs=1 skip=128 count=4 2>/dev/null | od -An -c | tr -d ' ')" 'PE\0\0' "PE signature"
    eq "$(dd if="$T/hello.exe" bs=1 skip=152 count=2 2>/dev/null | od -An -tx1 | tr -d ' ')" '0b02' "PE32+ magic"
    grep -q 'kernel32.dll' "$T/hello.exe" || fail "no kernel32.dll import"
}

t_xplat_extern_needs_an_import_target() {
    cat >"$T/ext.trk" <<'EOF'
extern "kernel32.dll" fn GetCurrentProcessId() -> u32;
fn main() -> i64 { return GetCurrentProcessId(); }
EOF
    trackc check "$T/ext.trk"
    rc_ok "check accepts extern"
    trackc build "$T/ext.trk" -o "$T/ext"
    rc_fail "linux build of an import"
    has "$ERR" "needs --target x86_64-windows" "message"
}

t_xplat_windows_hello_runs_under_wine() {
    need_wine
    in_repo "$TRACKC" build --target x86_64-windows examples/hello.trk -o "$T/hello.exe"
    rc_ok "build"
    win_run "$T" "$T/hello.exe"
    rc_ok "run"
    eq "$OUT" "42" "stdout"
}

t_xplat_windows_extern_call_under_wine() {
    need_wine
    cat >"$T/ext.trk" <<'EOF'
extern "kernel32.dll" fn GetStdHandle(which: i64) -> i64;
extern "kernel32.dll" fn WriteFile(h: i64, buf: i64, n: i64, done: i64, ov: i64) -> i32;
extern "kernel32.dll" fn MulDiv(a: i64, b: i64, c: i64) -> i32;
fn main() -> i64 {
    WriteFile(GetStdHandle(-11), "via WriteFile\n", 14, 0, 0);
    print(MulDiv(-7, 6, 2));
    return 3;
}
EOF
    trackc build --target x86_64-windows "$T/ext.trk" -o "$T/ext.exe"
    rc_ok "build"
    win_run "$T" "$T/ext.exe"
    rc_is 3 "exit code"
    eq "$OUT" "via WriteFile
-21" "stdout"
}

# The examples print their goldens under wine, arguments, files and
# sockets (Winsock) included. error_handling prints the size of
# /etc/hostname, which only the Linux CI job pins.
t_xplat_windows_examples_print_their_goldens() {
    need_wine
    local e n=0 g="$ROOT/tests/cases/examples"
    for e in arithmetic borrow generics hello linear_auto_free macro_test net_demo slice_and_types stdlib_demo structs_arrays union_enum_test use_test v030_features_demo v040_features_demo; do
        n=$((n + 1))
        in_repo "$TRACKC" build --target x86_64-windows "examples/$e.trk" -o "$T/$e.exe"
        rc_ok "$e: build"
        win_run "$ROOT/examples" "$T/$e.exe"
        out_is "$g/$e.out" "$e: output"
        rc_is "$(cat "$g/$e.code")" "$e: exit code"
    done
    [ "$n" -ge 14 ] || fail "only $n inputs"
}

# trackc.exe, built for Windows, builds hello.exe identical to the one
# the Linux trackc builds.
t_xplat_windows_trackc_builds_hello() {
    need_wine
    in_repo "$TRACKC" build --target x86_64-windows compiler/src/main.trk -o "$T/trackc.exe"
    rc_ok "trackc.exe build"
    in_repo "$TRACKC" build --target x86_64-windows examples/hello.trk -o "$T/h1.exe"
    rc_ok "hello.exe build"
    win_run "$ROOT" "$T/trackc.exe" build --target x86_64-windows examples/hello.trk -o "$T/h2.exe"
    rc_ok "trackc.exe builds hello.exe"
    same_file "$T/h1.exe" "$T/h2.exe" "hello.exe from both compilers"
}

# The runtime smoke test passes under wine: files removed while handles
# are still open are gone at once, as on Linux.
t_xplat_windows_rt_smoke_under_wine() {
    need_wine
    in_repo "$TRACKC" build --target x86_64-windows compiler/rt/tests/rt_smoke.trk -o "$T/rt_smoke.exe"
    rc_ok "build"
    win_run "$ROOT" "$T/rt_smoke.exe"
    out_is "$ROOT/tests/cases/native_runs/rt_smoke.out" "output"
    rc_is "$(cat "$ROOT/tests/cases/native_runs/rt_smoke.code")" "exit code"
}

# Integer division by zero ends a Windows process with NTSTATUS
# 0xC0000094 (STATUS_INTEGER_DIVIDE_BY_ZERO); wine passes its low byte,
# 0x94 = 148, to the Linux parent. Linux keeps SIGFPE (136).
t_xplat_windows_divzero_is_status_integer_divide_by_zero() {
    need_wine
    in_repo "$TRACKC" build --target x86_64-windows tests/native/divzero.trk -o "$T/divzero.exe"
    rc_ok "build"
    win_run "$ROOT" "$T/divzero.exe"
    out_is "$ROOT/tests/cases/native_runs/divzero.out" "output before the trap"
    rc_is 148 "exit code (0xC0000094 & 0xff)"
}
