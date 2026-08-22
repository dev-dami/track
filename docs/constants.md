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

## Examples

### Buffer Configuration

```track
const BUFFER_SIZE = 1024;
const CHANNELS = 2;
const SAMPLE_RATE = 44100;

let buffer: [i32; 1024];
```

### Hardware Registers

```track
const GPIO_BASE = 0x40021000;
const GPIO_MODER = GPIO_BASE + 0x00;
const GPIO_OTYPER = GPIO_BASE + 0x04;
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
