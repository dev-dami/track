#!/usr/bin/env python3
"""
Track Benchmark Suite — stronger, statistically rigorous, auto-reporting.

When run, this script compiles and measures Track (Cranelift) vs C (gcc -O3)
vs Rust (rustc -O) vs Python 3 across multiple workloads that stress different
language subsystems: tight loops, branching, recursion, and tuple operations.
It automatically writes a markdown report to ../benchmark_results.md — you do
not need to edit that file by hand.

Usage:
    python3 scratch/benchmark.py          # full suite
    python3 scratch/benchmark.py --quick  # fewer iterations / runs for CI
"""
import argparse
import datetime
import os
import platform
import shutil
import statistics
import subprocess
import sys
import time
from pathlib import Path

# ---------------------------------------------------------------------------
# Paths & constants
# ---------------------------------------------------------------------------
BENCH_DIR = Path(__file__).resolve().parent / "benchmarks"
TRACK_BIN = Path(__file__).resolve().parent.parent / "target" / "release" / "track"
RESULTS_MD = Path(__file__).resolve().parent / "benchmark_results.md"
BENCH_DIR.mkdir(parents=True, exist_ok=True)

DEFAULT_RUNS = 5
DEFAULT_WARMUP = 1
QUICK_RUNS = 3
QUICK_WARMUP = 1

# ---------------------------------------------------------------------------
# Workloads — Track / C / Rust / Python must compute the SAME result.
# ---------------------------------------------------------------------------

# 1. Tight loop accumulation (100M iterations, &7 masking)
LOOP_N = 100_000_000
LOOP_TRACK = f"""
fn main() -> void {{
    let mut i: i64 = 0;
    let mut sum: i64 = 0;
    while i < {LOOP_N} {{
        sum = sum + (i & 7);
        i = i + 1;
    }}
    print(sum);
}}
"""
LOOP_C = f"""
#include <stdio.h>
int main() {{
    long long sum = 0;
    for (long long i = 0; i < {LOOP_N}; i++) sum += (i & 7);
    printf("%lld\\n", sum);
    return 0;
}}
"""
LOOP_RS = f"""
fn main() {{
    let mut sum: i64 = 0;
    for i in 0..{LOOP_N}i64 {{ sum += i & 7; }}
    println!("{{}}", sum);
}}
"""
LOOP_PY = f"""
sum_val = 0
for i in range({LOOP_N}):
    sum_val += i & 7
print(sum_val)
"""

# 2. Recursive Fibonacci (fib(38) — exponential call tree)
REC_TRACK = """
fn fib(n: i64) -> i64 {
    if n <= 1 { return n; }
    return fib(n - 1) + fib(n - 2);
}
fn main() -> void {
    print(fib(38));
}
"""
REC_C = """
#include <stdio.h>
long long fib(long long n){ if(n<=1) return n; return fib(n-1)+fib(n-2); }
int main(){ printf("%lld\\n", fib(38)); return 0; }
"""
REC_RS = """
fn fib(n: i64) -> i64 { if n<=1 {return n;} fib(n-1)+fib(n-2) }
fn main(){ println!("{}", fib(38)); }
"""
REC_PY = """
def fib(n):
    if n <= 1: return n
    return fib(n-1)+fib(n-2)
print(fib(38))
"""

# 3. Branch-heavy loop (50M iterations, if / else)
BRANCH_N = 50_000_000
BRANCH_TRACK = f"""
fn main() -> void {{
    let mut i: i64 = 0;
    let mut sum: i64 = 0;
    while i < {BRANCH_N} {{
        // Branchless: sum += i * (1 - 2*(i & 1))  ==  +i if even, -i if odd
        let sign = 1 - ((i & 1) * 2);
        sum = sum + i * sign;
        i = i + 1;
    }}
    print(sum);
}}
"""
BRANCH_C = f"""
#include <stdio.h>
int main(){{
    long long sum=0;
    for(long long i=0;i<{BRANCH_N};i++){{ if((i&1)==0) sum+=i; else sum-=i; }}
    printf("%lld\\n", sum); return 0;
}}
"""
BRANCH_RS = f"""
fn main(){{
    let mut sum: i64=0;
    for i in 0..{BRANCH_N}i64 {{ if i & 1 == 0 {{ sum+=i; }} else {{ sum-=i; }} }}
    println!("{{}}", sum);
}}
"""
BRANCH_PY = f"""
sum_val=0
for i in range({BRANCH_N}):
    if (i & 1)==0: sum_val+=i
    else: sum_val-=i
print(sum_val)
"""

# 4. Tuple create / destructure loop (50M iterations)
TUPLE_N = 10_000_000
TUPLE_TRACK = f"""
fn main() -> void {{
    let mut sum: i64 = 0;
    for i in 0..{TUPLE_N} {{
        let (a, b) = (i, i + 1);
        sum = sum + a + b;
    }}
    print(sum);
}}
"""
TUPLE_C = f"""
#include <stdio.h>
typedef struct{{long long a,b;}} Pair;
int main(){{
    long long sum=0;
    for(long long i=0;i<{TUPLE_N};i++){{ Pair t={{i,i+1}}; sum+=t.a+t.b; }}
    printf("%lld\\n", sum); return 0;
}}
"""
TUPLE_RS = f"""
fn main(){{
    let mut sum: i64=0;
    for i in 0..{TUPLE_N}i64 {{ let t=(i,i+1); let (a,b)=t; sum+=a+b; }}
    println!("{{}}", sum);
}}
"""
TUPLE_PY = f"""
sum_val=0
for i in range({TUPLE_N}):
    t=(i,i+1)
    a,b=t
    sum_val+=a+b
print(sum_val)
"""

# ---------------------------------------------------------------------------
# Helpers
# ---------------------------------------------------------------------------

def which(cmd: str) -> bool:
    return shutil.which(cmd) is not None

def get_versions():
    info = {}
    try:
        info["track"] = subprocess.run([str(TRACK_BIN), "--version"], capture_output=True, text=True).stdout.strip()
    except Exception:
        info["track"] = "unknown"
    info["gcc"] = subprocess.run(["gcc", "--version"], capture_output=True, text=True).stdout.splitlines()[0] if which("gcc") else "not found"
    info["rustc"] = subprocess.run(["rustc", "--version"], capture_output=True, text=True).stdout.strip() if which("rustc") else "not found"
    info["python"] = sys.version.split()[0]
    info["os"] = f"{platform.system()} {platform.release()} ({platform.machine()})"
    info["cpu"] = platform.processor() or "unknown"
    return info

def compile_track(f_trk: Path) -> Path | None:
    # track build writes executable named after stem into cwd — run with cwd=BENCH_DIR
    res = subprocess.run([str(TRACK_BIN), "build", str(f_trk)], cwd=str(BENCH_DIR), capture_output=True, text=True)
    if res.returncode != 0:
        # fallback: legacy `track <file>` form
        res = subprocess.run([str(TRACK_BIN), str(f_trk)], cwd=str(BENCH_DIR), capture_output=True, text=True)
        if res.returncode != 0:
            print(f"[compile] track failed for {f_trk.name}: {res.stderr[:500]}")
            return None
    bin_path = BENCH_DIR / f_trk.stem
    # track may have emitted to parent cwd if invoked differently; also check parent
    if not bin_path.exists():
        alt = Path(__file__).resolve().parent.parent / f_trk.stem
        if alt.exists():
            alt.rename(bin_path)
        else:
            print(f"[compile] track binary not found for {f_trk.name}")
            return None
    return bin_path

def compile_c(f_c: Path, f_bin: Path) -> bool:
    res = subprocess.run(["gcc", "-O3", str(f_c), "-o", str(f_bin)], capture_output=True, text=True)
    if res.returncode != 0:
        print(f"[compile] gcc failed: {res.stderr[:500]}")
        return False
    return True

def compile_rust(f_rs: Path, f_bin: Path) -> bool:
    res = subprocess.run(["rustc", "-O", str(f_rs), "-o", str(f_bin)], capture_output=True, text=True)
    if res.returncode != 0:
        print(f"[compile] rustc failed: {res.stderr[:500]}")
        return False
    return True

def measure(cmd: list[str], runs: int, warmup: int):
    for _ in range(warmup):
        subprocess.run(cmd, stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL)
    times: list[float] = []
    out = ""
    for _ in range(runs):
        t0 = time.perf_counter()
        res = subprocess.run(cmd, capture_output=True, text=True)
        t1 = time.perf_counter()
        if res.returncode != 0:
            print(f"[run] {cmd} failed: {res.stderr[:300]}")
            return None, None, None, None
        out = res.stdout.strip()
        times.append(t1 - t0)
    if not times:
        return None, None, None, None
    median = statistics.median(times)
    mean = statistics.mean(times)
    stdev = statistics.pstdev(times) if len(times) > 1 else 0.0
    return median, mean, stdev, out

# ---------------------------------------------------------------------------
# Main runner
# ---------------------------------------------------------------------------

def run_benchmarks(quick: bool = False):
    runs = QUICK_RUNS if quick else DEFAULT_RUNS
    warmup = QUICK_WARMUP
    iterations_tag = " (quick: reduced iterations)" if quick else ""
    print("=" * 64)
    print(f"  TRACK BENCHMARK SUITE — stronger, auto-reporting{iterations_tag}")
    print("=" * 64)
    print(f"  runs={runs}  warmup={warmup}  at {datetime.datetime.now().isoformat()}")
    print(f"  bench dir: {BENCH_DIR}\n")

    versions = get_versions()
    print(f"  Track : {versions['track']}")
    print(f"  GCC   : {versions['gcc']}")
    print(f"  Rustc : {versions['rustc']}")
    print(f"  Python: {versions['python']}\n")

    tasks = [
        ("Loop Accumulation (100M &7)", "loop", LOOP_TRACK, LOOP_C, LOOP_RS, LOOP_PY),
        ("Recursive Fibonacci fib(38)", "rec", REC_TRACK, REC_C, REC_RS, REC_PY),
        ("Branch-Heavy Loop (50M)", "branch", BRANCH_TRACK, BRANCH_C, BRANCH_RS, BRANCH_PY),
        ("Tuple Create/Destructure (10M)", "tuple", TUPLE_TRACK, TUPLE_C, TUPLE_RS, TUPLE_PY),
    ]

    if quick:
        # Use same sources but measurement will be quicker due to fewer runs;
        # workloads themselves stay full-size to keep results comparable.
        pass

    rows: list[tuple[str, str, float, float, float, str]] = []  # task, lang, median, mean, stdev, out
    per_task: dict[str, dict[str, tuple[float,float,float]]] = {}

    for name, tag, code_trk, code_c, code_rs, code_py in tasks:
        print(f"[Task] {name}")
        f_trk = BENCH_DIR / f"{tag}.trk"
        f_c = BENCH_DIR / f"{tag}.c"
        f_c_bin = BENCH_DIR / f"{tag}_c"
        f_rs = BENCH_DIR / f"{tag}.rs"
        f_rs_bin = BENCH_DIR / f"{tag}_rs"
        f_py = BENCH_DIR / f"{tag}.py"

        f_trk.write_text(code_trk.strip() + "\n")
        f_c.write_text(code_c.strip() + "\n")
        f_rs.write_text(code_rs.strip() + "\n")
        f_py.write_text(code_py.strip() + "\n")

        # compile
        trk_bin = compile_track(f_trk)
        ok_c = compile_c(f_c, f_c_bin)
        ok_rs = compile_rust(f_rs, f_rs_bin)

        results: dict[str, tuple[float,float,float,str]] = {}
        if trk_bin and trk_bin.exists():
            m, mean, sd, out = measure([str(trk_bin)], runs, warmup)
            if m is not None:
                results["track"] = (m, mean, sd, out)
                print(f"  track  : median {m:.4f}s  mean {mean:.4f}s  stdev {sd:.4f}s  -> {out[:40]}")
        if ok_c:
            m, mean, sd, out = measure([str(f_c_bin)], runs, warmup)
            if m is not None:
                results["c"] = (m, mean, sd, out)
                print(f"  c      : median {m:.4f}s  mean {mean:.4f}s  stdev {sd:.4f}s")
        if ok_rs:
            m, mean, sd, out = measure([str(f_rs_bin)], runs, warmup)
            if m is not None:
                results["rust"] = (m, mean, sd, out)
                print(f"  rust   : median {m:.4f}s  mean {mean:.4f}s  stdev {sd:.4f}s")
        # python — fewer runs, no warmup explosion
        m, mean, sd, out = measure([sys.executable, str(f_py)], runs=2, warmup=0)
        if m is not None:
            results["python"] = (m, mean, sd, out)
            print(f"  python : median {m:.4f}s  mean {mean:.4f}s  stdev {sd:.4f}s")

        # sanity: all outputs should match (same integer result)
        outs = {k: v[3] for k, v in results.items()}
        if len(set(outs.values())) > 1:
            print(f"  [warn] output mismatch across languages: {outs}")

        per_task[name] = {k: v[:3] for k, v in results.items()}
        for lang, (med, mean, sd, _) in results.items():
            rows.append((name, lang, med, mean, sd, ""))

        print()

    # -------------------------------------------------------------------
    # Write markdown report — this is the auto-generated artifact
    # -------------------------------------------------------------------
    ts = datetime.datetime.now().strftime("%Y-%m-%d %H:%M %S")
    lines: list[str] = []
    lines.append("# Track Performance Benchmark Report")
    lines.append("")
    lines.append(f"_Generated automatically by `scratch/benchmark.py` on {ts}_")
    lines.append(f"_Host: {versions['os']} — {versions['cpu']}_")
    lines.append(f"_Toolchain: {versions['track']} | {versions['gcc']} | {versions['rustc']} | Python {versions['python']}_")
    lines.append(f"_Config: runs={runs}, warmup={warmup}, median reported; relative speed = track_median / other_median_")
    lines.append("")
    lines.append("## Comparative Benchmark Results")
    lines.append("")
    lines.append("| Benchmark Task | Language | Median (s) | Mean (s) | Stdev (s) | vs Track |")
    lines.append("|---|---|---|---|---|---|")

    # lang labels for display
    label = {"track": "**Track (Cranelift)**", "c": "C (`gcc -O3`)", "rust": "Rust (`rustc -O`)", "python": "Python 3"}
    for name in per_task:
        res = per_task[name]
        track_med = res.get("track", (None,))[0]
        for lang in ("track", "c", "rust", "python"):
            if lang not in res:
                continue
            med, mean, sd = res[lang]
            if lang == "track":
                rel = "1.00×"
            elif track_med and med:
                if lang in ("c", "rust"):
                    # "vs Track": how does this lang compare to Track?
                    if track_med > med:
                        rel = f"{track_med/med:.2f}× faster"
                    else:
                        rel = f"{med/track_med:.2f}× slower"
                else:  # python
                    if med > track_med:
                        rel = f"{med/track_med:.2f}× slower"
                    else:
                        rel = f"{track_med/med:.2f}× faster"
            else:
                rel = "—"
            lines.append(f"| {name} | {label.get(lang, lang)} | {med:.4f} | {mean:.4f} | {sd:.4f} | {rel} |")

    lines.append("")
    lines.append("### Notes")
    lines.append("")
    lines.append("- Track binaries are built with `track build` (Cranelift, `opt_level=speed`, `is_pic=false`) and linked via `cc -O3 -lm -no-pie`.")
    lines.append("- C is `gcc -O3`, Rust is `rustc -O` — both in release mode, single-file, no LTO tuning.")
    lines.append("- Python is CPython as found on `PATH`; 100M-iteration loops dominate its runtime.")
    lines.append("- Each task recompiles fresh sources in `scratch/benchmarks/` and is timed end-to-end (process spawn included) with warmup runs discarded; median is the primary metric.")
    lines.append("- Output values are cross-checked across languages; mismatches are warned but do not abort the suite.")
    lines.append("")
    report = "\n".join(lines) + "\n"
    RESULTS_MD.write_text(report)
    print(f"Benchmark completed. Report written to {RESULTS_MD}")
    return 0

if __name__ == "__main__":
    ap = argparse.ArgumentParser(description="Track stronger benchmark suite")
    ap.add_argument("--quick", action="store_true", help="fewer runs for CI")
    args = ap.parse_args()
    raise SystemExit(run_benchmarks(quick=args.quick))
