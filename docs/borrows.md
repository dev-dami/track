# Borrows and Escape Analysis in Track

Track enforces memory safety without a garbage collector or complex lifetime annotations. We achieve this using a combination of **linear types**, **lexical lenses**, and **safe references** with compiler-enforced escape checks.

## Reference Types (`&T`)

A reference type is created by prefixing a type with `&` (e.g. `&i64`, `&Buffer`). It represents a safe, read-only borrow of an existing variable. 

References themselves have **Copy semantics**, meaning they can be freely duplicated, passed to functions, and copied without consuming the original reference.

### Address-Of (`&`)

To create a reference to a variable, use the address-of operator `&`:

```track
let x = 42;
let r = &x; // r has type &i32 (or &i64 depending on inference)
```

### Dereference (`*`)

To read the value pointed to by a reference, prefix the expression with the dereference operator `*`:

```track
let val = *r;
```

---

## Active Borrow-Locking

While a variable is borrowed, its owner cannot move, consume, or mutably access the underlying resource. This prevents use-after-free and data races.

If you attempt to move a variable while a borrow is active, the compiler rejects the program:

```track
fn main() -> void {
    let x = [1, 2, 3]; // Arrays are linear resources (non-copy)
    let r = &x;        // x is now Locked
    let y = x;         // Error: x is frozen (either locked in a lens or borrowed).
}
```

The resource is automatically unlocked once the active reference goes out of scope (e.g. at the end of the current function block).

---

## Escape Analysis

To prevent dangling pointers, Track runs compile-time escape analysis. A function is prohibited from returning a reference to a local variable allocated on its own stack frame.

### Example of Invalid Escape

```track
fn get_local_ref() -> &i64 {
    let x = 99;
    return &x; // Error: Cannot return reference to local variable 'x' (escapes function scope).
}
```

References can only escape a function if they borrow from one of the function's parameters:

```track
// Valid: returns a reference borrowing from a parameter
fn choose_first(a: &i64, b: &i64) -> &i64 {
    return a;
}
```
