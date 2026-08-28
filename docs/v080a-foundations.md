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

1. Lexer parity and span representation.
2. Diagnostic formatting and native test fixtures.
3. AST and parser subset.
4. C-emitter interface, followed by checker and complete code generation.
