# Native seam: the Track runtime and what a native backend must provide

Goal: `trackc` compiles `compiler/src/*.trk` straight to a Linux x86-64
executable, with no C compiler and no C source in the chain. Two halves:

- **the runtime written in Track** (`compiler/rt/*.trk`), which replaces
  `runtime/track_runtime.c` and needs no libc;
- **the native backend** (x86-64 code + ELF writer), which lowers Track,
  including the five machine builtins below.

Until the native backend lands, `trackc build --rt=track` proves the
runtime half through the C backend: it links the Track runtime instead of
the C runtime and builds with `cc -ffreestanding -nostdlib -static` (no
libc, no `.rt.c` file). trackc built that way self-compiles to C that is
byte-identical to trackc on the C runtime, and reaches its own fixed point.

## Machine builtins

Registered in `checker.trk` (`register_builtins`) and lowered inline in
`cgen.trk` (`gen_raw_builtin`). Every argument and result is a word.

| builtin | meaning | C lowering |
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
| `mem.trk` | state page, size-class allocator on `mmap`, `alloc`/`dealloc`/`mem_realloc`, memory limit, raw copy/zero/strlen, `track_words_copy`, `track_runtime_init` |
| `io.trk` | `rt_start`, `sys_exit`, buffered stdout, stderr, `print*`, `track_out`/`track_err`, `read_line`, `track_abort`, `track_trap_div`, int formatting |
| `str.trk` | `str_*`, `int_to_str`, `track_str_bytes`, `char_*`, `path_*` |
| `vec.trk` | `vec_*`, `track_i64_collection_*`, `track_sb_*`, `track_array_*`, `array_len`, `track_bad_field`, `math_*`, `add`/`sum`/`sub` |
| `sys.trk` | argv, `env_*`, `sys_exec`/`process_spawn` (fork + execve `/bin/sh -c` + wait4), `clock_ms`, `file_*`, `track_file_*`, `dir_exists`, `net_socket_*` |

All 110 functions exported by `runtime/track_runtime.c` exist in Track with
the same name and word-ABI behaviour (docs/bootstrap.md, "Value model"):
strings are NUL-terminated byte pointers; handles to collections and string
builders point to `[data, len, cap]` records, the same layout as the C
structs; booleans are 0/1. Deliberate differences: file handles are
`[fd]` records and writes through them are unbuffered (the C runtime
buffers them in stdio until exit); stdout is buffered in a 64 KiB runtime
buffer and flushed when full, before any stderr write, before `sys_exec`,
and at exit.

The rt modules import nothing; trackc loads all of them after the program.
`scripts/embed_rt.sh` embeds them into `compiler/src/runtime_trk.trk` (as
`embed_runtime.sh` does for the C runtime); `scripts/bootstrap.sh` checks it
is current. Edit `compiler/rt/*.trk`, then run `scripts/embed_rt.sh`.

**Runtime state.** Track has no globals, so the runtime keeps its state
(argc/argv/envp, heap bump pointer, free-list heads, stdout buffer, RNG,
scratch for syscalls) in one 8 KiB page mapped at the fixed address
`RT_BASE = 0x200000000000` with `MAP_FIXED_NOREPLACE` in
`track_runtime_init`. The native backend needs no data section for the
runtime, but must not place segments at that address.

**Allocator.** Power-of-two blocks from 32 bytes, one header word
(`requested_size << 8 | class`), per-class free lists, 64 MiB `mmap`
arenas. `alloc` zeroes and counts against the memory limit; collections,
builders, and strings use the uncounted `rt_malloc`, as the C runtime uses
`malloc`.

## Entry contract (`_start`)

The kernel enters `_start` with `rsp` 16-byte aligned and pointing at:

```
[rsp]        argc
[rsp+8]      argv[0] ... argv[argc-1], 0
             envp[0] ... 0
             auxv ...
```

The native backend must emit (this is exactly what the C backend emits in
`--rt=track` mode, `cgen.trk: track_runtime_entry`):

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

In the C backend the runtime functions are Track functions (`tk_alloc`);
runtime names the emitter calls directly (`alloc` for struct and union
cells, `track_array_get/set/slice`, `track_words_copy`, `track_bad_field`,
`track_trap_div`, `track_abort` for `abort`) are emitted as
`__attribute__((alias))` symbols. A native backend can simply resolve those
names to the Track functions.

## Runtime calls trackc itself makes

From the generated C of `compiler/src/main.trk` (calls in code, not in the
embedded runtime text), the runtime surface the compiler needs:

- emitted by cgen: `alloc`, `track_array_get`, `track_array_set`,
  `track_array_slice`, `track_words_copy`, `track_bad_field`,
  `track_trap_div`, `track_abort`;
- called through `base.trk`/`main.trk`: `track_i64_collection_{new,len,push,get,set,pop,truncate}`,
  `track_sb_{new,push,push_byte,push_int,len,str,clear}`, `track_str_bytes`,
  `str_{len,eq,concat,substr,char_at,starts_with,ends_with}`, `int_to_str`,
  `track_out`, `track_err`, `track_file_read`, `track_file_write_all`,
  `file_exists`, `file_remove`, `sys_exec`, `sys_exit`, `os_args_count`,
  `os_arg`, `env_get`, `env_exists`, `clock_ms`;
- entry: `track_runtime_init` (C runtime) / `rt_start` (Track runtime).
