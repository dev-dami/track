# Track

**Track** is a low-level systems programming language designed for deterministic memory management, zero-cost abstractions, and real-time execution.

- **No Garbage Collector.** Resource lifecycles are verified at compile time; there is no collector and only a small runtime, in C or in Track.
- **Linear Ownership.** Rejects use-after-free and double-free. The self-hosted compiler infers where each allocation's life ends and places it on the stack or frees it there ([docs/lifecycles.md](docs/lifecycles.md)).
- **Lexical Lenses.** Scoped mutable access without manual pointer arithmetic or ownership transfer.
- **Native Backend.** `trackc build --native` writes a static x86-64 Linux executable itself, with no C compiler, assembler or linker. The Rust bootstrap compiler uses Cranelift.

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

Track is self-hosting: `compiler/` is the Track compiler written in Track
(`compiler/src/`, plus its runtime in `compiler/rt/`). It compiles itself
to a verified fixed point, through C or natively. The Rust compiler in
`src/` is needed only to bootstrap from nothing.

```bash
# Build trackc with the Rust bootstrap compiler
(cd compiler && yard build)          # -> compiler/target/trackc

compiler/target/trackc build examples/hello.trk -o hello
compiler/target/trackc run examples/generics.trk
compiler/target/trackc emit-c examples/hello.trk   # generated C on stdout
compiler/target/trackc build --native examples/hello.trk -o hello  # no C compiler

# Stage 0 -> 1 -> 2 -> 3, then check the fixed point
scripts/bootstrap.sh
# Native trackc rebuilds itself with no C toolchain on PATH (n2 = n3 = n4)
scripts/selfhost.sh
```

See [docs/bootstrap.md](docs/bootstrap.md) for the pipeline, the shared
word-ABI runtime, the C backend contract and the native backend, and
[docs/native.md](docs/native.md) for the runtime written in Track.

## Testing

```bash
cargo test                      # Rust suites, incl. the self-hosting parity tests
(cd compiler && yard test)      # native compiler tests
scripts/bootstrap.sh            # bootstrap fixed point
scripts/selfhost.sh             # native fixed point, no C toolchain
bench/native.sh                 # native vs C vs Rust scoreboard
```

## License

[MIT](LICENSE)
