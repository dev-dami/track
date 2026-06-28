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

**Track is a systems programming language for deterministic software.**

It combines **linear ownership**, **compile-time borrow checking**, and **zero-cost abstractions** to eliminate resource management bugs without a garbage collector, runtime, or lifetime annotations.

> *If resource management can be verified at compile time, it should never cost anything at runtime.*

## Hello World

```track
fn main() -> void {
    print("Hello, Track!");
}
```

## Why Track?

Track targets bare-metal firmware, audio DSP, robotics, and other real-time systems where dynamic allocation is prohibited and deterministic memory behavior is a hard requirement.

```track
// Linear types prevent leaks — no manual free needed
let mut v: Vec = vec_init(16);
vec_push(&mut v, 42);
// v is automatically freed when spent

// Lexical lenses for scoped mutable access
let user = User { name: "Alice", age: 30 };
with user -> u {
    u.age = 31;
}
// user is restored to Active
```

## Features

| Feature | Description |
|---------|-------------|
| Linear ownership | Prevents use-after-free, double-free, and leaks |
| Automatic cleanup | Resources freed at scope exit — no manual `free` |
| Borrow references | Compile-time checked shared access |
| Lexical lenses | Scoped mutable access without ownership transfer |
| Pattern matching | Exhaustive matching over enums and unions |
| Compile-time macros | Code generation with `@macro` |
| `@use` imports | Explicit module system with paths and aliases |
| LLVM backend | Native machine code generation |

## Getting Started

### Prerequisites

- Rust 2021 edition
- LLVM 22 development libraries

### Build and Install

```bash
git clone https://github.com/dev-dami/track.git
cd track
./install.sh
```

### Run a Program

```bash
track build hello.trk
./hello
```

## Documentation

- [Borrows and Escape Analysis](docs/borrows.md)
- [Enums and Unions](docs/enums.md)
- [Pattern Matching](docs/patterns.md)
- [Imports](docs/imports.md)
- [Constants](docs/constants.md)
- [Macros](docs/macros.md)
- [Standard Library](docs/stdlib.md)
- [Yard Package Manager](docs/yard.md)
- [LSP Server](docs/lsp.md)
- [Syntax Highlighting](grammars/README.md)
- [CHANGELOG](CHANGELOG.md)

## Yard (Package Manager)

```bash
track yard init my_project
track yard add <package>
track yard build
track yard run
```

## LSP

```bash
track-lsp
```

Provides diagnostics, auto-completion, and hover documentation for `.trk` files and `track` code blocks in markdown.

## Roadmap

### v0.5 — Standard Library
- Memory, string, and I/O functions
- Linear type automatic cleanup

### v0.6 — Tooling
- LSP server
- TextMate grammar for syntax highlighting

### v0.7 — Concurrency
- Channel-based message passing
- Static thread allocation

### v1.0 — Stable Release
- Language specification
- ABI stability
- Cross-compilation (ARM, RISC-V, Xtensa)

## License

Track is distributed under the [MIT License](LICENSE).
