# Track

**Track** is a low-level systems programming language designed for deterministic memory safety, zero-cost abstractions, and real-time execution.

- **No Garbage Collector, No Runtime.** Resource lifecycles are verified and freed at compile time.
- **Linear Ownership.** Eliminates use-after-free, double-free, and memory leaks.
- **Lexical Lenses.** Scoped mutable access without manual pointer arithmetic or ownership transfer.
- **Standalone Cranelift Compiler Backend.** Fast native code generation without host LLVM dynamic library dependencies.

---

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

---

## Usage

```bash
# Type-check a source file
track check main.trk

# Compile to native executable
track build main.trk
./main

# Compile and run immediately
track run main.trk

# Package Management with Yard
track yard init my_app
track yard check
track yard build
track yard run
```

## Installation & Testing

```bash
# Build compiler release binary
cargo build --release

# Run full test suite
cargo test
```

## License

[MIT](LICENSE)
