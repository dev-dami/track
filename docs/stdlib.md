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

## Slices & Byte Primitives

```track
// Slices ([]T) are fat views { ptr, len }
let arr: [i64; 5] = [10, 20, 30, 40, 50];
let slice: []i64 = arr[1..4];

// u8 and i8 byte primitives
let byte_val: u8 = 255;
let signed_byte: i8 = -128;
let buf: ptr<u8> = alloc(1024);
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

// Substring find (index or -1)
let idx = str_find("hello world", "world"); // 6

// Parse string to int
let val = str_to_int("42"); // 42

// Format int to owned Str
let s = int_to_str(1337);

// Environment variables
let path = env_get("PATH");
```

## Extended File System & OS

```track
// File size
let bytes = file_size("data.txt");

// File remove / delete
file_remove("temp.txt");

// Execute system command
let exit_code = sys_exec("echo Hello from Track");
```

## Extended Math & Utilities

```track
let x = math_abs(-5);
let y = math_max(10, 20);
let z = math_min(10, 20);
let pow = math_pow(2, 8);
let sqrt_val = math_sqrt(16);
let clamped = math_clamp(150, 0, 100); // 100
let rng = math_random(); // PRNG u64
```

## Network Socket API (`std/net`)

```track
// Create a TCP server listening on port 8080
let server_fd = net_socket_tcp_listen(8080);

// Connect a TCP client to 127.0.0.1:8080
let client_fd = net_socket_connect("127.0.0.1", 8080);

// Accept incoming client connection
let conn_fd = net_socket_accept(server_fd);

// Send & receive raw bytes
let bytes_sent = net_socket_send(client_fd, buf, len);
let bytes_read = net_socket_recv(conn_fd, buf, max_len);

// Close sockets
net_socket_close(conn_fd);
net_socket_close(server_fd);
net_socket_close(client_fd);
```

## Character, Path & String Extensions (v0.3.0)

```track
// Char & Byte Operations
let is_d: bool = char_is_digit(0x35);       // true ('5')
let is_a: bool = char_is_alpha(0x41);       // true ('A')
let upper: u8 = char_to_upper(0x61);         // 'A'

// String Search & Slicing
let starts: bool = str_starts_with("track_compiler", "track"); // true
let sub: Str = str_substr("hello world", 0, 5);              // "hello"
let trimmed: Str = str_trim("   content   ");                 // "content"
let ch: u8 = str_char_at("track", 0);                        // 't'

// Path Utilities
let name: Str = path_basename("/usr/bin/track");            // "track"
let ext: Str = path_ext("main.trk");                         // "trk"
let joined: Str = path_join("compiler", "lexer.trk");       // "compiler/lexer.trk"
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
