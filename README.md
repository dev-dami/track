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
to a verified fixed point, through C or natively. A clean checkout
bootstraps with only `cc`. `scripts/bootstrap.sh` builds the committed C
seed (`bootstrap/seed/`), and Rust is not needed. The Rust compiler in
`src/` remains an optional stage 0 (`--from-rust`). The native stages of
the default run need an x86-64 Linux host; elsewhere, use
`scripts/bootstrap.sh --no-native`.

```bash
# cc builds bootstrap/seed/ -> stage1; stage1 -> stage2 -> stage3; check the
# C and binary fixed point, then the native fixed point (n2 = n3 = n4)
scripts/bootstrap.sh                 # trackc is build/bootstrap/stage3/trackc
scripts/bootstrap.sh --from-rust     # same, with the Rust stage 0 (cargo)
scripts/update_seed.sh               # regenerate the seed after compiler changes

T=build/bootstrap/stage3/trackc
$T build examples/hello.trk -o hello
$T run examples/generics.trk
$T emit-c examples/hello.trk         # generated C on stdout
$T build --native examples/hello.trk -o hello  # no C compiler

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
scripts/bootstrap.sh            # seed bootstrap: C and native fixed points
scripts/update_seed.sh --check  # seed matches compiler/src
scripts/selfhost.sh             # native fixed point, no C toolchain
bench/native.sh                 # native vs C vs Rust scoreboard
```

## License

[MIT](LICENSE)
