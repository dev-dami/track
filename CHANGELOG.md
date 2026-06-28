# Changelog

All notable changes to the Track programming language and toolchain will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [0.4.0] — 2026-06-28

### Added
- **Reference Types (`&T`)**: Added `TrackType::Ref` variant representing safe borrows.
- **Address-Of & Deref operators**: Added lexing, parsing, type-checking, and codegen for taking the address (`&`) of variables and loading through them (`*`).
- **Escape Analysis**: Implemented safe return checks ensuring reference values do not outlive the local variables they borrow (pointer safety).
- **Borrow-Locking**: Implemented compiler-level locks preventing moves or mutation of resources while active borrows/references exist.
- **Reference Examples**: Added `examples/borrow.trk`, `examples/escape_err.trk`, and `examples/borrow_lock_err.trk`.

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
