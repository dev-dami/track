export interface StdlibModule {
  name: string;
  category: string;
  description: string;
  functions: {
    name: string;
    signature: string;
    description: string;
    example: string;
    errorModel: 'Status Code' | 'Sentinel (-1)' | 'Boolean Predicate' | 'None' | 'Fatal Abort';
  }[];
}

export const stdlibModules: StdlibModule[] = [
  {
    name: 'std/io',
    category: 'I/O & Streams',
    description: 'Standard output, error reporting, formatted printing, and terminal stream operations.',
    functions: [
      {
        name: 'print_str',
        signature: 'fn print_str(s: Str) -> void',
        description: 'Outputs a string to standard output with automatic flush.',
        example: 'print_str("Hello Track!");',
        errorModel: 'None',
      },
      {
        name: 'print_int',
        signature: 'fn print_int(n: i64) -> void',
        description: 'Formats and prints a signed 64-bit integer to stdout.',
        example: 'print_int(42);',
        errorModel: 'None',
      },
      {
        name: 'print_hex',
        signature: 'fn print_hex(val: u64) -> void',
        description: 'Prints a value in hexadecimal notation prefixed by 0x.',
        example: 'print_hex(0xDEADBEEF);',
        errorModel: 'None',
      },
      {
        name: 'print_err',
        signature: 'fn print_err(msg: Str) -> void',
        description: 'Outputs an error message to standard error stream (stderr).',
        example: 'print_err("fatal initialization failure");',
        errorModel: 'None',
      },
      {
        name: 'read_line',
        signature: 'fn read_line() -> Str',
        description: 'Reads a line of text from standard input into an owned Str buffer.',
        example: 'let input = read_line();',
        errorModel: 'None',
      },
    ],
  },
  {
    name: 'std/fs',
    category: 'File System',
    description: 'POSIX file descriptor operations, file copying, size queries, and deletion.',
    functions: [
      {
        name: 'file_open',
        signature: 'fn file_open(path: ptr<u8>, mode: ptr<u8>) -> i32',
        description: 'Opens a file descriptor. Returns null file descriptor (0) on failure.',
        example: 'let fd = file_open("config.trk", "r");\nif (fd == 0) { abort("cannot open"); }',
        errorModel: 'Sentinel (-1)',
      },
      {
        name: 'file_read_all',
        signature: 'fn file_read_all(fd: i32) -> Str',
        description: 'Reads the entire file into an owned string. Automatically frees when spent.',
        example: 'let content = file_read_all(fd);',
        errorModel: 'None',
      },
      {
        name: 'file_write',
        signature: 'fn file_write(fd: i32, content: &Str) -> i32',
        description: 'Writes byte buffer to file descriptor. Returns 0 on success.',
        example: 'file_write(fd, &content);',
        errorModel: 'Status Code',
      },
      {
        name: 'file_exists',
        signature: 'fn file_exists(path: ptr<u8>) -> bool',
        description: 'Boolean predicate checking file presence before access.',
        example: 'if (file_exists("data.bin")) { /* ... */ }',
        errorModel: 'Boolean Predicate',
      },
      {
        name: 'file_size',
        signature: 'fn file_size(path: ptr<u8>) -> i64',
        description: 'Returns file length in bytes, or -1 if file cannot be statted.',
        example: 'let bytes = file_size("kernel.bin");',
        errorModel: 'Sentinel (-1)',
      },
      {
        name: 'file_copy',
        signature: 'fn file_copy(src: ptr<u8>, dst: ptr<u8>) -> i32',
        description: 'Copies file from source path to destination path. Returns 0 on success.',
        example: 'let res = file_copy("src.trk", "bak.trk");',
        errorModel: 'Status Code',
      },
      {
        name: 'file_remove',
        signature: 'fn file_remove(path: ptr<u8>) -> i32',
        description: 'Deletes a file from the filesystem. Returns 0 on success, -1 on failure.',
        example: 'file_remove("temp.lock");',
        errorModel: 'Status Code',
      },
    ],
  },
  {
    name: 'std/net',
    category: 'Networking',
    description: 'High-performance POSIX TCP server, client connection, and raw packet transfer.',
    functions: [
      {
        name: 'net_socket_tcp_listen',
        signature: 'fn net_socket_tcp_listen(port: i32) -> i32',
        description: 'Binds and listens on local TCP port. Returns server socket file descriptor or -1.',
        example: 'let server_fd = net_socket_tcp_listen(8080);',
        errorModel: 'Sentinel (-1)',
      },
      {
        name: 'net_socket_connect',
        signature: 'fn net_socket_connect(host: ptr<u8>, port: i32) -> i32',
        description: 'Establishes a client TCP connection to target host and port.',
        example: 'let client_fd = net_socket_connect("127.0.0.1", 8080);',
        errorModel: 'Sentinel (-1)',
      },
      {
        name: 'net_socket_accept',
        signature: 'fn net_socket_accept(server_fd: i32) -> i32',
        description: 'Accepts incoming client connection. Returns connected peer socket descriptor.',
        example: 'let peer_fd = net_socket_accept(server_fd);',
        errorModel: 'Sentinel (-1)',
      },
      {
        name: 'net_socket_send',
        signature: 'fn net_socket_send(fd: i32, buf: ptr<u8>, len: i64) -> i64',
        description: 'Transmits raw bytes over active socket. Returns bytes sent.',
        example: 'let sent = net_socket_send(peer_fd, buf, 1024);',
        errorModel: 'Sentinel (-1)',
      },
      {
        name: 'net_socket_recv',
        signature: 'fn net_socket_recv(fd: i32, buf: ptr<u8>, max_len: i64) -> i64',
        description: 'Receives raw bytes from socket into target buffer.',
        example: 'let read_bytes = net_socket_recv(peer_fd, buf, 4096);',
        errorModel: 'Sentinel (-1)',
      },
      {
        name: 'net_socket_close',
        signature: 'fn net_socket_close(fd: i32) -> void',
        description: 'Closes and disposes active network socket descriptor.',
        example: 'net_socket_close(client_fd);',
        errorModel: 'None',
      },
    ],
  },
  {
    name: 'std/str',
    category: 'String & Memory',
    description: 'Heap string allocation, substring search, splitting, trim, and conversion primitives.',
    functions: [
      {
        name: 'str_from_literal',
        signature: 'fn str_from_literal(raw: ptr<u8>) -> Str',
        description: 'Constructs an owned Str structure from a null-terminated string literal.',
        example: 'let s = str_from_literal("track");',
        errorModel: 'None',
      },
      {
        name: 'str_len',
        signature: 'fn str_len(s: &Str) -> u32',
        description: 'Calculates the byte length of a string reference in O(1).',
        example: 'let length = str_len(&s);',
        errorModel: 'None',
      },
      {
        name: 'str_eq',
        signature: 'fn str_eq(a: &Str, b: &Str) -> bool',
        description: 'Tests if two strings have equal length and identical byte content.',
        example: 'let same = str_eq(&a, &b);',
        errorModel: 'None',
      },
      {
        name: 'str_concat',
        signature: 'fn str_concat(a: &Str, b: &Str) -> Str',
        description: 'Allocates a new buffer containing the concatenated content of both strings.',
        example: 'let joined = str_concat(&first, &last);',
        errorModel: 'None',
      },
      {
        name: 'str_find',
        signature: 'fn str_find(s: Str, needle: Str) -> i32',
        description: 'Finds index of needle in string, or returns -1 if not found.',
        example: 'let idx = str_find("hello world", "world"); // 6',
        errorModel: 'Sentinel (-1)',
      },
      {
        name: 'str_is_int',
        signature: 'fn str_is_int(s: Str) -> bool',
        description: 'Boolean predicate verifying that string contains valid numeric digits.',
        example: 'if (str_is_int(token)) { let n = str_to_int(token); }',
        errorModel: 'Boolean Predicate',
      },
      {
        name: 'str_to_int',
        signature: 'fn str_to_int(s: Str) -> i64',
        description: 'Parses string to signed integer. Pair with str_is_int.',
        example: 'let val = str_to_int("1337");',
        errorModel: 'None',
      },
      {
        name: 'str_substr',
        signature: 'fn str_substr(s: Str, start: u32, len: u32) -> Str',
        description: 'Extracts a new owned substring slice.',
        example: 'let sub = str_substr(raw, 0, 4);',
        errorModel: 'None',
      },
      {
        name: 'str_trim',
        signature: 'fn str_trim(s: Str) -> Str',
        description: 'Returns a trimmed string with leading and trailing whitespace removed.',
        example: 'let clean = str_trim(dirty_input);',
        errorModel: 'None',
      },
    ],
  },
  {
    name: 'std/os & std/process',
    category: 'System & Process',
    description: 'Process spawning, environment variable queries, and memory ceiling limits.',
    functions: [
      {
        name: 'os_args_count',
        signature: 'fn os_args_count() -> i32',
        description: 'Returns total count of command-line arguments (argc).',
        example: 'let count = os_args_count();',
        errorModel: 'None',
      },
      {
        name: 'os_arg',
        signature: 'fn os_arg(index: i32) -> Str',
        description: 'Fetches command-line argument by zero-based index (argv[i]).',
        example: 'let target_file = os_arg(1);',
        errorModel: 'None',
      },
      {
        name: 'env_exists',
        signature: 'fn env_exists(key: ptr<u8>) -> bool',
        description: 'Tests if an environment variable is present in the current process.',
        example: 'if (env_exists("TRACK_DEBUG")) { /* ... */ }',
        errorModel: 'Boolean Predicate',
      },
      {
        name: 'env_get',
        signature: 'fn env_get(key: ptr<u8>) -> Str',
        description: 'Reads value of environment variable. Pair with env_exists.',
        example: 'let path = env_get("PATH");',
        errorModel: 'None',
      },
      {
        name: 'process_spawn',
        signature: 'fn process_spawn(command: ptr<u8>) -> i32',
        description: 'Spawns a child process and waits for termination. Returns process exit code.',
        example: 'let status = process_spawn("cc -O2 -c out.c -o out.o");',
        errorModel: 'Status Code',
      },
      {
        name: 'sys_set_memory_limit',
        signature: 'fn sys_set_memory_limit(bytes: u64) -> void',
        description: 'Enforces a strict virtual memory ceiling on the active process.',
        example: 'sys_set_memory_limit(128 * 1024 * 1024); // 128MB max',
        errorModel: 'None',
      },
      {
        name: 'sys_get_memory_used',
        signature: 'fn sys_get_memory_used() -> u64',
        description: 'Returns current resident set size (RSS) memory consumption in bytes.',
        example: 'let bytes_used = sys_get_memory_used();',
        errorModel: 'None',
      },
      {
        name: 'abort',
        signature: 'fn abort(msg: ptr<u8>) -> void',
        description: 'Prints fatal message to stderr and immediately terminates process with code 134.',
        example: 'abort("unrecoverable heap corruption");',
        errorModel: 'Fatal Abort',
      },
    ],
  },
  {
    name: 'mem/alloc',
    category: 'Memory Primitives',
    description: 'Low-level manual memory allocators, zeroing, and block reallocation.',
    functions: [
      {
        name: 'alloc',
        signature: 'fn alloc(bytes: u64) -> ptr<u8>',
        description: 'Allocates raw uninitialized memory block. Freed automatically when spent.',
        example: 'let buf: ptr<u8> = alloc(4096);',
        errorModel: 'None',
      },
      {
        name: 'mem_realloc',
        signature: 'fn mem_realloc(p: ptr<u8>, new_size: u64) -> ptr<u8>',
        description: 'Resizes an existing memory block.',
        example: 'let expanded = mem_realloc(buf, 8192);',
        errorModel: 'None',
      },
      {
        name: 'memset',
        signature: 'fn memset(dest: ptr<u8>, val: i32, count: u64) -> void',
        description: 'Fills target memory block with specified byte value.',
        example: 'memset(buf, 0, 1024);',
        errorModel: 'None',
      },
      {
        name: 'memcpy',
        signature: 'fn memcpy(dest: ptr<u8>, src: ptr<u8>, count: u64) -> void',
        description: 'Copies count bytes from source memory buffer to destination.',
        example: 'memcpy(dst, src, 1024);',
        errorModel: 'None',
      },
      {
        name: 'memcmp',
        signature: 'fn memcmp(a: ptr<u8>, b: ptr<u8>, count: u64) -> i32',
        description: 'Compares two memory buffers. Returns 0 if identical.',
        example: 'let matches = (memcmp(a, b, len) == 0);',
        errorModel: 'None',
      },
    ],
  },
];
