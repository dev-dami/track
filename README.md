# Track

**Track** is a low-level systems programming language designed for deterministic memory management, zero-cost abstractions, and real-time execution.

- **No Garbage Collector, No Runtime.** Resource lifecycles are verified and freed at compile time.
- **Linear Ownership.** Eliminates use-after-free, double-free, and memory leaks.
- **Lexical Lenses.** Scoped mutable access without manual pointer arithmetic or ownership transfer.
- **Cranelift Standalone Backend.** Fast native code generation without system LLVM dynamic library dependencies.

## Hello World

```track
import "std/io";

fn main() -> void {
    io::print("Hello, Track!");
}
```

## Linear Ownership & Scoped Access

```track
import "std/io";

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

Requires Rust (2021 edition).

```bash
# Build release binaries
cargo build --release

# Run installer script
./scripts/install.sh
```

## Usage

### Single-File Compiler (`track`)

```bash
# Type-check a source file
track check examples/hello.trk

# Compile to native executable
track build examples/hello.trk
./hello
```

### Package Manager & Build Orchestrator (`yard`)

```bash
# Create a new Track project
yard init my_app

# Type-check project
yard check

# Build project to target/my_app
yard build

# Build and execute project
yard run
```

## Testing

```bash
cargo test
```

## License

[MIT](LICENSE)
