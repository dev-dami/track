# Constants

Compile-time constants with explicit values. No hidden evaluation.

## Basic Constants

```track
const BUFFER_SIZE = 1024;
const SAMPLE_RATE = 44100;
const DEBUG = 1;
```

## Type Aliases (`type Alias = TargetType;`)

Track supports type aliases for domain clarity and clean code organization:

```track
type ByteBuf = []u8;
type FilePath = Str;
type DeviceId = u64;

fn process_buffer(buf: ByteBuf, path: FilePath) -> void {
    print_str(path.data);
}
```

## Syntax

```
const NAME = expression;
type ALIAS_NAME = TargetType;
```

## Rules

- Evaluated at compile time
- Immutable after definition
- No hidden evaluation
- Type inferred from value
- Usable in register addresses and other constant expressions (array sizes currently require literal integers — const array sizing is planned, see the roadmap)
- Folded by both compilers: integer literals, other constants, unit enum variants, unary `-` and `!`, `+ - * / %`, `& |`, `<< >>`, comparisons and `&& ||` (comparisons and logic give `0` or `1`)
- A constant `/` or `%` by zero, or `INT64_MIN / -1`, is a compile error (E0200), in constants and in explicit enum discriminants
- A parameter or local named like a constant shadows it: in `fn d(TWO: i64) -> i64 { return 7 / TWO; }` the division happens at run time and `d(0)` flushes stdout and traps (SIGFPE)
- Integer literals are decimal; hex literals such as `0x10` are not lexed yet

## Examples

### Buffer Configuration

```track
const BUFFER_SIZE = 1024;
const CHANNELS = 2;
const SAMPLE_RATE = 44100;

// array sizes take literals: [i32; 1024], not [i32; BUFFER_SIZE]
```

### Hardware Registers

```track
const GPIO_BASE = 1073876992;   // 0x40021000
const GPIO_MODER = GPIO_BASE + 0;
const GPIO_OTYPER = GPIO_BASE + 4;
```

### Magic Numbers

```track
const MAX_RETRY = 3;
const TIMEOUT_MS = 5000;
const CHUNK_SIZE = 64;
```

## Constants vs Variables

| Feature | `const` | `let` |
|---------|---------|-------|
| Evaluation | Compile-time | Runtime |
| Mutability | Immutable | Mutable with `mut` |
| Scope | File-level | Block-level |
| Overhead | Zero | Stack allocation |
