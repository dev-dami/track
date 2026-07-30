# Anonymous Tuples & Destructuring

Track supports anonymous fixed-size heterogenous tuples, element access via numerical dot indexing, and pattern destructuring.

## 1. Tuple Types & Literals

Tuple types are declared using `(T1, T2, ...)` and created using parentheses:

```track
// Declaration and Initialization
let pair: (i64, bool) = (42, true);
let triple: (i32, Str, i64) = (1, "track", 100);

// Single-element tuple (requires trailing comma)
let single: (i32,) = (5,);
```

## 2. Element Indexing (`.0`, `.1`)

Access tuple components using zero-based numerical dot notation:

```track
let point: (i64, i64) = (10, 20);
let x = point.0; // 10
let y = point.1; // 20
```

## 3. Tuple Destructuring (`let (a, b) = ...`)

Unpack values directly into scope variables:

```track
let (a, b) = (10, 20);
print(a + b); // 30
```

## 4. Pattern Matching with Tuples

Tuples can be matched in `match` statements:

```track
let pair = (0, 42);
match pair {
    (0, y) => print(y),
    (x, 0) => print(x),
    (x, y) => print(x + y),
}
```
