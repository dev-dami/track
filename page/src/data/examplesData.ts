export interface CodeExample {
  id: string;
  title: string;
  category: 'Ownership' | 'Lenses' | 'Generics' | 'Networking' | 'Patterns' | 'Toolchain';
  description: string;
  filename: string;
  code: string;
  expectedOutput?: string;
  highlights?: string[];
}

export const codeExamples: CodeExample[] = [
  {
    id: 'hello',
    title: 'Hello World',
    category: 'Toolchain',
    description: 'Minimal entrypoint with std/io and void main function.',
    filename: 'examples/hello.trk',
    code: `import "std/io";

fn main() -> void {
    io::print("Hello, Track!");
}`,
    expectedOutput: 'Hello, Track!',
  },
  {
    id: 'linear-auto-free',
    title: 'Linear Ownership & Auto-Free',
    category: 'Ownership',
    description: 'Demonstrates automatic compile-time memory reclamation at spend points without garbage collection.',
    filename: 'examples/linear_auto_free.trk',
    code: `import "std/io";

struct Buffer {
    data: ptr<u8>,
    len: u32,
    cap: u32,
}

fn make_buffer(size: u32) -> Buffer {
    return Buffer {
        data: alloc(size),
        len: 0,
        cap: size,
    };
}

fn consume_buffer(buf: Buffer) -> void {
    io::print_str("Buffer consumed. Automatically freed at exit.");
    // buf is spent here!
}

fn main() -> void {
    let b = make_buffer(512);
    // b is Active
    consume_buffer(b);
    // b is now Spent — compiler prevents reuse!
}`,
    expectedOutput: 'Buffer consumed. Automatically freed at exit.',
    highlights: ['Automatic destructor emission at spend points', 'Zero use-after-free or double-free'],
  },
  {
    id: 'lexical-lens',
    title: 'Lexical Lenses for Scoped Mutation',
    category: 'Lenses',
    description: 'Scoped mutable view using the with construct without lifetime parameters.',
    filename: 'examples/borrow.trk',
    code: `import "std/io";

struct User {
    age: i32,
    score: i64,
}

fn main() -> void {
    let mut u = User { age: 30, score: 100 };
    
    // Lexical lens block: u becomes Locked
    with u -> user {
        user.age = 31;
        user.score = user.score + 50;
    }
    // u transitions back to Active!
    
    io::print_int(u.age);
    io::print_int(u.score);
}`,
    expectedOutput: "31\n150",
    highlights: ['No lifetime annotations (\'a)', 'Zero escaping of lens aliases'],
  },
  {
    id: 'generics-demo',
    title: 'Monomorphized Generics',
    category: 'Generics',
    description: 'Generic function templates specialized at compile time without dynamic dispatch.',
    filename: 'examples/generics.trk',
    code: `import "std/io";

fn identity<T>(x: T) -> T {
    return x;
}

fn pair<T, U>(a: T, b: U) -> (T, U) {
    return (a, b);
}

fn main() -> void {
    let n = identity(42);
    let flag = identity(true);
    
    let p = pair(n, flag);
    let (val, ok) = p;
    
    if (ok) {
        io::print_int(val);
    }
}`,
    expectedOutput: '42',
    highlights: ['Monomorphization pass src/mono.rs', 'Zero vtable / boxing overhead'],
  },
  {
    id: 'error-handling',
    title: 'Explicit Stack Error Convention',
    category: 'Ownership',
    description: 'Multi-value error return tuples and predicate guards without Result wrappers.',
    filename: 'examples/error_handling.trk',
    code: `import "std/io";
import "std/fs";

fn load_data_file(path: Str) -> (i64, i32) {
    if (!file_exists(path.data)) {
        return (0, 1); // Error code 1: File not found
    }
    let size = file_size(path.data);
    return (size, 0); // 0 = Success
}

fn main() -> void {
    let path = str_from_literal("config.trk");
    let (bytes, err) = load_data_file(path);
    
    if (err != 0) {
        io::print_err("Error: Config not found");
        return;
    }
    
    io::print_int(bytes);
}`,
    expectedOutput: 'Error: Config not found',
    highlights: ['No ? operator or unwinding', 'Destructured tuples on stack'],
  },
  {
    id: 'net-tcp-server',
    title: 'POSIX TCP Network Server',
    category: 'Networking',
    description: 'High-speed raw TCP socket server using Track standard library network primitives.',
    filename: 'examples/net_demo.trk',
    code: `import "std/io";
import "std/net";

fn main() -> void {
    let port = 8080;
    let server_fd = net_socket_tcp_listen(port);
    
    if (server_fd < 0) {
        abort("fatal: failed to bind TCP port 8080");
    }
    
    io::print_str("Listening on 0.0.0.0:8080...");
    let conn_fd = net_socket_accept(server_fd);
    
    let buf: ptr<u8> = alloc(1024);
    let bytes_read = net_socket_recv(conn_fd, buf, 1024);
    
    io::print_str("Packet received!");
    net_socket_close(conn_fd);
    net_socket_close(server_fd);
}`,
    expectedOutput: 'Listening on 0.0.0.0:8080...',
    highlights: ['Direct POSIX socket interface', 'Zero runtime wrapper overhead'],
  },
  {
    id: 'patterns-and-tuples',
    title: 'Pattern Matching & Arm Guards',
    category: 'Patterns',
    description: 'Nested pattern matching over anonymous tuples and tagged unions with expression guards.',
    filename: 'examples/v040_features_demo.trk',
    code: `import "std/io";

union Packet {
    Data(i32, bool),
    Ping,
    Disconnect,
}

fn handle_packet(p: Packet) -> void {
    match p {
        Packet::Data(len, verified) if verified => {
            io::print_str("Verified data packet length:");
            io::print_int(len);
        },
        Packet::Data(len, _) => {
            io::print_str("Unverified packet");
        },
        Packet::Ping => {
            io::print_str("Pong");
        },
        Packet::Disconnect => {
            io::print_str("Connection closed");
        },
    }
}

fn main() -> void {
    let p = Packet::Data(256, true);
    handle_packet(p);
}`,
    expectedOutput: "Verified data packet length:\n256",
  },
];
