# Imports

Track uses `@use()` for explicit module imports. Resolved at compile time.

## Basic Import

```track
@use("std::io")

fn main() -> void {
    io::print("hello");
}
```

## Import Specific Items

```track
@use("std::io::{print, read}")

fn main() -> void {
    print("hello");
}
```

## Import with Alias

```track
@use("std::io") as console

fn main() -> void {
    console::print("hello");
}
```

## Import Specific Items with Alias

```track
@use("math::vec::{add, sum}") as math

fn main() -> void {
    math::add(1, 2);
}
```

## Syntax

```
@use("path::to::module")
@use("path::to::module::{item1, item2}")
@use("path::to::module") as alias
@use("path::to::module::{item1}") as alias
```

## Built-in Modules

| Module | Functions | Description |
|--------|-----------|-------------|
| `std::io` | `print`, `read` | I/O operations |
| `math::vec` | `add`, `sub` | Vector math |

## Rules

- Explicit paths only—no hidden imports
- Resolved at compile time—no runtime overhead
- `::` separates path segments
- `{}` selects specific items
- `as` creates an alias
- Linear types apply to imported resources
