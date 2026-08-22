# Track Performance Benchmark Report

_Generated automatically by `scratch/benchmark.py` on 2026-08-22 14:07 22_
_Host: Linux 7.1.8-arch1-3 (x86_64) — unknown_
_Toolchain: track 0.6.0 | gcc (GCC) 16.2.1 20260810 | rustc 1.96.0 (ac68faa20 2026-05-25) | Python 3.14.7_
_Config: runs=5, warmup=1, median reported; relative speed = track_median / other_median_

## Comparative Benchmark Results

| Benchmark Task | Language | Median (s) | Mean (s) | Stdev (s) | vs Track |
|---|---|---|---|---|---|
| Loop Accumulation (100M &7) | **Track (Cranelift)** | 0.1569 | 0.1569 | 0.0184 | 1.00× |
| Loop Accumulation (100M &7) | C (`gcc -O3`) | 0.2309 | 0.2306 | 0.0036 | 1.47× slower |
| Loop Accumulation (100M &7) | Rust (`rustc -O`) | 0.2319 | 0.2326 | 0.0014 | 1.48× slower |
| Loop Accumulation (100M &7) | Python 3 | 27.3152 | 27.3152 | 0.3376 | 174.12× slower |
| Recursive Fibonacci fib(38) | **Track (Cranelift)** | 0.0017 | 0.0018 | 0.0005 | 1.00× |
| Recursive Fibonacci fib(38) | C (`gcc -O3`) | 0.2504 | 0.2510 | 0.0021 | 149.42× slower |
| Recursive Fibonacci fib(38) | Rust (`rustc -O`) | 0.4420 | 0.4436 | 0.0039 | 263.69× slower |
| Recursive Fibonacci fib(38) | Python 3 | 14.8316 | 14.8316 | 0.0070 | 8849.21× slower |
| Branch-Heavy Loop (50M) | **Track (Cranelift)** | 0.0820 | 0.0842 | 0.0120 | 1.00× |
| Branch-Heavy Loop (50M) | C (`gcc -O3`) | 0.1215 | 0.1218 | 0.0017 | 1.48× slower |
| Branch-Heavy Loop (50M) | Rust (`rustc -O`) | 0.1313 | 0.1327 | 0.0031 | 1.60× slower |
| Branch-Heavy Loop (50M) | Python 3 | 17.0307 | 17.0307 | 3.6265 | 207.73× slower |
| Tuple Create/Destructure (10M) | **Track (Cranelift)** | 0.0207 | 0.0208 | 0.0004 | 1.00× |
| Tuple Create/Destructure (10M) | C (`gcc -O3`) | 0.0287 | 0.0288 | 0.0002 | 1.39× slower |
| Tuple Create/Destructure (10M) | Rust (`rustc -O`) | 0.0271 | 0.0270 | 0.0007 | 1.31× slower |
| Tuple Create/Destructure (10M) | Python 3 | 5.9271 | 5.9271 | 0.0934 | 286.66× slower |

### Notes

- Track binaries are built with `track build` (Cranelift, `opt_level=speed`, `is_pic=false`) and linked via `cc -O3 -lm -no-pie`.
- C is `gcc -O3`, Rust is `rustc -O` — both in release mode, single-file, no LTO tuning.
- Python is CPython as found on `PATH`; 100M-iteration loops dominate its runtime.
- Each task recompiles fresh sources in `scratch/benchmarks/` and is timed end-to-end (process spawn included) with warmup runs discarded; median is the primary metric.
- Output values are cross-checked across languages; mismatches are warned but do not abort the suite.

