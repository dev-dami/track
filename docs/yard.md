# Yard — Track Package Manager

Yard is the package manager and build orchestrator for the Track programming language. It is integrated directly into the `track` toolchain.

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
# Output: yard 0.3.0
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

This compiles each `.trk` file in the source directory and links them together to produce a standalone native executable under `target/<project_name>`.

### `yard run`

Builds and immediately executes the package binary:

```bash
yard run
```

### `yard add <package>`

Adds a new dependency to the project's `Track.toml` manifest:

```bash
# Add a local path dependency
yard add my_library --path ../my_library

# Add a Git dependency
yard add logger --git https://github.com/example/logger.git
```

### `yard check`

Runs tokenization, parsing, and type-checking on all source files in the project without performing codegen:

```bash
yard check
```
