# Shared helpers for the cargo-free test runner (tests/run.sh).
#
# A suite is tests/sh/<suite>.sh; each `t_<name>() { ... }` in it is one
# test. The runner sources this file and the suite, then calls the test in
# a fresh bash process with:
#
#   ROOT     the repository root (absolute)
#   TRACKC   the trackc under test (absolute)
#   T        a private, empty scratch directory (cwd at start)
#
# A test passes when its function returns 0. Every assertion helper calls
# `fail`, which prints the message and exits 1, so the first broken check
# ends the test (as an assert! did in the Rust suites).
#
# `cap CMD...` runs a command with its output in files: stdout in $T/.out,
# stderr in $T/.err, the exit status in RC. OUT and ERR hold the same text
# (trailing newlines stripped, as $(...) does); use `out_is` / `cmp` on the
# files when trailing newlines matter.

set -u

fail() {
    printf 'FAIL: %s\n' "$*" >&2
    exit 1
}

note() { printf '%s\n' "$*" >&2; }

cap() {
    "$@" >"$T/.out" 2>"$T/.err"
    RC=$?
    OUT=$(cat "$T/.out")
    ERR=$(cat "$T/.err")
    return 0
}

# cap_in DIR CMD...: cap with DIR as the working directory.
cap_in() {
    local dir=$1
    shift
    (cd "$dir" && "$@") >"$T/.out" 2>"$T/.err"
    RC=$?
    OUT=$(cat "$T/.out")
    ERR=$(cat "$T/.err")
    return 0
}

# Show the captured streams after a failure message.
ctx() {
    printf -- '--- rc=%s\n--- stdout:\n%s\n--- stderr:\n%s\n' "$RC" "$(head -c 4000 "$T/.out")" "$(head -c 4000 "$T/.err")"
}

trackc() { cap "$TRACKC" "$@"; }
trackc_in() {
    local dir=$1
    shift
    cap_in "$dir" "$TRACKC" "$@"
}

# Assertions --------------------------------------------------------------

eq() { # eq GOT WANT MSG
    [ "$1" = "$2" ] || fail "$3: got [$1], want [$2]"
}

ne() { # ne GOT UNWANTED MSG
    [ "$1" != "$2" ] || fail "$3: got [$1]"
}

rc_is() { # rc_is WANT MSG  (against the last cap)
    [ "$RC" = "$1" ] || fail "$2: exit $RC, want $1
$(ctx)"
}

rc_ok() { rc_is 0 "$1"; }

rc_fail() { # rc_fail MSG: any non-zero exit
    [ "$RC" != 0 ] || fail "$1: exit 0, want failure
$(ctx)"
}

has() { # has HAYSTACK NEEDLE MSG
    case "$1" in
        *"$2"*) ;;
        *) fail "$3: missing [$2] in:
$1" ;;
    esac
}

lacks() { # lacks HAYSTACK NEEDLE MSG
    case "$1" in
        *"$2"*) fail "$3: unexpected [$2] in:
$1" ;;
        *) ;;
    esac
}

starts() { # starts HAYSTACK PREFIX MSG
    case "$1" in
        "$2"*) ;;
        *) fail "$3: does not start with [$2]:
$1" ;;
    esac
}

# count HAYSTACK NEEDLE: non-overlapping occurrences (str::matches().count()).
count() {
    local hay=$1 needle=$2 n=0
    [ -n "$needle" ] || { echo 0; return; }
    while :; do
        case "$hay" in
            *"$needle"*) n=$((n + 1)); hay=${hay#*"$needle"} ;;
            *) break ;;
        esac
    done
    echo "$n"
}

# pos HAYSTACK NEEDLE: byte offset of the first match, or a huge number.
pos() {
    case "$1" in
        *"$2"*) local pre=${1%%"$2"*}; echo "${#pre}" ;;
        *) echo 999999999 ;;
    esac
}

# out_is FILE MSG: the last captured stdout equals FILE byte for byte.
out_is() {
    cmp -s "$T/.out" "$1" || fail "$2: stdout differs from $1
$(diff "$1" "$T/.out" | head -n 40)"
}

# same_file A B MSG
same_file() {
    cmp -s "$1" "$2" || fail "$3: $1 and $2 differ"
}

# Programs ----------------------------------------------------------------

# observe EXE [ARGS...]: run EXE from $T; stdout in $T/.out, stderr $T/.err,
# status RC (a signal is 128+N, so SIGFPE is 136).
observe() { cap "$@"; }

# same_run A B MSG: two observed runs (stdout file + code) must agree.
# Usage: observe a; save_run a; observe b; save_run b; same_run a b msg
save_run() {
    cp "$T/.out" "$T/.run_$1.out"
    echo "$RC" >"$T/.run_$1.rc"
}
same_run() {
    cmp -s "$T/.run_$1.out" "$T/.run_$2.out" \
        || fail "$3: stdout differs
$(diff "$T/.run_$1.out" "$T/.run_$2.out" | head -n 40)"
    [ "$(cat "$T/.run_$1.rc")" = "$(cat "$T/.run_$2.rc")" ] \
        || fail "$3: exit $(cat "$T/.run_$1.rc") vs $(cat "$T/.run_$2.rc")"
}

# build SRC EXE [FLAGS...]: trackc build, must succeed.
build() {
    local src=$1 exe=$2
    shift 2
    trackc build "$@" "$src" -o "$exe"
    rc_ok "build $* $src"
}

build_native() { build "$1" "$2" --native; }

# write FILE TEXT: write TEXT verbatim (printf %s, no added newline).
write() { printf '%s' "$2" >"$1"; }

# trk_files DIR: sorted *.trk paths in DIR (one per line).
trk_files() {
    local f
    for f in "$1"/*.trk; do
        [ -e "$f" ] && echo "$f"
    done | LC_ALL=C sort
}

stem() { local b=${1##*/}; echo "${b%.*}"; }

# Toolchain probes ----------------------------------------------------------

# have_asan: gcc can build and run an AddressSanitizer program (cached).
have_asan() {
    if [ -z "${TRACK_HAVE_ASAN:-}" ]; then
        TRACK_HAVE_ASAN=no
        if command -v gcc >/dev/null 2>&1; then
            local d
            d=$(mktemp -d "$T/.asan.XXXX")
            printf 'int main(void){return 0;}\n' >"$d/a.c"
            if gcc -fsanitize=address "$d/a.c" -o "$d/a" >/dev/null 2>&1 && "$d/a" >/dev/null 2>&1; then
                TRACK_HAVE_ASAN=yes
            fi
            rm -rf "$d"
        fi
        export TRACK_HAVE_ASAN
    fi
    [ "$TRACK_HAVE_ASAN" = yes ]
}

have() { command -v "$1" >/dev/null 2>&1; }

skip() {
    note "SKIP: $*"
    exit 0
}

# Binary files --------------------------------------------------------------

# u8_at/u16_at/u32_at/u64_at FILE OFFSET: little-endian unsigned integer.
u8_at() { od -An -v -t u1 -j "$2" -N 1 "$1" | tr -d ' \n'; }
u16_at() { od -An -v -t u2 --endian=little -j "$2" -N 2 "$1" | tr -d ' \n'; }
u32_at() { od -An -v -t u4 --endian=little -j "$2" -N 4 "$1" | tr -d ' \n'; }
u64_at() { od -An -v -t u8 --endian=little -j "$2" -N 8 "$1" | tr -d ' \n'; }

fsize() { wc -c <"$1" | tr -d ' '; }

# grep_bytes FILE STRING: FILE contains STRING as raw bytes.
grep_bytes() { LC_ALL=C grep -qaF -- "$2" "$1"; }

# in_repo CMD...: cap with the repository root as the working directory.
in_repo() { cap_in "$ROOT" "$@"; }

# stdout_is TEXT MSG: the last captured stdout is exactly TEXT (printf
# escapes such as \n are expanded; trailing newlines count).
stdout_is() {
    printf -- "$1" >"$T/.want"
    cmp -s "$T/.out" "$T/.want" || fail "$2: stdout differs
$(diff "$T/.want" "$T/.out" | head -n 40)"
}
