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

### `track yard init <name>`

Scaffolds a new Track project in a directory matching `<name>`:

```bash
track yard init my_project
```

This creates the default folder layout, configures `Track.toml`, and writes a simple "hello world" program to `src/main.trk`.

### `track yard build`

Builds the current project and all of its dependency packages:

```bash
track yard build
```

This compiles each `.trk` file in the source directory to an LLVM object file and links them together to produce a native executable under `target/<project_name>`.

### `track yard run`

Builds and immediately executes the package binary:

```bash
track yard run
```

### `track yard add <package>`

Adds a new dependency to the project's `Track.toml` manifest:

```bash
# Add a local path dependency
track yard add my_library --path ../my_library

# Add a Git dependency (resolved in later versions)
track yard add logger --git https://github.com/example/logger.git
```

### `track yard check`

Runs tokenization, parsing, and type-checking on all source files in the project without performing LLVM codegen:

```bash
track yard check
```
