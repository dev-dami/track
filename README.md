# Track

**Track** is a low-level systems programming language designed for deterministic memory management, zero-cost abstractions, and real-time execution.

- **No Garbage Collector.** Resource lifecycles are verified at compile time; there is no collector and only a small runtime, written in Track.
- **Linear Ownership.** Rejects use-after-free and double-free. The self-hosted compiler infers where each allocation's life ends and places it on the stack or frees it there ([docs/lifecycles.md](docs/lifecycles.md)).
- **Lexical Lenses.** Scoped mutable access without manual pointer arithmetic or ownership transfer.
- **Native Backend.** `trackc build` writes a static x86-64 Linux executable itself, with no C compiler, assembler or linker.
- **Track only.** The repository holds no C or Rust. Track bootstraps from a committed native seed binary.
- **Cross-compiling.** `trackc build --target x86_64-windows` writes a Windows PE executable from any host.

| target | `trackc build --target` | status |
|---|---|---|
| x86_64-linux | default on Linux | static ELF |
| x86_64-windows | `x86_64-windows` | PE32+ console .exe (kernel32); trackc.exe self-hosts |
| aarch64-linux, x86_64/aarch64-macos, aarch64-windows | | not yet |

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

### Windows

In PowerShell (no Linux machine, compiler or admin rights needed):

```powershell
irm https://raw.githubusercontent.com/dev-dami/track/main/scripts/install.ps1 | iex
```

It fetches the sources (git if present, else the zip), bootstraps
`trackc.exe` from `bootstrap\seed\trackc.exe` with `scripts\bootstrap.ps1`
(seed → stage1 → stage2 = stage3), builds `yard.exe`, installs both to
`%USERPROFILE%\.track\bin` and adds that to the user PATH. From a
checkout: `pwsh -File scripts\bootstrap.ps1` leaves
`build\bootstrap\stage3\trackc.exe`. Paths may use `\` or `/` and drive
letters; yard runs its commands through `cmd.exe`. Integer division by
zero exits with `0xC0000094` (STATUS_INTEGER_DIVIDE_BY_ZERO); sockets use
Winsock.

### Build from Source

Requires an x86-64 Linux host and POSIX tools; nothing else. No C
compiler, assembler or linker is used: Track bootstraps from
`bootstrap/seed/trackc`, a committed static trackc binary.

```bash
scripts/bootstrap.sh                        # seed -> stage1..3; trackc is build/bootstrap/stage3/trackc
build/bootstrap/stage3/trackc build tools/yard/main.trk -o yard   # yard, written in Track
./scripts/install.sh                        # or: bootstrap and install trackc + yard
```

## Usage

### Compiler (`trackc`)

```bash
# Type-check a source file
trackc check examples/hello.trk

# Compile to a static x86-64 Linux executable
trackc build examples/hello.trk -o hello
./hello

# Language server for editors
trackc lsp
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
to a verified fixed point. A clean checkout bootstraps from
`bootstrap/seed/trackc`, the committed static x86-64 Linux trackc binary;
`scripts/bootstrap.sh` needs no C compiler. (Earlier versions bootstrapped
from a Rust compiler, then from a committed C seed; both are gone, along
with the C backend.)

```bash
# seed -> stage1 -> stage2 -> stage3 under env -i PATH=/nonexistent;
# stage2 and stage3 must be byte-identical
scripts/bootstrap.sh                 # trackc is build/bootstrap/stage3/trackc
scripts/update_seed.sh               # regenerate the seed after compiler changes

T=build/bootstrap/stage3/trackc
$T build examples/hello.trk -o hello
$T build -g examples/hello.trk -o hello   # with a symbol table
$T run examples/generics.trk

# trackc rebuilds itself with nothing on PATH (n2 = n3 = n4)
scripts/selfhost.sh
```

See [docs/bootstrap.md](docs/bootstrap.md) for the pipeline, the word-ABI
runtime and the native backend, and
[docs/native.md](docs/native.md) for the runtime written in Track.

## Testing

```bash
scripts/bootstrap.sh            # seed bootstrap: stage2 = stage3
tests/run.sh -j 4               # language, runtime, native, CLI and tools suites
tests/yard/run.sh               # yard behaviour
tests/lsp/run.sh                # recorded `trackc lsp` sessions
scripts/update_seed.sh --check  # seed matches compiler/src
scripts/selfhost.sh             # fixed point with nothing on PATH
bench/native.sh                 # native scoreboard (cycles, faults, self-compile)
```

## License

[MIT](LICENSE)
