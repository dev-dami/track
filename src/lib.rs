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
#include <ctype.h>
#include <stdint.h>
#include <errno.h>
#include <math.h>

// Math Extensions
double math_abs(double x) { return fabs(x); }
double math_min(double a, double b) { return a < b ? a : b; }
double math_max(double a, double b) { return a > b ? a : b; }
double math_pow(double base, double exp) { return pow(base, exp); }
double math_sqrt(double x) { return sqrt(x); }
double math_floor(double x) { return floor(x); }
double math_ceil(double x) { return ceil(x); }
double math_round(double x) { return round(x); }

void print_err(const char* s) {
    if (s) {
        fprintf(stderr, "%s\n", s);
        fflush(stderr);
    }
}
void eprint(const char* s) {
    print_err(s);
}
long long str_contains(const char* s, const char* sub_str) {
    if (!s || !sub_str) return 0;
    return strstr(s, sub_str) != NULL ? 1 : 0;
}

static size_t g_allocated_bytes = 0;
static size_t g_max_memory_limit = 536870912; // 512MB default memory boundary limit

void sys_set_memory_limit(long long bytes) {
    if (bytes > 0) {
        g_max_memory_limit = (size_t)bytes;
    }
}

long long sys_get_memory_used(void) {
    return (long long)g_allocated_bytes;
}

void* alloc(size_t size) {
    if (size == 0) return NULL;
    if (g_allocated_bytes + size > g_max_memory_limit) {
        fprintf(stderr, "\nTrack Runtime Error: Process memory boundary limit exceeded! (Allocated: %zu bytes, Requested: %zu bytes, Limit: %zu bytes)\n", g_allocated_bytes, size, g_max_memory_limit);
        fflush(stderr);
        exit(137);
    }
    size_t total = size + sizeof(size_t);
    size_t* ptr = (size_t*)malloc(total);
    if (!ptr) {
        fprintf(stderr, "\nTrack Runtime Error: Out of memory allocation failure!\n");
        fflush(stderr);
        exit(137);
    }
    *ptr = size;
    g_allocated_bytes += size;
    return (void*)(ptr + 1);
}

void dealloc(void* ptr) {
    if (!ptr) return;
    size_t* raw = (size_t*)ptr - 1;
    size_t size = *raw;
    if (g_allocated_bytes >= size) {
        g_allocated_bytes -= size;
    } else {
        g_allocated_bytes = 0;
    }
    free(raw);
}

typedef struct { int* data; int len; int cap; } Vec;
Vec vec_init(int cap) {
    Vec v;
    size_t alloc_cap = cap > 0 ? (size_t)cap : 16;
    v.data = (int*)alloc(alloc_cap * sizeof(int));
    v.len = 0;
    v.cap = (int)alloc_cap;
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
        dealloc(v.data);
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
static int is_valid_string_pointer(const void* ptr) {
    if (!ptr || (uintptr_t)ptr < 0x10000) return 0;
    int pfd[2];
    if (pipe(pfd) < 0) return 0;
    ssize_t res = write(pfd[1], ptr, 1);
    close(pfd[0]);
    close(pfd[1]);
    return res == 1;
}

void print(long long val) {
    if (val >= 0x10000 && is_valid_string_pointer((const void*)(uintptr_t)val)) {
        const char* str = (const char*)(uintptr_t)val;
        int is_str = 1;
        int len = 0;
        while (len < 4096) {
            unsigned char c = (unsigned char)str[len];
            if (c == 0) break;
            if (c < 9 || (c > 13 && c < 32) || c > 126) {
                is_str = 0;
                break;
            }
            len++;
        }
        if (is_str && len > 0) {
            printf("%s\n", str);
            return;
        }
    }
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
        if (errno == EADDRINUSE) {
            fprintf(stderr, "Track Network Error: Port conflict! Port %d is already in use by another process.\n", port);
        } else {
            fprintf(stderr, "Track Network Error: Failed to bind TCP socket on port %d (errno: %d).\n", port, errno);
        }
        fflush(stderr);
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

// Char & Byte Operations
int char_is_digit(unsigned char c) { return isdigit(c) ? 1 : 0; }
int char_is_alpha(unsigned char c) { return isalpha(c) ? 1 : 0; }
int char_is_alphanumeric(unsigned char c) { return isalnum(c) ? 1 : 0; }
int char_is_space(unsigned char c) { return isspace(c) ? 1 : 0; }
unsigned char char_to_upper(unsigned char c) { return (unsigned char)toupper(c); }
unsigned char char_to_lower(unsigned char c) { return (unsigned char)tolower(c); }

// String Extensions
int str_starts_with(const char* s, const char* prefix) {
    if (!s || !prefix) return 0;
    size_t len_s = strlen(s);
    size_t len_p = strlen(prefix);
    if (len_p > len_s) return 0;
    return strncmp(s, prefix, len_p) == 0 ? 1 : 0;
}
int str_ends_with(const char* s, const char* suffix) {
    if (!s || !suffix) return 0;
    size_t len_s = strlen(s);
    size_t len_p = strlen(suffix);
    if (len_p > len_s) return 0;
    return strcmp(s + len_s - len_p, suffix) == 0 ? 1 : 0;
}
Str str_substr(const char* s, long long start, long long len) {
    Str res;
    res.data = NULL;
    res.len = 0;
    if (!s || start < 0) return res;
    size_t s_len = strlen(s);
    if ((size_t)start >= s_len) return res;
    size_t actual_len = (size_t)len;
    if ((size_t)start + actual_len > s_len) {
        actual_len = s_len - (size_t)start;
    }
    res.data = (char*)malloc(actual_len + 1);
    memcpy(res.data, s + start, actual_len);
    res.data[actual_len] = '\0';
    res.len = (int)actual_len;
    return res;
}
Str str_trim(const char* s) {
    Str res;
    res.data = NULL;
    res.len = 0;
    if (!s) return res;
    while (*s && isspace((unsigned char)*s)) s++;
    if (*s == 0) {
        res.data = (char*)calloc(1, 1);
        return res;
    }
    const char* end = s + strlen(s) - 1;
    while (end > s && isspace((unsigned char)*end)) end--;
    size_t len = (size_t)(end - s + 1);
    res.data = (char*)malloc(len + 1);
    memcpy(res.data, s, len);
    res.data[len] = '\0';
    res.len = (int)len;
    return res;
}
unsigned char str_char_at(const char* s, long long idx) {
    if (!s || idx < 0) return 0;
    size_t len = strlen(s);
    if ((size_t)idx >= len) return 0;
    return (unsigned char)s[idx];
}

// Memory & Vec Extensions
void* mem_realloc(void* ptr, size_t new_size) {
    if (!ptr) return alloc(new_size);
    if (new_size == 0) {
        dealloc(ptr);
        return NULL;
    }
    size_t* raw = (size_t*)ptr - 1;
    size_t old_size = *raw;
    if (g_allocated_bytes - old_size + new_size > g_max_memory_limit) {
        fprintf(stderr, "\nTrack Runtime Error: Process memory boundary limit exceeded! (Allocated: %zu bytes, Requested: %zu bytes, Limit: %zu bytes)\n", g_allocated_bytes, new_size, g_max_memory_limit);
        fflush(stderr);
        exit(137);
    }
    size_t total = new_size + sizeof(size_t);
    size_t* new_raw = (size_t*)realloc(raw, total);
    if (!new_raw) {
        fprintf(stderr, "\nTrack Runtime Error: Out of memory reallocation failure!\n");
        fflush(stderr);
        exit(137);
    }
    *new_raw = new_size;
    g_allocated_bytes = g_allocated_bytes - old_size + new_size;
    return (void*)(new_raw + 1);
}
void vec_reserve(Vec* v, int cap) {
    if (v && cap > v->cap) {
        v->data = (int*)mem_realloc(v->data, (size_t)cap * sizeof(int));
        v->cap = cap;
    }
}
void vec_clear(Vec* v) {
    if (v) v->len = 0;
}
int vec_len(const Vec* v) {
    return v ? v->len : 0;
}

// Path Extensions
Str path_basename(const char* path) {
    Str s;
    s.data = NULL;
    s.len = 0;
    if (!path) return s;
    const char* last_slash = strrchr(path, '/');
    const char* name = last_slash ? last_slash + 1 : path;
    s.len = (int)strlen(name);
    s.data = (char*)malloc((size_t)s.len + 1);
    memcpy(s.data, name, (size_t)s.len + 1);
    return s;
}
Str path_ext(const char* path) {
    Str s;
    s.data = NULL;
    s.len = 0;
    if (!path) return s;
    const char* dot = strrchr(path, '.');
    if (!dot || dot == path) return s;
    s.len = (int)strlen(dot + 1);
    s.data = (char*)malloc((size_t)s.len + 1);
    memcpy(s.data, dot + 1, (size_t)s.len + 1);
    return s;
}
Str path_join(const char* a, const char* b) {
    Str s;
    s.data = NULL;
    s.len = 0;
    if (!a && !b) return s;
    if (!a) {
        s.len = (int)strlen(b);
        s.data = (char*)malloc((size_t)s.len + 1);
        memcpy(s.data, b, (size_t)s.len + 1);
        return s;
    }
    if (!b) {
        s.len = (int)strlen(a);
        s.data = (char*)malloc((size_t)s.len + 1);
        memcpy(s.data, a, (size_t)s.len + 1);
        return s;
    }
    size_t len_a = strlen(a);
    size_t len_b = strlen(b);
    int needs_slash = (len_a > 0 && a[len_a - 1] != '/' && (len_b == 0 || b[0] != '/')) ? 1 : 0;
    size_t total = len_a + (size_t)needs_slash + len_b;
    s.data = (char*)malloc(total + 1);
    memcpy(s.data, a, len_a);
    if (needs_slash) {
        s.data[len_a] = '/';
        memcpy(s.data + len_a + 1, b, len_b);
    } else {
        memcpy(s.data + len_a, b, len_b);
    }
    s.data[total] = '\0';
    s.len = (int)total;
    return s;
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
