# Imports

Track uses file-path based imports (`import "path/to/module"`) for explicit, file-level dependency resolution at compile time with zero runtime overhead.

## Basic Import

```track
import "std/io";

fn main() -> void {
    io::print("hello");
}
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

## Built-in Modules

| Module | Functions | Description |
|--------|-----------|-------------|
| `std/io` | `print`, `read` | Standard I/O operations |
| `math/vec` | `add`, `sub` | Vector math functions |

## Rules

- Explicit file paths—no hidden global imports.
- Resolved at compile time—zero runtime overhead.
- `::` separates module scopes and imports specific items.
- `as` creates a local module alias.
- Linear type rules and borrow safety apply to all imported resources.
