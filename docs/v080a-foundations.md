# v0.8.0a: Native Compiler Foundations

## Decision record

The Rust bootstrap compiler continues to use Cranelift. The self-hosted
compiler's first backend emits portable C and calls the platform C compiler
through Track's explicit process API. This avoids making the Track compiler
depend on a native reimplementation or FFI surface for Cranelift, while still
allowing Stage 1 to compile Track sources without Rust compiler logic.

## Exit criteria

- The native lexer classifies every token required by the parser and has
  regression tests for reserved words, primitive types, operators, comments,
  strings, and invalid input.
- `compiler/src/` contains independently testable source-span, diagnostic,
  AST, and collection modules.
- A checked-in backend interface documents C output and the exact external C
  compiler command used by the bootstrap flow.
- Module discovery and emitted-file ordering are deterministic.

## Delivery order

1. Lexer parity and span representation. ✅
2. Diagnostic formatting and native test fixtures. ✅
3. AST and parser subset.
4. C-emitter interface, followed by checker and complete code generation.

## Phase 2 contract

The native compiler now represents source ranges as end-exclusive byte offsets
plus precomputed one-based line and column positions. `lex_next_spanned` is the
parser-facing lexer entry point and preserves the exact byte range after trivia
has been skipped.

Diagnostics use a deterministic, terminal-neutral text format:

```text
error[E1001]: expected expression
 --> fixture.trk:2:13
  |
2 | let value = ;
  |             ^ expression required after '='
```

The formatter clamps invalid offsets, gives empty spans a visible one-character
marker, and limits a multiline highlight to its primary source line. Golden
fixtures live under `compiler/tests/fixtures/`; `yard test` copies non-Track
fixture files into each isolated native test package before execution.

Run the native gate from `compiler/`:

```sh
../target/debug/yard test
```
