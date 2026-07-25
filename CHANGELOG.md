# Changelog

All notable changes to the Track programming language and toolchain will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [0.1.6] — 2026-07-25

### Added
- **Cranelift Standalone Backend**: Migrated primary compiler backend to Cranelift 0.108 (`ObjectModule`), eliminating system LLVM dynamic library dependencies and generating fast standalone native executables.
- **First-Class Slices (`[]T`)**: Added native slice type support `{ ptr, len }` for slice types like `[]u8`, `[]i64`.
- **Sub-slicing Syntax (`a[start..end]`)**: Added range sub-slicing over arrays and slices with `a[0..5]`, `a[start..]`, `a[..end]`, and `a[..]`.
- **8-Bit Integer Types (`u8`, `i8`)**: Added `u8` and `i8` keywords, AST types, type checking, and Cranelift IR lowering.
- **Yard Parallel Builder & Cache**: Multi-threaded worker pool (`ParallelBuilder`), SHA-256 fingerprinting cache (`BuildCache`), and fast linker auto-detection (`mold` / `lld`).
- **CLI Command `--version`**: Added `track --version` / `-v` flag outputting current compiler version.
- **Example Program**: Added `examples/slice_and_types.trk`.

### Added
- **LSP Server**: Added `track-lsp` binary implementing Language Server Protocol for IDE support.
- **Diagnostics**: Real-time error checking for `.trk` files and `track` code blocks in markdown.
- **Auto-completion**: Completion for keywords, types, macros, and enum/union variants.
- **Hover Documentation**: Hover information for language constructs.
- **TextMate Grammar**: Added `track.tmLanguage.json` for syntax highlighting in GitHub and VS Code.
- **LSP Documentation**: Added [src/lsp/mod.rs](file:///home/dev/track/src/lsp/mod.rs) and [grammars/README.md](file:///home/dev/track/grammars/README.md).

## [0.4.0] — 2026-06-28

### Added
- **Reference Types (`&T`)**: Added `TrackType::Ref` variant representing safe borrows.
- **Address-Of & Deref operators**: Added lexing, parsing, type-checking, and codegen for taking the address (`&`) of variables and loading through them (`*`).
- **Escape Analysis**: Implemented safe return checks ensuring reference values do not outlive the local variables they borrow (pointer safety).
- **Borrow-Locking**: Implemented compiler-level locks preventing moves or mutation of resources while active borrows/references exist.
- **Reference Examples**: Added `examples/borrow.trk`, `examples/escape_err.trk`, and `examples/borrow_lock_err.trk`.
- **`@use()` modules**: Implemented comptime module import syntax supporting full path importing, aliasing (`as`), and specific item import selection (`path::{a, b}`). Added namespaced identifiers (`namespace::name`) to the parser and checker.
- **`const` definitions**: Added support for parsing and evaluating compile-time constant declarations (`const BUFFER_SIZE = 1024;`).
- **`@macro` meta-operations**: Added support for defining and calling compile-time macros. Includes expression macros (e.g. `@bit(n)`), statement macros (e.g. `@assert(cond)`), block macros (e.g. `@timer { body }`), and compile-time evaluation built-ins (e.g. `@fib_comptime(n)`).
- **Macro Examples**: Added [examples/macro_test.trk](file:///home/dev/track/examples/macro_test.trk).
- **Plain Enums**: Added `enum` keyword for type-safe enumerations without associated data. Supports optional underlying integer types (`: u8`, `: i32`, etc.).
- **Tagged Unions**: Added `union` keyword for variants with associated data. Each variant can hold a different type.
- **Pattern Matching**: Added `match` expression for exhaustive control flow over enums and unions. Supports wildcard patterns, binding patterns, and guard conditions.
- **String Arrays**: Added null-terminated string array assignment (`let buf: [u8; 16] = "hello";`).
- **Union/Enum Examples**: Added [examples/union_enum_test.trk](file:///home/dev/track/examples/union_enum_test.trk).

## [0.3.0] — 2026-06-28

### Added
- **LLVM IR Codegen**: Added full IR generation via the `inkwell` crate for functions, loops, branches, variables, arrays, structures, and control flow.
- **Native Binary Compiler**: Added object file emission (`TargetMachine`) and linker (`cc`) integration to output working native executables.
- **Synthesized Entrypoint**: Added automatic wrapping of top-level scripts into a C-ABI compliant `main` function returning `i32` 0 on success.
- **Yard Package Manager**: Integrated the new Track package manager under `track yard` with commands:
  - `init`: Scaffolds a new package layout.
  - `build`: Resolves dependency trees and builds all source files.
  - `run`: Builds and executes the package binary.
  - `add`: Declares new path/git/registry dependencies in `Track.toml`.
  - `check`: Performs static linear type checking without codegen.
- **CLI Subcommands**: Refactored `track` binary to parse `build`, `run`, `check`, and `yard` subcommands.

## [0.2.0] — 2026-06-21

### Added
- Struct literal disambiguation inside conditionals.
- CFG state merging for branches (`if`/`else`) and loops (`while`).
- Primitive copy semantics via static type inference.
- Array indexing, address-of (`&`), and pointer arithmetic.
- Uniform Function Call Syntax (UFCS).
- Lexical lens blocks (`with ->` expression blocks).

## [0.1.0] — 2026-06-14

### Added
- Lexer using `logos` with token span tracking.
- Recursive descent parser with operator precedence.
- Typed AST.
- Custom Linear Checker for `Active`/`Spent`/`Locked` resource states.
- Compile-time use-after-free and double-free checks.
