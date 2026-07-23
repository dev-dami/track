# Track Core Language Specification (v0.1)

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
- **`Borrowed`**: A read-only reference (`&T`) exists. Track v0.1 permits at most one active shared borrow per value; general `Shared(n)` borrow counts are reserved for a later version.
- **`Locked`**: An exclusive lexical lens (`with`) is currently active.
- **`Spent`**: Ownership has been moved or transferred.

### Transition Table

| Operation | Pre-State | Post-State | Validated Constraint |
| :--- | :--- | :--- | :--- |
| **Move Value** (`let y = x;`) | `Active` | `Spent` | `x` cannot be used after move |
| **Create Lens** (`with u -> user`) | `Active` | `Locked` | `u` frozen from moves/borrows |
| **Exit Lens Block** | `Locked` | `Active` | Restores `u` ownership |
| **Shared Borrow** (`let r = &x;`) | `Active` | `Borrowed` | `x` frozen from moves; v0.1 permits at most one active shared borrow per value |
| **End Borrow Scope** | `Borrowed` | `Active` | Restores full ownership |

---

## 3. Move Semantics & Spend Points

An **Owned Linear** value has exactly one owner at any point during execution.

### Spend Points
A linear resource is consumed (spent) by:
1. **Move Assignment**: `let y = x;` transfers ownership from `x` to `y`. `x` transitions to state `Spent`.
2. **Function Call Transfer**: Passing an owned value into a function parameter transfers ownership to the callee.
3. **Implicit Scope Cleanup**: If a linear resource remains `Active` at scope exit, the compiler automatically emits cleanup deallocation code (`vec_free`, `str_free`).

### Struct Ownership & Field Move Policy (v0.1)
Structs in Track v0.1 are moved as **atomic units**. Moving any field or the struct itself consumes the entire struct instance, preventing use-after-free or partial double-destruction.

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

## 6. Diagnostics & Error Reporting

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
