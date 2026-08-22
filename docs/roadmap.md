# Track Language Roadmap: Path to Self-Bootstrapping (v0.1.6 → v0.9.0)

This roadmap outlines the incremental milestone releases for the Track programming language and toolchain leading up to full compiler self-bootstrapping in **v0.9.0**.

---

## Progress Overview

```
[v0.1.6] Slices & Primitives        ✅ DONE
[v0.2.0] UFCS, Lenses & CFG Merging ✅ DONE
[v0.3.0] Perf & Self-Hosting Prep   ✅ DONE
[v0.4.0] Tuples & Pattern Matching  ✅ DONE
[v0.5.0] Explicit Error Handling    🚀 CURRENT TARGET
[v0.6.0] Monomorphized Generics     ⏳ PLANNED
[v0.7.0] Track Lexer in Track       ⏳ PLANNED
[v0.8.0] Track Parser, Checker & Codegen ⏳ PLANNED
[v0.9.0] Self-Bootstrapping Verified 🎯 MILESTONE
```

---

## Milestone Breakdown

### v0.1.6 — Slices, Strings & Memory Primitives ✅
- First-class slices (`[]T`) with `.len()` and sub-slicing `arr[start..end]`.
- Owned heap string type (`Str`) and string formatting/concatenation.
- Byte types (`u8`, `i8`) and safe reference borrows (`&T`).
- Variable mutability enforcement (`let mut`).
- Built-in POSIX TCP Socket API (`std/net`).
- Standalone `yard` CLI build tool (`yard init`, `yard build`, `yard run`, `yard add`, `yard check`).

---

### v0.2.0 — UFCS, Lexical Lenses & CFG Merging ✅
- Struct literal disambiguation inside conditionals.
- CFG state merging for branches (`if`/`else`) and loops (`while`).
- Primitive copy semantics via static type inference.
- Array indexing, address-of (`&`), and pointer arithmetic.
- Uniform Function Call Syntax (UFCS).
- Lexical lens blocks (`with ->` expression blocks).

---

### v0.3.0 — Performance & Self-Hosting Readiness ✅
- Shared `Arc<dyn TargetIsa>` across parallel worker threads; eliminated per-file ISA re-initialization.
- Allocation and O(N²) bottleneck eliminations across checker, codegen, LSP, and build cache.
- `yard lint` and `yard clean` commands; `Track.toml` lock file serialization.
- New stdlib functions: `os_args_count`, `os_arg`, `dir_exists`, `file_copy`, `process_spawn`, `sys_exec`, `sys_set_memory_limit`, `sys_get_memory_used`, `env_get`, `str_starts_with`, `str_ends_with`, `str_contains`.
- `track-lsp` binary ships with release builds.

---

### v0.3.x — Type Aliases ✅ / Const Array Sizing ⏳
- **Type Aliases** ✅:
  - Type alias syntax (`type ByteBuf = []u8;`).
- **Const Expressions in Array Sizes** ⏳ PLANNED:
  - Constant array sizing (`const BUF_SIZE = 1024; let buf: [u8; BUF_SIZE];`).

---

### v0.4.0 — Advanced Pattern Matching & Tuples ✅
- **Nested Pattern Matching**:
  - Matching on nested structs, tuples, and tagged unions (`match val { Value::Int(x) => ..., Value::Tuple(a, b) => ... }`).
- **Match Arm Guard Expressions**:
  - Pattern guards (`match x { n if n > 10 => ..., _ => ... }`).
- **Tuples & Destructuring**:
  - Anonymous tuple types (`(i64, Str, bool)`).
  - Destructuring assignment (`let (a, b) = get_pair();`).

---

### v0.5.0 — Explicit Stack-Based Error Handling 🚀 CURRENT TARGET
Track rejects wrapper-based error handling: **no `Option`/`Result` types, no `?`
operator, no hidden control flow, no stack unwinding.** Errors are ordinary
values that live on the call stack and are passed around explicitly.

- **Error Codes as Copy Primitives**:
  - Failing functions return plain status values (`i32` code, `bool` ok) — zero allocation, zero wrapping.
- **Multi-Value Returns via Tuples**:
  - `(T, i32)` returns with v0.4 tuple destructuring (`let (val, err) = read_file(path);`).
  - Linear payloads move out of the error tuple per normal ownership rules.
- **Out-Params via References & Lenses**:
  - C-style explicit outputs (`fn read_all(path: Str, out: &Str) -> i32`) for hot paths.
- **Explicit Propagation**:
  - Callers branch on the error value and return it upward by hand — every error path is visible in the source.
- **Stdlib Convention Audit**:
  - All failing stdlib functions standardized on the `(value, err)` / out-param convention.
- **Fatal Abort Primitive**:
  - `abort(msg)` — print message and exit the process; no unwinding, frames are simply discarded.

---

### v0.6.0 — Monomorphized Generics ⏳ PLANNED
- **Monomorphized Generics**:
  - Generic structs (`struct Stack<T> { data: []T, len: u64 }`).
  - Generic function definitions (`fn identity<T>(x: T) -> T`).
  - Compile-time monomorphization pass producing optimized concrete types.

---

### v0.7.0 — Self-Hosted Compiler Component 1: Track Lexer in Track (`compiler/lexer.trk`)
- **Native Tokenizer**:
  - Port Lexer from Rust to 100% native Track code in `compiler/lexer.trk`.
  - Token stream representation using `union Token` and `[]u8` string slices.
  - Native unit test suite (`yard test`).

---

### v0.8.0 — Self-Hosted Compiler Components 2 & 3: Parser, Checker & Codegen in Track (`compiler/parser.trk`, `compiler/checker.trk` & `compiler/codegen.trk`)
- **Native Parser**:
  - Port AST representation (`union Expr`) and recursive descent parser to native Track code in `compiler/parser.trk`.
  - Detailed compile-time error reporting with source code line/column highlighting.
- **Native Checker**:
  - Port `LinearChecker` borrow-checker, escape analysis, and type inference to Track in `compiler/checker.trk`.
- **Native Codegen**:
  - Port Cranelift / C code emitter to Track in `compiler/codegen.trk`.

---

### v0.9.0 — Bootstrap Milestone (Stage 0 → Stage 1 → Stage 2 Verification) 🎯
- **Stage 0 (Bootstrap)**: The Rust `track` compiler compiles `compiler/*.trk` → produces `track_stage1` executable.
- **Stage 1 (Self-Compile)**: `track_stage1` compiles `compiler/*.trk` → produces `track_stage2` executable.
- **Stage 2 (Verification)**: `track_stage2` compiles `compiler/*.trk` → produces `track_stage3` executable.
- **Verification Gate**: Assert byte-for-byte binary identity between `track_stage2` and `track_stage3` (`cmp track_stage2 track_stage3`). Full self-hosting achieved!
