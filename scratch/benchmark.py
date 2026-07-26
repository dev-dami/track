import os
import subprocess
import time
import statistics
import sys
from pathlib import Path

BENCH_DIR = Path("/home/dev/track/scratch/benchmarks")
TRACK_BIN = Path("/home/dev/track/target/release/track")
YARD_BIN = Path("/home/dev/track/target/release/yard")
RESULTS_MD = Path("/home/dev/track/scratch/benchmark_results.md")

BENCH_DIR.mkdir(parents=True, exist_ok=True)

# ---------------------------------------------------------
# Benchmark Sources
# ---------------------------------------------------------

# 1. Loop Throughput Benchmark (1 billion iterations)
LOOP_TRACK = """
fn main() -> void {
    let mut i: i64 = 0;
    let mut sum: i64 = 0;
    while i < 100000000 {
        sum = sum + (i & 7);
        i = i + 1;
    }
    print(sum);
}
"""

LOOP_C = """
#include <stdio.h>
int main() {
    long long sum = 0;
    for (long long i = 0; i < 100000000; i++) {
        sum += (i & 7);
    }
    printf("%lld\\n", sum);
    return 0;
}
"""

LOOP_RS = """
fn main() {
    let mut sum: i64 = 0;
    for i in 0..100000000i64 {
        sum += i & 7;
    }
    println!("{}", sum);
}
"""

LOOP_PY = """
sum_val = 0
for i in range(100000000):
    sum_val += i & 7
print(sum_val)
"""

# 2. Recursive Function Call Benchmark (fib(38))
REC_TRACK = """
fn fib(n: i64) -> i64 {
    if n <= 1 { return n; }
    return fib(n - 1) + fib(n - 2);
}
fn main() -> void {
    let res = fib(38);
    print(res);
}
"""

REC_C = """
#include <stdio.h>
long long fib(long long n) {
    if (n <= 1) return n;
    return fib(n - 1) + fib(n - 2);
}
int main() {
    printf("%lld\\n", fib(38));
    return 0;
}
"""

REC_RS = """
fn fib(n: i64) -> i64 {
    if n <= 1 { return n; }
    fib(n - 1) + fib(n - 2)
}
fn main() {
    println!("{}", fib(38));
}
"""

REC_PY = """
def fib(n):
    if n <= 1: return n
    return fib(n - 1) + fib(n - 2)
print(fib(38))
"""

# ---------------------------------------------------------
# Runner Functions
# ---------------------------------------------------------

def measure(cmd, runs=3):
    durations = []
    for _ in range(runs):
        start = time.perf_counter()
        res = subprocess.run(cmd, stdout=subprocess.PIPE, stderr=subprocess.PIPE, text=True)
        end = time.perf_counter()
        if res.returncode != 0:
            print(f"Error running {cmd}: {res.stderr}")
            return None
        durations.append(end - start)
    return statistics.median(durations)

def run_benchmarks():
    print("==================================================")
    print("       TRACK BENCHMARK SUITE VS C / RUST / PYTHON ")
    print("==================================================")
    
    tasks = [
        ("Loop Accumulation (100M Iterations)", "loop", LOOP_TRACK, LOOP_C, LOOP_RS, LOOP_PY),
        ("Recursive Fibonacci (fib(38))", "rec", REC_TRACK, REC_C, REC_RS, REC_PY),
    ]

    report_lines = [
        "# Track Performance Benchmark Report\n",
        "## Comparative Benchmark Results\n",
        "| Benchmark Task | Language | Median Execution Time (s) | Relative Speed vs Track |\n",
        "|---|---|---|---|\n"
    ]

    for name, tag, code_track, code_c, code_rs, code_py in tasks:
        print(f"\n[Task: {name}]")
        
        # Files
        f_trk = BENCH_DIR / f"{tag}.trk"
        f_obj = BENCH_DIR / f"{tag}.o"
        f_trk_bin = BENCH_DIR / f"{tag}_trk"
        
        f_c = BENCH_DIR / f"{tag}.c"
        f_c_bin = BENCH_DIR / f"{tag}_c"
        
        f_rs = BENCH_DIR / f"{tag}.rs"
        f_rs_bin = BENCH_DIR / f"{tag}_rs"

        f_py = BENCH_DIR / f"{tag}.py"

        f_trk.write_text(code_track)
        f_c.write_text(code_c)
        f_rs.write_text(code_rs)
        f_py.write_text(code_py)

        # 1. Compile Track
        subprocess.run([str(TRACK_BIN), str(f_trk)], check=True)
        built_bin = Path(f"/home/dev/track/{tag}")
        if built_bin.exists():
            built_bin.rename(f_trk_bin)

        # 2. Compile C (gcc -O3)
        subprocess.run(["gcc", "-O3", str(f_c), "-o", str(f_c_bin)], check=True)

        # 3. Compile Rust (rustc -O)
        subprocess.run(["rustc", "-O", str(f_rs), "-o", str(f_rs_bin)], check=True)

        # Measure Track
        t_track = measure([str(f_trk_bin)])
        # Measure C
        t_c = measure([str(f_c_bin)])
        # Measure Rust
        t_rust = measure([str(f_rs_bin)])
        # Measure Python
        t_py = measure([sys.executable, str(f_py)], runs=2)

        print(f"  - Track (Cranelift) : {t_track:.4f} s (1.00x)")
        print(f"  - C (gcc -O3)       : {t_c:.4f} s ({t_track / t_c:.2f}x faster)")
        print(f"  - Rust (rustc -O)   : {t_rust:.4f} s ({t_track / t_rust:.2f}x faster)")
        print(f"  - Python 3          : {t_py:.4f} s ({t_py / t_track:.2f}x slower than Track)")

        report_lines.append(f"| {name} | **Track** | **{t_track:.4f}s** | 1.00x |")
        report_lines.append(f"| | C (`gcc -O3`) | {t_c:.4f}s | {t_track/t_c:.2f}x faster |")
        report_lines.append(f"| | Rust (`rustc -O`) | {t_rust:.4f}s | {t_track/t_rust:.2f}x faster |")
        report_lines.append(f"| | Python 3 | {t_py:.4f}s | {t_py/t_track:.2f}x slower |")

    report_content = "\n".join(report_lines)
    RESULTS_MD.write_text(report_content)
    print("\nBenchmark completed successfully! Results written to:", RESULTS_MD)

if __name__ == "__main__":
    run_benchmarks()
