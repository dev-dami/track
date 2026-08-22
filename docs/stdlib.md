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

## Hash Map (Illustrative Sketch)

The stdlib does not ship a hash map yet — the following is a sketch of how you
would define one yourself using the memory primitives above:

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
let f = file_open("data.txt", "r"); // returns null (0) if open fails — check before use!
if (f == 0) {
    abort("fatal: cannot open data.txt");
}
let content = file_read_all(f);
// f is automatically closed when spent

// Write file
let f = file_open("out.txt", "w");
file_write(f, &content);
// f is automatically closed when spent
```

```track
// Substring find (index or -1)
let idx = str_find("hello world", "world"); // 6

// Parse string to int (pair with str_is_int — returns 0 on invalid input)
let val = str_to_int("42"); // 42

// Format int to owned Str
let s = int_to_str(1337);

// Environment variables
if (env_exists("PATH")) {
    let path = env_get("PATH");
}
```

## Extended File System & OS (`std/fs`, `std/os`, `std/process`)

```track
// Command-line arguments
let count: i32 = os_args_count();             // argc
let arg0: Str = os_arg(0);                    // argv[0]

// Directory & File utilities
let exists: bool = dir_exists("src");          // true if directory exists
let copied: i32 = file_copy("src.txt", "dst.txt"); // 0 on success
let bytes: i64 = file_size("data.txt");
file_remove("temp.txt");

// Process execution
let exit_code: i32 = process_spawn("echo Hello from Track");
let old_exec: i32 = sys_exec("ls -la");

// Memory boundary enforcement
sys_set_memory_limit(64 * 1024 * 1024); // 64 MB process limit
let used: u64 = sys_get_memory_used();
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

## POSIX Network Socket API (`std/net`)

```track
// Create a TCP server listening on port 8080
let server_fd = net_socket_tcp_listen(8080);

// Connect a TCP client to 127.0.0.1:8080
let client_fd = net_socket_connect("127.0.0.1", 8080);

// Accept incoming client connection
let conn_fd = net_socket_accept(server_fd);

// Send & receive raw bytes
let bytes_sent: i64 = net_socket_send(client_fd, buf, len);
let bytes_read: i64 = net_socket_recv(conn_fd, buf, max_len);

// Close sockets
net_socket_close(conn_fd);
net_socket_close(server_fd);
net_socket_close(client_fd);
```

## Character, Path & String Extensions (`std/char`, `std/str`, `std/path`)

```track
// Char & Byte Operations
let is_d: bool = char_is_digit(0x35);          // true ('5')
let is_a: bool = char_is_alpha(0x41);          // true ('A')
let is_al: bool = char_is_alphanumeric(0x35);   // true
let is_s: bool = char_is_space(0x20);          // true (' ')
let upper: u8 = char_to_upper(0x61);            // 'A'
let lower: u8 = char_to_lower(0x41);            // 'a'

// String Search & Slicing
let starts: bool = str_starts_with("track_compiler", "track"); // true
let ends: bool = str_ends_with("main.trk", ".trk");             // true
let contains: bool = str_contains("track_compiler", "comp");    // true
let sub: Str = str_substr("hello world", 0, 5);                 // "hello"
let trimmed: Str = str_trim("   content   ");                    // "content"
let ch: u8 = str_char_at("track", 0);                           // 't'

// Memory & Vector Extensions
let new_buf: ptr<u8> = mem_realloc(buf, 2048);
vec_reserve(&mut v, 64);
vec_clear(&mut v);
let count: i32 = vec_len(&v);

// Path Utilities
let name: Str = path_basename("/usr/bin/track");               // "track"
let ext: Str = path_ext("main.trk");                            // "trk"
let joined: Str = path_join("compiler", "lexer.trk");          // "compiler/lexer.trk"
```

## Rules

- No hidden allocations
- No garbage collector
- Linear types handle freeing automatically
- No manual free calls — compiler inserts them at spend points
- Stdlib functions are just wrappers around C runtime / native OS calls
- All functions are comptime-resolved when possible

## Error Conventions (v0.5)

Stdlib functions never throw, unwrap, or hide failures. They follow the
explicit error-passing convention (see [errors.md](errors.md)):

| Pattern | Functions |
|---------|-----------|
| Status code (`0` ok / `-1` fail) | `file_remove`, `file_copy`, `process_spawn`, `sys_exec`, `net_socket_*` |
| Sentinel value | `str_find` (-1), `file_size` (-1), `net_socket_recv` (-1) |
| Boolean predicate | `file_exists`, `dir_exists`, `env_exists`, `str_starts_with`, `str_ends_with`, `str_contains`, `str_is_int` |

Fatal, unrecoverable states are handled with `abort(msg)` — print to stderr
and exit with status 134. There is no unwinding.

```track
let fd = net_socket_tcp_listen(8080);
if (fd < 0) {
    abort("fatal: cannot bind port 8080");
}
```

