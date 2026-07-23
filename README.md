# Track

**Track** is a low-level systems programming language designed for deterministic memory management, zero-cost abstractions, and real-time execution.

- **No Garbage Collector, No Runtime.** Resource lifecycles are verified and freed at compile time.
- **Linear Ownership.** Eliminates use-after-free, double-free, and memory leaks.
- **Lexical Lenses.** Scoped mutable access without manual pointer arithmetic or ownership transfer.
- **Direct LLVM Backend.** Compiles directly to optimized native machine code.

## Hello World

```track
fn main() -> void {
    print("Hello, Track!");
}
```

## Linear Ownership & Scoped Access

```track
// Linear ownership — freed automatically at spend point
let mut v: Vec = vec_init(16);
vec_push(&mut v, 42);

// Lexical lens block for scoped mutation
let mut u = User { age: 30 };
with u -> user {
    user.set_age(31);
}
```

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

## Testing

```bash
cargo test
```

## License

[MIT](LICENSE)
