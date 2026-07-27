# Track Language Roadmap: Path to Self-Bootstrapping (v0.1.6 → v0.9.0)

This roadmap outlines the incremental milestone releases for the Track programming language and toolchain leading up to full compiler self-bootstrapping in **v0.9.0**.

---

## Progress Overview

```
[v0.1.6] Slices & Primitives        ✅ DONE
[v0.2.0] Standard Library & OS       🚀 CURRENT TARGET
[v0.3.0] Monomorphized Generics      ⏳ PLANNED
[v0.4.0] Advanced Pattern Matching   ⏳ PLANNED
[v0.5.0] Option/Result & '?' Op      ⏳ PLANNED
[v0.6.0] Track Lexer in Track        ⏳ PLANNED
[v0.7.0] Track Parser & AST in Track ⏳ PLANNED
[v0.8.0] Track Checker & Codegen     ⏳ PLANNED
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

### v0.2.0 — Extended Standard Library & OS Abstractions 🚀 (Next Up)
- **Directory & File Operations (`std/fs`, `std/path`)**:
  - Directory iteration (`dir_read(path)`), path joins, extension checking, metadata.
- **Process & Environment (`std/os`, `std/process`)**:
  - Command-line argument vector access (`os_args() -> []Str`).
  - Child process spawning (`process_spawn(cmd, args) -> Process`).
- **Dynamic Collections (`std/map`, `std/vec`)**:
  - Dictionary/map implementation (`Map<K, V>`).
  - Vector manipulation utilities (`vec_reserve`, `vec_shrink`, `vec_slice`).

---

### v0.3.0 — Monomorphized Generics & Type Aliases
- **Monomorphized Generics**:
  - Generic structs (`struct Stack<T> { data: []T, len: u64 }`).
  - Generic function definitions (`fn identity<T>(x: T) -> T`).
  - Compile-time monomorphization pass producing optimized concrete types.
- **Type Aliases**:
  - Type alias syntax (`type ByteBuf = []u8;`).
  - Const expressions and constant array sizing (`const BUF_SIZE = 1024; let buf: [u8; BUF_SIZE];`).

---

### v0.4.0 — Advanced Pattern Matching & Tuples
- **Nested Pattern Matching**:
  - Matching on nested structs and tagged unions (`match val { Value::Int(x) => ..., Value::Tuple(a, b) => ... }`).
- **Match Arm Guard Expressions**:
  - Pattern guards (`match x { n if n > 10 => ..., _ => ... }`).
- **Tuples & Destructuring**:
  - Anonymous tuple types (`(i64, Str, bool)`).
  - Destructuring assignment (`let (a, b) = get_pair();`).

---

### v0.5.0 — Core Error Handling (`Option`, `Result` & `?`)
- **First-Class Error Handling Primitives**:
  - `Option<T>` (`Some(T)`, `None`) and `Result<T, E>` (`Ok(T)`, `Err(E)`).
- **Try Operator (`?`)**:
  - Propagation operator (`let content = file_read_all("input.txt")?;`).
- **Panic & Abort Handling**:
  - `panic("message")` runtime handler with stack frame printing.

---

### v0.6.0 — Self-Hosted Compiler Component 1: Track Lexer in Track (`compiler/lexer.trk`)
- **Native Tokenizer**:
  - Port Lexer from Rust to 100% native Track code in `compiler/lexer.trk`.
  - Token stream representation using `union Token` and `[]u8` string slices.
  - Native unit test suite (`yard test`).

---

### v0.7.0 — Self-Hosted Compiler Component 2: Track Parser & AST in Track (`compiler/parser.trk`)
- **Native Parser**:
  - Port AST representation (`union Expr`) and recursive descent parser to native Track code in `compiler/parser.trk`.
  - Detailed compile-time error reporting with source code line/column highlighting.

---

### v0.8.0 — Self-Hosted Compiler Component 3: Type Checker & Codegen in Track (`compiler/checker.trk` & `compiler/codegen.trk`)
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
