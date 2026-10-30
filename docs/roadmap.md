# Track Language Roadmap: Path to Self-Bootstrapping (v0.1.6 → v1.0.0)

This roadmap outlines the incremental milestone releases for the Track programming language and toolchain leading up to full compiler self-bootstrapping, reached in **v1.0.0**.

---

## Progress Overview

```
[v0.1.6] Slices & Primitives        ✅ DONE
[v0.2.0] UFCS, Lenses & CFG Merging ✅ DONE
[v0.3.0] Perf & Self-Hosting Prep   ✅ DONE
[v0.4.0] Tuples & Pattern Matching  ✅ DONE
[v0.5.0] Explicit Error Handling    ✅ DONE
[v0.6.0] Monomorphized Generics     ✅ DONE
[v0.7.0] Track Lexer in Track       ✅ DONE
[v0.8.0] Track Parser, Checker & Codegen ✅ DONE
[v0.9.0] Self-Bootstrapping Verified     ✅ DONE
[v1.0.0] Self-Hosted Release            ✅ DONE
```

---

## Milestone Breakdown

### v0.1.6 — Slices, Strings & Memory Primitives ✅
- First-class slices (`[]T`) with sub-slicing `arr[start..end]` (length via `array_len` since v1.0).
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

### v0.5.0 — Explicit Stack-Based Error Handling ✅
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
- **Stdlib Convention Audit** ✅:
  - Failing functions standardized on the `(value, err)` / out-param / predicate convention.
  - Ambiguous sentinels disambiguated with predicate companions (`str_is_int`, `env_exists`).
- **Fatal Abort Primitive** ✅:
  - `abort(msg)` — print message and exit the process with status 134; no unwinding, frames are simply discarded.

---

### v0.6.0 — Monomorphized Generics ✅
- **Generic Function Definitions** ✅:
  - `fn identity<T>(x: T) -> T` with inference at call sites.
  - Multi-param functions (`fn pair<T, U>(a: T, b: U) -> (T, U)`) and nested generic calls.
  - Compile-time monomorphization pass (`src/mono.rs`) producing per-site mangled concrete functions (`identity__Ti32`).
- **Generic Structs** ⏳ — `struct Stack<T>` parses the `<T>` header; full field-generic substitution and `Stack<T>` as a type argument are planned as a follow-on (type `Stack<i32>` syntax).

---

### v0.7.0 — Self-Hosted Compiler Component 1: Track Lexer in Track (`compiler/src/`) ✅
- **Native Tokenizer** (`compiler/src/token.trk` + `compiler/src/lexer.trk`):
  - Ported `src/lexer/mod.rs` (logos) to 100% native Track: `union Token` (50+ variants), `lex_next`/`lex_string`/`lex_number`/`lex_ident` with `str_char_at`/`str_len`/`str_substr`/`char_is_*`.
  - `compiler` is a `yard` package (`compiler/Track.toml` → `compiler/target/trackc`); artifacts isolated from repo root.
  - Split from monolithic `compiler/lexer.trk` into `token.trk` (types) + `lexer.trk` (scanning) + `main.trk` (demo/count/dump).
- **Checker & Runtime Fixes**:
  - `src/checker/mod.rs:78` — `is_copy_var` treats `::` variant ctors as copy (no Spent/Active merge).
  - `src/checker/mod.rs:1090` — local `import "lexer"` fallback loading `src/*.trk` / `compiler/src/*.trk` for split modules.
  - `src/lib.rs:462` — missing `str_len`/`str_eq` runtime (were declared but not linked).
  - `src/yard/*` — `yard test` (discovers `src/*_test.trk` + `tests/**/*.trk`, builds temp package per test, runs).
- **Native Unit Test Suite** (`compiler/src/lexer_test.trk` + `yard test`):
  - 10 tests via `count_tokens` (let/import/fn/if-else/ops/cmp/logic/shift/string+comment) + `lex_next` kind check; `yard test` ✓ `All 1 Track test(s) passed`.

---

### v0.8.0a — Native Compiler Foundations ✅
- **Lexer parity gate** ✅: native lexer recognizes every host reserved word and
  primitive type before parser work begins; native tests prevent regressions.
- **Source spans and diagnostics** ✅: normalized end-exclusive byte spans,
  one-based source positions, deterministic code-frame formatting, and native
  golden fixtures are implemented under `compiler/src/` and `compiler/tests/`.
- **AST, collections, and parser subset** ✅: compact source-ranged AST nodes,
  growable opaque 64-bit storage, arithmetic precedence, foundational
  statements, and structured parser errors are implemented and natively tested.
- **Backend contract** ✅: the self-hosted compiler emits portable C as its first
  backend and invokes the platform C compiler through the existing explicit OS
  process API. Cranelift remains the bootstrap compiler backend, not a library
  to reimplement in Track. See [bootstrap.md](bootstrap.md#c-backend-contract).
- **Bootstrap build ordering and artifacts** ✅: Yard sorts module discovery,
  dependency resolution, build failures, link inputs, and cache metadata.
  Source-relative object paths prevent duplicate module basenames from racing
  or colliding with the runtime. Stage 2/3 binary identity remains the v0.9.0 gate.

### v0.8.0b — Native Parser ✅
- Full grammar in `compiler/src/parser.trk` over a node pool (`ast.trk`), with
  line/column code frames for every lexer and parser error.
### v0.8.0c — Native Checker ✅
- The ownership checker (moves, borrows, lenses, CFG merge, loop rules, escape
  analysis) in `compiler/src/checker.trk`, verdict-for-verdict with the Rust checker.
### v0.8.0d — Native Codegen ✅
- Portable C99 emitter (`compiler/src/cgen.trk`) on the shared word-ABI runtime
  (`runtime/track_runtime.c`).

---

### v0.9.0 — Bootstrap Milestone (Stage 0 → Stage 1 → Stage 2 Verification) ✅
- **Stage 0 (Bootstrap)**: The Rust `track` compiler compiles `compiler/*.trk` → produces `track_stage1` executable.
- **Stage 1 (Self-Compile)**: `track_stage1` compiles `compiler/*.trk` → produces `track_stage2` executable.
- **Stage 2 (Verification)**: `track_stage2` compiles `compiler/*.trk` → produces `track_stage3` executable.
- **Verification Gate**: stage1 and stage2 emit byte-identical C, and `cmp track_stage2 track_stage3` passes. Run with `scripts/bootstrap.sh`.

---

### v1.0.0 — Self-Hosted Release ✅
- `trackc` ships `build`, `run`, `check`, `emit-c`, and `tokens`.
- One word-ABI runtime shared by both compilers; the Cranelift backend moved to
  the same value model, fixing the miscompilations listed in the CHANGELOG.
- CI gates on the bootstrap fixed point and on behavioural parity between the
  two compilers.

- Structs, field access/update, struct patterns, and bounds-checked arrays
  with slicing now have runtime representations in both compilers.

### Next
- Emit deallocation at scope exit from the ownership analysis (linear values
  are currently reclaimed at process exit).
- Generic structs (`struct Stack<T>`), and retiring the Rust compiler once
  `yard` itself is ported.
