# Track Core Language Specification (v0.5)

Track is a low-level systems programming language designed for deterministic memory safety without a garbage collector or complex lifetime annotations.

---

## 1. Value Categories

Every value in Track belongs to one of four value categories:

| Category | Semantics | Copyable? | Movable? | Scope Bound |
| :--- | :--- | :--- | :--- | :--- |
| **Owned Linear** | Unique ownership of heap/stack resource | No | Yes | Dynamic / Move |
| **Lexical Lens** | Exclusive, non-escaping mutable view | No | No | `with` Block |
| **Reference (`&T`)** | Read-only borrow | Yes | Yes | Borrowed Scope |
| **Copy Primitive** | Primitive values (`i32`, `i64`, `bool`) | Yes | Yes | Value |

---

## 2. Ownership State Machine & Transitions

The ownership checker tracks variables using four explicit states:

- **`Active`**: Value is initialized and owned.
- **`Borrowed`**: A read-only reference (`&T`) exists. Track permits active shared borrows per value.
- **`Locked`**: An exclusive lexical lens (`with`) is currently active.
- **`Spent`**: Ownership has been moved or transferred.

### Transition Table

| Operation | Pre-State | Post-State | Validated Constraint |
| :--- | :--- | :--- | :--- |
| **Move Value** (`let y = x;`) | `Active` | `Spent` | `x` cannot be used after move |
| **Create Lens** (`with u -> user`) | `Active` | `Locked` | `u` frozen from moves/borrows |
| **Exit Lens Block** | `Locked` | `Active` | Restores `u` ownership |
| **Shared Borrow** (`let r = &x;`) | `Active` | `Borrowed` | `x` frozen from moves |
| **End Borrow Scope** | `Borrowed` | `Active` | Restores full ownership |

---

## 3. Move Semantics & Spend Points

An **Owned Linear** value has exactly one owner at any point during execution.

### Spend Points
A linear resource is consumed (spent) by:
1. **Move Assignment**: `let y = x;` transfers ownership from `x` to `y`. `x` transitions to state `Spent`.
2. **Function Call Transfer**: Passing an owned value into a function parameter transfers ownership to the callee.
3. **Implicit Scope Cleanup**: If a linear resource remains `Active` at scope exit, the compiler automatically emits cleanup deallocation code (`vec_free`, `str_free`).

### Struct Ownership & Field Move Policy
Structs in Track are moved as **atomic units**. Moving any field or the struct itself consumes the entire struct instance, preventing use-after-free or partial double-destruction.

---

## 4. Lexical Lenses & Non-Escaping Guarantee

A **Lexical Lens** provides temporary, exclusive mutable access to a target resource via the `with` construct:

```track
let mut u = User { age: 30 };
with u -> user {
    user.set_age(31);
}
```

### Core Research Hypothesis:
> *Can deterministic memory safety be made substantially easier to reason about by restricting mutable access to non-escaping lexical lenses rather than general lifetime-based borrows?*

### Invariants:
1. **Lexical Exclusivity**: While a lens is active, the underlying target resource (`u`) is in state `Locked`. It cannot be moved, borrowed, or accessed.
2. **Non-Escaping Guarantee**: A lens reference (`user`) is valid **only** within the lexical boundaries of the `with` block. It cannot be assigned to an outer variable, returned, or stored in a heap structure.
3. **Zero Lifetime Annotations**: Lens exclusivity is enforced purely by block scope boundaries without lifetime parameter syntax (`'a`).

---

## 5. Control-Flow Merge Rules & Loop Back-Edge Propagation

### Conditional Merge Rule (`if / else`)
At a CFG merge point, a variable must have the identical state across all incoming execution paths:

```track
let v = vec_init(16);
if cond {
    consume(v); // v state -> Spent
} else {
    // v state -> Active
}
// ERROR: Resource 'v' has inconsistent state after if/else (Then: Spent, Else: Active)
```

### Loop Back-Edge Rule (`while`)
The ownership state after a loop is the merge of:

1. the state on the path where the loop is skipped, and
2. the fixed-point state reached through the loop body and back-edge.

A variable is usable after the loop only if the merged state is consistent across all possible iteration counts, including zero iterations. If a variable can be `Active` when the loop is skipped but `Spent` after one or more iterations, the post-loop state is rejected unless the loop body restores the variable on every back-edge path.

---

## 6. Anonymous Tuples & Destructuring

Tuples are heterogeneous value containers specified with `(T1, T2, ...)`.

- **Element Access**: Zero-based numerical dot notation (`pair.0`, `pair.1`).
- **Destructuring**: Unpack tuple elements into fresh variables using `let (a, b) = expr;`.
- **Ownership**: Moving a linear component out of a tuple moves the component according to linear ownership rules.

---

## 7. Advanced Pattern Matching & Arm Guards

Pattern matching expressions (`match`) evaluate patterns sequentially against a target value.

- **Arm Guards**: Arms may contain conditional expressions (`pattern if guard => body`).
- **Nested Patterns**: Patterns recursively match tuple structures `(p1, p2)`, union variants `Variant(p1, p2)`, literals (`0`, `true`), and struct fields.

---

## 8. Explicit Error Handling Convention

Track rejects wrapper-based error handling. There are no `Option`/`Result`
types, no `?` operator, no exceptions, and no stack unwinding. Errors are
ordinary stack values passed around explicitly.

1. **Status Codes**: Failing functions return copy-type status values (`i32`, `bool`). `0` = success.
2. **Tuple Returns**: `(T, i32)` pairs a payload with an error code; destructured at the call site via `let (val, err) = ...`.
3. **Out-Params**: `&T` parameters for explicit outputs on hot paths.
4. **Predicates**: Ambiguous sentinels (`str_to_int`, `env_get`) are paired with boolean predicates (`str_is_int`, `env_exists`) that must be checked first.
5. **Fatal Abort**: `abort(msg)` prints to stderr and exits with status 134. Frames are discarded without linear cleanup — abort is not an error channel.
6. **Explicit Propagation**: Every error path is visible in the source; there is no implicit propagation.

---

## 9. Diagnostics & Error Reporting

Track compiler diagnostics report span location, root cause, and ownership state transitions:

```
error[TK201]: use of moved/spent variable `v`
  --> examples/move_error.trk:5:13
   |
 2 | let v = vec_init(16);
 3 | let x = v;
   |         - value moved here
 4 |
 5 | vec_push(&mut v, 42);
   |             ^ value used after move
```
