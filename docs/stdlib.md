# Standard Library

C-style callable functions. No hidden allocations. Linear types handle freeing automatically.

## Core Concept

```track
// You define the data structure
struct Vec {
    data: ptr<i32>,
    len: u32,
    cap: u32,
}

// Stdlib gives you functions to operate on it
let mut v: Vec = vec_init(16);
vec_push(&mut v, 42);
let val = vec_get(&v, 0);
// v is automatically freed when spent (linear type)
```

## Memory

```track
// Allocate raw memory
let buf: ptr<u8> = alloc(1024);

// buf is freed when spent — no manual free needed

// Zero memory
memset(buf, 0, 1024);

// Copy memory
memcpy(dst, src, len);

// Compare memory
let cmp = memcmp(a, b, len);
```

## Strings

```track
// String is just a byte buffer with length
struct Str {
    data: ptr<u8>,
    len: u32,
}

// Create from literal
let s: Str = str_from_literal("hello");

// Length
let len = str_len(&s);

// Compare
let eq = str_eq(&a, &b);

// Concatenate (allocates new buffer)
let combined = str_concat(&a, &b);

// s is automatically freed when spent
```

## Dynamic Arrays (Your Own Vec)

```track
struct Vec {
    data: ptr<i32>,
    len: u32,
    cap: u32,
}

// Initialize with capacity
let mut v: Vec = vec_init(16);

// Push element
vec_push(&mut v, 42);

// Get element (bounds check)
let val = vec_get(&v, 0);

// Set element
vec_set(&v, 0, 100);

// Pop last element
let popped = vec_pop(&mut v);

// v is automatically freed when spent
```

## Hash Map (Your Own)

```track
struct Entry {
    key: Str,
    value: i32,
    next: ptr<Entry>,
}

struct HashMap {
    buckets: ptr<ptr<Entry>>,
    size: u32,
    cap: u32,
}

// Initialize
let mut map: HashMap = hashmap_init(64);

// Insert
hashmap_insert(&mut map, "key", 42);

// Get
let val = hashmap_get(&map, "key");

// Remove
hashmap_remove(&mut map, "key");

// map is automatically freed when spent
```

## I/O

```track
// Print to stdout
print_str("hello");
print_int(42);
print_hex(0xFF);

// Read from stdin
let line = read_line();

// File operations
let f = file_open("data.txt", FILE_READ);
let content = file_read_all(f);
// f is automatically closed when spent

// Write file
let f = file_open("out.txt", FILE_WRITE);
file_write(f, &content);
// f is automatically closed when spent
```

## Math

```track
let x = math_abs(-5);
let y = math_max(10, 20);
let z = math_min(10, 20);
let pow = math_pow(2, 8);
let sqrt_val = math_sqrt(16.0);
```

## Example: Dynamic Buffer

```track
struct Buffer {
    data: ptr<u8>,
    len: u32,
    cap: u32,
}

@macro buffer_init(cap: u32) -> Buffer {
    return Buffer {
        data: alloc(cap),
        len: 0,
        cap: cap,
    };
}

@macro buffer_append(b: ptr<Buffer>, byte: u8) -> void {
    if (b->len < b->cap) {
        b->data[b->len] = byte;
        b->len = b->len + 1;
    }
}

fn main() -> void {
    let mut buf = buffer_init(256);
    buffer_append(&mut buf, 0x48);  // 'H'
    buffer_append(&mut buf, 0x69);  // 'i'
    print_int(buf.len);  // 2
    // buf is automatically freed when spent
}
```

## Rules

- No hidden allocations
- No garbage collector
- Linear types handle freeing automatically
- No manual free calls — compiler inserts them at spend points
- Stdlib functions are just wrappers around LLVM intrinsics
- All functions are comptime-resolved when possible
