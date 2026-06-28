````md
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

Track is a systems programming language for deterministic software.

It combines **linear types**, **compile-time borrow checking**, and **lexical lenses** to eliminate resource leaks, use-after-free, and hidden allocations without requiring a garbage collector or runtime.

Track targets firmware, robotics, audio DSP, operating systems, and other real-time software where deterministic memory behavior is essential.

```track
let user = User { name: "Alice", age: 30 };

with user -> u {
    print(u);
}

print(user);
````

## Design Principles

* Deterministic execution
* Compile-time memory safety
* Zero-cost abstractions
* No garbage collector
* No hidden memory allocations
* Explicit control flow

---

## Features

### Linear Types

User-defined types are linear resources. Ownership moves by default, preventing resource leaks, double frees, and use-after-free errors entirely at compile time.

```track
let data = Data { x: 42 };

compute(data);

compute(data); // error: use after free
```

Primitive types (`i32`, `u64`, `bool`, `ptr<T>`) have copy semantics.

### Borrow References

Shared references allow read-only access without transferring ownership.

```track
fn read(buf: &Buffer) {
    print(buf);
}
```

Borrowed values cannot be moved or mutably accessed until the borrow ends.

### Lexical Lenses

Lexical lenses provide scoped mutable access without transferring ownership.

```track
let user = User { name: "Alice", age: 30 };

with user -> u {
    u.age += 1;
}

print(user);
```

Resources are automatically unlocked when the lens scope exits.

### Uniform Function Call Syntax

Any function may be called as if it were a method on its first argument.

```track
fn advance(ptr: ptr<i32>, offset: i32) -> ptr<i32> {
    return ptr + offset;
}

let next = ptr.advance(16);
```

UFCS is purely syntactic sugar and introduces no runtime overhead.

### Direct Memory Access

Pointers and array indexing compile directly to LLVM instructions.

```track
let buffer = [10, 20, 30, 40];

let first = buffer[0];

let ptr = &first;

let next = ptr + 8;
```

---

## Type System

| Type                      | Semantics | Description         |
| ------------------------- | --------- | ------------------- |
| `i8`, `i16`, `i32`, `i64` | Copy      | Signed integers     |
| `u8`, `u16`, `u32`, `u64` | Copy      | Unsigned integers   |
| `bool`                    | Copy      | Boolean             |
| `void`                    | Copy      | Unit type           |
| `ptr<T>`                  | Copy      | Raw pointer         |
| `[T; N]`                  | Linear    | Fixed-size array    |
| `struct`                  | Linear    | User-defined record |

---

## Compiler Pipeline

```
Source (.trk)
      │
      ▼
Lexer
      │
      ▼
Parser
      │
      ▼
Type Checking
      │
      ▼
Linear Analysis
      │
      ▼
LLVM IR
      │
      ▼
Native Binary
```

| Stage          | Implementation    | Description                                    |
| -------------- | ----------------- | ---------------------------------------------- |
| Lexer          | `logos`           | Tokenization with span tracking                |
| Parser         | Recursive descent | Operator precedence and UFCS parsing           |
| Type Checker   | Custom            | Static type checking                           |
| Linear Checker | Custom            | Ownership, borrow checking, lifecycle tracking |
| Codegen        | `inkwell`         | LLVM IR generation and object emission         |

---

## Documentation

Additional documentation is available in the `docs` directory.

* **[Borrows and Escape Analysis](docs/borrows.md)** — borrow references, dereferencing, borrow locking, and escape analysis.
* **[Yard Package Manager](docs/yard.md)** — package layout, `Track.toml`, dependency management, and workflows.
* **[CHANGELOG](CHANGELOG.md)** — complete version history and development milestones.

---

## Building

### Prerequisites

* Rust 2021
* LLVM 22 development libraries

```bash
cargo build --release
```

The compiler will be available at:

```text
target/release/track
```

---

## Yard

Yard is Track's integrated package manager.

```bash
track yard init my_project

track yard add <package>

track yard build

track yard run
```

Yard handles project scaffolding, dependency resolution, and build orchestration.

---

## Roadmap

### v0.5 — Standard Library

* Memory-mapped I/O
* Ring buffers
* Lock-free queues
* Fixed-point arithmetic
* Hardware register abstractions
* Interrupt-safe data structures

### v0.6 — Tooling

* Language Server Protocol
* Formatter
* Linter
* Documentation generator
* Test framework

### v0.7 — Concurrency

* Channels
* Static thread allocation
* Priority-aware scheduling
* Interrupt-safe concurrency primitives

### v1.0 — Stable Release

* Stable language specification
* Stable ABI
* Cross compilation (ARM, RISC-V, Xtensa)
* Comprehensive test suite

### Future

* Generics
* Compile-time evaluation
* C FFI
* WebAssembly backend
* Incremental compilation

---

## Philosophy

Track is designed around a simple principle:

> If memory behavior can be verified at compile time, it should not incur runtime cost.

Ownership, borrowing, and resource lifetimes are enforced statically, allowing generated code to remain as predictable as handwritten C while eliminating entire classes of memory errors.

---

## License

Track is distributed under the [MIT License](LICENSE).

```
```
