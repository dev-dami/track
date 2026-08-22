# Pattern Matching

Pattern matching provides exhaustive control flow for enums and unions.

## Basic Syntax

```track
match expression {
    Pattern1 => result1,
    Pattern2 => result2,
    _ => default,
}
```

## Matching Enums

```track
enum Color {
    Red,
    Green,
    Blue,
}

let color = Color::Red;

match color {
    Color::Red => print("red"),
    Color::Green => print("green"),
    Color::Blue => print("blue"),
}
```

## Matching Unions

```track
union Value {
    Int(i32),
    Bool(bool),
}

let val: Value = Value::Int(42);

match val {
    Value::Int(x) => print(x),
    Value::Bool(x) => print(x),
}
```

## Wildcard Pattern

Use `_` to catch unmatched cases:

```track
match color {
    Color::Red => print("red"),
    _ => print("other"),
}
```

## Block Bodies

Use `{}` for multi-statement arms:

```track
match val {
    Value::Int(x) => {
        print("integer:");
        print(x);
    },
    _ => print("other"),
}
```

## Guard Conditions

Add conditions with `if`:

```track
match val {
    Value::Int(x) if (x > 0) => print("positive"),
    Value::Int(x) if (x < 0) => print("negative"),
    Value::Int(x) => print("zero"),
    _ => print("other"),
}
```

## Tuple Patterns & Destructuring

Match directly on tuples or destructure in `let` bindings:

```track
// Destructuring Let
let (x, y) = (10, 20);

// Tuple Pattern Matching
let point = (5, 0);
match point {
    (0, 0) => print("origin"),
    (x, 0) => print("on x-axis"),
    (0, y) => print("on y-axis"),
    (x, y) => print("point"),
}
```

## Struct Patterns

Struct patterns match on struct fields. Fields can bind directly by name or
match a nested pattern after `:`:

```track
struct Point {
    x: i32,
    y: i32,
}

let p = Point { x: 0, y: 42 };

match p {
    Point { x: 0, y } => print(y),          // binds y by name
    Point { x, y: 0 } => print(x),
    Point { x, y } => print(x + y),
}
```

## Nested Pattern Matching

Patterns can be nested recursively:

```track
match result {
    Result::Ok((val, true)) => print(val),
    Result::Ok((_, false)) => print("disabled"),
    Result::Err(code) if code > 500 => print("server error"),
    _ => print("unknown"),
}
```

## Rules

- **Exhaustive**: Compiler errors if cases are missing
- **No fallthrough**: Each arm is independent
- **No hidden control flow**: Compiles to jump table or branches
- **Linear safety**: Matched union and tuple values are safely bound or moved
