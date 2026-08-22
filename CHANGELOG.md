# Changelog

All notable changes to the Track programming language and toolchain will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [0.5.0] — 2026-08-22

### Added
- **Explicit stack-based error handling** adopted as the official error model: no `Option`/`Result` types, no `?` operator, no hidden control flow, no unwinding. Error codes as copy primitives, `(value, err)` tuple returns via v0.4 destructuring, out-params via `&T`. Documented in `docs/errors.md`; demo in `examples/error_handling.trk`.
- **`abort(msg)` builtin**: print message to stderr and exit with status 134. No unwinding — frames are discarded by design. Exported via `std/sys`.
- **`str_is_int(s)`**: predicate companion for `str_to_int`, distinguishing a genuine `0` from a parse failure.
- **`env_exists(key)`**: predicate companion for `env_get`.

### Fixed
- **Unary `!` miscompiled as bitwise NOT (`bnot`)**: `if (!flag)` with `flag == 1` evaluated `-2` (truthy) and took the wrong branch. Now compiles to a proper logical NOT (`val == 0`).
- **Top-level `const`s silently resolved to 0 at codegen** when referenced from non-main functions. Constants are now collected in a first pass (including arithmetic over other constants) and emitted as immediate values everywhere.
- `track --version` hardcoded the version string; now derived from `CARGO_PKG_VERSION`.

## [0.4.0] — 2026-07-30

### Added
- **First-Class Anonymous Tuples**: Tuple types `(T1, T2)`, literals `(a, b)`, and element indexing notation `tuple.0`, `tuple.1`.
- **Tuple Destructuring**: Support for `let (a, b) = expr;` statements with variable pattern matching.
- **Nested Pattern Matching**: Extended pattern matching support for tuple patterns `(p1, p2)`, multi-binding variant patterns `Variant(p1, p2)`, literal patterns (`0`, `true`), and struct patterns.
- **Match Arm Guards**: Expression guards on match arms (`match x { n if n > 10 => ..., _ => ... }`).
- **Demo Program**: Added `examples/v040_features_demo.trk` demonstrating all v0.4.0 features.

---

## [0.3.0] — 2026-07-27

### Performance & Correctness
- **Self-hosting readiness**: `CodeGen::create_default_isa()` extracted; shared `Arc<dyn TargetIsa>` across all parallel worker threads — eliminates per-file ISA re-initialization.
- **Eliminated redundant allocations**: `sig_params` in codegen borrows signature params (`&`) instead of cloning per function call.
- **Scope map optimization**: `FnDef` and `MacroDef` scope exit now uses `std::mem::take` (zero-allocation pointer swap) instead of cloning 6–7 `HashMap`s per scope.
- **IfElse scope map optimization**: Reduced from 6 `HashMap` clones to 1 clone + `std::mem::replace`/`std::mem::take` for else branch.
- **Eliminated double-pass FnDef signature registration**: `check_program` consolidated to two passes (declarations in pass 1, expression checking in pass 2).
- **LSP document revision counter**: Stale diagnostic results returned by `analyze_document_async` are discarded if a newer edit has arrived — prevents publishing diagnostics for superseded keystrokes.
- **LSP AST caching**: `analyze_source_static` returns the AST alongside diagnostics; completion handler reuses cached AST instead of re-tokenizing/re-parsing the full document.
- **`find_lens_alias` fast-path**: Returns `None` immediately when `lens_aliases` is empty (true for 99%+ of code).
- **`update_borrow_states` fast-path**: Early-return with a linear scan to reset `Locked → Active` when no borrows or lens locks exist.
- **Levenshtein candidate filter**: Skips `levenshtein_distance` for function candidates whose name length differs from the target by more than 2.
- **Provenance sort/dedup guard**: Skipped entirely when provenance vector has 0 or 1 elements.

### Added
- `track lint` command to `yard` CLI (lint without build).
- `yard clean` command (`yard clean` removes `target/` directory).
- New standard library functions: `os_args_count`, `os_arg`, `dir_exists`, `file_copy`, `process_spawn`, `sys_exec`, `sys_set_memory_limit`, `sys_get_memory_used`, `env_get`, `str_starts_with`, `str_ends_with`, `str_contains`.
- `Track.toml` serialization for lock files.
- `track-lsp` binary now ships with the release build.

### Fixed
- Various N+1 and O(N²) bottleneck eliminations across checker, LSP, and build cache (see commit `04c91a3`).
- LSP missing closing brace in completion handler (commit `4a3ddd`).

---

## [0.2.0] — 2026-06-21

### Added
- Struct literal disambiguation inside conditionals.
- CFG state merging for branches (`if`/`else`) and loops (`while`).
- Primitive copy semantics via static type inference.
- Array indexing, address-of (`&`), and pointer arithmetic.
- Uniform Function Call Syntax (UFCS).
- Lexical lens blocks (`with ->` expression blocks).

---

## [0.1.0] — 2026-06-14

### Added
- Lexer using `logos` with token span tracking.
- Recursive descent parser with operator precedence.
- Typed AST.
- Custom Linear Checker for `Active`/`Spent`/`Locked` resource states.
- Compile-time use-after-free and double-free checks.
