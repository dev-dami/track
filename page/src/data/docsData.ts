export interface DocChapter {
  id: string;
  title: string;
  subtitle: string;
  category: 'Getting Started' | 'Core Language' | 'Advanced Systems' | 'Toolchain & Compiler';
  content: string;
  codeSnippets?: {
    title: string;
    language: 'track' | 'bash' | 'toml';
    code: string;
  }[];
}

export const docsChapters: DocChapter[] = [
  {
    id: 'intro',
    title: 'Introduction & Design Philosophy',
    subtitle: 'Deterministic memory safety without lifetime annotations or garbage collection',
    category: 'Getting Started',
    content: `
Track is a modern, low-level systems programming language designed from the ground up for predictable latency, deterministic resource management, zero-cost abstractions, and real-time execution.

### The Problem Track Solves

Systems programmers have historically been forced into a difficult triad of trade-offs:
1. **Manual Memory Management (C / C++)**: Extreme performance and control, but plagued with spatial and temporal memory safety bugs (use-after-free, double-free, buffer overflows, memory leaks).
2. **Garbage Collection (Go / Java / C#)**: Safe memory management, but at the cost of non-deterministic stop-the-world pauses, hidden runtime allocations, increased memory overhead, and unsuitable predictability for kernels and embedded runtimes.
3. **Complex Lifetime Annotations (Rust)**: Safe and GC-free, but introduces significant cognitive complexity with infectious lifetime parameters (\`'a\`, \`'b\`), borrow-checker fighting, and complex reference graphs.

### Track's Core Innovations

Track answers this with three foundational principles:
- **No Garbage Collector, No Hidden Runtime**: Resource lifecycles are statically verified at compile time. Destruction code (\`vec_free\`, \`str_free\`) is automatically emitted at deterministically computed spend points.
- **Linear Ownership State Machine**: Resources have exactly one active owner. Every variable transitions strictly between 4 states: \`Active\`, \`Borrowed\`, \`Locked\`, and \`Spent\`.
- **Lexical Lenses (\`with\` blocks)**: Scoped, non-escaping mutable views that eliminate lifetime annotations altogether.
- **Cranelift Standalone Backend**: Instant compilation with native machine code generation for x86_64 and aarch64, avoiding heavy LLVM dynamic dependencies.
- **Explicit Stack-Based Error Handling**: No monadic wrappers (\`Option\` / \`Result\`), no \`?\` hidden control flow, no stack unwinding. Errors are first-class stack values.
    `,
    codeSnippets: [
      {
        title: 'hello.trk — Standard I/O and Void Main',
        language: 'track',
        code: `import "std/io";

fn main() -> void {
    io::print("Hello, Track systems world!");
}`,
      },
    ],
  },
  {
    id: 'installation',
    title: 'Installation & Toolchain Setup',
    subtitle: 'Get up and running with track, yard, and track-lsp in seconds',
    category: 'Getting Started',
    content: `
Track provides a unified, single-binary toolchain consisting of the standalone compiler (\`track\`), the package manager & test runner (\`yard\`), and the Language Server Protocol daemon (\`track-lsp\`).

### Quick Installation via Shell Script

On Linux (x86_64, aarch64) or macOS, install the complete Track toolchain using the official installer:

\`\`\`bash
curl -fsSL https://raw.githubusercontent.com/dev-dami/track/main/scripts/install.sh | bash
\`\`\`

This downloads and configures \`track\`, \`yard\`, and \`track-lsp\` into your \`/usr/local/bin\` or \`~/.local/bin\`.

### Building from Source

Track's host compiler is implemented in Rust (2024 edition) and builds cleanly using Cargo:

\`\`\`bash
# Clone the repository
git clone https://github.com/dev-dami/track.git
cd track

# Build release binaries (track, yard, track-lsp)
cargo build --release

# Run local installer
./scripts/install.sh
\`\`\`

### Verifying Toolchain Installation

\`\`\`bash
track --version
# track 0.7.0

yard --version
# yard 0.7.0
\`\`\`
    `,
    codeSnippets: [
      {
        title: 'Terminal Quickstart',
        language: 'bash',
        code: `# Create a new Track package
yard init my_kernel_module
cd my_kernel_module

# Type-check without building
yard check

# Build native binary to target/my_kernel_module
yard build

# Run project executable
yard run

# Execute test suite
yard test`,
      },
    ],
  },
  {
    id: 'ownership',
    title: 'Linear Ownership & Spend Points',
    subtitle: 'Compile-time use-after-free and double-free prevention with zero runtime cost',
    category: 'Core Language',
    content: `
Every value in Track belongs to one of four value categories:

| Category | Semantics | Copyable? | Movable? | Scope Bound |
| :--- | :--- | :--- | :--- | :--- |
| **Owned Linear** | Unique ownership of heap/stack resource | No | Yes | Dynamic / Move |
| **Lexical Lens** | Exclusive, non-escaping mutable view | No | No | \`with\` Block |
| **Reference (\`&T\`)** | Read-only borrow | Yes | Yes | Borrowed Scope |
| **Copy Primitive** | Primitive values (\`i32\`, \`i64\`, \`bool\`) | Yes | Yes | Value |

### The 4 Ownership States

The Track compiler tracks variable state across basic blocks:
1. **\`Active\`**: Value is initialized, alive, and owned.
2. **\`Borrowed\`**: One or more read-only references (\`&T\`) exist. The owner is frozen from moves.
3. **\`Locked\`**: An exclusive lexical lens (\`with\`) is active. The owner is frozen.
4. **\`Spent\`**: Ownership has been moved or transferred. Attempting to use a spent variable yields error \`TK201\`.

### Spend Points & Automatic Cleanup

A linear resource is consumed (spent) by:
1. **Move Assignment**: \`let y = x;\` transfers ownership from \`x\` to \`y\`. \`x\` transitions to \`Spent\`.
2. **Function Call Transfer**: Passing an owned value by value into a function parameter transfers ownership to the callee.
3. **Implicit Scope Cleanup**: If a linear resource remains \`Active\` at scope exit, the compiler automatically inserts destructor cleanup code.

### Struct Atomicity Policy
Structs in Track are moved as **atomic units**. Moving any individual field or the struct itself consumes the entire instance, preventing partial uninitialized states or double destruction.
    `,
    codeSnippets: [
      {
        title: 'Linear Allocation & Auto Free',
        language: 'track',
        code: `import "std/io";

struct Buffer {
    data: ptr<u8>,
    len: u32,
    cap: u32,
}

fn create_buffer(capacity: u32) -> Buffer {
    return Buffer {
        data: alloc(capacity),
        len: 0,
        cap: capacity,
    };
}

fn process(buf: Buffer) -> void {
    // Ownership transferred here; buf is freed when this function exits!
    io::print_int(buf.cap);
}

fn main() -> void {
    let mut b = create_buffer(1024);
    // b is Active
    process(b); 
    // b is now Spent!
    
    // ERROR: use of moved variable \`b\` [TK201]
    // io::print_int(b.len); 
}`,
      },
    ],
  },
  {
    id: 'lenses',
    title: 'Lexical Lenses & Non-Escaping Guarantees',
    subtitle: 'Scoped mutable access without general lifetime parameters',
    category: 'Core Language',
    content: `
In Rust, mutable borrows require lifetime annotations whenever functions or data structures return or store references. This leads to complex lifetime syntax like \`fn update<'a, 'b: 'a>(target: &'a mut Context<'b>)\`.

Track replaces general mutable references with **Lexical Lenses** using the \`with\` construct.

### Core Research Hypothesis

> *Can deterministic memory safety be made substantially easier to reason about by restricting mutable access to non-escaping lexical lenses rather than general lifetime-based borrows?*

### The \`with\` Block Syntax

\`\`\`track
let mut u = User { age: 30, score: 100 };

with u -> user {
    user.set_age(31);
    user.increment_score(10);
}
// Outer variable 'u' is restored to Active state here
\`\`\`

### Strict Invariants

1. **Lexical Exclusivity**: While a lens is active inside the \`with\` block, the outer target resource (\`u\`) is locked into the \`Locked\` state. It cannot be moved, borrowed, or accessed outside the lens.
2. **Non-Escaping Guarantee**: The lens alias (\`user\`) is valid **strictly** within the lexical scope of the block. It cannot be assigned to an outer variable, returned from a function, or stored in a heap data structure.
3. **Zero Lifetime Annotations**: Enforced completely by block scope boundaries without any \`'a\` lifetime parameters.
    `,
    codeSnippets: [
      {
        title: 'Scoped Lexical Lens Example',
        language: 'track',
        code: `struct Engine {
    rpm: i32,
    temp_celsius: i32,
}

fn calibrate(mut eng: Engine) -> Engine {
    // Enter lexical lens block
    with eng -> e {
        e.rpm = 3000;
        e.temp_celsius = 85;
    }
    // eng is automatically unlocked and Active again
    return eng;
}`,
      },
    ],
  },
  {
    id: 'types',
    title: 'Type System & Memory Primitives',
    subtitle: 'Primitives, pointer types, slices, and tagged unions',
    category: 'Core Language',
    content: `
Track's type system is statically checked, strictly typed, and monomorphic at machine level.

### Primitive Copy Types
Copy types can be duplicated without consuming the original:
- **Integers**: \`i8\`, \`i16\`, \`i32\`, \`i64\`
- **Unsigned**: \`u8\`, \`u16\`, \`u32\`, \`u64\`
- **Booleans**: \`bool\` (\`true\` / \`false\`)
- **Unit**: \`void\`

### Memory Pointer Types (\`ptr<T>\`)
Raw pointers represent untracked memory addresses, primarily used in stdlib runtime implementation and C FFI:
\`\`\`track
let buf: ptr<u8> = alloc(1024);
memset(buf, 0, 1024);
\`\`\`

### Fat Slices (\`[]T\`)
Slices are fat pointers containing a pointer and a length:
\`\`\`track
let arr: [i64; 5] = [10, 20, 30, 40, 50];
let slice: []i64 = arr[1..4];
\`\`\`

### Tagged Unions (\`union\`)
Unions are linear tagged variants capable of holding heterogeneous typed payloads:
\`\`\`track
union Value {
    Int(i64),
    Float(i64),
    Text(Str),
    Flag(bool),
}
\`\`\`
    `,
    codeSnippets: [
      {
        title: 'Slices & Unions Demo',
        language: 'track',
        code: `union Node {
    Leaf(i32),
    Branch(i32, i32),
    Empty,
}

fn describe(n: Node) -> void {
    match n {
        Node::Leaf(val) => print_int(val),
        Node::Branch(left, right) => {
            print_int(left);
            print_int(right);
        },
        Node::Empty => print_str("none"),
    }
}`,
      },
    ],
  },
  {
    id: 'tuples-patterns',
    title: 'Tuples, Destructuring & Pattern Matching',
    subtitle: 'Anonymous tuples, deep pattern matching, and match arm guards',
    category: 'Core Language',
    content: `
Track v0.4.0 introduced first-class anonymous tuples and advanced nested pattern matching.

### Anonymous Tuples
Tuples group multiple heterogeneous types without requiring nominal struct declarations:
- **Tuple Types**: \`(i64, bool)\`, \`(Str, i32, ptr<u8>)\`
- **Indexing**: Numerical zero-based dot notation: \`pair.0\`, \`pair.1\`
- **Destructuring**: \`let (status, payload) = get_packet();\`

### Pattern Matching with Guards
The \`match\` expression evaluates patterns sequentially. Arm guards allow conditional filtering:

\`\`\`track
let pair: (i64, bool) = (42, true);
let (n, flag) = pair;

match n {
    x if x > 100 => io::print("very large"),
    x if x > 10  => io::print("medium"),
    0            => io::print("zero"),
    _            => io::print("small"),
}
\`\`\`
    `,
    codeSnippets: [
      {
        title: 'Nested Pattern Matching',
        language: 'track',
        code: `union Result {
    Ok((i32, bool)),
    Err(i32),
}

fn handle_response(res: Result) -> void {
    match res {
        Result::Ok((val, flag)) if flag => {
            io::print_str("success with active flag");
            io::print_int(val);
        },
        Result::Ok((val, _)) => {
            io::print_str("success");
            io::print_int(val);
        },
        Result::Err(code) => {
            io::print_err("failed with status");
            io::print_int(code);
        },
    }
}`,
      },
    ],
  },
  {
    id: 'errors',
    title: 'Explicit Stack-Based Error Handling',
    subtitle: 'Zero monadic wrappers, zero unwinding, 100% visible control flow',
    category: 'Core Language',
    content: `
Track intentionally rejects wrapper-based error handling. There are no \`Option\` or \`Result\` types, no hidden \`?\` control-flow operators, and no stack unwinding.

### The 5 Error Conventions

| Style | Signature Shape | Use Case |
| :--- | :--- | :--- |
| **Status Code** | \`-> i32\` (\`0\` = ok) | Simple success/failure |
| **Tuple Return** | \`-> (T, i32)\` | Value + status return destructured at call site |
| **Out-Param** | \`-> i32\` with \`out: &T\` | Hot paths, zero allocation |
| **Boolean Predicate** | \`-> bool\`, checked first | Ambiguous sentinels (\`str_is_int\`, \`env_exists\`) |
| **Fatal Abort** | \`abort(msg)\` | Unrecoverable process termination (exit 134) |

### Why Track Avoids \`Result<T, E>\`
1. **Zero Allocation / Zero Wrapper Tax**: Status codes and tuples fit into registers or stack frames.
2. **Explicit Audit Trail**: Every error path is directly visible in the source code without hidden early returns.
3. **Linear Safety Preserved**: When destructuring \`(val, err)\`, the payload follows normal linear ownership rules.
    `,
    codeSnippets: [
      {
        title: 'Explicit Error Handling Example',
        language: 'track',
        code: `fn read_config(path: Str) -> (i64, i32) {
    let exists = file_exists(path.data);
    if (!exists) {
        return (0, 1); // 1 = Not Found
    }
    let size = file_size(path.data);
    if (size < 0) {
        return (0, 2); // 2 = Stat Failed
    }
    return (size, 0); // 0 = Success
}

fn main() -> void {
    let (size, err) = read_config(str_from_literal("/etc/track.conf"));
    if (err != 0) {
        print_err("Could not read configuration file");
        return;
    }
    print_int(size);
}`,
      },
    ],
  },
  {
    id: 'generics',
    title: 'Monomorphized Generics (v0.6.0)',
    subtitle: 'Zero-cost template monomorphization without vtables or dynamic dispatch',
    category: 'Core Language',
    content: `
Generic functions in Track are compile-time templates that specialize into concrete native machine code at each distinct call site.

### Defining Generic Functions

\`\`\`track
fn identity<T>(x: T) -> T {
    return x;
}

fn pair<T, U>(a: T, b: U) -> (T, U) {
    return (a, b);
}
\`\`\`

### Monomorphization Pipeline (\`src/mono.rs\`)

1. **Call Site Type Inference**: Type parameters (\`T\`, \`U\`) are inferred automatically from actual argument types.
2. **Specialization & Mangling**: Each distinct combination produces a mangled concrete function (e.g. \`identity__Ti32\`, \`pair__Ti32_Ubool\`).
3. **Direct Emitted Cranelift Machine Code**: No dynamic boxing, no runtime vtable pointer indirection.
    `,
    codeSnippets: [
      {
        title: 'Monomorphized Generics Demo',
        language: 'track',
        code: `fn swap_pair<A, B>(p: (A, B)) -> (B, A) {
    let (first, second) = p;
    return (second, first);
}

fn main() -> void {
    let orig: (i32, bool) = (100, true);
    // Specializes swap_pair__Ti32_Ubool
    let swapped = swap_pair(orig); 
    
    let (flag, val) = swapped;
    if (flag) {
        print_int(val);
    }
}`,
      },
    ],
  },
  {
    id: 'yard',
    title: 'Yard — Package Manager & Build Orchestrator',
    subtitle: 'Unified build tool, package resolver, lockfiles, and test framework',
    category: 'Toolchain & Compiler',
    content: `
\`yard\` is Track's integrated package manager, build orchestrator, and test runner.

### Package Structure (\`Track.toml\`)

\`\`\`toml
[package]
name = "hyper_engine"
version = "0.7.0"
authors = ["Damilare Osibanjo <dami@devdamilare.tech>"]

[dependencies]
logger = { path = "../logger" }

[build]
src = "src"
\`\`\`

### Yard Commands

- **\`yard init <name>\`**: Scaffolds a new Track project directory with boilerplate.
- **\`yard check\`**: Performs rapid lexical scanning, parsing, and ownership verification without codegen.
- **\`yard lint\`**: Runs static analysis lint rules for code safety.
- **\`yard build\`**: Compiles all \`.trk\` modules and links them into \`target/<package_name>\`.
- **\`yard run\`**: Builds and executes the native binary.
- **\`yard test\`**: Automatically discovers and executes all \`src/*_test.trk\` and \`tests/**/*.trk\` test suites.
- **\`yard clean\`**: Clears intermediate compilation caches.
    `,
    codeSnippets: [
      {
        title: 'Yard Project Layout',
        language: 'bash',
        code: `my_project/
├── Track.toml
├── src/
│   ├── main.trk
│   ├── lexer.trk
│   └── lexer_test.trk
└── target/
    └── my_project`,
      },
    ],
  },
  {
    id: 'lsp',
    title: 'Language Server Protocol (LSP) & IDE Setup',
    subtitle: 'Real-time diagnostics, auto-completion, and markdown track-block checking',
    category: 'Toolchain & Compiler',
    content: `
Track ships with a dedicated LSP daemon (\`track-lsp\`) built on top of Tower-LSP and Tokio.

### Key Capabilities
- **Live Diagnostics**: Real-time ownership checking, type errors, and span highlights in \`.trk\` files.
- **Markdown Code Block Checking**: \`track-lsp\` parses and type-checks \`\`\`track code blocks inside \`.md\` documentation files!
- **Fast Auto-Completion**: Keywords, stdlib modules, structs, and tagged union variants.
- **AST Caching & Stale Result Discarding**: Prevents publishing outdated diagnostics on fast keystrokes.

### VS Code Configuration

Add this to your \`.vscode/settings.json\`:

\`\`\`json
{
  "language-server.track": {
    "command": "track-lsp",
    "filePatterns": ["*.trk", "*.md"]
  }
}
\`\`\`

### TextMate Syntax Highlighting
TextMate grammar is provided at \`grammars/track.tmLanguage.json\`.
    `,
    codeSnippets: [
      {
        title: 'VS Code Extension Setup',
        language: 'bash',
        code: `mkdir -p ~/.vscode/extensions/track-syntax/syntaxes
cp grammars/track.tmLanguage.json ~/.vscode/extensions/track-syntax/syntaxes/`,
      },
    ],
  },
  {
    id: 'self-hosting',
    title: 'Self-Hosted Compiler & Roadmap to v0.9.0',
    subtitle: 'Track lexer in native Track and the 3-stage bootstrapping gate',
    category: 'Toolchain & Compiler',
    content: `
Track is actively executing its path to full compiler self-bootstrapping.

### Milestone Progress Overview

\`\`\`
[v0.1.6] Slices & Memory Primitives     ✅ DONE
[v0.2.0] UFCS, Lenses & CFG Merging     ✅ DONE
[v0.3.0] Perf & Self-Hosting Readiness  ✅ DONE
[v0.4.0] Tuples & Pattern Matching      ✅ DONE
[v0.5.0] Explicit Error Handling        ✅ DONE
[v0.6.0] Monomorphized Generics         ✅ DONE
[v0.7.0] Self-Hosted Lexer in Track     ✅ DONE (compiler/src/lexer.trk)
[v0.8.0] Parser, Checker & C-Emitter    ⏳ IN PROGRESS
[v0.9.0] 3-Stage Bootstrapping Verified 🎯 MILESTONE
\`\`\`

### v0.7.0 Milestone: Self-Hosted Lexer
In v0.7.0, Track successfully ported the host lexer from Rust into 100% native Track (\`compiler/src/lexer.trk\`), recognizing 50+ token variants with native union matching and 10 unit test suites.

### The 3-Stage Bootstrapping Gate (v0.9.0)
1. **Stage 0**: Host Rust \`track\` compiler compiles \`compiler/*.trk\` → emits \`track_stage1\` executable.
2. **Stage 1**: \`track_stage1\` compiles \`compiler/*.trk\` → emits \`track_stage2\` executable.
3. **Stage 2**: \`track_stage2\` compiles \`compiler/*.trk\` → emits \`track_stage3\` executable.
4. **Verification**: Assert byte-for-byte binary equality (\`cmp track_stage2 track_stage3\`).
    `,
    codeSnippets: [
      {
        title: 'compiler/src/token.trk — Native Token Union',
        language: 'track',
        code: `union Token {
    KwLet,
    KwMut,
    KwFn,
    KwWith,
    KwMatch,
    KwReturn,
    KwStruct,
    KwUnion,
    Ident(Str),
    Number(i64),
    StringLit(Str),
    Arrow,
    FatArrow,
    ColonColon,
    Eof,
}`,
      },
    ],
  },
];
