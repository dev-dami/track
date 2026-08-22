# Monomorphized Generics (v0.6.0)

Track uses **compile-time monomorphization**: generic functions are templates that produce specialized concrete functions for each call site. No vtables, no boxing, no runtime cost.

## Defining Generic Functions

```track
fn identity<T>(x: T) -> T {
    return x;
}

fn pair<T, U>(a: T, b: U) -> (T, U) {
    return (a, b);
}
```

`T`, `U` are type parameters. They can appear as `T`, `[]T`, `ptr<T>`, `(T, U)` anywhere a type can.

## How It Works

At the call site the compiler infers `T` from argument types (variables use the type of their `let` binding; literals default to `i32`/`bool`).

```track
let a: i32 = 5;
identity(a)        // T = i32  →  identity__Ti32
let b: i64 = 10;
identity(b)        // T = i64  →  identity__Ti64
pair(1, true)      // T = i32, U = bool  →  pair__Ti32_Ubool
```

Each distinct `(name, concrete types)` pair produces one mangled function (`name__{params}`) emitted as ordinary Cranelift code. Call sites are rewritten to the mangled name before checking and code generation, so the checker sees only concrete bodies.

## Nested Generic Calls

Inferred types flow through generic results:

```track
let z = pair(identity(1), identity(2)); // infers (i32, i32)
```

## When Inference Needs Help

If an argument is a bare variable whose type can't be inferred, annotate it:

```track
let x: i64 = 42;
identity(x)   // ok — x's type is known
identity(y)   // error: cannot infer T — annotate y first
```

## Current Limitations (v0.6.0)

- **Generic functions only.** Generic structs/records (`struct Stack<T>`) parse the `<T>` but field-generic substitution is planned for a follow-up (type-argument handling for `Stack<T>` as a type is not yet monomorphized).
- Generic type arguments on types (`let s: Stack<i32>`) are not yet parsed — use concrete aliases or `T` directly.

## See Also

- `examples/generics.trk` — working demo (`identity`, `pair`, nested calls)
- `src/mono.rs` — the monomorphization pass (template table → per-site mangled instances)
