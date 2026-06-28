# Constants

Compile-time constants with explicit values. No hidden evaluation.

## Basic Constants

```track
const BUFFER_SIZE = 1024;
const SAMPLE_RATE = 44100;
const PI = 3.14159;
```

## Syntax

```
const NAME = expression;
```

## Rules

- Evaluated at compile time
- Immutable after definition
- No hidden evaluation
- Type inferred from value
- Can be used in array sizes, register addresses, etc.

## Examples

### Buffer Configuration

```track
const BUFFER_SIZE = 1024;
const CHANNELS = 2;
const SAMPLE_RATE = 44100;

let buffer: [f32; BUFFER_SIZE];
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
