#!/usr/bin/env bash
# tests/yard/run.sh — behaviour tests for the Track yard (tools/yard/*.trk).
#
# Needs no cargo. Each case mirrors one test in tests/yard_tests.rs (named in
# its header), plus a few CLI cases docs/yard.md describes.
#
#   tests/yard/run.sh                 build yard with trackc, then test it
#   YARD_BIN=/path/to/yard tests/yard/run.sh   test an existing yard binary
#
# trackc: $TRACKC, else `trackc` on PATH, else build/bootstrap/stage3/trackc.
set -u
ROOT=$(cd "$(dirname "$0")/../.." && pwd)
WORK=$(mktemp -d "${TMPDIR:-/tmp}/yard-run.XXXXXX")
trap 'rm -rf "$WORK"' EXIT

if [ -z "${TRACKC:-}" ]; then
    TRACKC=$(command -v trackc || echo "$ROOT/build/bootstrap/stage3/trackc")
fi
export TRACKC
if [ -z "${YARD_BIN:-}" ]; then
    YARD_BIN="$WORK/yard"
    "$TRACKC" build --native "$ROOT/tools/yard/main.trk" -o "$YARD_BIN" >/dev/null || {
        echo "FAIL: cannot build tools/yard/main.trk with $TRACKC"; exit 1; }
fi
Y="$YARD_BIN"

pass=0
fail=0
failed=""
current=""
ok=1

begin() { current=$1; ok=1; }
check() { # check <description> <command...>
    local what=$1; shift
    if ! "$@" >/dev/null 2>&1; then
        echo "  $current: FAILED: $what"
        ok=0
    fi
}
finish() {
    if [ $ok = 1 ]; then
        pass=$((pass + 1)); echo "ok   $current"
    else
        fail=$((fail + 1)); failed="$failed $current"; echo "FAIL $current"
    fi
}
in_dir() { local d=$1; shift; (cd "$d" && "$@"); }
contains() { grep -qF -- "$2" "$1"; }

# --- test_yard_manifest_serialization -------------------------------------
begin manifest_serialization
P="$WORK/ser/test_proj"
mkdir -p "$WORK/ser"
check "init" "$Y" init "$P"
sed -i 's/^version = "0.1.0"/version = "1.2.3"/' "$P/Track.toml"
# Load + save round trip (add rewrites the whole manifest).
check "load/save round trip" in_dir "$P" "$Y" add roundtrip_dep
check "name serialized" contains "$P/Track.toml" 'name = "test_proj"'
check "version serialized" contains "$P/Track.toml" 'version = "1.2.3"'
in_dir "$P" "$Y" check >"$WORK/ser.out" 2>&1
check "reloaded name and version" contains "$WORK/ser.out" "Checking test_proj v1.2.3"
finish

# --- test_yard_init_add_check_in_temp_dir ---------------------------------
begin init_add_check
P="$WORK/iac/my_app"
mkdir -p "$WORK/iac"
check "init" "$Y" init "$P"
check "Track.toml" test -f "$P/Track.toml"
check "src/main.trk" test -f "$P/src/main.trk"
check ".gitignore" test -f "$P/.gitignore"
check "check" in_dir "$P" "$Y" check
check "add" in_dir "$P" "$Y" add some_dep --version 0.2.0
check "dependency saved" contains "$P/Track.toml" 'some_dep = "0.2.0"'
in_dir "$P" "$Y" build >"$WORK/iac.out" 2>&1
check "dependency reloaded" contains "$WORK/iac.out" "Using dependency: some_dep v0.2.0 (registry version: 0.2.0)"
finish

# --- test_yard_parallel_build_and_incremental_cache -----------------------
begin parallel_build_and_incremental_cache
P="$WORK/par/multi_app"
mkdir -p "$WORK/par"
check "init" "$Y" init "$P"
echo 'fn helper1() -> i32 { return 10; }' >"$P/src/helper1.trk"
echo 'fn helper2() -> i32 { return 20; }' >"$P/src/helper2.trk"
check "build" in_dir "$P" "$Y" build
check "executable" test -x "$P/target/multi_app"
check "cache metadata" test -f "$P/target/.cache_meta.json"
in_dir "$P" "$Y" build >"$WORK/par.out" 2>&1
check "rebuild" test $? = 0
check "rebuild hits the cache" contains "$WORK/par.out" "(3 files, 3 cached)"
finish

# --- test_yard_lint_and_clean ---------------------------------------------
begin lint_and_clean
P="$WORK/lint/lint_app"
mkdir -p "$WORK/lint"
check "init" "$Y" init "$P"
check "lint" in_dir "$P" "$Y" lint
check "build" in_dir "$P" "$Y" build
check "target exists" test -d "$P/target"
check "clean" in_dir "$P" "$Y" clean
check "target removed" test ! -e "$P/target"
finish

# --- test_yard_version_reports --------------------------------------------
begin version_reports
"$Y" --version >"$WORK/ver.out" 2>&1
check "version 1.0.0" contains "$WORK/ver.out" "1.0.0"
finish

# --- test_yard_build_generics_project -------------------------------------
begin build_generics_project
P="$WORK/gen/gen_app"
mkdir -p "$WORK/gen"
check "init" "$Y" init "$P"
printf '%s' 'fn identity<T>(x: T) -> T { return x; } fn main() -> void { let a: i32 = 9; print(identity(a)); }' >"$P/src/main.trk"
check "build" in_dir "$P" "$Y" build
check "executable" test -x "$P/target/gen_app"
check "check" in_dir "$P" "$Y" check
check "lint" in_dir "$P" "$Y" lint
check "runs" test "$("$P/target/gen_app")" = 9
finish

# --- test_yard_nested_modules_have_isolated_stable_artifacts --------------
begin nested_modules_isolated_stable_artifacts
P="$WORK/art"
mkdir -p "$P/src/left" "$P/src/right"
printf '[package]\nname = "artifacts"\nversion = "0.1.0"\nauthors = []\n\n[dependencies]\n\n[build]\nsrc = "src"\n' >"$P/Track.toml"
echo 'fn main() -> void { print(42); }' >"$P/src/main.trk"
echo 'fn left_helper() -> i32 { return 10; }' >"$P/src/left/helper.trk"
echo 'fn right_helper() -> i32 { return 20; }' >"$P/src/right/helper.trk"
echo 'fn user_runtime() -> i32 { return 30; }' >"$P/src/_track_runtime.trk"
OBJS="main.o left/helper.o right/helper.o _track_runtime.o"
stamps() { for o in $OBJS; do stat -c '%y' "$P/target/objects/$o"; done; }
check "build" in_dir "$P" "$Y" build
for o in $OBJS; do check "object $o" test -f "$P/target/objects/$o"; done
check "equal basenames do not collide" bash -c "! cmp -s '$P/target/objects/left/helper.o' '$P/target/objects/right/helper.o'"
cp "$P/target/.cache_meta.json" "$WORK/meta.json"
stamps >"$WORK/stamps1"
check "warm build" in_dir "$P" "$Y" build
check "warm metadata identical" cmp "$WORK/meta.json" "$P/target/.cache_meta.json"
stamps >"$WORK/stamps2"
check "warm build keeps objects" cmp "$WORK/stamps1" "$WORK/stamps2"
rm -rf "$P/target"
check "clean build" in_dir "$P" "$Y" build
check "clean metadata identical" cmp "$WORK/meta.json" "$P/target/.cache_meta.json"
for o in $OBJS; do check "rebuilt $o" test -f "$P/target/objects/$o"; done
check "executable prints 42" test "$("$P/target/artifacts")" = 42
finish

# --- test_yard_build_errors_follow_source_order ---------------------------
begin build_errors_follow_source_order
P="$WORK/err"
mkdir -p "$P/src"
printf '[package]\nname = "errors"\nversion = "0.1.0"\n' >"$P/Track.toml"
printf 'fn invalid( {' >"$P/src/z.trk"
printf 'fn invalid( {' >"$P/src/a.trk"
for i in 1 2 3 4; do
    in_dir "$P" "$Y" build >"$WORK/err.out" 2>&1
    check "build $i fails" test $? = 1
    grep -m1 'yard error:' "$WORK/err.out" >"$WORK/err.first"
    check "build $i reports a.trk first" contains "$WORK/err.first" "a.trk"
done
finish

# --- test_yard_dependency_resolution_is_sorted ----------------------------
begin dependency_resolution_is_sorted
P="$WORK/ord"
mkdir -p "$P/src"
printf '[package]\nname = "ordered"\nversion = "0.1.0"\n\n[dependencies]\nzebra = "1.0"\nalpha = "1.0"\nmiddle = "1.0"\n' >"$P/Track.toml"
echo 'fn main() -> void { print(1); }' >"$P/src/main.trk"
in_dir "$P" "$Y" build 2>/dev/null | sed -n 's/^  Using dependency: \([a-z]*\) .*/\1/p' | tr '\n' ' ' >"$WORK/ord.out"
check "alpha middle zebra" test "$(cat "$WORK/ord.out")" = "alpha middle zebra "
finish

# --- every module is linked, as the Rust yard linked every object ---------
# main calls a function from another module without importing it.
begin implicit_module_link
P="$WORK/imp/app"
mkdir -p "$WORK/imp"
check "init" "$Y" init "$P"
echo 'fn helper(x: i32) -> i32 { return x+1; }' >"$P/src/h.trk"
echo 'fn main() -> void { print(helper(2)); }' >"$P/src/main.trk"
check "check" in_dir "$P" "$Y" check
check "build" in_dir "$P" "$Y" build
check "executable prints 3" test "$("$P/target/app" 2>/dev/null)" = 3
check "run prints 3" test "$(in_dir "$P" "$Y" run 2>/dev/null | tail -n 1)" = 3
finish

# Nested modules, and a module both imported explicitly (by main and by a
# nested module) and present implicitly: each module is defined once.
begin nested_and_explicit_imports_link_once
P="$WORK/nest"
mkdir -p "$P/src/left" "$P/src/util/deep"
printf '[package]\nname = "nest"\nversion = "0.1.0"\n' >"$P/Track.toml"
echo 'fn helper(x: i32) -> i32 { return x+1; }' >"$P/src/h.trk"
printf 'import "h";\nfn left_value() -> i32 { return helper(10); }\n' >"$P/src/left/a.trk"
echo 'fn deep_value() -> i32 { return 100; }' >"$P/src/util/deep/d.trk"
printf 'import "h";\nfn main() -> void { print(helper(2)); print(left_value()); print(deep_value()); }\n' >"$P/src/main.trk"
check "check" in_dir "$P" "$Y" check
check "build" in_dir "$P" "$Y" build
check "prints 3 11 100" test "$("$P/target/nest" 2>/dev/null | tr '\n' ' ')" = "3 11 100 "
finish

# check and build agree on a call to a function no module defines.
begin check_and_build_agree_on_undefined
P="$WORK/undef"
mkdir -p "$P/src"
printf '[package]\nname = "undef"\nversion = "0.1.0"\n' >"$P/Track.toml"
echo 'fn helper(x: i32) -> i32 { return x+1; }' >"$P/src/h.trk"
echo 'fn main() -> void { print(missing(2)); }' >"$P/src/main.trk"
check "build fails" bash -c "cd '$P' && ! '$Y' build"
check "check fails" bash -c "cd '$P' && ! '$Y' check"
in_dir "$P" "$Y" check >"$WORK/undef.out" 2>&1
check "check names the function" contains "$WORK/undef.out" "missing"
echo 'fn missing(x: i32) -> i32 { return x*2; }' >"$P/src/m.trk"
check "check passes once defined" in_dir "$P" "$Y" check
check "build passes once defined" in_dir "$P" "$Y" build
check "prints 4" test "$("$P/target/undef" 2>/dev/null)" = 4
finish

# The incremental cache: an unchanged project is all cached and keeps its
# image; editing a module that main does not import rebuilds the image.
begin linked_image_cache
P="$WORK/lcache"
mkdir -p "$P/src"
printf '[package]\nname = "lcache"\nversion = "0.1.0"\n' >"$P/Track.toml"
echo 'fn helper(x: i32) -> i32 { return x+1; }' >"$P/src/h.trk"
echo 'fn main() -> void { print(helper(2)); }' >"$P/src/main.trk"
check "build" in_dir "$P" "$Y" build
stat -c '%y' "$P/target/lcache" >"$WORK/lcache.s1" 2>/dev/null
in_dir "$P" "$Y" build >"$WORK/lcache.out" 2>&1
check "warm build all cached" contains "$WORK/lcache.out" "(2 files, 2 cached)"
stat -c '%y' "$P/target/lcache" >"$WORK/lcache.s2" 2>/dev/null
check "warm build keeps the image" cmp "$WORK/lcache.s1" "$WORK/lcache.s2"
echo 'fn helper(x: i32) -> i32 { return x+5; }' >"$P/src/h.trk"
in_dir "$P" "$Y" build >"$WORK/lcache.out" 2>&1
check "edit rebuilds" contains "$WORK/lcache.out" "(2 files, 0 cached)"
check "edited module is linked" test "$("$P/target/lcache" 2>/dev/null)" = 7
finish

# --- CLI cases from docs/yard.md ------------------------------------------
begin cli_run_test_add_errors
P="$WORK/cli/cli_app"
mkdir -p "$WORK/cli"
check "init" "$Y" init "$P"
check "init refuses an existing directory" bash -c "! '$Y' init '$P'"
check "run prints 42" test "$(in_dir "$P" "$Y" run 2>/dev/null | tail -n 1)" = 42
mkdir -p "$P/tests"
echo 'fn main() -> i64 { return 0; }' >"$P/tests/pass.trk"
check "test passes" in_dir "$P" "$Y" test
echo 'fn main() -> void { abort(); }' >"$P/tests/zfail.trk"
check "failing test fails" bash -c "cd '$P' && ! '$Y' test"
rm "$P/tests/zfail.trk"
mkdir -p "$WORK/cli/lib"
check "add --path" in_dir "$P" "$Y" add lib --path ../lib
check "path dependency table" contains "$P/Track.toml" "[dependencies.lib]"
"$Y" bogus >"$WORK/cli.out" 2>&1
check "unknown command" contains "$WORK/cli.out" "yard error: Unknown yard command: 'bogus'"
in_dir "$WORK" "$Y" build >"$WORK/cli.out" 2>&1
check "no Track.toml" contains "$WORK/cli.out" "No Track.toml found"
in_dir "$P" "$Y" add >"$WORK/cli.out" 2>&1
check "add usage" contains "$WORK/cli.out" "Usage: yard add <package>"
finish

echo
echo "yard: $pass passed, $fail failed"
[ $fail = 0 ] || { echo "failed:$failed"; exit 1; }
