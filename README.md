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

### Enums

Plain enums for type-safe states and modes. No associated data—use unions for tagged data.

```track
enum Color {
    Red,
    Green,
    Blue,
}

enum Status : u8 {
    Active,
    Locked,
    Spent,
}

let color = Color::Red;
```

### Unions

Tagged unions for variants with associated data. Each variant holds a different type.

```track
union Value {
    Int(i32),
    Float(f64),
    Bool(bool),
}

union Result(T, E) {
    Ok(T),
    Err(E),
}

let val: Value = Value::Int(42);
```

### Pattern Matching

Exhaustive match expressions for enums and unions. No fallthrough—every case must be handled.

```track
match color {
    Color::Red => print("red"),
    Color::Green => print("green"),
    Color::Blue => print("blue"),
}

match val {
    Value::Int(x) => print(x),
    Value::Float(x) => print(x),
    Value::Bool(x) => print(x),
}

// Wildcard catch-all
match val {
    Value::Int(x) => print(x),
    _ => print("other"),
}
```

### `@use` Imports

Explicit imports with `@use()`. Supports full paths, specific items, and aliasing.

```track
@use("std::io")
@use("math::vec::{add, sub}")
@use("utils::log") as logger

fn main() -> void {
    io::print("hello");
    add(1, 2);
    logger::log("done");
}
```

### `const` Definitions

Compile-time constants with explicit values. No hidden evaluation.

```track
const BUFFER_SIZE = 1024;
const SAMPLE_RATE = 44100;
const PI = 3.14159;
```

### `@macro` System

Compile-time macros for code generation and meta-operations. Uses `@` prefix to signal meta-operation.

```track
// Expression macro
@macro bit(n: u32) -> u32 {
    return 1 << n;
}

let LED_PIN = @bit(5);

// Statement macro
@macro assert(condition: bool) -> void {
    if (!condition) {
        @compile_error("assertion failed");
    }
}

@assert(x > 0);

// Block macro
@macro timer(body: block) -> void {
    let start = @now();
    body;
    let end = @now();
    print(end - start);
}

@timer {
    // code to measure
}
```

## Type System

| Type | Semantics | Description |
|------|-----------|-------------|
| `i8`, `i16`, `i32`, `i64` | Copy | Signed integers |
| `u8`, `u16`, `u32`, `u64` | Copy | Unsigned integers |
| `bool` | Copy | Boolean |
| `void` | Copy | Unit type |
| `ptr<T>` | Copy | Raw pointer to `T` |
| `&T` | Copy | Borrowed reference |
| `[T; N]` | Linear | Fixed-size array |
| `Struct { ... }` | Linear | User-defined record |
| `enum { ... }` | Copy | Tagged enumeration |
| `union { ... }` | Linear | Tagged union with data |
| `str` | Copy | Compile-time string literal |

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
## Documentation

Detailed documentation is available in the [/docs](file:///home/dev/track/docs) folder:

- **[Borrows and Escape Analysis](file:///home/dev/track/docs/borrows.md)**: Reference types (`&T`), dereferencing (`*`), compile-time borrow-locking, and escape safety.
- **[Enums and Unions](file:///home/dev/track/docs/enums.md)**: Plain enums, tagged unions, and type-safe states.
- **[Pattern Matching](file:///home/dev/track/docs/patterns.md)**: Exhaustive match expressions for enums and unions.
- **[Imports](file:///home/dev/track/docs/imports.md)**: `@use()` module imports with paths and aliases.
- **[Constants](file:///home/dev/track/docs/constants.md)**: Compile-time constant definitions.
- **[Macros](file:///home/dev/track/docs/macros.md)**: `@macro` system for code generation.
- **[Yard Package Manager](file:///home/dev/track/docs/yard.md)**: Yard package layout, `Track.toml`, and command-line workflows.
- **[LSP Server](file:///home/dev/track/src/lsp/mod.rs)**: Language server for IDE support.
- **[Syntax Highlighting](file:///home/dev/track/grammars/README.md)**: TextMate grammar for GitHub and VS Code.

See [CHANGELOG.md](file:///home/dev/track/CHANGELOG.md) for a historical record of all changes.

## Building

**Prerequisites:**
- Rust 2021 edition
- LLVM 22 development libraries

```bash
cargo build --release
```

The `track` binary will be in `target/release/`.

## Installation

```bash
./install.sh
```

This builds and installs `track` and `track-lsp` to `/usr/local/bin`.

## LSP Server

Track includes a language server for IDE support:

```bash
# Start the LSP server
track-lsp
```

Features:
- Diagnostics for `.trk` files
- Diagnostics for `track` code blocks in markdown files
- Auto-completion for keywords, types, macros, and enum/union variants
- Hover documentation
- Syntax highlighting via TextMate grammar

### VS Code Integration

Add to your `settings.json`:

```json
{
  "language-server.track": {
    "command": "track-lsp",
    "filePatterns": ["*.trk", "*.md"]
  }
}
```

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

### v0.4 - Borrows, Enums, and Macros (Complete)

- Borrow references (`fn read(buf: &Buffer)`)
- Escape analysis for pointer safety
- Active borrow-locking to prevent moves/mutations of borrowed resources
- Shared references (`&T`)
- Enums (`enum Color { Red, Green, Blue }`)
- Tagged unions (`union Value { Int(i32), Float(f64) }`)
- Pattern matching (`match val { ... }`)
- `@use()` imports with paths, items, and aliasing
- `const` compile-time constants
- `@macro` system for code generation

### v0.5 - Standard Library (Planned)

- Memory-mapped I/O abstractions
- Ring buffers and lock-free queues
- Fixed-point arithmetic
- Hardware register access
- Interrupt-safe data structures

### v0.6 - Tooling (Complete)

- LSP server (`track-lsp`)
- TextMate grammar for syntax highlighting
- Diagnostics for `.trk` and markdown files
- Auto-completion and hover documentation

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
- C FFI without wrappers
- WebAssembly target
- Incremental compilation

## License

Track is distributed under the [MIT License](LICENSE).
