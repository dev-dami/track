# Track Performance Benchmark Report

_Generated automatically by `scratch/benchmark.py` on 2026-08-22 13:16 21_
_Host: Linux 7.1.8-arch1-3 (x86_64) — unknown_
_Toolchain: track 0.6.0 | gcc (GCC) 16.2.1 20260810 | rustc 1.96.0 (ac68faa20 2026-05-25) | Python 3.14.7_
_Config: runs=5, warmup=1, median reported; relative speed = track_median / other_median_

## Comparative Benchmark Results

| Benchmark Task | Language | Median (s) | Mean (s) | Stdev (s) | vs Track |
|---|---|---|---|---|---|
| Loop Accumulation (100M &7) | **Track (Cranelift)** | 0.1448 | 0.1531 | 0.0199 | 1.00× |
| Loop Accumulation (100M &7) | C (`gcc -O3`) | 0.0712 | 0.0778 | 0.0147 | 2.04× faster |
| Loop Accumulation (100M &7) | Rust (`rustc -O`) | 0.0337 | 0.0329 | 0.0055 | 4.29× faster |
| Loop Accumulation (100M &7) | Python 3 | 25.5256 | 25.5256 | 0.8328 | 176.25× slower |
| Recursive Fibonacci fib(38) | **Track (Cranelift)** | 0.4837 | 0.4841 | 0.0108 | 1.00× |
| Recursive Fibonacci fib(38) | C (`gcc -O3`) | 0.1641 | 0.1681 | 0.0128 | 2.95× faster |
| Recursive Fibonacci fib(38) | Rust (`rustc -O`) | 0.2651 | 0.2673 | 0.0052 | 1.82× faster |
| Recursive Fibonacci fib(38) | Python 3 | 15.4853 | 15.4853 | 0.3674 | 32.01× slower |
| Branch-Heavy Loop (50M) | **Track (Cranelift)** | 0.1101 | 0.1109 | 0.0061 | 1.00× |
| Branch-Heavy Loop (50M) | C (`gcc -O3`) | 0.0371 | 0.0380 | 0.0035 | 2.97× faster |
| Branch-Heavy Loop (50M) | Rust (`rustc -O`) | 0.0661 | 0.0684 | 0.0074 | 1.67× faster |
| Branch-Heavy Loop (50M) | Python 3 | 14.1946 | 14.1946 | 0.0905 | 128.94× slower |
| Tuple Create/Destructure (10M) | **Track (Cranelift)** | 0.0210 | 0.0211 | 0.0001 | 1.00× |
| Tuple Create/Destructure (10M) | C (`gcc -O3`) | 0.0010 | 0.0010 | 0.0000 | 21.38× faster |
| Tuple Create/Destructure (10M) | Rust (`rustc -O`) | 0.0020 | 0.0020 | 0.0002 | 10.49× faster |
| Tuple Create/Destructure (10M) | Python 3 | 6.6211 | 6.6211 | 0.1403 | 314.67× slower |

### Notes

- Track binaries are built with `track build` (Cranelift, `opt_level=speed`, `is_pic=false`) and linked via `cc -O3 -lm -no-pie`.
- C is `gcc -O3`, Rust is `rustc -O` — both in release mode, single-file, no LTO tuning.
- Python is CPython as found on `PATH`; 100M-iteration loops dominate its runtime.
- Each task recompiles fresh sources in `scratch/benchmarks/` and is timed end-to-end (process spawn included) with warmup runs discarded; median is the primary metric.
- Output values are cross-checked across languages; mismatches are warned but do not abort the suite.

