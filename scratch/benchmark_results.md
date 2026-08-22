# Track Performance Benchmark Report

_Generated automatically by `scratch/benchmark.py` on 2026-08-22 16:20 23_
_Host: Linux 7.1.8-arch1-3 (x86_64) — unknown_
_Toolchain: track 0.6.0 | gcc (GCC) 16.2.1 20260810 | rustc 1.96.0 (ac68faa20 2026-05-25) | Python 3.14.7_
_Config: runs=5, warmup=1, median reported; relative speed = track_median / other_median_

## Comparative Benchmark Results

| Benchmark Task | Language | Median (s) | Mean (s) | Stdev (s) | vs Track |
|---|---|---|---|---|---|
| Loop Accumulation (100M &7) | **Track (Cranelift)** | 0.1173 | 0.1182 | 0.0082 | 1.00× |
| Loop Accumulation (100M &7) | C (`gcc -O3`) | 0.0569 | 0.0573 | 0.0038 | 2.06× faster |
| Loop Accumulation (100M &7) | Rust (`rustc -O`) | 0.0330 | 0.0319 | 0.0014 | 3.55× faster |
| Loop Accumulation (100M &7) | Python 3 | 23.0819 | 23.0819 | 0.0413 | 196.77× slower |
| Recursive Fibonacci fib(38) | **Track (Cranelift)** | 0.4808 | 0.4739 | 0.0134 | 1.00× |
| Recursive Fibonacci fib(38) | C (`gcc -O3`) | 0.1734 | 0.1712 | 0.0059 | 2.77× faster |
| Recursive Fibonacci fib(38) | Rust (`rustc -O`) | 0.2808 | 0.2818 | 0.0028 | 1.71× faster |
| Recursive Fibonacci fib(38) | Python 3 | 15.1748 | 15.1748 | 2.3252 | 31.56× slower |
| Branch-Heavy Loop (50M) | **Track (Cranelift)** | 0.4936 | 0.4966 | 0.0070 | 1.00× |
| Branch-Heavy Loop (50M) | C (`gcc -O3`) | 0.0359 | 0.0348 | 0.0022 | 13.74× faster |
| Branch-Heavy Loop (50M) | Rust (`rustc -O`) | 0.0535 | 0.0553 | 0.0064 | 9.23× faster |
| Branch-Heavy Loop (50M) | Python 3 | 14.6039 | 14.6039 | 0.4937 | 29.58× slower |
| Tuple Create/Destructure (10M) | **Track (Cranelift)** | 0.0207 | 0.0209 | 0.0003 | 1.00× |
| Tuple Create/Destructure (10M) | C (`gcc -O3`) | 0.0010 | 0.0010 | 0.0000 | 21.21× faster |
| Tuple Create/Destructure (10M) | Rust (`rustc -O`) | 0.0012 | 0.0012 | 0.0001 | 17.19× faster |
| Tuple Create/Destructure (10M) | Python 3 | 6.6320 | 6.6320 | 0.2114 | 319.70× slower |

### Notes

- Track binaries are built with `track build` (Cranelift, `opt_level=speed`, `is_pic=false`) and linked via `cc -O3 -lm -no-pie`.
- C is `gcc -O3`, Rust is `rustc -O` — both in release mode, single-file, no LTO tuning.
- Python is CPython as found on `PATH`; 100M-iteration loops dominate its runtime.
- Each task recompiles fresh sources in `scratch/benchmarks/` and is timed end-to-end (process spawn included) with warmup runs discarded; median is the primary metric.
- Output values are cross-checked across languages; mismatches are warned but do not abort the suite.

