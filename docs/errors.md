# Explicit Error Handling

Track does not use `Option`/`Result` wrapper types, a `?` operator, exceptions,
or stack unwinding. Errors are ordinary values that live on the call stack and
are passed around explicitly. Every error path is visible in the source.

## The Convention

| Style | Shape | Use When |
|-------|-------|----------|
| Status code | `-> i32` (`0` = ok) | Simple success/failure, no payload |
| Tuple return | `-> (T, i32)` | Returning a value *and* an error code |
| Out-param | `-> i32` with `out: &T` | Hot paths, large payloads, C interop |
| Fatal abort | `abort(msg)` | Unrecoverable errors — print and exit |

## 1. Error Codes as Copy Primitives

Failing functions return plain status values. Zero allocation, zero wrapping:

```track
fn remove_temp(path: ptr<u8>) -> i32 {
    return file_remove(path); // 0 on success, -1 on failure
}

let code = remove_temp("tmp.bin");
if (code != 0) {
    print_err("could not remove tmp.bin");
}
```

## 2. Multi-Value Returns via Tuples

Return `(value, error_code)` and destructure at the call site:

```track
fn read_config() -> (i64, i32) {
    let ok = file_exists("/etc/app.cfg");
    if (!ok) {
        return (0, 1);
    }
    return (42, 0);
}

let (val, err) = read_config();
if (err != 0) {
    abort("fatal: cannot continue without config");
}
print(val);
```

Linear payloads follow normal ownership rules: moving the value out of the
tuple moves it; the error slot is a copy-type `i32`.

## 3. Explicit Propagation

There is no `?`. Callers branch on the error value and return it upward by
hand — propagation is always visible:

```track
fn open_and_size(path: ptr<u8>) -> (i64, i32) {
    let ok = file_exists(path);
    if (!ok) {
        return (-1, 1); // propagate: not found
    }
    let size = file_size(path);
    if (size < 0) {
        return (-1, 2); // propagate: stat failed
    }
    return (size, 0);
}
```

## 4. Out-Params via References

For hot paths or large payloads, write through a reference and return only the
status:

```track
fn fill(buf: &Vec, n: i32) -> i32 {
    if (n < 0) {
        return -1;
    }
    vec_reserve(buf, n);
    return 0;
}
```

## 5. `abort(msg)` — Fatal Errors

For unrecoverable states, `abort(msg)` prints the message to stderr and exits
the process with status `134`. There is no unwinding — frames are discarded and
linear cleanup is skipped **by design**: abort is for processes that are about
to die anyway.

```track
if (retries == 0) {
    abort("unreachable: gave up after max retries");
}
```

## Rules

- No hidden control flow — every error check is written out
- Error slots are copy types (`i32`, `bool`); payloads stay linear
- `0` means success; nonzero codes are function-defined
- Never ignore a returned error code on a path you can't defend
- `abort` is a last resort, not an error channel
