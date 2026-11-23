# The native backend and the Track runtime

`trackc` compiles `compiler/src/*.trk` straight to a native executable,
with no C compiler and no C source in the chain: a static ELF for
x86_64-linux, a PE32+ console executable for x86_64-windows (see
"Targets"). Two halves:

- **the runtime written in Track** (`compiler/rt/*.trk`), which needs no
  libc;
- **the native backend** (x86-64 code + ELF writer), which lowers Track,
  including the five machine builtins below.

`trackc build` (`ngen.trk`, `x64.trk`, `elf.trk`) writes the executable
itself, and `scripts/selfhost.sh` rebuilds trackc that way with no program
reachable (docs/bootstrap.md). The C backend and the C runtime
(`runtime/track_runtime.c`) that once stood beside them are gone (card NOC).

## Targets

`trackc build|run|check --target <arch>-<os>` (also `--target=<t>` or
`TRACKC_TARGET=<t>`) picks the target; the default is the host. Arch is
`x86_64` or `aarch64`, OS is `linux`, `macos` or `windows`; any other
pair is an error (exit 2). `check` accepts every known pair; `build`
generates code for:

| target | output | status |
|---|---|---|
| x86_64-linux | static ELF, Linux syscalls | default; output byte-identical to earlier trackc |
| x86_64-windows | PE32+ console .exe, kernel32 imports | builds and runs (tested under wine); trackc.exe self-hosts |
| aarch64-linux | static ELF, A64 lifted from ngen's x86-64 (ngen_a64.trk) | builds and runs (tested under qemu) |
| x86_64-macos | Mach-O MH_EXECUTE, libSystem imports, ad-hoc signature | builds; structure and signature checked offline, runs in CI on an Intel Mac [CI only] |
| aarch64-macos | the same Mach-O for arm64, A64 lifted as for aarch64-linux | builds; checked offline, runs in CI on Apple silicon [CI only] |
| aarch64-windows | | "not supported yet" |

On a Mac the default target is x86_64-macos (arm64 Macs run it under
Rosetta 2); pass `--target aarch64-macos` for native code.

`run` only runs host-target programs. The target picks the OS layer:
trackc loads `rt/os/<os>.trk` with the portable rt modules and passes
the OS to ngen (`generate_native_for`, NG_TARGET).

### The OS layer

Every `compiler/rt/os/<os>.trk` defines this interface (the names the
other rt modules, ngen and the checker builtins call):

- process: `os_args_count`, `os_arg`, `env_get`, `env_exists`,
  `sys_exec`, `process_spawn`, `clock_ms`;
- files: `file_open`, `file_close`, `file_read_all`, `file_write`,
  `file_exists`, `file_remove`, `file_size`, `dir_exists`, `file_copy`,
  `track_file_read`, `track_file_write_all`;
- network: `net_socket_tcp_listen`, `_accept`, `_connect`, `_send`,
  `_recv`, `_close`;
- off Linux only: `os_start()` (the entry; builds argc/argv and calls
  `rt_start`) and `os_syscall(nr, a, b, c, d, e)`.

The portable modules (`mem`, `io`, `libc`) still make their few kernel
calls with `syscall6` and Linux numbers (read, write, open, close, stat,
lseek, mmap, munmap, mremap, madvise, pipe, access, getpid, kill, unlink,
clock_gettime, exit_group, fchmod). On Linux that is the `syscall`
instruction; on any other target ngen lowers `syscall6(nr, a..f)` to a
call of the OS layer's `os_syscall(nr, a..e)` (the runtime never passes
a sixth argument), which maps each number to the OS. This keeps
x86_64-linux output byte-identical: every rt function is emitted in load
order, so moving Linux code between files would change every binary.

### Imported functions (`extern`)

```track
extern "kernel32.dll" fn WriteFile(h: i64, buf: i64, n: i64, done: i64, ov: i64) -> i32;
```

declares a function in a system library (`extern` is contextual: only
`extern` followed by a string starts one). Calls type-check like any
function; arguments are words. The return type picks the result width:
`i32` sign-extends `eax`, `u32`/`bool` zero-extends it (Win32 `BOOL` and
`DWORD`), anything else keeps `rax`.

The backend (`ffi.trk`) records an import table, (library, symbol)
pairs, and per call site a fixup (code position, import id); the image
writer lays out the table and fills the fixups. For x86_64-windows each
call is `call [rip + IAT slot]` with the Win64 ABI: arguments 1-4 in
rcx, rdx, r8, r9, the rest on the stack above 32 bytes of shadow space,
rsp 16-byte aligned (`ffi_win64_call`); pe.trk writes one import
descriptor per library. For x86_64-macos the call is the same
`call [rip + slot]`, the slot a `__got` entry, with the System V ABI:
arguments 1-6 in rdi, rsi, rdx, rcx, r8, r9, the rest at [rsp], rsp
16-byte aligned, al = 0 (`ffi_sysv_call`). For aarch64-macos the A64
translator turns that sequence into an AAPCS64 call (`ax_import_call`:
x0-x5 from the System V registers, x6/x7 from the first two stack words,
adrp + ldr of the slot, blr); variadic arguments, which Apple's arm64 ABI
puts on the stack, are not supported, so the macOS layer calls no
variadic function with variadic arguments (open(2) without a mode,
creat(2) to create). aarch64-windows reuses the tables and the
translator hook with its own call sequence. On a target without import
support a call to an `extern` function is an error.

`as "symbol"` after the signature names the library's symbol when the
Track name differs (a runtime name such as `write` is taken, or the
symbol is not an identifier):

```track
extern "libSystem.B.dylib" fn c_write(fd: i64, buf: i64, n: i64) -> i64 as "write";
```

### Mach-O layout (`macho.trk`)

Segments are 16 KiB aligned in the file and in memory (a byte's file
offset is its address minus 0x100000000), so arm64's page size holds:

| segment | contents |
|---|---|
| `__PAGEZERO` | 4 GiB, no access |
| `__TEXT` (r-x) | mach header and load commands; `__text` at file offset 0x4000, the entry (LC_MAIN) at its first byte |
| `__DATA` (rw-) | `__data` (ngen's data, the code padded to 16 KiB so rip-relative / adrp references hold), `__got` (one slot per import, S_NON_LAZY_SYMBOL_POINTERS) |
| `__LINKEDIT` (r--) | bind opcodes, export trie (`__mh_execute_header`), symbol table, indirect symbol table, string table, code signature |

Load commands: the four segments, LC_DYLD_INFO_ONLY (bind opcodes, no
rebases: the code is position independent, the image is MH_PIE),
LC_SYMTAB, LC_DYSYMTAB, LC_LOAD_DYLINKER `/usr/lib/dyld`, LC_UUID (from
the SHA-256 of code and data, so builds are reproducible),
LC_BUILD_VERSION (macOS, minos 11.0), LC_MAIN (stack 32 MiB), one
LC_LOAD_DYLIB per library (`"libSystem.B.dylib"` is
`/usr/lib/libSystem.B.dylib`), LC_CODE_SIGNATURE. Each import is bound
with BIND_TYPE_POINTER into its `__got` slot, two-level namespace, the
library ordinal being its LC_LOAD_DYLIB. On x86_64 `stat`, `fstat` and
`lstat` bind `$INODE64`, the 64-bit-inode layout arm64 has under the
plain names.

The entry is called as main(argc, argv, envp, apple); it passes argc and
argv to `os_start` (rt/os/macos.trk), which calls `rt_start`. The
cputype is x86_64 (0x01000007) or arm64 (0x0100000c); the x86_64 fixups
are rip displacements, the arm64 ones adrp + ldr pairs (`mo_patch_a64`).

**Code signature.** Arm64 macOS refuses unsigned code, so every image
carries an ad-hoc, linker-signed signature as ld64 writes it: a
SuperBlob (0xfade0cc0) with one CodeDirectory (0xfade0c02, version
0x20400, flags `adhoc | linker-signed`, no special slots, identifier =
the output file name), SHA-256 (computed in Track) of every 4 KiB page of
the file up to the signature, `codeLimit` = the signature offset, exec
segment = `__TEXT` (main binary). `tests/tools/macho_check.py` re-parses
an image independently and recomputes every hash; `tests/sh/macos.sh`
runs it over hello, the examples and trackc for both macOS targets.

Not checked off a Mac: running. CI's `macos` job runs the cross-built
binaries on an Intel runner (x86_64-macos) and an Apple-silicon one
(aarch64-macos), against the example goldens, and has trackc rebuild
hello there byte for byte.

### PE32+ layout (`pe.trk`)

DOS header and stub, COFF header (AMD64, relocations stripped), PE32+
optional header (console subsystem, image base 0x140000000, 256 MiB
stack reserve and 16 MiB committed: ngen frames may skip guard pages),
then `.text` (RVA 0x1000, entry at its first byte), `.data` (ngen pads
the code to a page for this target so rip-relative data references hold)
and `.idata`. The Windows entry aligns the stack and calls `os_start`;
`sys_exit` ends in `ExitProcess`. File descriptors are HANDLEs, 0-2 the
standard handles; paths and arguments are UTF-8 in Track and UTF-16 at
the kernel32 boundary.

Sockets use Winsock (ws2_32.dll; WSAStartup on the first call).
`track_trap_div` ends the process with `0xC0000094`
(STATUS_INTEGER_DIVIDE_BY_ZERO), the code of a hardware divide fault; other
`kill` signals exit with 128 + signo. `unlink` renames the file to a
scratch name before deleting it, so the name is gone at once even where
a file with open handles stays visible until they close (wine, FAT).
`sys_exec` runs `cmd.exe /s /c "cmd"`. trackc treats `\` as a path
separator too.

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
| `os/linux.trk` | the Linux OS layer: argv, `env_*`, `sys_exec`/`process_spawn` (fork + execve `/bin/sh -c` + wait4), `clock_ms`, `file_*`, `track_file_*`, `dir_exists`, `net_socket_*` |
| `os/windows.trk` | the Windows OS layer (kernel32 through `extern`): the same interface, plus `os_start` and `os_syscall` |
| `os/macos.trk` | the macOS OS layer (libSystem through `extern`): the same interface, plus `os_start` and `os_syscall` (Linux numbers to libSystem calls; open/mmap flags, the stat layout, clock ids and socket options translated; -errno from `__error()`; `system(3)` for processes; environ from `_NSGetEnviron`) |
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
