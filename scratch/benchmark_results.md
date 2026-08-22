# Track Performance Benchmark Report

_Generated automatically by `scratch/benchmark.py` on 2026-08-22 13:00 10_
_Host: Linux 7.1.8-arch1-3 (x86_64) — unknown_
_Toolchain: track 0.6.0 | gcc (GCC) 16.2.1 20260810 | rustc 1.96.0 (ac68faa20 2026-05-25) | Python 3.14.7_
_Config: runs=5, warmup=1, median reported; relative speed = track_median / other_median_

## Comparative Benchmark Results

| Benchmark Task | Language | Median (s) | Mean (s) | Stdev (s) | vs Track |
|---|---|---|---|---|---|
| Loop Accumulation (100M &7) | **Track (Cranelift)** | 0.1364 | 0.1334 | 0.0090 | 1.00× |
| Loop Accumulation (100M &7) | C (`gcc -O3`) | 0.0604 | 0.0608 | 0.0048 | 2.26× faster |
| Loop Accumulation (100M &7) | Rust (`rustc -O`) | 0.0310 | 0.0301 | 0.0023 | 4.40× faster |
| Loop Accumulation (100M &7) | Python 3 | 25.7743 | 25.7743 | 1.3953 | 188.96× slower |
| Recursive Fibonacci fib(38) | **Track (Cranelift)** | 0.4860 | 0.4854 | 0.0029 | 1.00× |
| Recursive Fibonacci fib(38) | C (`gcc -O3`) | 0.1746 | 0.1754 | 0.0097 | 2.78× faster |
| Recursive Fibonacci fib(38) | Rust (`rustc -O`) | 0.2641 | 0.2647 | 0.0032 | 1.84× faster |
| Recursive Fibonacci fib(38) | Python 3 | 13.8404 | 13.8404 | 0.4402 | 28.48× slower |
| Branch-Heavy Loop (50M) | **Track (Cranelift)** | 0.5576 | 0.5562 | 0.0107 | 1.00× |
| Branch-Heavy Loop (50M) | C (`gcc -O3`) | 0.0356 | 0.0356 | 0.0024 | 15.67× faster |
| Branch-Heavy Loop (50M) | Rust (`rustc -O`) | 0.0600 | 0.0589 | 0.0045 | 9.29× faster |
| Branch-Heavy Loop (50M) | Python 3 | 16.8748 | 16.8748 | 1.6321 | 30.27× slower |
| Tuple Create/Destructure (10M) | **Track (Cranelift)** | 0.0216 | 0.0221 | 0.0011 | 1.00× |
| Tuple Create/Destructure (10M) | C (`gcc -O3`) | 0.0014 | 0.0014 | 0.0001 | 14.95× faster |
| Tuple Create/Destructure (10M) | Rust (`rustc -O`) | 0.0014 | 0.0014 | 0.0001 | 15.76× faster |
| Tuple Create/Destructure (10M) | Python 3 | 7.2954 | 7.2954 | 0.0958 | 337.76× slower |

### Notes

- Track binaries are built with `track build` (Cranelift, `opt_level=speed`, `is_pic=false`) and linked via `cc -O3 -lm -no-pie`.
- C is `gcc -O3`, Rust is `rustc -O` — both in release mode, single-file, no LTO tuning.
- Python is CPython as found on `PATH`; 100M-iteration loops dominate its runtime.
- Each task recompiles fresh sources in `scratch/benchmarks/` and is timed end-to-end (process spawn included) with warmup runs discarded; median is the primary metric.
- Output values are cross-checked across languages; mismatches are warned but do not abort the suite.

