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

## 2. Move Semantics & Spend Points

An **Owned Linear** value has exactly one owner at any point during execution.

### Spend Points
A linear resource is consumed (spent) by:
1. **Move Assignment**: `let y = x;` transfers ownership from `x` to `y`. `x` transitions to state `Spent`.
2. **Function Call Transfer**: Passing an owned value into a function parameter transfers ownership to the callee.
3. **Implicit Scope Cleanup**: If a linear resource remains `Active` at scope exit, the compiler automatically emits cleanup deallocation code (`vec_free`, `str_free`).

---

## 3. Lexical Lenses & Non-Escaping Guarantee

A **Lexical Lens** provides temporary, exclusive mutable access to a target resource via the `with` construct:

```track
let mut u = User { age: 30 };
with u -> user {
    user.set_age(31);
}
```

### Invariants:
1. **Lexical Exclusivity**: While a lens is active, the underlying target resource (`u`) is in state `Locked`. It cannot be moved, borrowed, or accessed.
2. **Non-Escaping Guarantee**: A lens reference (`user`) is valid **only** within the lexical boundaries of the `with` block. It cannot be assigned to an outer variable, returned, or stored in a heap structure.
3. **Zero Lifetime Annotations**: Lens exclusivity is enforced purely by block scope boundaries without lifetime parameter syntax (`'a`).

---

## 4. Control-Flow Merge Rules (CFG Inconsistency)

When control flow splits (`if / else`), the type-checker evaluates ownership paths independently and verifies consistency at the merge point:

```track
let v = vec_init(16);
if cond {
    consume(v); // v state -> Spent
} else {
    // v state -> Active
}
// ERROR: Resource 'v' has inconsistent state after if/else (Then: Spent, Else: Active)
```

### Rule:
At a CFG merge point, a variable must have the identical state across all incoming execution paths. If one branch consumes a linear resource while another branch preserves it, the compiler emits a compile-time error requiring explicit path reconciliation.

---

## 5. Diagnostics & Error Reporting

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
