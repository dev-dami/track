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
    Float(f64),
    Bool(bool),
}

let val: Value = Value::Int(42);

match val {
    Value::Int(x) => print(x),
    Value::Float(x) => print(x),
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

## Rules

- **Exhaustive**: Compiler errors if cases are missing
- **No fallthrough**: Each arm is independent
- **No hidden control flow**: Compiles to jump table or branches
- **Linear safety**: Matched union values are consumed
