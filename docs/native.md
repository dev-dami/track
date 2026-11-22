# The native backend and the Track runtime

`trackc` compiles `compiler/src/*.trk` straight to a Linux x86-64
executable, with no C compiler and no C source in the chain. Track targets
x86-64 Linux only. Two halves:

- **the runtime written in Track** (`compiler/rt/*.trk`), which needs no
  libc;
- **the native backend** (x86-64 code + ELF writer), which lowers Track,
  including the five machine builtins below.

`trackc build` (`ngen.trk`, `x64.trk`, `elf.trk`) writes the executable
itself, and `scripts/selfhost.sh` rebuilds trackc that way with no program
reachable (docs/bootstrap.md). The C backend and the C runtime
(`runtime/track_runtime.c`) that once stood beside them are gone (card NOC).

## Machine builtins

Registered in `checker.trk` (`register_builtins`) and lowered inline by
`ngen.trk`. Every argument and result is a word.

| builtin | meaning | C equivalent |
|---|---|---|
| `syscall6(nr, a, b, c, d, e, f) -> i64` | Linux x86-64 `syscall`: `rax=nr, rdi=a, rsi=b, rdx=c, r10=d, r8=e, r9=f`; returns `rax` (negative errno on failure). Clobbers `rcx`, `r11`, memory. | inline asm `syscall` |
| `load64(addr) -> i64` | 8-byte load | `((tk_w *)addr)[0]` |
| `store64(addr, v) -> i64` | 8-byte store; result 0 | `((tk_w *)addr)[0] = v` |
| `load8(addr) -> i64` | zero-extended byte load | `*(unsigned char *)addr` |
| `store8(addr, v) -> i64` | store the low byte of `v`; result 0 | `*(unsigned char *)addr = v` |

A user function with the same name shadows the builtin (as for every
runtime name). The native backend must emit these as instructions, not
calls; `store64`/`store8` must not be reordered across `syscall6`.

## Runtime modules (`compiler/rt/`)

| module | contents |
|---|---|
| `mem.trk` | state page, size-class allocator on `mmap`, `alloc`/`dealloc`/`mem_realloc`, memory limit, raw copy/zero/strlen, `track_alloc_nz`, `track_words_copy`, `track_runtime_init` |
| `io.trk` | `rt_start`, `sys_exit`, buffered stdout, stderr, `print*`, `track_out`/`track_err`, `read_line`, `track_abort`, `track_trap_div`, int formatting |
| `str.trk` | `str_*`, `int_to_str`, `track_str_bytes`, `char_*`, `path_*` |
| `vec.trk` | `vec_*`, `track_i64_collection_*`, `track_sb_*`, `track_array_*`, `array_len`, `track_bad_field`, `math_*`, `add`/`sum`/`sub` |
| `sys.trk` | argv, `env_*`, `sys_exec`/`process_spawn` (fork + execve `/bin/sh -c` + wait4), `clock_ms`, `file_*`, `track_file_*`, `dir_exists`, `net_socket_*` |
| `libc.trk` | `open64`, `write`, `fchmod`, `close`, `memset`: the libc names `elf.trk` calls, written in Track |

The runtime keeps the word-ABI behaviour of the retired C runtime
(docs/bootstrap.md, "Value model"): strings are NUL-terminated byte
pointers; handles to collections and string builders point to
`[data, len, cap]` records; booleans are 0/1. File handles are `[fd]`
records and writes through them are unbuffered; stdout is buffered in a 64 KiB runtime
buffer and flushed when full, before any stderr write, before `sys_exec`,
and at exit.

The rt modules import nothing; trackc loads all of them after the program.
`scripts/embed_runtime.sh` embeds them into `compiler/src/runtime_trk.trk`;
`scripts/embed_runtime.sh --check` fails when it is stale. Edit
`compiler/rt/*.trk`, then run `scripts/embed_runtime.sh` and
`scripts/update_seed.sh`.

**Runtime state.** Track has no globals, so the runtime keeps its state
(argc/argv/envp, heap bump pointer, free-list heads, stdout buffer, RNG,
scratch for syscalls) in one 8 KiB page mapped at the fixed address
`RT_BASE = 0x200000000000` with `MAP_FIXED_NOREPLACE` in
`track_runtime_init`. The native backend needs no data section for the
runtime, but must not place segments at that address.

**Allocator.** Power-of-two blocks from 32 bytes, one header word
(`requested_size << 8 | class`), per-class free lists, 64 MiB `mmap`
arenas. Blocks over 64 KiB get a prefaulted mapping of their own that
grows with `mremap`; freeing one keeps it in the large-block cache (at
most 16 mappings, 128 MiB; past that the oldest are unmapped), and the
next large request takes the smallest cached mapping that fits, so a
free-and-reallocate loop makes no syscalls. Cached blocks are not in use
for the memory limit. `alloc` zeroes and counts against the memory limit;
collections, builders, and strings use the uncounted `rt_malloc`, as the
C runtime uses `malloc`.

## Entry contract (`_start`)

The kernel enters `_start` with `rsp` 16-byte aligned and pointing at:

```
[rsp]        argc
[rsp+8]      argv[0] ... argv[argc-1], 0
             envp[0] ... 0
             auxv ...
```

The native backend emits:

```
_start:
    xor  %ebp, %ebp
    mov  (%rsp), %rdi        # argc
    lea  8(%rsp), %rsi       # argv
    and  $-16, %rsp
    call rt_start            # Track: rt_start(argc, argv); never returns
    hlt
```

`rt_start` maps the state page, derives `envp = argv + 8 * (argc + 1)`,
calls the program's `main()`, flushes stdout, and exits with `exit_group`
and main's result. Calls follow the System V convention for word
arguments (`rdi, rsi, rdx, rcx, r8, r9`, result in `rax`).

Runtime names the emitter calls directly (`alloc`, `track_alloc_nz` for
struct, tuple, array and union cells, `track_array_get/set/slice`,
`track_words_copy`, `track_bad_field`, `track_trap_div`, `track_abort` for
`abort`) resolve to the Track functions.

## Symbols (`.symtab`)

`trackc build -g` (or `--symbols`) adds an ELF symbol table, so
`nm`, `objdump -d`, `perf report` and `gdb` show function names. The
PT_LOAD segment and every loaded byte are the same as without the flag;
after them `elf.trk` appends `.symtab`, `.strtab`, `.shstrtab` and six section headers (null,
`.text` = the code, `.data` = the strings and constants after it, then the
three tables). Each function `ngen.trk` emits out of line gets one
`STB_GLOBAL STT_FUNC` symbol in `.text` with its exact code range
(`ng_symbol`): `_start`, every user and runtime function, and both entries
of a ret-slot function (`<f>__rs` and `<f>`). A function whose every call
was inlined still has its out-of-line copy and its symbol; inlined bodies
inside a caller are attributed to the caller. Cold stubs belong to the
function that emitted them.

Opt-in, not always on: the table names the ~180 runtime functions in
every program, so small programs grow by ~7 KB (hello 27,735 to 34,656
bytes, +25%), over the 10% budget, though native trackc grows only +2.6%
(1,484,541 to 1,522,480). Without `-g` the output is byte-identical to
before symbols existed, so selfhost stages and goldens are untouched.
Profile with:

```
trackc build -g compiler/src/main.trk -o /tmp/tc
perf record /tmp/tc build compiler/src/main.trk -o /tmp/out
perf report --stdio
```

## Runtime calls trackc itself makes

The runtime surface the compiler needs (calls in code, not in the
embedded runtime text):

- emitted by ngen: `alloc`, `track_alloc_nz`, `track_array_get`, `track_array_set`,
  `track_array_slice`, `track_words_copy`, `track_bad_field`,
  `track_trap_div`, `track_abort`;
- called through `base.trk`/`main.trk`: `track_i64_collection_{new,len,push,get,set,pop,truncate}`,
  `track_sb_{new,push,push_byte,push_int,len,str,clear}`, `track_str_bytes`,
  `str_{len,eq,concat,substr,char_at,starts_with,ends_with}`, `int_to_str`,
  `track_out`, `track_err`, `track_file_read`, `track_file_write_all`,
  `file_exists`, `file_remove`, `sys_exec`, `sys_exit`, `os_args_count`,
  `os_arg`, `env_get`, `env_exists`, `clock_ms`;
- entry: `rt_start`.
