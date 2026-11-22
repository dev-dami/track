# Self-Hosting and the Bootstrap (v1.0.0)

Track 1.0 is self-hosting: `trackc`, the compiler in `compiler/src/`, is
written entirely in Track and compiles itself to a fixed point.

```sh
scripts/bootstrap.sh
# ==> stage 0: yard builds compiler/ (Rust bootstrap compiler)
# ==> stage 1: stage1 compiles compiler/src/main.trk
# ==> stage 2: stage2 compiles compiler/src/main.trk
# ✓ stage1 and stage2 emit identical C (72502 lines)
# ✓ stage2 and stage3 executables are byte-identical (635080 bytes)
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
trackc build --native hello.trk -o hello     # static x86-64 ELF, no C compiler
trackc build --native -g hello.trk -o hello  # ... with a symbol table
trackc run hello.trk arg1 arg2      # build to a temporary file and run
trackc check hello.trk              # parse + ownership check only
trackc emit-c hello.trk -o hello.c  # inspect the generated C
trackc tokens hello.trk             # token stream
trackc life hello.trk               # inferred lifecycle summaries
trackc life --place hello.trk       # placement of every allocation site
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
| `life.trk` | lifecycle summaries, errors E0211-E0214, warnings W0301-W0305, the placement plan ([lifecycles.md](lifecycles.md)) |
| `cgen.trk` | the C99 backend |
| `ngen.trk`, `x64.trk`, `elf.trk` | the native backend (below) |
| `runtime_c.trk` | generated copy of `runtime/track_runtime.c` |
| `runtime_trk.trk` | generated copy of the Track runtime, `compiler/rt/*.trk` |
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
- division by zero (and `INT64_MIN / -1`) flushes buffered stdout, then
  raises `SIGFPE` (shell status 136) under the C and the Track runtime alike
  (native `INT64_MIN / -1` traps in hardware without the flush); a
  constant zero divisor or constant `INT64_MIN / -1` (also in enum
  discriminants) is E0200 at compile time in both checkers; `exit(n)`
  flushes stdout and exits with `n`; shifts mask the amount to 6 bits and
  `>>` is logical; arithmetic wraps;
- `for x in a..b` binds `a`, `a + 1`, ..., `b - 1`; `for x in arr` binds
  each element of array `arr` in order (`arr` moves into the loop; iterate
  `&arr` to keep it); an empty array runs the body zero times, and a scalar
  iterable is a type error (count with `for i in 0..n`). A range is
  only a `for` iterator or a slice index: trackc rejects one used as a
  value (`let r = 0..3;`) with E0200.

- structs are heap cells `[tag, fields...]` with fields in declaration
  order; the tag is a hash of the struct name, so separately compiled
  modules agree. `p.x` reads a field and `p.x = v` writes one; a field name
  declared by several structs dispatches on the tag;
- arrays are heap cells `[length, elements...]`; `a[i]` and `a[i] = v` are
  bounds checked (out of range aborts with status 134), `a[s..e]` / `a[s..]`
  copy a slice, and `array_len(&a)` returns the length.

## C backend contract

`trackc build app.trk -o out/app` creates `out/` if missing, writes
`out/app.c` (the program) and `out/app.rt.c` (the runtime), then runs, from
inside `out/`:

```sh
$CC -std=gnu99 -O2 -fwrapv -fno-strict-aliasing -fno-builtin -w \
    -o app app.c <cache>/rt-<key>.o
```

`trackc run` is the same with `-O1` for the program (faster turnaround;
`build` stays at `-O2`). `$CC` defaults to `cc`.

The runtime is compiled once to `<cache>/rt-<key>.o` (same flags, always
`-O2`, compiled from `rt-<key>.c` in the cache directory) and linked by every
later build. `<key>` is a hash of `$CC`, the flags and the runtime source
text, so changing any of them makes a new object. The cache directory is
`$TRACKC_CACHE`, else `$XDG_CACHE_HOME/trackc`, else `~/.cache/trackc`; if it
cannot be used, `app.rt.c` is compiled with the program as before. The
program stays one translation unit. Compiling inside the output directory
means object files record only base names, and the cached runtime object is
named by its key, so identical inputs give identical executables wherever
they are built.
The generated C needs nothing but
`<stdint.h>`: all values are `tk_w` (`int64_t`), Track function `f` becomes
`tk_f`, locals are `lN`, temporaries `tN`, tuple slots `sN`, and every
counter restarts per function. Calls to functions that are neither Track
functions nor runtime functions are declared as externs from their first
call site.

## Native backend (no C toolchain)

`trackc build --native app.trk -o app` lowers the checked AST straight to
x86-64 machine code and writes a static Linux ELF executable: no C
compiler, assembler or linker runs.

| Module | Role |
| :--- | :--- |
| `x64.trk` | instruction encoder (one byte per vector word, rel32 patching) |
| `ngen.trk` | lowering from the AST, construct for construct like `cgen.trk` |
| `elf.trk` | one RWX `PT_LOAD` segment at `0x400000`; written with raw syscalls |

Every expression leaves its value in `rax` (`rcx`, `rdx` are scratch);
operands wait on the machine stack; locals and temporaries are
`rbp`-relative slots, the busiest ones (weighted by loop depth) in
`rbx`, `r12`-`r15`, `r8`, `r9`, which a function pushes after `rbp` and
pops before returning when it uses them.
Functions of at most six parameters take them in `rdi`, `rsi`, `r10`,
`r11`, `rdx`, `rcx`; with more, the caller pushes the arguments left to
right, the callee reads argument `i` of `n` at `[rbp + 16 + 8(n-1-i)]` and
pops them on return (`ret 8n`).
A function that makes no calls, syscalls or scratch-heavy inline
expansions is a leaf: parameters stay in their argument registers, other
hot locals use the free ones of `rdi`, `rsi`, `r10`, `r11` at no save
cost, and with every used slot in a register it gets no frame at all.
`return f(args)` is a jump (a tail call), so it never grows the stack;
between functions of at most six parameters it does not stop its function
from being a leaf. With more than six parameters on either side the frame
is rebuilt in place: stack arguments move up over the caller's own (a
top-down copy, as the areas may overlap) with the return address below
them, so the callee's `ret 8n` leaves the stack as the caller's return
would have. Accumulator recursion
(`return f(a) + f(b)`) becomes a loop and inlines into itself three levels
deep, testing a leading `if cond { return e; }` guard before each
remaining call (its arguments computed straight into argument registers).
Each such loop tests its guard at the bottom, with the guarded return out
of line, so an iteration takes one branch. `str_char_at` is inline: an
SSE2 scan of aligned 16-byte windows that touches only `rax`, `rcx`,
`rdx`, `xmm0`, `xmm1`; in a loop that writes no memory (arithmetic, local
assignments, `str_char_at`, `str_len` and calls of functions that qualify)
the length of each never-reassigned local string it indexes is computed
once before the loop, and each call is an unsigned bounds test and a byte
load. Likewise, in a loop made only of such code plus collection get, set
and len, each never-reassigned, never-redeclared local handle has its data
pointer and length loaded once before the loop, and each access is one
unsigned bounds compare and a load or store (out of range: a cold stub
giving 0). A call of a collection forwarder, a function that passes its
`i64` parameters unchanged to one collection get, set, len or push
(`vget`, `vset`, ...), directly or through another forwarder, is emitted
as that primitive on the call's own arguments (`ng_forward`); a get by a
constant index compares it with the length directly, and a get or set
whose handle is itself such a get (`vget(cx(c, K), n * W)`) and whose
index is arithmetic on locals and constants computes the index first
into `rdx`, with no push and pop around the inner get. `x * (2^k -+ 1) +- y` is a shift and two adds with the `y -+ x`
side independent of the shift. Indexes of a never-reassigned array literal by `x & C`
with `C` below its length skip the null and bounds checks. A `while` body that ends in
an else-less `if` (then only assignments and calls that use none of the
body's `let` names) is laid out latch first: the tail and the loop test are
emitted a second time ahead of the body, where the `if`'s false branch
jumps, so an iteration takes one taken branch whether or not the `if` body
runs. The failure and slow paths of the inline collection get, set and
push are stubs after the function that jump back, so the fast path falls
through. `(x & C) == 0` is one `test`; an in-bounds element store whose
value is unused needs no temporary; `y + a[i]` (also `-`, `&`, `|`) with an
in-bounds index and `a`, `i` in registers reads the element as a memory
operand; `n % d` by a constant adds the sign
fix-up `s & -d` to `n` beside the quotient multiply instead of after it. `syscall6`, `load64`, `store64`, `load8`,
`store8` compile to instructions, as do the libc names `elf.trk` uses
(`open64`, `write`, `fchmod`, `close`, `memset`) so that the Rust
bootstrap compiler, which knows no machine builtins, links them from libc. String
literals follow the code. `_start` calls `rt_start(argc, argv)`, then
`main()`, then `sys_exit(result)` (the compiler/rt `rt_start` runs main
and exits itself, so the rest is a fallback; see docs/native.md).

Tuple, struct, array and payload-union literals are heap cells unless
the placement plan (`life.trk`, `trackc life --place`) puts them on the
stack; then the cell is a run of contiguous frame slots with the heap
layout (word `i` at `[rbp + d + 8i]`, every word written), one run per
literal site reused on each execution, and its address is the value. A
function with such a cell keeps its `rbp` frame even as a leaf. A local
handed to any call inside a `return` value (`return f(&p)`, or the
accumulator form) is never a frame cell, since a tail call drops the frame
before the callee runs and an accumulator loop reruns the body over the
same slots. Only `let (a, b) = (x, y)` binds tuple elements without any
cell. `local.field = v` for a field only one struct has is one store
through the local's register, and an inlined call's `&local` argument
shares the local's slot. Identical string
literals share one copy, so `"abc" == "abc"` is `1` as with the C
compiler's literal merging. A program without `main` gets an empty one
that runs its top-level statements, as on the C path.

Cells the plan marks `heap-autofree` are freed as the C backend frees
them (card L12a; cgen's `gen_frees`/`gen_drop` are the specification):
at block end, at every `return` after its value is read, and the old cell
at each reassignment once the new one is built; `deep` sites run drop
glue into their owned child cells. A tail call frees after its arguments
are computed and before the jump, unless an argument could hold one of
the cells (anything but arithmetic or a scalar-typed read): then it is an
ordinary call. A latch-first loop whose body declares such a cell sends
the skipped `if` to the frees ahead of the latch, and an accumulator loop
frees before its back edge (functions that would pass a cell to the
recursive call keep plain recursion). An argument temporary the plan
frees after the recursive call (`x + f(.., &mk(n), ..)`) is carried by
the loop instead: it is parked in a slot zeroed before the loop, a back
edge frees the parked cell when it stores a fresh temporary in its place
(a pass-through keeps it), and every loop exit frees it. A return that
lets another parameter, a local or an argument point into a parked cell
keeps its call (cards F3, F3b). The free is inlined (`ng_dealloc_rax`:
bytes in use down, block onto its class's free list; large blocks call
`dealloc`), and the inlined allocator pops a freed block of 32 or 64
bytes and zeroes its payload before it bumps the arena.

Functions with a caller return slot (`life_ret_slot_fn`, card L10) are
emitted twice (card L12b): `<f>__rs` takes the address of the result cell
as one more argument after its own (the next argument register, or
pushed last when that makes more than six words, popped by the callee's
`ret imm16`), writes it at each `return` once every word is in a temp,
and returns it in rax; `return g(..)` passes it on as a tail call; `<f>`
allocates the cell and calls `<f>__rs`. `let x = f(..)` with `x` stack
passes a run of frame slots, `x = f(..)` with `x` stack or heap-autofree
passes `x`'s own cell (nothing is freed), and every other call uses
`<f>`. An inlined ret-slot body writes the caller's cell, or a heap cell
where the call would use `<f>`.

The runtime is Track code loaded with the program: the compiler/rt
modules embedded in `runtime_trk.trk`. It is loaded only after the user
program has been checked (also for `--rt=track`), so its plain-`i64`
definitions of `str_concat`, `vec_push`... never replace the builtin
signatures the ownership checker types the program with: `build --native`
rejects exactly what `trackc check` rejects.

A program may define any name the runtime uses internally (`sum`,
`write`, `O_RDONLY`...). When it does, the runtime's function or constant
is renamed to a fresh `__rt_<name>` together with the runtime's own uses
of it (`isolate_runtime` in symbols.trk), so the program's calls reach its
definition and the runtime's calls reach the runtime's. The few names the
code generators call by name (`alloc`, `dealloc`, `str_eq`, `str_len`,
`str_char_at`, `array_len`, `sys_exit`, `memset`, `rt_start`, `track_*`
(among them `track_alloc_nz`, the non-zeroing `alloc` for cells),
and the runtime-state layout constants `RT_BASE`, `RS_USED`, `RS_HEAP`,
`RS_LIMIT`, `RS_HEAP_END`, `RS_FREE` that ngen's inline allocator reads)
stay reserved: defining one under `--native`/`--rt=track` is E0201. On the
C path a function `f` is the C symbol `tk_f`, or `tku_f` when `tk_f` is a
C prelude symbol (`tk_w`, `tk_shl`, `tk_div`...).

The image carries a `PT_GNU_STACK` header (read/write, no execute) and
runs on the kernel's default stack (usually 8 MiB). Only leaf frames
and tail calls are optimised away, so very deep non-tail recursion (about
a million calls) overflows the stack and dies with SIGSEGV where the C
build at `-O2` may survive.
The natively built `trackc` compiles itself to a byte-identical fixed
point (`tests/native_tests.rs`); `scripts/native_parity.sh <trackc>`
compares native and C builds of the examples.

### Bootstrapping from .trk alone

```sh
scripts/selfhost.sh
# stage  built by                                bytes    ms  sha256
# n1     seed (build/bootstrap/stage3/trackc)  1484541   457  07c2e45109dd6957
# n2     n1, env -i PATH=/nonexistent          1484541   202  07c2e45109dd6957
# n3     n2, env -i PATH=/nonexistent          1484541   173  07c2e45109dd6957
# n4     n3, env -i PATH=/nonexistent          1484541   236  07c2e45109dd6957
# ✓ n2 = n3 = n4: Track reproduces itself from .trk with no C toolchain
# ✓ processes started by n4 while compiling itself: 0 (strace)
```

A seed `trackc` (any build, by default the stage3 of `scripts/bootstrap.sh`)
compiles `compiler/src/main.trk` with `--native` once. From then on each
native `trackc` compiles the compiler, runtime included, from `.trk` source
with an empty environment and no reachable programs. The compiler, the
runtime (`compiler/rt/`) and the machine-code backend are all Track; CI runs
the script on every push.

## The runtime

`runtime/track_runtime.c` is the single source of truth. The Rust compiler
embeds it with `include_str!`; `scripts/embed_runtime.sh` regenerates
`compiler/src/runtime_c.trk`, which holds the source as Track string
literals plus each function's prototype and arity. CI runs
`scripts/embed_runtime.sh --check` so the two copies cannot drift.

`trackc build --rt=track` links the same runtime written in Track
(`compiler/rt/*.trk`) instead, with no libc; see [native.md](native.md).

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
