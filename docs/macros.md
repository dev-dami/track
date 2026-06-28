# Macros

Compile-time macros for code generation and meta-operations. Uses `@` prefix.

## Expression Macros

Return a value:

```track
@macro bit(n: u32) -> u32 {
    return 1 << n;
}

let LED_PIN = @bit(5);
```

## Statement Macros

Perform actions:

```track
@macro assert(condition: bool) -> void {
    if (!condition) {
        @compile_error("assertion failed");
    }
}

@assert(x > 0);
```

## Block Macros

Wrap code blocks:

```track
@macro timer(body: block) -> void {
    let start = @now();
    body;
    let end = @now();
    print(end - start);
}

@timer {
    // code to measure
}
```

## Compile-time Built-ins

```track
@fib_comptime(40)    // evaluated at compile time
@now()               // current timestamp
@compile_error(msg)  // compile-time error
@compile_warning(msg) // compile-time warning
```

## Syntax

### Definition

```
@macro name(params) -> ReturnType {
    body
}
```

### Invocation

```
@name(args)
@name(args) { body }
```

## Examples

### Register Definition

```track
@macro register(addr: u32, mask: u32) -> u32 {
    return addr | (mask << 8);
}

let GPIO_BASE = @register(0x40021000, 0x00FF);
```

### Pin Definition

```track
@macro pin(port: u32, pin: u32) -> u32 {
    return (port << 8) | pin;
}

let led = @pin(1, 5);
```

### Loop Unrolling

```track
@macro unroll_4(body: block) -> void {
    body;
    body;
    body;
    body;
}

@unroll_4 {
    process();
}
```

## Rules

- `@` prefix signals meta-operation
- Evaluated at compile time
- No runtime overhead
- Type-checked at definition
- Can generate code, compute values, or transform syntax
