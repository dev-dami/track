export interface BlogPost {
  id: string;
  slug: string;
  title: string;
  date: string;
  author: {
    name: string;
    role: string;
    avatar?: string;
  };
  readTime: string;
  summary: string;
  tags: string[];
  content: string;
}

export const blogPosts: BlogPost[] = [
  {
    id: 'lexical-lenses-vs-lifetimes',
    slug: 'lexical-lenses-vs-lifetimes',
    title: 'Why We Replaced General Lifetime Annotations with Lexical Lenses',
    date: 'August 28, 2026',
    author: {
      name: 'Damilare Osibanjo',
      role: 'Systems Architect & Track Author',
    },
    readTime: '8 min read',
    summary: 'How Track achieved deterministic memory safety without a garbage collector by restricting mutable access to non-escaping lexical lenses rather than infectious lifetime parameters.',
    tags: ['Memory Safety', 'Type Theory', 'Compiler Design', 'Lenses'],
    content: `
Memory safety without a garbage collector is the holy grail of modern systems programming. Rust proved that affine types and borrow checking can eliminate use-after-free, double-free, and data races at compile time.

However, Rust's borrow checker relies fundamentally on **general lifetime parameters** (\`'a\`, \`'b: 'a\`). In practice, lifetime parameters have proven to be the single highest cognitive barrier in systems programming. They are viral: a reference stored in a struct infects the struct definition, which infects every caller, which infects interfaces and traits.

### The Research Question

When designing Track, we asked a fundamental question:

> *Can deterministic memory safety be made substantially easier to reason about by restricting mutable access to non-escaping lexical lenses rather than general lifetime-based borrows?*

### The Lexical Lens Invariant

In Track, mutable access is strictly scoped to **Lexical Lenses** declared with the \`with\` keyword:

\`\`\`track
let mut engine = Engine { rpm: 1200, fuel_rate: 45 };

// Enter lexical lens
with engine -> e {
    e.rpm = 3500;
    e.fuel_rate = 80;
    // 'e' cannot escape this block!
}
// 'engine' is unlocked and Active again
\`\`\`

During the execution of the \`with\` block, two compile-time rules are enforced:
1. **Target Freezing**: The target variable (\`engine\`) is transitioned to the \`Locked\` state. It cannot be moved, borrowed, or accessed outside the lens alias.
2. **Lexical Confinement**: The alias (\`e\`) cannot be assigned to outer variables, cannot be returned from the function, and cannot be placed into heap structs.

Because the alias is physically incapable of escaping the block boundary, the compiler needs **zero lifetime annotations**. The lifetime of the mutable borrow is identically equal to the lexical block itself.

### Comparison: Rust vs Track

\`\`\`rust
// Rust: Lifetime annotations cascade
struct Context<'a> {
    config: &'a mut Config,
}

impl<'a> Context<'a> {
    fn update<'b>(&'b mut self, delta: i32) where 'a: 'b {
        self.config.val += delta;
    }
}
\`\`\`

\`\`\`track
// Track: Pure linear ownership + lexical lens
struct Context {
    config: Config,
}

fn update(mut ctx: Context, delta: i32) -> Context {
    with ctx.config -> c {
        c.val = c.val + delta;
    }
    return ctx;
}
\`\`\`

By eliminating lifetime parameters while retaining strict linear ownership, Track delivers memory safety that is both mechanically sound and human-readable.
    `,
  },
  {
    id: 'self-hosted-lexer-v070',
    slug: 'self-hosted-lexer-v070',
    title: 'Track v0.7.0: Building a 100% Native Lexer with Tagged Unions in Track',
    date: 'August 24, 2026',
    author: {
      name: 'Damilare Osibanjo',
      role: 'Systems Architect & Track Author',
    },
    readTime: '6 min read',
    summary: 'A deep dive into porting the host Rust lexer into native Track code, implementing 50+ token variants with tagged unions, and testing with the new yard test engine.',
    tags: ['Compiler', 'Self-Hosting', 'v0.7.0', 'Yard'],
    content: `
Self-hosting is the rite of passage for any systems language. In Track v0.7.0, we officially completed Component 1 of the self-hosted compiler: **a 100% native lexer written in Track itself** (\`compiler/src/lexer.trk\`).

### The Native Token Union

The host compiler previously used Rust's \`logos\` crate. In Track, we represent tokens as a native tagged union with over 50 variants:

\`\`\`track
union Token {
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
}
\`\`\`

### Linear vs Copy Semantics in the Checker

Porting the lexer exposed a fascinating interaction in our linear type checker: constructor values like \`Token::KwLet\` were initially being treated as linear resources. When a \`match\` arm returned a unit variant, the variable was marked \`Spent\`, triggering merge conflicts at branch exits.

In v0.7.0, we updated \`is_copy_var\` in the checker (\`src/checker/mod.rs:78\`) to recognize \`::\` qualified unit constructors as Copy primitives. This enabled clean pattern matching across the scanner:

\`\`\`track
fn lex_symbol(ch: u8) -> Token {
    match ch {
        0x2B => Token::Plus,
        0x2D => Token::Minus,
        0x2A => Token::Star,
        0x2F => Token::Slash,
        _    => Token::Unknown,
    }
}
\`\`\`

### Testing with the New \`yard test\` Engine

Alongside the self-hosted lexer, v0.7.0 introduced \`yard test\`. It scans for \`src/*_test.trk\` files, creates isolated test packages, compiles them against the native module, and reports test results.

The lexer test suite (\`compiler/src/lexer_test.trk\`) runs 10 test vectors verifying comments, string escapes, arithmetic operators, bitshifts, and pattern tokens.

Next stop: the native recursive descent parser in v0.8.0!
    `,
  },
  {
    id: 'ditching-result-type',
    slug: 'ditching-result-type',
    title: 'The Case Against Monadic Error Handling: Why Track Prefers Stack Tuples',
    date: 'August 22, 2026',
    author: {
      name: 'Damilare Osibanjo',
      role: 'Systems Architect & Track Author',
    },
    readTime: '7 min read',
    summary: 'Why monadic wrappers like Result<T, E> and hidden ? operators obscure critical systems control flow, and how explicit stack tuples and predicates provide zero-cost safety.',
    tags: ['Error Handling', 'Architecture', 'v0.5.0', 'No-GC'],
    content: `
Modern languages like Rust, Swift, and Zig have largely converged on algebraic error types or error sets. While \`Result<T, E>\` is superior to unchecked exceptions, it introduces hidden ergonomic and performance costs:
1. **Monadic Wrapping Tax**: Every return value must be packed into a discriminant enum tag, creating register spills and ABI layout overhead for large structures.
2. **Hidden Control Flow**: The \`?\` operator conceals branch instructions and stack frame returns inside seemingly innocuous expressions.
3. **Double Destructuring**: In low-level OS code, error codes are often simple integers (like POSIX \`errno\`), making full generic sum types excessive.

### Track's 5-Pronged Error Convention

In Track v0.5.0, we formalized our official error philosophy: **no \`Option\`/\`Result\` types, no \`?\` operator, no hidden control flow, and no stack unwinding.**

Instead, errors are explicit stack values:

\`\`\`track
// 1. Status codes for simple actions
fn file_remove(path: ptr<u8>) -> i32;

// 2. Stack tuples for values with errors
fn read_config(path: Str) -> (i64, i32) {
    let exists = file_exists(path.data);
    if (!exists) {
        return (0, 1); // Not found
    }
    return (42, 0); // Success
}

// 3. Destructured at call site
let (val, err) = read_config(cfg_path);
if (err != 0) {
    abort("fatal: missing required configuration");
}
\`\`\`

### Linear Payloads Stay Linear

A crucial advantage in Track is how linear types interact with tuple destructuring:

\`\`\`track
let (buffer, err) = load_packet();
if (err != 0) {
    // If error, buffer is freed immediately at scope exit!
    return err;
}
// Otherwise buffer is consumed by downstream logic
send_packet(buffer);
\`\`\`

Zero allocation, zero wrapper boxing, and every branch is 100% visible during security and performance audits.
    `,
  },
  {
    id: 'cranelift-native-backend',
    slug: 'cranelift-native-backend',
    title: 'Cranelift as a First-Class Backend: Ditching LLVM\'s Overhead for Instant Builds',
    date: 'August 18, 2026',
    author: {
      name: 'Damilare Osibanjo',
      role: 'Systems Architect & Track Author',
    },
    readTime: '9 min read',
    summary: 'Why Track chose Cranelift for its standalone native backend over heavy LLVM dependencies, achieving sub-millisecond compilation and portable JIT execution.',
    tags: ['Cranelift', 'Compilers', 'Performance', 'Code Generation'],
    content: `
Building a new programming language in the 2020s usually means defaulting to LLVM. While LLVM generates phenomenal peak-optimized machine code, it imposes heavy penalties:
- Massive build times and multi-gigabyte compiler toolchains
- Heavy dynamic library dependencies (\`libLLVM.so\`) that make standalone binary distribution painful
- High memory usage during compilation

For Track, we chose **Cranelift** (the code generator developed by the Bytecode Alliance) as our primary standalone backend.

### The Cranelift Advantage

1. **Sub-Millisecond Code Generation**: Cranelift compiles intermediate representation (CLIF IR) to native machine code at hundreds of megabytes per second.
2. **Zero System Dependencies**: Track compiles into a single, fully statically linked binary with no external LLVM shared libraries required.
3. **Clean Memory Layout**: Cranelift provides direct low-level control over calling conventions, stack frame layout, and register allocation.

### Cranelift IR Mapping in Track

When Track compiles an ownership-verified AST, it emits CLIF IR instructions directly:

\`\`\`clif
function u0:0(i64) -> i64 fast {
block0(v0: i64):
    v1 = iconst.i64 42
    v2 = iadd v0, v1
    return v2
}
\`\`\`

In v0.3.0, we introduced a shared \`Arc<dyn TargetIsa>\` across parallel compilation workers, eliminating per-file ISA re-initialization and cutting multi-file package build times by over 40%.
    `,
  },
  {
    id: 'yard-build-orchestrator',
    slug: 'yard-build-orchestrator',
    title: 'Yard: Designing an Integrated Build Orchestrator for Deterministic Systems',
    date: 'August 10, 2026',
    author: {
      name: 'Damilare Osibanjo',
      role: 'Systems Architect & Track Author',
    },
    readTime: '5 min read',
    summary: 'The design and architecture of Yard, the package manager, dependency resolver, lockfile serializer, and native test orchestrator for Track.',
    tags: ['Yard', 'Build Systems', 'Tooling', 'Testing'],
    content: `
A modern systems language is only as good as its developer tooling. C and C++ suffered for decades from a fragmented ecosystem of Makefiles, CMake, Meson, and autotools.

Track treats build orchestration as a first-class citizen with **Yard** (\`yard\`).

### Unified Manifest: \`Track.toml\`

Yard packages are declared with a clean TOML schema:

\`\`\`toml
[package]
name = "crypto_core"
version = "0.7.0"
authors = ["Damilare Osibanjo <dami@devdamilare.tech>"]

[dependencies]
hash_utils = { path = "../hash_utils" }

[build]
src = "src"
\`\`\`

### Key Architectural Pillars

- **Zero-Config Defaults**: Running \`yard build\` in any package automatically resolves dependencies, discovers all \`.trk\` modules, builds in parallel, and links an executable into \`target/\`.
- **Integrated Test Discovery (\`yard test\`)**: Automatically compiles unit test files (\`src/*_test.trk\`) and integration tests (\`tests/**/*.trk\`) as isolated temporary packages, executing them in parallel.
- **Fast Fast-Checking (\`yard check\` & \`yard lint\`)**: Tokenizes, parses, and runs the linear checker in milliseconds without touching native backend codegen.
    `,
  },
];
