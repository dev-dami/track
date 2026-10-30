# Track

**Track** is a low-level systems programming language designed for deterministic memory management, zero-cost abstractions, and real-time execution.

- **No Garbage Collector.** Resource lifecycles are verified at compile time; there is no collector and only a small C runtime.
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

// Linear ownership — the checker rejects use after a move
let mut v: Vec = vec_init(16);
vec_push(&mut v, 42);

// Lexical lens block for scoped mutation
let mut u = User { age: 30 };
with u -> user {
    user.set_age(31);
}

// Anonymous tuples, destructuring & pattern matching (v0.4)
let pair: (i64, bool) = (42, true);
let (n, flag) = pair;
match n {
    x if x > 10 => io::print("big"),
    _ => io::print("small"),
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

# Lint project (check without build)
yard lint

# Build project to target/my_app
yard build

# Build and execute project
yard run

# Run native Track tests (src/*_test.trk, tests/**/*.trk)
yard test
```

### Self-Hosted Compiler (`trackc`)

Track is self-hosting: `compiler/` is the Track compiler written in Track.
It compiles itself to a verified fixed point.

```bash
# Build trackc with the Rust bootstrap compiler
(cd compiler && yard build)          # -> compiler/target/trackc

compiler/target/trackc build examples/hello.trk -o hello
compiler/target/trackc run examples/generics.trk
compiler/target/trackc emit-c examples/hello.trk   # generated C on stdout

# Stage 0 -> 1 -> 2 -> 3, then check the fixed point
scripts/bootstrap.sh
```

See [docs/bootstrap.md](docs/bootstrap.md) for the pipeline, the shared
word-ABI runtime, and the C backend contract.

## Testing

```bash
cargo test                      # Rust suites, incl. the self-hosting parity tests
(cd compiler && yard test)      # native compiler tests
scripts/bootstrap.sh            # bootstrap fixed point
```

## License

[MIT](LICENSE)
