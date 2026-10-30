# Self-Hosting and the Bootstrap (v1.0.0)

Track 1.0 is self-hosting: `trackc`, the compiler in `compiler/src/`, is
written entirely in Track and compiles itself to a fixed point.

```sh
scripts/bootstrap.sh
# ==> stage 0: yard builds compiler/ (Rust bootstrap compiler)
# ==> stage 1: stage1 compiles compiler/src/main.trk
# ==> stage 2: stage2 compiles compiler/src/main.trk
# ✓ stage1 and stage2 emit identical C (18954 lines)
# ✓ stage2 and stage3 executables are byte-identical (209880 bytes)
```

| Stage | Compiler | Builds |
| :--- | :--- | :--- |
| 0 | Rust `yard` / `track` (Cranelift) | `compiler/` → `stage1/trackc` |
| 1 | `stage1/trackc` | `compiler/src/main.trk` → `stage2/trackc` |
| 2 | `stage2/trackc` | `compiler/src/main.trk` → `stage3/trackc` |

The gate has two parts. Stage 1 (built by Rust) and stage 2 (built by
Track) must emit byte-identical C for the compiler, which shows the two
builds of `trackc` behave identically. Stage 2 and stage 3 executables
must then be byte-identical. CI runs the script on every push, and
`tests/selfhost_tests.rs` repeats the check under `cargo test`.

## Using trackc

```sh
trackc build hello.trk -o hello     # native executable via the C compiler
trackc run hello.trk arg1 arg2      # build to a temporary file and run
trackc check hello.trk              # parse + ownership check only
trackc emit-c hello.trk -o hello.c  # inspect the generated C
trackc tokens hello.trk             # token stream
```

`trackc` loads the root file and every local module it imports. Imports
resolve beside the importing file first (`<dir>/<path>.trk`,
`<dir>/<path>/mod.trk`), then with the Rust compiler's project-relative
candidates (`src/`, the current directory, `compiler/src/`, `compiler/`).
`std`, `std/io`, `std/fs`, `std/sys`, `std/mem`, and `math/vec` are
built-in namespaces.

## Pipeline

| Module | Role |
| :--- | :--- |
| `base.trk` | word vectors, string builders, interning, the compiler context, diagnostics |
| `lexer.trk` | tokens as `(kind, value, start, end)` records; keywords interned first |
| `ast.trk` | node pool: seven words per node, child lists in a list pool |
| `parser.trk` | recursive descent over the full grammar, same precedence as `src/parser/` |
| `loader.trk` | whole-program loading in the order the Rust checker visits modules |
| `symbols.trk` | functions, enum/union layouts, aliases, integer constants |
| `checker.trk` | the ownership checker |
| `cgen.trk` | the C99 backend |
| `runtime_c.trk` | generated copy of `runtime/track_runtime.c` |
| `main.trk` | command-line driver |

Every compiler table is a runtime-owned vector of 64-bit words referenced
by an `i64` handle, so handles are copy values for the ownership checker.
Names are interned once; per-name facts (function definition, variant
discriminant, current variable slot, C local binding) live in a
fixed-width side table indexed by name id, giving O(1) lookups without
hash maps.

## Value model (the word ABI)

Both compilers lower every Track value to one 64-bit word, and both link
the same runtime, `runtime/track_runtime.c`, where every exported function
takes and returns words:

- integers and booleans are plain words; comparisons produce `0`/`1`;
- `&&` and `||` short-circuit and produce `0`/`1`;
- strings are NUL-terminated `char*`; references (`&x`) are the value itself;
- tuples are pointers to word arrays. A tuple literal lives in a slot of the
  enclosing function; `return (a, b)` and returning a local that holds a
  tuple literal copy it to the heap;
- enums are their discriminant (explicit `= value`, otherwise previous + 1);
- values of a union with any payload variant are heap cells
  `[tag, payload...]`, and `match` loads the tag from the cell;
- division by zero (and `INT64_MIN / -1`) raises `SIGFPE`; shifts mask the
  amount to 6 bits and `>>` is logical; arithmetic wraps.

Array literals, indexing, and struct literals are parsed and ownership
checked but have no runtime representation yet; both backends evaluate
them to `0`.

## C backend contract

`trackc build app.trk -o out/app` writes `out/app.c` (the program) and
`out/app.rt.c` (the runtime), then runs, from inside `out/`:

```sh
$CC -std=gnu99 -O2 -fwrapv -fno-strict-aliasing -fno-builtin -w \
    -o app app.c app.rt.c
```

`$CC` defaults to `cc`. Compiling inside the output directory means object
files record only base names, so identical inputs give identical
executables wherever they are built. The generated C needs nothing but
`<stdint.h>`: all values are `tk_w` (`int64_t`), Track function `f` becomes
`tk_f`, locals are `lN`, temporaries `tN`, tuple slots `sN`, and every
counter restarts per function. Calls to functions that are neither Track
functions nor runtime functions are declared as externs from their first
call site.

## The runtime

`runtime/track_runtime.c` is the single source of truth. The Rust compiler
embeds it with `include_str!`; `scripts/embed_runtime.sh` regenerates
`compiler/src/runtime_c.trk`, which holds the source as Track string
literals plus each function's prototype and arity. CI runs
`scripts/embed_runtime.sh --check` so the two copies cannot drift.

## Ownership checker

`checker.trk` implements the same rules as `src/checker/` (see
[SPEC.md](../SPEC.md)): moves of non-copy values, borrows that freeze their
target while the reference is live, lens locks, the `if`/`else` merge rule,
no consumption of outer resources in loops, reference escape analysis, lens
escape analysis, forbidden manual frees, and the "did you mean" check for
misspelled calls. Instead of cloning maps at every branch it keeps a table
of variable slots (name, state, type, mutability, lens flag, the previous
slot with the same name, provenance) and snapshots slot states at
branches. `tests/selfhost_tests.rs` asserts that both checkers return the
same verdict on a shared set of accepted and rejected programs.

## Testing

```sh
cargo test --test selfhost_tests   # fixed point, parity, verdicts, native tests
(cd compiler && yard test)          # native tests built by the Rust compiler
(cd compiler && target/trackc run tests/parser_test.trk)  # ... or by trackc
```
