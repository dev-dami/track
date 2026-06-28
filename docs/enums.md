# Enums and Unions

Track provides plain enums for type-safe states and tagged unions for variants with associated data.

## Enums

Enums define a fixed set of named constants. They are copy types.

```track
enum Color {
    Red,
    Green,
    Blue,
}

enum Status : u8 {
    Active,
    Locked,
    Spent,
}
```

### Syntax

```
enum Name {
    Variant1,
    Variant2,
    Variant3,
}

enum Name : underlying_type {
    Variant1,
    Variant2,
}
```

### Rules

- Enums are copy types (can be freely copied)
- Optional underlying integer type (`: u8`, `: i32`, etc.)
- Variants are accessed with `::` syntax: `Color::Red`
- No associated data—use unions for that

## Unions

Unions are tagged variants that can hold different types. They are linear types.

```track
union Value {
    Int(i32),
    Float(f64),
    Bool(bool),
}
```

### Syntax

```
union Name {
    Variant1(Type1),
    Variant2(Type2),
    Variant3,
}
```

### Rules

- Unions are linear types (cannot be copied)
- Each variant holds a different type
- Variants accessed with `::` syntax: `Value::Int(42)`
- Use pattern matching to extract values

## Examples

### State Machine

```track
enum State {
    Idle,
    Running,
    Stopped,
}

let state = State::Idle;

match state {
    State::Idle => start(),
    State::Running => process(),
    State::Stopped => cleanup(),
}
```

### Error Handling

```track
union Result(T, E) {
    Ok(T),
    Err(E),
}

let value: Result(i32, str) = Result::Ok(42);

match value {
    Result::Ok(x) => print(x),
    Result::Err(msg) => print(msg),
}
```

### Hardware Registers

```track
enum Register : u32 {
    GPIO_MODER = 0x00,
    GPIO_OTYPER = 0x04,
    GPIO_OSPEEDR = 0x08,
}

let reg = Register::GPIO_MODER;
```
