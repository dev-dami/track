# Event lifecycles and stack-first memory (design, card L0)

Status: implemented. Cards L1 to L15 have landed, on top of K2; the
sections below are the design, with *as built* notes where the code went
further. `life.trk` infers a summary for every function and a placement
for every allocation site, dumped by `trackc life --place`: `stack`,
`ret-slot`, `heap-autofree`, `temp-autofree` (either may add `deep`, and
`strs` when the cell owns Str fields), or `heap`. Both backends follow
the same plan.

- L1, L2: wall ms and page faults in `bench/native.sh`; the warnings
  channel and `--deny-warnings`.
- L3 to L6: origin sets, interprocedural summaries, errors E0211 to
  E0214 and warnings W0301 to W0305 (§2).
- L7: the shared placement plan (`PLACE`/`ENDS`).
- L8, L11: struct, tuple, array and payload-union literals planned
  `stack` live in `tk_w sN[k]` slots (cgen) or contiguous frame slots
  (ngen, `ng_site_cell`).
- L9, L12a: `heap-autofree` cells are freed at block end, at a `return`
  once its value is read, and the old cell at each fresh reassignment;
  `deep` sites run drop glue into fresh child cells. ngen inlines the
  free and pops the free list in its inlined alloc.
- L10, L12b: functions whose result is exactly `fresh`, of a flat struct
  or tuple type, get a `<f>__rs` entry with a hidden result-cell
  parameter; callers pass a stack slot or the local's own cell.
- L13, L13b: fresh Str and runtime handles (phase 2, §3.5) are freed at
  end of life, and as temporaries right after a call that only lends
  them (`temp-autofree`).
- L13c, L13d: fresh cell temporaries are stack-placed or freed after
  their consumer, including cells whose Str fields are static or
  borrowed, discarded cell statements, and native temporaries of
  inlined consumers.
- L13e, L15: Str fields a cell provably owns are freed with it (§3.6),
  including fields filled by call results and moved fresh locals.
- L14: cells whose every word is written use `track_alloc_nz` and skip
  zeroing.
- F1: a `return` hidden inside an `if` or `match` expression (in a `let`
  value, condition, call argument, operand or field) is seen by every
  walker that decides freshness (`rets_walk`, cached per function as
  `LT_RETS`), so a hidden `return "literal"` no longer lets the plan free
  a static Str.

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

Every example below is in `docs/lifecycles/`. Before L5 and L6 each
**passed `trackc check` and ran** [observed: stage3 trackc at 62d8d50].
Each is a logic error that only worked because cells were never freed.
trackc now reports each one with the diagnostic shown [observed].

### Hard errors (definite bugs)

| code | example | trackc before L5 | diagnostic |
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

*As built (L5w):* the four rules are widened; every case below has an
error test and a look-alike that must check clean in
`tests/selfhost_tests.rs` (`LIFE_ERRORS`, `LIFE_CLEAN`).

- **E0211** also fires for an owned place read through a `&` variable
  (`take(r.p)`, `take(*r)`, `r.0`, `r[i]`) passed to a by-value parameter
  that dies or escapes; the message names the place (`'r.p' is only
  lent`). A generic parameter `v: T` binds to the argument's own type: a
  bare `&` variable there is lent (`eat(r)` is fine), but an owned place
  through a reference is consumed (`eat(r.p)` is E0211). Copy places
  (`r.n`) and `&r.p` never fire.
- **E0212** also covers union payload literals
  (`return U::P(&p)`: "inside the returned payload of 'U::P'"), `&`
  aliases of a local inside a returned literal (`let b = &p; return (b, 7)`:
  "..., through 'b'"), and variables bound to a literal that holds `&local`,
  returned bare or inside a literal (`let t = (&p, 7); let u = t;
  return u;`: "through 'u'"). Aliases of a parameter, and copies read
  out of a holder (`return (u.1, 8)`), never fire.
- **E0213** records targets for `r = q` (q a `&` variable: r gets q's
  targets) and `r = f(&b)` when `f`'s summary says its result derives
  from exactly one `&` parameter and nothing else (no FRESH, STATIC or
  UNKNOWN bit); that argument's targets become r's.
- **E0214** also checks tuple literals whose *expected* type makes two
  elements owned: a declared return type (`-> (Point, Point)` with
  `return (r, r)`), a `let` annotation, a parameter type, or a struct
  field type. A tuple's inferred type keeps `&` elements as references,
  so `(r, r)` as `(&Point, &Point)` is fine. The message reads
  "elements 0 and 1 of the tuple both take ownership of the borrowed 'r'".

Not covered: stores into a holder's field (`t.0 = &p`), `if`/`match`
values as reference targets (a may-point, not a must-point), and
owned payloads filled from a borrow (`U::P(r)`: the variant constructor
has no summary, so its arguments are not checked yet).

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

*As built (L2), following this plan.* Before it trackc had only
errors (`fail_at` exits), and `format_warning` in `diagnostic.trk` had
no caller.

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
L13e frees the ones a cell provably owns (§3.6).

**Allocators.** Both runtimes already have `dealloc`: the C bump/free
list in `runtime/track_runtime.c:195` and the size classes in
`compiler/rt/mem.trk`. When this was written nothing called them from
generated code; L9 and L12a free through them. `alloc` zeroes every
cell, but a cell from a literal has all its words written, so L14 added
`track_alloc_nz`, which skips the zeroing. Measurement v2 below shows the
cost it removed.

### 3.4 Native backend specifics

ngen keeps every value in `rax` and temporaries in frame slots or
registers, so stack cells are just extra slots (`ng_site_cell`). As
built (L12a), a free is inlined (`ng_dealloc_rax`: bytes in use down,
block onto its class's free list; large blocks call `dealloc`). See
docs/bootstrap.md, "Native backend", for the native details.

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

*As built (L13):* `life.trk` names the runtime functions whose result is
a new object the caller owns, by kind, each checked against both
runtimes: Str (`str_from_literal`, `str_concat`, `int_to_str`,
`str_substr`, `str_trim`, `path_basename`, `path_ext`, `path_join`,
`track_sb_str`, `read_line`, `file_read_all`, `track_file_read`,
`env_get`, `os_arg`), Vec (`vec_init`), i64 collection
(`track_i64_collection_new`, `track_str_bytes`) and string builder
(`track_sb_new`). The frees are `track_str_free`, `vec_free`,
`track_i64_collection_free` and `track_sb_free` (all but `vec_free` new,
identical in both runtimes; `str_free` stays a no-op because user code
may pass it a literal). A program function whose every `return` is a
fresh call of one kind (`text_cat`, `vnew`) counts as that kind (least
fixpoint). The proof is syntactic, on top of the summaries:

- `let x = f(..)`, not the last statement of its block, is heap-autofree
  when every later mention of `x` (mode 5 of `pl_use_ok`) is an argument
  `x` or `&x` to a call that neither keeps nor frees it: a runtime
  parameter whose end is `die` only (an element a call returns is a word
  the free never touches, so a derived result is fine), or a parameter of
  a program function that only lends it. Arithmetic, aliasing, return,
  `return f(.., x)`, fields, indexing, lenses and macros all keep it.
- every `x = v` must make a fresh object of the same kind; the old one is
  freed after `v` is built.
- a parameter is lent (`LT_HLENT`) when every mention is such an argument
  and it is never assigned (mode 6, greatest fixpoint), so `vget`,
  `vlen` and `out` lend their handle and callers may still free it.

The frees run where L9/L12 free cells (block end, `return`,
reassignment), in both backends. Expression temporaries
(`str_len(int_to_str(i))`) are freed by L13b (Str and handles) and L13c
(cells): `pl_call_args` plans an argument a temporary when
`pl_temp_arg_ok` holds (the parameter is lent or `die` only, and the
call's result neither derives from it nor has an unknown origin). A cell
temporary (`pl_temp_cell`) is a stack cell when it is a flat literal or
ret-slot call within the frame budget and not inside a `return` value,
else freed after the consumer (`G_TEMPS` / `NG_TEMPS`, code `-1 - site`)
with drop glue when `pl_deep_fresh` holds and the callee only reads it.
A Str field is never freed by drop glue (a literal is static, and
`track_str_free` frees unconditionally). L13d: a non-generic function
whose every `return` is `pl_deep_fresh` and whose summary result is
fresh with no unknown part and no cell parameter (a Str field may be
static or a parameter's) is a fresh cell site too (`LT_GFN`,
`pl_glue_fns`), so `weight(&make_named(i))` frees the shell; a fresh
cell call used as a discarded statement is its own consumer and freed
deep right after it returns. ngen frees only the temporaries pushed
during the current `ng_call` (a function inlined into itself holds a
copy of the call node), and a branch condition does not inline a call
with temporaries. ASan-built self-compile
of `compiler/src/main.trk`: 771,893 leaked allocations (64.7 MB) before,
584,001 (56.9 MB) after, 0 errors [observed].

### 3.6 Owned Str fields (card L13e)

Drop glue frees Str fields per site, never per type. A deep site's
`LT_SMASK` bit f says its struct cell owns Str field f, and both
backends free that word (`track_str_free`, `gen_drop_strs` /
`ng_drop_strs`) before the glue frees the rest. A bit is set only when:

- the field's value at every construction reaching the free is a fresh
  Str call (`hd_call_kind` HK_STR: `int_to_str`, `str_concat`, the
  builder's result, a program function whose every `return` is one).
  For a local that is the `let` literal and every `x = v`, which must be
  a struct literal with that field fresh too; for a temporary, its
  literal. A literal, a parameter, a variable or a field read never sets
  it, so a field that may be fresh or static is never freed;
- no later use can move the string out or swap it: a read `x.f` is only
  a lent argument (`hd_arg_ok`: the callee neither keeps, frees nor
  returns it) that is not an argument of a returned call (which ngen may
  turn into a tail call after the frees), or a binary operand. `x` or
  `&x` only goes to a struct parameter that lends its Str fields
  (`LT_SFLENT`, the least fixpoint of the same rule over the callee's
  body). `let s = x.f`, `return x.f`, `x.f = v`, destructuring or
  matching `x` clear the bit.

Since Str is linear, a fresh Str in a field has no other owner, so these
rules make the cell its only holder when it dies. A struct literal
owning a fresh Str field is heap-autofree (or temp-autofree) with drop
glue even where it would fit a stack cell, so the free has a point to
run; a fresh Str call field no longer stops a literal from being
`pl_deep_fresh`. Only the cell's own fields count: Str fields of child
cells still leak. The plan prints `deep strs` for such sites. ASan-built
self-compile of `compiler/src/main.trk` (same input): 298,630 leaked
allocations before, 298,635 after [observed]; the compiler builds no
struct with Str fields, so its leaks are untouched (largest buckets:
bump cells, i64 collections, `str_concat`, string builders).

#### Call results and moves (card L15)

Two more values count as a fresh Str in a field:

- a call result. `LT_SFRES` gives, per function, the Str field bits its
  result cell owns: the greatest fixpoint over functions whose result
  is a struct with a fresh shell and child cells (summary result exactly
  `fresh`, or `LT_GFN`) and whose body ends in `return`. Each starts
  with every Str field bit; a pass keeps only the bits every `return`
  fills fresh (a literal by the rules here, or a call by the callee's
  current `LT_SFRES`). Marks only clear, so it stops. A local or
  temporary built by such a call, or reassigned one, owns those fields;
- a moved fresh local. `LT_SFMOVE` marks `let s = <fresh call>` whose
  last mention in the rest of its block is a move (a struct literal
  field value, or `y = s`), every other mention lent (`hd_arg_ok`) and
  none under a loop, `for`, match or destructuring `let`, nor a
  rebinding `let s`. So `s` has no other owner and the move runs at most
  once per binding. Such a field is fresh for `sf_init_mask`; `y = s`,
  for a heap-autofree handle or Str `y` of the same kind, is a fresh
  reassignment in mode 5 of `pl_use_ok`: `y` frees its old value there
  and its last at its end, and `s` is never freed itself (the move is
  not a lent use).

A local owning Str fields is never rewritten in place by `x = f(..)` of
a ret-slot call (the flag it binds is 0), so the old strings are freed
with the old cell. Child cells' Str fields still leak: the mask names
the cell's own words, and a nested mask would need per-child drop glue.

ASan-built self-compile (each compiler built from its own sources with
its own lifecycle rules, LSan, `malloc_context_size=6`): 255,856 leaked allocations at
e286ddf, 234,096 after [observed]. The 25,260 row collections of
`levenshtein` (`prev = cur` per row) are freed; it now reads its result
into a local before returning, since `return vget(prev, m)` keeps `prev`
(mode 5 refuses `return f(.., x)`). The rest grew with the source. The
largest remaining buckets are not freeable here: `tk_punct` bump cells
are tokens the token list keeps, and `ng_prim`'s jump lists go to the
cold-path list (`ng_cold_back`).

---

## 4. Measured motivation

This section is the baseline the design started from, at 62d8d50,
before L1 and with the naive native backend. Machine: the shared host,
loaded. Runs are medians [observed then].

`bench/native.sh` reports user-mode megacycles.

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

**After L1 to L15 and the native race** (`RACE_RUNS=3 bench/native.sh`
at 8b96bb2, same loaded host) [observed]:

```
prog         native      C    rust   nat/C   faults nat/C/rust
array_mix      315M    355M   1499M   0.89        5/62/77
fib            318M    364M   1021M   0.87        5/61/61
loop_arith    2638M   2844M   3237M   0.93        6/62/62
sieve          699M    869M   1242M   0.80     7/15685/39127
strings        166M    214M   1098M   0.78     7/2118/21966
structs        174M    222M    443M   0.78      6/62/109496
tokenize       573M    727M   2114M   0.79        8/65/9429
```

The Rust column is the stage 0 compiler, which frees nothing.

---

## 5. Work chart

Every card in this chart has landed.

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

Both were settled on the recommended default: L13 frees strings and
handles on the origin proof alone, and warnings print by default.

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
