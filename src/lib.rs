pub mod ast;
pub mod checker;
pub mod codegen;
pub mod lexer;
pub mod lsp;
pub mod parser;
pub mod yard;

use std::fs;
use std::path::{Path, PathBuf};
use std::process;

pub const RUNTIME_C_SOURCE: &str = r#"
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <time.h>
#include <unistd.h>
#include <sys/socket.h>
#include <netinet/in.h>
#include <arpa/inet.h>
#include <fcntl.h>
#include <sys/stat.h>

void* alloc(size_t size) { return malloc(size); }
void dealloc(void* ptr) { if (ptr) free(ptr); }

typedef struct { int* data; int len; int cap; } Vec;
Vec vec_init(int cap) {
    Vec v;
    v.data = (int*)malloc((size_t)cap * sizeof(int));
    v.len = 0;
    v.cap = cap;
    return v;
}
void vec_push(Vec* v, int val) {
    if (v && v->data && v->len < v->cap) {
        v->data[v->len++] = val;
    }
}
int vec_get(const Vec* v, int idx) {
    if (v && v->data && idx >= 0 && idx < v->len) {
        return v->data[idx];
    }
    return 0;
}
void vec_set(Vec* v, int idx, int val) {
    if (v && v->data && idx >= 0 && idx < v->len) {
        v->data[idx] = val;
    }
}
int vec_pop(Vec* v) {
    if (v && v->len > 0) {
        return v->data[--v->len];
    }
    return 0;
}
void vec_free(Vec v) {
    if (v.data) {
        free(v.data);
    }
}
typedef struct { char* data; int len; } Str;
void str_free(Str s) {
    if (s.data) {
        free(s.data);
    }
}
void* file_open(const char* path, const char* mode) {
    return (void*)fopen(path, mode);
}
void file_close(void* f) {
    if (f) {
        fclose((FILE*)f);
    }
}
int file_exists(const char* path) {
    return access(path, F_OK) == 0 ? 1 : 0;
}
long long clock_ms(void) {
    struct timespec ts;
    clock_gettime(CLOCK_MONOTONIC, &ts);
    return (long long)ts.tv_sec * 1000LL + (long long)ts.tv_nsec / 1000000LL;
}
void sys_exit(int code) {
    exit(code);
}
void print(long long val) {
    printf("%lld\n", val);
}
long long add(long long a, long long b) {
    return a + b;
}
long long sum(long long a, long long b) {
    return a + b;
}
long long sub(long long a, long long b) {
    return a - b;
}
long long str_find(const char* s, const char* sub_str) {
    if (!s || !sub_str) return -1;
    const char* p = strstr(s, sub_str);
    return p ? (long long)(p - s) : -1;
}
long long str_to_int(const char* s) {
    if (!s) return 0;
    return atoll(s);
}
Str int_to_str(long long val) {
    char buf[64];
    snprintf(buf, sizeof(buf), "%lld", val);
    Str s;
    s.len = (int)strlen(buf);
    s.data = (char*)malloc(s.len + 1);
    memcpy(s.data, buf, s.len + 1);
    return s;
}
int file_remove(const char* path) {
    if (!path) return -1;
    return unlink(path) == 0 ? 0 : -1;
}
long long file_size(const char* path) {
    if (!path) return -1;
    FILE* f = fopen(path, "rb");
    if (!f) return -1;
    fseek(f, 0, SEEK_END);
    long long sz = ftell(f);
    fclose(f);
    return sz;
}
long long math_clamp(long long val, long long min_val, long long max_val) {
    if (val < min_val) return min_val;
    if (val > max_val) return max_val;
    return val;
}
static unsigned long long g_rng_state = 0x853c49e65d8dbb29ULL;
unsigned long long math_random(void) {
    unsigned long long x = g_rng_state;
    x ^= x << 13;
    x ^= x >> 7;
    x ^= x << 17;
    g_rng_state = x;
    return x;
}
int sys_exec(const char* cmd) {
    if (!cmd) return -1;
    return system(cmd);
}
Str env_get(const char* key) {
    Str s;
    s.data = NULL;
    s.len = 0;
    if (!key) return s;
    const char* val = getenv(key);
    if (val) {
        s.len = (int)strlen(val);
        s.data = (char*)malloc((size_t)s.len + 1);
        memcpy(s.data, val, (size_t)s.len + 1);
    }
    return s;
}
int net_socket_tcp_listen(int port) {
    int fd = socket(AF_INET, SOCK_STREAM, 0);
    if (fd < 0) return -1;
    int opt = 1;
    setsockopt(fd, SOL_SOCKET, SO_REUSEADDR, &opt, sizeof(opt));
    struct sockaddr_in addr;
    memset(&addr, 0, sizeof(addr));
    addr.sin_family = AF_INET;
    addr.sin_addr.s_addr = INADDR_ANY;
    addr.sin_port = htons((uint16_t)port);
    if (bind(fd, (struct sockaddr*)&addr, sizeof(addr)) < 0) {
        close(fd);
        return -1;
    }
    if (listen(fd, 128) < 0) {
        close(fd);
        return -1;
    }
    return fd;
}
int net_socket_accept(int server_fd) {
    if (server_fd < 0) return -1;
    struct sockaddr_in client_addr;
    socklen_t addrlen = sizeof(client_addr);
    return accept(server_fd, (struct sockaddr*)&client_addr, &addrlen);
}
int net_socket_connect(const char* host, int port) {
    if (!host) return -1;
    int fd = socket(AF_INET, SOCK_STREAM, 0);
    if (fd < 0) return -1;
    struct sockaddr_in addr;
    memset(&addr, 0, sizeof(addr));
    addr.sin_family = AF_INET;
    addr.sin_port = htons((uint16_t)port);
    if (inet_pton(AF_INET, host, &addr.sin_addr) <= 0) {
        close(fd);
        return -1;
    }
    if (connect(fd, (struct sockaddr*)&addr, sizeof(addr)) < 0) {
        close(fd);
        return -1;
    }
    return fd;
}
long long net_socket_send(int fd, const void* data, size_t len) {
    if (fd < 0 || !data) return -1;
    return (long long)send(fd, data, len, 0);
}
long long net_socket_recv(int fd, void* buf, size_t max_len) {
    if (fd < 0 || !buf) return -1;
    return (long long)recv(fd, buf, max_len, 0);
}
void net_socket_close(int fd) {
    if (fd >= 0) close(fd);
}
static int g_argc = 0;
static char** g_argv = NULL;
int os_args_count(void) {
    return g_argc;
}
Str os_arg(int idx) {
    Str s;
    s.data = NULL;
    s.len = 0;
    if (g_argv && idx >= 0 && idx < g_argc) {
        const char* arg = g_argv[idx];
        s.len = (int)strlen(arg);
        s.data = (char*)malloc((size_t)s.len + 1);
        memcpy(s.data, arg, (size_t)s.len + 1);
    }
    return s;
}
int dir_exists(const char* path) {
    if (!path) return 0;
    struct stat st;
    if (stat(path, &st) == 0 && S_ISDIR(st.st_mode)) {
        return 1;
    }
    return 0;
}
int file_copy(const char* src, const char* dst) {
    if (!src || !dst) return -1;
    FILE *in = fopen(src, "rb");
    if (!in) return -1;
    FILE *out = fopen(dst, "wb");
    if (!out) { fclose(in); return -1; }
    char buf[4096];
    size_t n;
    while ((n = fread(buf, 1, sizeof(buf), in)) > 0) {
        fwrite(buf, 1, n, out);
    }
    fclose(in);
    fclose(out);
    return 0;
}
int process_spawn(const char* cmd) {
    if (!cmd) return -1;
    return system(cmd);
}
"#;

/// Compile source string through Lexer -> Parser -> LinearChecker pipeline.
pub fn compile_source(source: &str) -> Result<Vec<ast::Expr>, String> {
    let tokens = lexer::Lexer::tokenize(source)?;
    let mut p = parser::Parser::new(tokens, source.to_string());
    let program = p.parse_program()?;
    let mut chk = checker::LinearChecker::new();
    chk.check_program(&program)?;
    Ok(program)
}

/// Full build: source -> object file -> linked executable in specified directory.
pub fn build_file_in_dir(filename: &str, out_dir: &Path) -> Result<PathBuf, String> {
    let source =
        fs::read_to_string(filename).map_err(|e| format!("Error reading '{}': {}", filename, e))?;

    let program = compile_source(&source)?;

    let mut cg = codegen::CodeGen::new("track_module");
    cg.compile_program(&program);

    let stem = Path::new(filename)
        .file_stem()
        .unwrap_or_default()
        .to_string_lossy()
        .to_string();

    let obj_path = out_dir.join(format!("{}.o", stem));
    let runtime_path = out_dir.join(format!("_track_runtime_{}.c", stem));
    let exe_path = out_dir.join(&stem);

    cg.write_object_file(&obj_path)?;
    fs::write(&runtime_path, RUNTIME_C_SOURCE)
        .map_err(|e| format!("Failed to write runtime helper: {}", e))?;

    let status = process::Command::new("cc")
        .arg(&obj_path)
        .arg(&runtime_path)
        .arg("-o")
        .arg(&exe_path)
        .arg("-lm")
        .arg("-no-pie")
        .status()
        .map_err(|e| format!("Linker error: {}", e))?;

    let _ = fs::remove_file(&obj_path);
    let _ = fs::remove_file(&runtime_path);

    if !status.success() {
        return Err(format!("Linker failed with exit code: {:?}", status.code()));
    }

    Ok(exe_path)
}

/// Full build placing executable in current directory.
pub fn build_file(filename: &str) -> Result<PathBuf, String> {
    build_file_in_dir(filename, Path::new("."))
}
