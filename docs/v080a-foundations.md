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
3. AST, collections, and parser subset. ✅
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

## Phase 3 contract

The native compiler now has three independently testable representation layers:

- `collections.trk` provides a bounds-checked, growable, opaque 64-bit
  collection. Its pointer-only ABI avoids coupling compiler records to the
  legacy `Vec<i32>` layout.
- `ast.trk` defines the native `Expr` variants and stores compact six-word
  nodes: kind, value, auxiliary value, start byte, end byte, and a
  variant-specific extra value.
- `parser.trk` consumes stable lexer records `(tag, start, end, next)` and
  builds the AST without copying token text. Identifiers and strings retain
  source ranges so later checker phases can recover their text on demand.

The Phase 3 grammar subset is deliberately bounded:

```text
program    := statement* EOF
statement  := "let" "mut"? IDENT "=" expression ";"
            | "return" expression? ";"
            | expression ";"
expression := factor (("+" | "-") factor)*
factor     := unary (("*" | "/" | "%") unary)*
unary      := ("!" | "-") unary | primary
primary    := INT | STRING | BOOL | IDENT | "(" expression ")"
```

Every parser failure returns an error code and end-exclusive byte range that
feeds directly into the Phase 2 diagnostic formatter. The complete Track
grammar—including functions, blocks, types, control flow, aggregates, and
patterns—remains the v0.8.0b parser milestone.

## Bootstrap build ordering and artifacts

Yard's build-order and artifact-naming milestone is complete:

- Source modules are discovered in sorted path order; dependency resolution
  follows dependency-name order. Parallel build failures are reported in source
  order, and `yard check` sorts diagnostics before printing them.
- Each source-relative module path maps to `target/objects/<module>.o`.
  For example, `src/lexer/helpers.trk` becomes
  `target/objects/lexer/helpers.o`. This preserves nested module identity and
  separates user objects from `target/_track_runtime.o`.
- The linker receives sorted object paths followed by the runtime object.
- `.cache_meta.json` serializes source keys in sorted order, with no timestamps
  or worker-order metadata. Cache-write failures fail the build explicitly.
  Existing JSON cache files remain readable; the new object paths force a
  rebuild when migrating from the old flat artifact layout.

The regression gate is `cargo test --test yard_tests`. It covers colliding
basenames, a user module named `_track_runtime.trk`, warm-cache reuse, stable
metadata after a clean build, and execution of the linked binary. These
guarantees concern build ordering and artifact identity; byte-for-byte native
binary reproducibility across toolchains or Stage 2/3 remains a separate gate.

The remaining v0.8.0a exit criterion is the native C-emitter interface and its
external compiler invocation contract.
