# Event lifecycles and stack-first memory (design, card L0)

Status: design, partly implemented. Base: `agent/track-race` at
62d8d50, with card K2 (branch `k2`, not yet landed) taken as given.
L7 landed: `life.trk` builds the placement plan (`PLACE`/`ENDS`, dumped by
`trackc life --place`) and cgen takes its stack tuples from it.
L8 landed: cgen puts struct, array and payload-union literals planned
`stack` in `tk_w sN[k]` slots (C structs: 78k faults, was 109k).
L9 landed: cgen frees `heap-autofree` struct, tuple, array and union
cells when their block ends or a `return` leaves it, and the old cell at
each fresh reassignment; sites dumped `deep` also run drop glue into
fresh child cells. Locals reassigned fresh cells are autofree, not stack
(C structs: 89 faults, wall ~180 ms median on the loaded host). Str and
handles wait for L13.
L11 landed: ngen puts the same `stack` literals in contiguous frame slots
(`ng_site_cell`; native structs 0.88 of C in `bench/native.sh`).
L10 landed: functions whose result is exactly `fresh`, of a flat struct
or tuple type, and whose every `return` is a literal or another such
call get a `tk_<f>__rs` entry with a hidden last cell parameter; callers
pass a stack slot or the local's own cell (no temp slot: the callee
stores only after every word is in a temp, so `acc = f(&acc)` is safe),
and others call the allocating `tk_<f>` (C structs: 74 faults, ~70 ms).
Native ret-slot calls wait for L12.

The ask: every function gets a lifecycle the compiler infers, with no
annotations. The compiler uses it to reject a new class of logic errors
and to place values on the stack, which is faster than the heap. No GC.

Evidence labels: [observed] means measured or run here; [estimated] means
derived from measurements; [assumed] means not checked.

---

## 1. The model

### 1.1 Lifecycle of a value

Every non-copy value (struct, tuple, array, union cell, `Str`, and, in
phase 2, runtime handles; see 3.5) goes through:

```
born ──> used* ──> ended
```

- **born**: `fresh` (a literal, a slice, or a fresh result of a call),
  `derived(k)` (it is, or lives inside, parameter `k`), or `static`
  (a string literal or a global).
- **used**: `read` (fields loaded, passed as `&`) or `mutate` (field
  store, lens body writes, a runtime mutator such as `vec_push`).
  Moving the value whole (binding, returning, storing it, passing it by
  value) is not a use. A borrowed value passed by value to a callee
  counts as `read`, since the callee may read through it.
- **ended**, exactly once on every path, by one of:
  - `die`: the last owner's scope exits. The frame ends the value.
  - `ret`: it flows into the function's result.
  - `esc`: it is stored somewhere that outlives the frame: a field,
    element or payload of a value that itself escapes or is returned, or
    an argument to an unknown function or raw builtin (`store64`, or
    arithmetic on a handle).
  - `move`: it passes by value to a callee. What happens next comes from
    the callee's summary (1.3).

Linear ownership (SPEC §2) already guarantees one owner per value and no
use after a move. Lifecycles add the other half: where life ends, and so
where the storage can live and when it can be freed.

### 1.2 Summary lattice

Per function, inferred, never written:

| part | domain | order (bottom first) |
|---|---|---|
| param `k` use | `none < read < mutate` | chain |
| param `k` end | subset of `{die, ret, esc}`; `none` for `&T` params that are only lent | set union, `esc` absorbs |
| result origin | subset of `{fresh, derived(k)..., static}`, plus `unknown` | set union, `unknown` is top |

The abstract value of an expression is an **origin set** in one 64-bit
word: bits 0..59 are parameters, then FRESH, STATIC and UNKNOWN. A
function with more than 60 parameters maps the rest to UNKNOWN. Joins are
`|`. The lattice height per function is at most
`2 + 3 * params + 3`, so a fixpoint ends quickly.

The domain is deliberately coarse: there are no per-field origins, and a
container that holds `x` gets an edge `container -> x`. If the container
escapes or is returned, `x` does too, applied transitively once at the
end of the function.

### 1.3 How the caller uses a summary

At a call `f(a0, a1, ...)`:

- `&x` argument, use `mutate`: `x` counts as mutated here (this feeds
  W-LC03). Use `read`: nothing changes.
- by-value argument, end ⊆ `{die}`: under the **caller-ends convention**,
  the callee never frees a parameter it only lets die. The caller ends
  `x` right after the call returns. The ABI then looks like a borrow, so
  the caller may keep `x` in its own stack slot. This also fixes the K2
  leak `fn f(a: Str, b: Str) -> Str { return b; }`: `a` is `die`, so the
  caller frees it. `b` is `ret`.
- by-value argument, end includes `ret`: the result's origin includes
  `x`'s origin. If `x` sat in a caller slot, the result can share that
  slot.
- by-value argument, end includes `esc`: `x` must be on the heap.
  Ownership leaves the frame.
- result origin `{fresh}` only, with a known size: the call gets a
  caller return slot (3.2). `static`: never freed. `unknown`: heap, owner
  frees.

### 1.4 Inference

1. **Call graph** (pass 1 of `check_program` already visits every
   `K_FN`): per function, the callee name ids, recorded while checking.
2. **SCCs** with Tarjan over name ids, in reverse topological order.
   Most of trackc's 452 functions are singleton SCCs [observed:
   `grep -c '^fn '`].
3. **Intraprocedural pass** per function: one forward walk over the
   checked AST. It keeps an origin set per variable slot, in a new slot
   field next to `V_BORROWS`, and records:
   - `read`/`mutate` facts per parameter bit, from field loads and
     stores, lens bodies, and callee use summaries;
   - sinks: return gives `ret`; a store into a container adds a container
     edge; an unknown call or raw builtin gives `esc`; a by-value
     argument applies the callee's end set;
   - if/match: origin sets join at the merge. K2 already drops diverging
     branches from the merge, so `return` inside an arm contributes `ret`
     and nothing else. Loops re-walk the body, each pass starting from
     the join of the pre-loop state and every earlier pass, until a pass
     leaves every slot's origin set (and the live set) unchanged. A fixed
     pass count is not enough: a value moves one slot per iteration, so
     an n-way rotation (`t = a; a = b; ...; d = t`) needs n - 1 passes
     before the origin reaches its last slot, plus one to see nothing
     change. Joins only add bits, so the passes are bounded by slots x
     bits; use and end facts come from the origins, so once the state
     repeats they are complete too. Nested loops run the inner fixpoint
     inside every outer pass.
4. **Fixpoint within an SCC**: start every member at bottom and re-run
   the members until no summary changes. The height bound above limits
   the iterations. In practice there are 2 passes, 3 for mutual
   recursion.
5. **Runtime functions** get summaries from a table in the new module,
   such as `str_concat: a read, b read, result fresh` and
   `vec_push: h mutate, v esc`. A name not in the table and not a Track
   function gets UNKNOWN for every argument, so it is conservative.
6. **Generics**: K2 checks each instantiation, but trackc emits one
   function per template (SPEC §9), so a template's summary is the join
   over its instantiations. A `T` bound to a Copy class contributes
   nothing. An opaque linear `T` is a value of unknown size, so it can be
   heap-placed and auto-freed but never stack-placed.

   *As built (L4):* SCC members run in load order, Gauss-Seidel style,
   and each record is joined with its previous value, so passes only add
   facts. A pass ends the iteration when no summary a call read during it
   changed afterwards (flags never count), so components whose summaries
   stay at bottom after the first read (most of trackc's word-typed code)
   take 1 pass; the self- and mutual-recursion goldens take 2.
   `trackc life` prints `scc size=N passes=P` for recursive functions.
   A callee end of `{}` (it diverges, or its summary is still bottom)
   ends nothing in the caller; `die` keeps an owned argument live in the
   caller and gives a borrowed one `die` (E0211's input); `ret` adds the
   argument's origin to the result. Generics: one summary per template,
   analysed with its type parameters as opaque linear values, which
   bounds the join over instantiations from above; a Copy instantiation
   passes the empty origin at the call site, so it loses nothing.

**Cost.** The walk does less per node than `check_expr`: no type
inference and no borrow-state updates. trackc checks itself in about
40 ms on a quiet machine [reported] and 70 ms (±22%) on today's loaded
machine [observed: `perf stat -r 5 trackc check compiler/src/main.trk`].
The extra walk with about 2 passes per SCC should cost at most about
10 ms [estimated]. Budget: **at most 15 ms extra, and `check` stays
under 75 ms**, which is half of the Rust checker's ~150 ms. Every
inference card measures this.

### 1.5 Where it lives

A new module `compiler/src/life.trk` runs after `check_program`. It needs
the checker's slot types and the K2 per-instantiation facts. It produces
two side tables indexed by AST node:

- `PLACE[node]` for every allocation site: `HEAP`, `STACK(words)` or
  `RETSLOT`;
- `ENDS[node]`: the values to end at that point (scope exit, `return`,
  `break`/`continue`, reassignment, call-after for `die` params).

**Both backends read these tables. Neither re-derives them.** K2's
`tuple_stays_local`/`tuple_use_ok` in `cgen.trk` is the special case
"origin fresh, sinks ⊆ {index, destructure, match, reassign}". Card L7
moves it into `life.trk`, so ngen gets it too.

### 1.6 How it composes

| feature | interaction |
|---|---|
| linear ownership | unchanged and still decides legality. `Spent` means moved. Lifecycles only choose the end point and the storage. |
| borrows | `provenance()` already tracks which names a reference points to. Lifecycles add a scope depth per slot, so a reference that outlives its target's scope is E-LC03. |
| lenses | the lens body's writes give the target `mutate`. Lens non-escape is already enforced, so a lens never adds `esc`. |
| generics | join over instantiations (1.4 step 6). |
| if / match | origins join; ends must agree on every path. The SPEC §5 merge rule already forces equal states. Frees are emitted per path. |
| loops | a value born in the body dies at the back edge and at `break`/`continue`. Its stack slot is reused every iteration. `acc = f(&acc)` ends the old `acc` after the right-hand side is evaluated. |
| early return | K2 accepts moves on early-return paths. Every `return` ends all live owned locals in the enclosing scopes, minus the returned value's origins. |
| `abort` | no frees (SPEC §8.5). |

---

## 2. Error and warning catalogue

Every example below is in `docs/lifecycles/` and **passes
`trackc check` today and runs** [observed: stage3 trackc at 62d8d50 plus
this branch]. Each is a logic error that only works because cells are
never freed.

### Hard errors (definite bugs)

| code | example | trackc today | diagnostic |
|---|---|---|---|
| E0211 | `e1_lent_consumed.trk`: `fn peek(r: &Point) -> i64 { return take(r); }` where `take(p: Point)` | ✓ passes, prints `1 2` | `error[E0211]: 'take' consumes its parameter 'p', but 'r' is only lent to 'peek'. Pass '&' and make 'p: &Point', or take ownership in 'peek'.` |
| E0212 | `e2_escape_tuple.trk`: `return (&p, 7);` with `p` a local | ✓ passes, prints `8` | `error[E0212]: local 'p' escapes 'pair' inside the returned tuple (element 0); it dies when 'pair' returns.` |
| E0212 | `e3_escape_struct.trk`: `return Holder { p: &p };` | ✓ passes, prints `1` | `error[E0212]: local 'p' escapes 'make' through field 'Holder.p'; it dies when 'make' returns.` |
| E0213 | `e4_outlive_block.trk`: `r = &b;` inside an `if` arm, then `print(r.x)` after it | ✓ passes, prints `3` | `error[E0213]: 'r' refers to 'b', which ends at the close of this block (line 9); 'r' is used after it at line 14.` |
| E0214 | `e5_end_twice.trk`: `Bag { a: r, b: r }` with `r: &Point` | ✓ passes, prints `3` | `error[E0214]: 'p' would be ended twice: fields 'Bag.a' and 'Bag.b' both take ownership of the borrowed 'r'.` |

E0211 and E0214 share a root cause. A borrowed origin (a `&T` parameter,
or a local reference) reaches an owned sink: a by-value parameter whose
end includes `die`/`esc`, or an owned field, element or payload. Today
the word ABI hides this because `&x` is the value itself. Once frees
exist, it becomes a double free or a use-after-free.

E0212 extends the current `check_escape`, which only follows `K_ADDR`,
`K_VAR`, `K_CALL`, `K_IF` and `K_LENS`, to struct, tuple, array and
payload literals through container edges.

*As built (L5):* all five examples are rejected with the texts above
(`life_fail_pending` in `life.trk`; `trackc life` still dumps them).
E0211 fires when a `&`-typed variable is passed to a by-value,
non-generic parameter whose end includes `die` or `esc`. E0212 checks
`&local` elements of returned tuple, array and struct literals (payload
literals are not covered yet). E0213 records `r = &x` targets and flags
a reference whose target's block closed when it is next read or stored
through. E0214 is found at the literal; when the borrowed value is a
`&` parameter, it is reported at each call site and names the caller's
value (`'p'` in e5), or at the literal if nothing calls the function.

### Warnings (suspicious, never fatal)

| code | example | diagnostic |
|---|---|---|
| W0301 | `w1_fresh_unread.trk`: `let origin = Point {..};` never used | `warning[W0301]: 'origin' is created and never read; remove it or name it '_origin'.` |
| W0302 | `w2_write_only.trk`: `p.y = 99;` with no later read of `p` | `warning[W0302]: write to 'p.y' is never read; 'p' ends at line 7.` |
| W0303 | `w3_mutated_unread.trk`: `bump(&c)` mutates through a lens, and `main` never reads `c` again | `warning[W0303]: 'bump' mutates 'c', but 'c' is never read afterwards.` |
| W0304 | `w4_discarded_result.trk`: `vadd(&a, &b);` as a statement | `warning[W0304]: result of 'vadd' is discarded, and 'vadd' has no other effect (it only reads its parameters and returns a fresh value).` |
| W0305 | `w5_consume_only_reads.trk`: `fn norm1(p: Point)` only reads `p` | `warning[W0305]: 'norm1' takes 'p' by value but only reads it; take '&Point' so callers keep it.` |

[observed] All five pass `trackc check` and run. "No other effect"
(W0304) means: every parameter's use is ≤ `read`, every end is `none`,
`die` or `ret`, the result is `fresh`, and the function calls nothing
with an effect (print, file or sys calls, any `esc`). That is one more
bit per summary, `pure`, joined like the others.

`_`-prefixed names silence W0301 and W0302. A bare `_` is a parse error
today [observed: `let _ = mk();` gives E0100]. Write
`let _unused = f();` to keep a result on purpose.

### The warnings channel

trackc has only errors today: `fail_at` exits. `diagnostic.trk` already
has `format_warning` [observed], but nothing calls it.

- `base.trk`: add `warn_at(c, pos, code, message)`, which appends
  `(pos, code, message)` to a new `CX_WARN` vector.
- After checking succeeds, `main.trk` sorts the warnings by position,
  which keeps the output deterministic, and prints them with
  `format_warning` to stderr. Then it prints
  `✓ Check passed: f.trk (2 warnings)`.
- Exit status is unchanged. `--deny-warnings` makes any warning exit 1,
  for CI.
- `build` and `run` print warnings the same way. The emitted C is
  unaffected, so the bootstrap's byte-identical gates are untouched.
- The compiler's own sources stay warning-clean. The L6 accept
  requires `trackc check --deny-warnings compiler/src/main.trk`.
- Rust checker parity (`tests/selfhost_tests.rs`, shared verdicts): the
  new errors are trackc-only, like K2's early-return moves. Mark the
  new test programs trackc-only. `compiler/` must still pass the Rust
  stage 0, and it does, because it holds handles as `i64`, which these
  rules never touch.

---

## 3. Placement

### 3.1 Rules

The word ABI stays: a stack cell is still a pointer word, with the same
layout `[tag|len, words...]`. Only where the words live changes.

| lifecycle of the allocation site | placement |
|---|---|
| life ends in this frame (`die` only, including callee params under caller-ends) | **stack slot** |
| leaves only by return (`ret`), and the function's result origin is `{fresh}` | **caller return slot** |
| anything with `esc`, unknown size, or over the stack cap | **heap, auto-free** at end of life |
| origin may be `static` | never freed |

Stack cap: at most 32 words per cell and 4 KiB of cells per frame.
Anything bigger goes to the heap and is still auto-freed. A cell born in
a loop reuses one slot, because it is dead by the back edge.

### 3.2 Per value kind and backend

| kind | C backend (`cgen.trk`) | native (`ngen.trk`) |
|---|---|---|
| tuple | already a `tk_w sN[k]` slot when K2's rule holds. L7 makes it come from `PLACE`. | contiguous run of `k` frame slots, cell address `rbp - 8*(base+k)`. The frame size is patched at function end (`x_patch32`), so extra slots cost nothing. |
| struct literal | `tk_w sN[1+fields]`, written in place of `alloc(..)` in `gen_struct_init` | as tuple, in `ng_struct_init` instead of `ng_cell` |
| array literal | `tk_w sN[1+len]` (length is static for literals); slices `a[s..e]` stay heap (fresh, auto-free) | same |
| union payload cell | `tk_w sN[1+max payload]` | same |
| `Str` | never stack (dynamic size, made by runtime). Heap; freed only when the origin is definitely `fresh`. Literals are `static`. | same |
| Vec / collection / string builder | never stack (they grow). Runtime free (`vec_free`, new `track_i64_collection_free`, `track_sb_free`) at end of life. Phase 2 (3.5). | same |

**Caller return slot.** A function whose result origin is exactly
`{fresh}` with a statically known cell size gets one hidden trailing
parameter `rs`, a pointer word. The callee writes its result cell into
`rs` instead of calling `alloc`. The caller passes a slot. If the
destination is the variable being reassigned and the right-hand side
reads it (`acc = vadd(&acc, &d)`), the caller passes a temporary slot
and copies `k` words afterwards. A later refinement alternates between
two slots instead of copying. Callers are all compiler-generated, so no
external ABI changes. Runtime functions never get `rs`.

### 3.3 Inserting frees (heap cells)

`ENDS` lists, per AST point, which owned locals end there:

- **scope exit**: every live owned local declared in the block, in
  reverse declaration order;
- **move** into a by-value parameter: `die` means the caller ends it
  right after the call. `ret`/`esc` means ownership left, so there is no
  free.
- **branches**: each arm ends its own locals. The merge rule guarantees
  the outer state is equal after every arm, so there are no "maybe
  freed" values.
- **early return**: all live owned locals in all enclosing scopes,
  except the returned value's origin;
- **loops**: body locals at the back edge, and at `break`/`continue`;
- **reassignment** `x = e`: evaluate `e`, end the old `x`, store;
- **match on a cell that moves its payload out**: free the cell shell;
  the bindings own the fields.

**Drop glue.** Freeing a struct, tuple or union cell first ends its owned
fields: linear-typed, non-reference fields that are cells. Each cell
type gets a generated `drop_<tag>` function. Reference fields are
skipped. Phase 1 skips `Str` fields, because a field's string may be
`static` and the representation does not say which (open question 1).

**Allocators.** Both runtimes already have `dealloc`: the C bump/free
list in `runtime/track_runtime.c:195` and the size classes in
`compiler/rt/mem.trk:298`. Nothing calls them from generated code yet.
The C free list `memset`s every reused cell, and `alloc` zeroes every
cell. A cell from a literal has all its words written, so card L14 adds
`alloc_nz` and drops the memset. Measurement v2 below shows this cost.

### 3.4 Native backend specifics

ngen keeps every value in `rax` and every temporary in a frame slot
[observed: `ngen.trk:41-57`], so stack cells are just extra slots. Frees
are calls to `dealloc` (`ng_call_rt`). The native-speed race is editing
ngen and x64. The native placement cards come after that work lands and
touch only `ng_cell` call sites and `ng_function`.

### 3.5 Phase 2: runtime handles typed `i64`

The heap programs and the compiler itself hold collections, builders and
strings as plain `i64` handles, for example
`let v: i64 = track_i64_collection_new(n)`. The checker treats `i64` as
copy, so ownership never tracks them. Origin sets still can: the runtime
table marks allocator results `fresh`. A handle local that only reaches
table entries with `read`/`mutate` and never reaches arithmetic,
`store64`, an unknown call, a container or the result ends at scope
exit. Every alias shares the origin bit, so if any alias escapes, none
is freed. This is where `sieve` and `strings` gain (section 4). It also
touches every table in the compiler, so it gets its own card and the
strictest gate: ASan/LSan self-compile.

---

## 4. Measured motivation

Machine: the shared host, loaded. Runs are medians [observed].

`bench/native.sh` reports user-mode megacycles. Native is the naive
backend that the race is improving.

```
prog         native      C    rust   nat/C
array_mix     7437M    334M  1342M   22.27
fib           1348M    375M  1007M    3.59
loop_arith    3009M   2840M  3257M    1.06
sieve         3238M    819M  1287M    3.95
strings       2983M    249M  1066M   11.98
structs       2412M    397M   453M    6.08
tokenize     16410M    809M  2172M   20.28
native self-compile (main.trk): 119 ms
```

`bench/race.sh` (wall ms, trackc C build vs Rust): check 75 vs 542;
structs 576 vs 585; sieve 516 vs 511; strings 231 vs 571; array_mix 144
vs 420.

**The scoreboard hides the heap cost.** It counts `cycles:u`, but never
freeing memory costs page faults in the kernel
(`perf stat -e task-clock,page-faults,cycles:u,cycles:k`):

| build | wall ms | page faults | user Mcyc | kernel Mcyc |
|---|---|---|---|---|
| C structs | 499 | 109,496 | 406 | 1,191 |
| C sieve | 404 | 39,126 | 808 | 365 |
| C strings | 164 | 21,965 | 244 | 236 |
| C tokenize | 325 | 9,428 | 777 | 130 |
| C array_mix | 110 | 77 | 344 | 8 |
| native structs | 1282 | 125,007 | 2,410 | 1,477 |
| native strings | 1072 | 41,077 | 2,992 | 375 |
| native sieve | 1206 | 39,074 | 3,241 | 580 |

`structs` allocates three cells per iteration for 4M iterations and
frees none, which is about 400 MB of fresh pages. `perf report` of
native structs shows the top kernel symbols are `clear_page_rep`,
`sync_regs` and `__alloc_pages`.

**Prototype: hand-placed `structs`, C backend.** Kept out of the tree;
reproduce with `build/l0proto/mk.py` and `run.sh`, which edit trackc's
emitted C. All 4 variants print identical output.

| variant | what changed | wall ms | faults | user Mcyc |
|---|---|---|---|---|
| v0 | today | 590 | 109,495 | 406 |
| v1 | `let d = Vec2{..}` in a stack slot (L8) | 477 | 78,240 | 311 |
| v2 | v1 + `dealloc` of dead `acc`/`body` (L9) | **152** | 78 | 452 |
| v3 | v1 + caller return slots, no heap at all (L10) | **135** | 61 | 341 |

`sieve` with its collection freed at the end of `sieve()` (phase 2,
L13), hand-edited: wall 402 → 327 ms (−19%), faults 39k → 16k.

**Estimated wins** [estimated from the above]:

- `structs`, C: 4.4× faster wall. It goes from tying Rust (585 ms) to
  beating it by about 4×. Native would lose most of its 1.48 G kernel
  cycles, about −40% wall.
- `sieve` and `strings`: −20 to −40% wall, from phase 2 frees.
  `strings` spends half its cycles in the kernel.
- `array_mix` and `fib`: no heap traffic, so no gain. Their gap is
  codegen (bounds-check calls, the naive native backend). Lifecycles
  give the facts for bounds-check elision on fixed-length stack arrays,
  but that is not in this chart.
- v2's user cycles rise (452 vs 406) because of the free-list `memset`
  and the `dealloc` call. L14 removes that.

L1 adds wall-ms and page-fault columns to `bench/native.sh`, so the
scoreboard measures what this work changes.

---

## 5. Work chart

Every accept includes the **self-hosting gate**:
`scripts/bootstrap.sh` (stage1/stage2 C identical, stage2/stage3
binaries identical), `scripts/selfhost.sh` (n2 = n3 = n4), and
`cargo test --test selfhost_tests`. The compiler is the biggest test.
"check budget" means `trackc check compiler/src/main.trk` stays at most
15 ms above the base and under 75 ms (median of 5).

| id | title | needs | touches | accept (tests + bench) |
|---|---|---|---|---|
| L1 | Honest scoreboard: wall ms + page faults | — | `bench/native.sh`, `bench/race.sh` | new columns print; structs shows over 100k faults; gate |
| L2 | Warnings channel | — | `base.trk`, `main.trk`, `diagnostic.trk` | a test program with a forced warning prints `warning[...]`, exits 0; `--deny-warnings` exits 1; output sorted by position; gate |
| L3 | `life.trk`: origin sets + intraprocedural summaries, `trackc life f.trk` dump | K2 landed | new `life.trk`, `checker.trk` (slot field), `main.trk` | golden dumps for `docs/lifecycles/*.trk` and bench/progs; check budget; gate |
| L4 | Interprocedural fixpoint + runtime summary table | L3 | `life.trk` | mutual-recursion and self-recursion tests reach fixpoint in ≤ 3 passes (dumped); summaries of `compiler/src` stable across 2 runs; check budget; gate |
| L5 | Hard errors E0211–E0214 | L4 | `life.trk`, `checker.trk` (scope depth), `tests/` | e1–e5 rejected with the texts in §2; all `examples/` still pass; marked trackc-only in parity tests; gate |
| L6 | Warnings W0301–W0305 | L2, L4 | `life.trk`, `compiler/src/*` (cleanups) | w1–w5 warn; `trackc check --deny-warnings compiler/src/main.trk` passes; check budget; gate |
| L7 | Shared placement plan (`PLACE`/`ENDS`); fold K2 `tuple_stays_local` into it | L4 | `life.trk`, `cgen.trk` | emitted C for the compiler byte-identical to before (no behaviour change); `trackc life --place` golden; gate |
| L8 | C: stack cells for struct/array/union literals | L7 | `cgen.trk` | parity tests pass; structs (C) at most 480 ms wall, under 80k faults; gate |
| L9 | C: auto-free heap cells + drop glue | L8 | `cgen.trk`, `runtime/track_runtime.c` (none expected) | ASan+LSan build of `examples/` and test corpus: 0 use-after-free, 0 leaked cells; ASan-built stage3 compiles itself; structs (C) at most 170 ms; gate |
| L10 | C: caller return slots | L9 | `cgen.trk` | structs (C) at most 150 ms wall, under 200 faults; tests for `acc = f(&acc)` aliasing; gate |
| L11 | Native: stack cells from the plan | L7, native race landed | `ngen.trk` | `scripts/native_parity.sh`; native structs faults under 80k; gate (selfhost.sh is the native fixed point) |
| L12 | Native: auto-free + return slots | L9, L10, L11 | `ngen.trk` | native structs wall −35% vs base, under 200 faults; native parity; gate |
| L13 | Phase 2: i64 handle and fresh `Str` auto-free | L9, L12 | `life.trk` (runtime table), `runtime/track_runtime.c`, `compiler/rt/{vec,str,mem}.trk`, embed scripts | `embed_runtime.sh --check` and `embed_rt.sh --check`; sieve (C) at most 340 ms, strings faults −50%; LSan self-compile shows fewer leaks, 0 errors; gate |
| L14 | `alloc_nz` for fully written cells; no memset on free-list reuse | L9 | both runtimes, `cgen.trk`, `ngen.trk` | v2-style user cycles ≤ base; gate |

Order: inference (L3–L6) lands before any placement (L7+), and C
placement (L8–L10) is separate from native (L11–L12). L1 and L2 can run
in parallel with L3.

---

## 6. Open questions for the user

1. **Should freeing extend to strings and runtime handles (phase 2,
   L13)?** These are where the code holds `Str`/`i64` values that might
   be literals. It needs either the origin proof alone (free only
   definitely-fresh values) or a runtime "is this a heap pointer" check.
   *Recommended default:* yes, origin proof only, with no runtime
   check. A value that may be static or may escape is never freed. That
   is safe and still wins on sieve and strings.
2. **Should warnings be on by default?** *Recommended default:* yes, for
   `check`, `build` and `run`. They never change the exit status, except
   that `--deny-warnings` (used by CI on `compiler/src`) turns them into
   errors.
