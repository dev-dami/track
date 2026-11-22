#!/bin/bash
# Cargo-free test runner for Track (trackc, its runtimes, the native
# backend, the bootstrap and the tools).
#
#   tests/run.sh [-j N] [-t SECS] [-k] [-l] [PATTERN...]
#
#   -j N      run N tests at once (default 1; TEST_JOBS)
#   -t SECS   per-test timeout (default 600; TEST_TIMEOUT)
#   -k        keep every test's scratch directory (failed ones are kept
#             unless -K is given)
#   -K        keep nothing, not even a failed test's scratch
#   -l        list the tests and exit
#   PATTERN   run only tests whose id (suite/name) contains a PATTERN
#
# Tests are bash functions `t_<name>` in tests/sh/<suite>.sh (helpers in
# tests/sh/lib.sh). Each runs in its own process under `timeout`, in an
# empty scratch directory. The runner prints one PASS/FAIL/TIMEOUT line per
# test, then a summary, and exits non-zero if any test failed.
#
# trackc under test: $TRACKC, else build/bootstrap/stage3/trackc (what
# scripts/bootstrap.sh leaves; BOOTSTRAP_DIR is honoured). Scratch goes
# under $TEST_TMP (default /tmp). Needs bash and POSIX tools only.
set -u

here=$(cd "$(dirname "$0")" && pwd)
ROOT=$(cd "$here/.." && pwd)
export ROOT

# --exec ID: the timed child; run one test function.
if [ "${1:-}" = --exec ]; then
    id=$2
    suite=${id%%/*}
    name=${id#*/}
    # shellcheck source=/dev/null
    . "$here/sh/lib.sh"
    # shellcheck source=/dev/null
    . "$here/sh/$suite.sh"
    ulimit -c 0 2>/dev/null
    cd "$T" || exit 1
    "t_$name"
    exit $?
fi

# --one ID: run ID under the timeout, print its line, record the verdict.
if [ "${1:-}" = --one ]; then
    id=$2
    safe=$(printf '%s' "$id" | tr '/' '.')
    T="$TEST_BASE/t/$safe"
    rm -rf "$T"
    mkdir -p "$T"
    export T
    log="$TEST_BASE/log/$safe.log"
    start=$(date +%s)
    timeout -k 10 "$TEST_TIMEOUT" bash "$0" --exec "$id" >"$log" 2>&1
    rc=$?
    secs=$(($(date +%s) - start))
    if [ "$rc" = 0 ]; then
        printf 'PASS %s (%ss)\n' "$id" "$secs"
        grep '^SKIP:' "$log" | sed "s|^|     |"
        : >"$TEST_BASE/res/$safe.pass"
        [ "$TEST_KEEP" = all ] || rm -rf "$T"
    else
        if [ "$rc" = 124 ] || [ "$rc" = 137 ]; then
            printf 'TIMEOUT %s (after %ss)\n' "$id" "$secs"
        else
            printf 'FAIL %s (exit %s, %ss)\n' "$id" "$rc" "$secs"
        fi
        tail -n 40 "$log" | sed 's/^/    | /'
        : >"$TEST_BASE/res/$safe.fail"
        [ "$TEST_KEEP" = none ] && rm -rf "$T"
    fi
    exit 0
fi

jobs=${TEST_JOBS:-1}
TEST_TIMEOUT=${TEST_TIMEOUT:-600}
TEST_KEEP=failed
list=0
while getopts 'j:t:kKlh' opt; do
    case "$opt" in
        j) jobs=$OPTARG ;;
        t) TEST_TIMEOUT=$OPTARG ;;
        k) TEST_KEEP=all ;;
        K) TEST_KEEP=none ;;
        l) list=1 ;;
        h) sed -n '2,25p' "$0" | sed 's/^# \{0,1\}//'; exit 0 ;;
        *) exit 2 ;;
    esac
done
shift $((OPTIND - 1))

ids=$(
    for f in "$here"/sh/*.sh; do
        suite=$(basename "$f" .sh)
        [ "$suite" = lib ] && continue
        sed -n 's/^t_\([A-Za-z0-9_]*\)() *{.*/\1/p' "$f" | sed "s|^|$suite/|"
    done
)
if [ $# -gt 0 ]; then
    ids=$(printf '%s\n' "$ids" | while read -r id; do
        for p in "$@"; do
            case "$id" in *"$p"*) echo "$id"; break ;; esac
        done
    done)
fi
[ -n "$ids" ] || { echo "run.sh: no tests match" >&2; exit 2; }
if [ "$list" = 1 ]; then
    printf '%s\n' "$ids"
    exit 0
fi

if [ -z "${TRACKC:-}" ]; then
    TRACKC="${BOOTSTRAP_DIR:-$ROOT/build/bootstrap}/stage3/trackc"
fi
case "$TRACKC" in /*) ;; *) TRACKC="$(pwd)/$TRACKC" ;; esac
[ -x "$TRACKC" ] || { echo "run.sh: no trackc at $TRACKC (run scripts/bootstrap.sh or set TRACKC)" >&2; exit 2; }
export TRACKC

TEST_BASE=$(mktemp -d "${TEST_TMP:-/tmp}/track-tests.XXXXXX")
mkdir -p "$TEST_BASE/t" "$TEST_BASE/log" "$TEST_BASE/res"
# One runtime-object cache for the run (tests that check caching set their
# own TRACKC_CACHE).
TRACKC_CACHE=${TRACKC_CACHE:-$TEST_BASE/cache}
export TEST_BASE TEST_TIMEOUT TEST_KEEP TRACKC_CACHE

n=$(printf '%s\n' "$ids" | wc -l)
echo "running $n tests with $TRACKC (-j $jobs, timeout ${TEST_TIMEOUT}s)"
start=$(date +%s)
printf '%s\n' "$ids" | xargs -P "$jobs" -I{} bash "$0" --one {}
secs=$(($(date +%s) - start))

pass=$(find "$TEST_BASE/res" -name '*.pass' | wc -l)
failed=$(find "$TEST_BASE/res" -name '*.fail' | sed 's|.*/||; s|\.fail$||' | LC_ALL=C sort)
nfail=$(printf '%s' "$failed" | grep -c . || true)
missing=$((n - pass - nfail))
echo "---"
echo "$pass passed, $nfail failed, $missing missing, ${secs}s"
if [ "$nfail" -gt 0 ] || [ "$missing" -gt 0 ]; then
    printf '%s\n' "$failed" | sed 's/^/  failed: /'
    [ "$TEST_KEEP" = none ] || echo "logs and failed scratch in $TEST_BASE"
    exit 1
fi
rm -rf "$TEST_BASE"
exit 0
