# Imports

Track uses file-path based imports (`import "path/to/module"`) for explicit, file-level dependency resolution at compile time with zero runtime overhead.

## Basic Import

```track
import "std/io";

fn main() -> void {
    io::print("hello");
}
```

The `use` keyword is accepted as a synonym for `import`:

```track
use "std/io";
```

## Import Specific Items

```track
import "std/io" :: { print, read };

fn main() -> void {
    print("hello");
}
```

## Import with Alias

```track
import "std/io" as console;

fn main() -> void {
    console::print("hello");
}
```

## Import Specific Items with Alias

```track
import "math/vec" as math :: { add, sub };

fn main() -> void {
    math::add(1, 2);
}
```

## Syntax Reference

```track
import "path/to/module";
import "path/to/module" :: { item1, item2 };
import "path/to/module" as alias;
import "path/to/module" as alias :: { item1 };
```

## Built-in Standard Library Modules

| Module | Core Functions | Description |
|--------|-----------|-------------|
| `std/io` | `print`, `print_int`, `read_line` | Standard I/O operations |
| `std/os` | `os_args_count`, `os_arg`, `env_get` | Command line & OS environment |
| `std/fs` | `dir_exists`, `file_copy`, `file_size`, `file_remove` | File system & directory operations |
| `std/process` | `process_spawn`, `sys_exec` | Subprocess creation and command execution |
| `std/path` | `path_basename`, `path_ext`, `path_join` | Path string manipulation |
| `std/char` | `char_is_digit`, `char_is_alpha`, `char_to_upper` | ASCII character classification & conversion |
| `std/str` | `str_starts_with`, `str_substr`, `str_trim`, `str_char_at` | High-performance string operations |
| `std/net` | `net_socket_tcp_listen`, `net_socket_connect`, `net_socket_send`, `net_socket_recv` | POSIX TCP socket networking |
| `math/vec` | `add`, `sub` | Vector math functions |

## Rules

- Explicit file paths—no hidden global imports.
- Resolved at compile time—zero runtime overhead.
- `::` separates module scopes and imports specific items.
- `as` creates a local module alias.
- Linear type rules and borrow safety apply to all imported resources.
