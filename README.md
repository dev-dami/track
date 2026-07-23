# Track

> **Core Research Hypothesis**: *Can deterministic memory safety be made substantially easier to reason about by restricting mutable access to non-escaping lexical lenses rather than general lifetime-based borrows?*

**Track** is an experimental low-level systems programming language designed for deterministic memory management, zero-cost abstractions, and real-time execution.

- **Linear Ownership Model**: Designed to prevent use-after-free, double-free, and memory leaks at compile time.
- **Lexical Lenses**: A lexical lens is an exclusive, non-escaping mutable view valid strictly within a `with` block that cannot be moved, stored, returned, or escape that lexical scope.
- **Direct LLVM Backend**: Compiles directly to optimized native machine code.
- **Zero Runtime / No Garbage Collector**: Resource lifecycles and deallocations are verified and inserted at compile time.

---

## Prototype Status

| Subsystem | Status |
| :--- | :--- |
| **Lexer (Logos)** | Implemented ([src/lexer/](file:///home/dev/track/src/lexer/)) |
| **Parser + AST** | Implemented ([src/parser/](file:///home/dev/track/src/parser/)) |
| **Linear Ownership Checker** | Implemented for moves, lenses, & branch merging ([src/checker/](file:///home/dev/track/src/checker/)) |
| **LLVM IR Backend (Inkwell)** | Implemented ([src/codegen/](file:///home/dev/track/src/codegen/)) |
| **Package Manager (`yard`)** | Implemented ([src/yard/](file:///home/dev/track/src/yard/)) |
| **LSP Language Server** | Implemented ([src/lsp/](file:///home/dev/track/src/lsp/)) |
| **Test Suite** | 53 passing tests across 12 test modules including `soundness_tests` ([tests/](file:///home/dev/track/tests/)) |
| **Language Specification** | Formalized in [SPEC.md](file:///home/dev/track/SPEC.md) |

---

## Hello World

```track
import "std/io";

fn main() -> void {
    io::print("Hello, Track!");
}
```

## Linear Ownership & Lexical Lenses

```track
import "std/io";

// Linear ownership — freed automatically at spend point
let mut v: Vec = vec_init(16);
vec_push(&mut v, 42);

// Lexical lens block for scoped mutation (non-escaping guarantee)
let mut u = User { age: 30 };
with u -> user {
    user.set_age(31);
}
```

---

## Installation

### One-line Install (curl)

```bash
curl -fsSL https://raw.githubusercontent.com/dev-dami/track/main/scripts/install.sh | bash
```

### Build from Source

Requires Rust (2021 edition) and LLVM 22 development libraries.

```bash
# Build release binaries
cargo build --release

# Run installer script
./scripts/install.sh
```

---

## Usage

```bash
# Type-check a source file
track check examples/hello.trk

# Compile to native executable
track build examples/hello.trk
./hello

# Run via package manager
track yard init my_app
track yard check
track yard build
```

---

## Testing

```bash
cargo test
```

---

## License

[MIT](LICENSE)
