# Ring Buffer

Fixed-size circular buffer for audio DSP, serial I/O, and real-time systems.

## Usage

```track
struct Ring {
    data: ptr<i32>,
    head: u32,
    tail: u32,
    cap: u32,
}

// Initialize
let mut r: Ring = ring_init(256);

// Push
ring_push(&mut r, 42);

// Pop
let val = ring_pop(&mut r);

// Peek
let front = ring_peek(&r);

// Check
let is_full = ring_full(&r);
let is_empty = ring_empty(&r);
let count = ring_count(&r);
```

## Rules

- Fixed capacity — no dynamic allocation
- Linear type — auto-freed when spent
- Lock-free single producer/consumer
