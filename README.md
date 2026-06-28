<p align="center">
  <img src="assets/track-logo.svg" alt="Track" width="400">
</p>

<p align="center">
  <a href="https://github.com/track-lang/track/actions"><img src="https://img.shields.io/github/actions/workflow/status/track-lang/track/ci.yml?branch=main&style=flat-square&label=build" alt="Build Status"></a>
  <a href="https://github.com/track-lang/track/blob/main/LICENSE"><img src="https://img.shields.io/github/license/track-lang/track?style=flat-square" alt="License"></a>
  <a href="https://github.com/track-lang/track/releases"><img src="https://img.shields.io/github/v/release/track-lang/track?style=flat-square&include_prereleases" alt="Release"></a>
  <a href="https://github.com/track-lang/track"><img src="https://img.shields.io/github/stars/track-lang/track?style=flat-square" alt="Stars"></a>
</p>

---

Track is a systems programming language that enforces zero-allocation memory safety at compile time using linear types and lexical lenses. It targets bare-metal firmware, audio DSP, robotics, and other real-time systems where dynamic allocation is prohibited and deterministic memory behavior is a hard requirement.

Track compiles to native machine code via LLVM. There is no garbage collector, no runtime, and no complex lifetime annotations.

## Design Principles

- **No hidden control flow.** All branches and jumps are explicit in the source.
- **No hidden memory allocations.** Linear types prevent resource leaks by construction.
- **No runtime overhead.** Safety checks occur entirely at compile time.
- **Zero-allocation by default.** Dynamic allocation is not available unless explicitly opted in.

## Features

### Linear Types

User-defined types (`struct`, `[T; N]`) are linear resources. A value transitions through a strict state machine: `Active` -> `Locked` or `Spent`. The compiler rejects use-after-free, double-free, and resource leaks at compile time.

```track
let data = Data { x: 42 };
let result = compute(data);   // data is now Spent
let result2 = compute(data);  // error: use after free
```

Primitive types (`i32`, `u64`, `bool`, `ptr<T>`) have copy semantics and are exempt from linearity constraints.

### Lexical Lenses

Lenses provide scoped mutable access to a resource without transferring ownership. The lens locks the resource for the duration of its block and releases it automatically on scope exit.

```track
let user = User { name: "Alice", age: 30 };

with user -> u {
    print(u);  // u is Active inside the lens
}

print(user);  // user is restored to Active
```

### Uniform Function Call Syntax

Any function can be called as a method on its first argument. This is syntactic sugar with no dynamic dispatch.

```track
fn advance(p: ptr<i32>, offset: i32) -> ptr<i32> {
    return p + offset;
}

let next = p.advance(16);  // desugars to: advance(p, 16)
```

### Direct Memory Access

Array indexing and pointer arithmetic compile directly to LLVM `getelementptr` instructions.

```track
let buffer = [10, 20, 30, 40];
let first = buffer[0];
let addr = &first;
let offset = addr + 8;
```

## Type System

| Type | Semantics | Description |
|------|-----------|-------------|
| `i8`, `i16`, `i32`, `i64` | Copy | Signed integers |
| `u8`, `u16`, `u32`, `u64` | Copy | Unsigned integers |
| `bool` | Copy | Boolean |
| `void` | Copy | Unit type |
| `ptr<T>` | Copy | Raw pointer to `T` |
| `[T; N]` | Linear | Fixed-size array |
| `Struct { ... }` | Linear | User-defined record |

## Compiler

```
.trk -> Lexer -> Parser -> Linear Checker -> LLVM IR -> Native Binary
```

| Stage | Implementation | Description |
|-------|---------------|-------------|
| Lexer | `logos` | Tokenization with span tracking for diagnostics |
| Parser | Recursive descent | Operator precedence climbing, UFCS resolution |
| Linear Checker | Custom | Lifecycle tracking, copy/move inference, CFG state merging |
| Codegen | `inkwell` | LLVM IR emission and object file output |

## Building

**Prerequisites:**
- Rust 2021 edition
- LLVM 22 development libraries

```bash
cargo build --release
```

The `track` binary will be in `target/release/`.

## Yard (Package Manager)

Yard is the package manager for Track. It handles dependency resolution, project scaffolding, and build configuration.

```bash
# Initialize a new project
track yard init my_project

# Add a dependency
track yard add <package>

# Build the project
track yard build

# Run the project
track yard run
```

*Yard is integrated as a subcommand in the `track` binary.*

## Roadmap

Development is organized into versioned milestones. See [CHANGELOG.md](CHANGELOG.md) for a complete list of changes.

### v0.1 - Core Language (Complete)

- Lexer with token span tracking
- Recursive descent parser with operator precedence
- Typed AST
- Linear checker (`Active`/`Spent`/`Locked` state machine)
- Compile-time use-after-free and double-free detection

### v0.2 - Control Flow and Primitives (Complete)

- Struct literal disambiguation in conditionals
- CFG state merging (`if`/`else`, `while`)
- Primitive copy semantics via static type inference
- Array indexing, address-of (`&`), pointer arithmetic
- UFCS
- Lexical lens blocks (`with ->`)

### v0.3 - LLVM IR Codegen (Complete)

- LLVM IR emission via `inkwell`
- Function, struct, array, and control flow codegen
- Object file output
- Conditional debug logging (suppressed in `--release`)
- Linear type lifecycle operations in codegen
- Lens block code generation
- Linker integration
- Yard package manager implementation (scaffolding, build orchestration, dependency routing)

### v0.4 - Borrows and Escape Analysis (Complete)

- Borrow references (`fn read(buf: &Buffer)`)
- Escape analysis for pointer safety
- Active borrow-locking to prevent moves/mutations of borrowed resources
- Shared references (`&T`)

### v0.5 - Standard Library (Planned)

- Memory-mapped I/O abstractions
- Ring buffers and lock-free queues
- Fixed-point arithmetic
- Hardware register access
- Interrupt-safe data structures

### v0.6 - Tooling (Planned)

- LSP server
- Test framework
- Formatter and linter
- Documentation generator

### v0.7 - Concurrency (Planned)

- Channel-based message passing
- Static thread allocation
- Priority-aware scheduling
- Bare-metal interrupt handlers with linear safety

### v1.0 - Stability (Planned)

- Stable language specification
- ABI stability
- Cross-compilation (ARM, RISC-V, Xtensa)
- Comprehensive test suite

### Future Considerations

- Generics and type parameters
- Comptime evaluation
- C FFI without wrappers
- WebAssembly target
- Incremental compilation

## License

Track is distributed under the [MIT License](LICENSE).
