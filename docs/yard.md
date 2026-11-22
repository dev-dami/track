# Yard — Track Package Manager

Yard is the package manager and build orchestrator for the Track programming language.

Yard is written in Track (`tools/yard/*.trk`) and drives the self-hosted compiler `trackc`. It replaced an earlier Rust implementation (`src/yard/`), since deleted; the behaviour below is unchanged.

## Building yard

```bash
trackc build --native tools/yard/main.trk -o yard
```

This needs no Rust toolchain and no C compiler. Put `trackc` next to the `yard` binary, or on `PATH`.

### Finding `trackc`

Yard looks for the compiler in this order:

1. `$TRACKC`, when it is set and not empty;
2. `trackc` in the same directory as the `yard` binary;
3. `trackc` on `PATH`.

### Source layout

| File | Contents |
|------|----------|
| `tools/yard/main.trk` | command dispatch, `init`, `add`, `run`, `clean`, `lint` |
| `tools/yard/builder.trk` | `build`, `check`, `test`, and how yard calls `trackc` |
| `tools/yard/manifest.trk` | `Track.toml` reader and writer |
| `tools/yard/cache.trk` | `target/.cache_meta.json` |
| `tools/yard/resolver.trk` | dependency resolution |
| `tools/yard/yutil.trk` | text, list, shell and FNV-1a helpers |

Yard uses only existing runtime builtins. It runs `mkdir -p`, `rm -rf`, `find`, `cp` and `cksum` through `sys_exec` (`/bin/sh -c`).

### Tests

```bash
tests/yard/run.sh                       # builds yard with trackc, then tests it
YARD_BIN=./yard tests/yard/run.sh       # tests an existing binary
```

Each case matches a test of the deleted Rust suite (`tests/yard_tests.rs`), named in its header.

## Project Structure

A typical Yard package layout looks like this:

```
my_project/
├── Track.toml
├── src/
│   └── main.trk
└── .gitignore
```

### `Track.toml`

The package manifest defines package metadata, build settings, and dependencies:

```toml
[package]
name = "my_project"
version = "0.1.0"
authors = ["Your Name <email@example.com>"]

[dependencies]
# Dependencies can be specified here

[build]
src = "src"
```

---

## Commands

### `yard --version` / `yard -v`

Displays current Yard toolchain and compiler version:

```bash
yard -v
# Output: yard 1.0.0
```

### `yard init <name>`

Scaffolds a new Track project in a directory matching `<name>`:

```bash
yard init my_project
```

This creates the default folder layout, configures `Track.toml`, and writes a simple "hello world" program to `src/main.trk`.

### `yard build`

Builds the current project and all of its dependency packages:

```bash
yard build
```

Each `.trk` module in the source directory is checked on its own with `trackc check`. Its artifact is a stamp, `target/objects/<module path>.o`, holding the key it was checked under. Modules check in parallel. If any fail, yard reports the first failure in source order. Files whose name contains `_test`, and files under a `tests/` directory, are left out of the build.

The executable holds every module of the package, nested ones included, as the Rust yard linked every object. A module can call a function from another module without importing it. Yard writes `target/_yard_root.trk`, with one `import "<src>/<module>"` per module. It then builds that file with `trackc build --native`, from the project root, into `target/<package name>`. A module that is also imported explicitly resolves to the same file and is loaded once. The package needs a `main.trk` holding `fn main`. Dependencies are not compiled into the image.

Builds are incremental. `target/.cache_meta.json` maps each source path to a hash of the yard version, the `trackc` binary (its `cksum`), every package source and that module's own text. Yard re-checks a module only when its hash changes or its stamp is missing. The image has its own entry, keyed by the same package fingerprint. An unchanged package keeps its image, and an edit to any module rebuilds it. A warm build leaves stamps and the image untouched, and it writes the same metadata, byte for byte, as a clean build does.

Dependencies are resolved in name order. Path dependencies must exist. Registry and git dependencies are reported and skipped, because neither is available yet.

### `yard run`

Builds and immediately executes the package binary:

```bash
yard run
```

### `yard check`

Runs `trackc check` (parsing, type and ownership checking, no codegen) on every source file in the project, in parallel. If those pass, it builds the same image `yard build` would, in a scratch directory. So check fails when build would, for example on a call to a function no module defines:

```bash
yard check
```

### `yard lint`

Lints all source files in the project for potential code style and safety issues:

```bash
yard lint
```

### `yard test`

Runs the package's Track tests: `src/*_test.trk`, `.trk` files in subdirectories of `src/`, and `tests/**/*.trk`. Each test becomes the `main.trk` of a temporary package. That package also gets the project's other top-level modules and the non-`.trk` files under `tests/fixtures`. The test passes if the program exits 0.

```bash
yard test
```

### `yard clean`

Removes the `target/` build cache directory:

```bash
yard clean
```

### `yard add <package>`

Adds a new dependency to the project's `Track.toml` manifest:

```bash
# Add a local path dependency
yard add my_library --path ../my_library

# Add a Git dependency
yard add logger --git https://github.com/example/logger.git

# Add a registry dependency (default version 0.1.0)
yard add some_dep --version 0.2.0
```

`Track.toml` is rewritten with simple dependencies (`name = "1.0"`) first, then one `[dependencies.<name>]` table for each path or git dependency. Within each group, dependencies are sorted by name.
