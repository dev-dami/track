export interface Milestone {
  version: string;
  title: string;
  status: 'completed' | 'in-progress' | 'planned' | 'milestone';
  date?: string;
  description: string;
  highlights: string[];
}

export const roadmapMilestones: Milestone[] = [
  {
    version: 'v0.1.6',
    title: 'Slices, Strings & Memory Primitives',
    status: 'completed',
    date: 'June 2026',
    description: 'First-class slices ([]T), owned heap string type (Str), POSIX TCP socket library, and standalone yard CLI.',
    highlights: ['First-class slices []T with .len()', 'Owned Str type with auto deallocation', 'POSIX socket networking (std/net)', 'Initial Yard build tool'],
  },
  {
    version: 'v0.2.0',
    title: 'UFCS, Lexical Lenses & CFG Merging',
    status: 'completed',
    date: 'June 2026',
    description: 'Lexical lens blocks (with ->), CFG state merging for if/else and loops, and Uniform Function Call Syntax.',
    highlights: ['Lexical lenses (with u -> user)', 'CFG branch state merging', 'Primitive copy inference', 'Pointer arithmetic & & borrows'],
  },
  {
    version: 'v0.3.0',
    title: 'Performance & Self-Hosting Readiness',
    status: 'completed',
    date: 'July 2026',
    description: 'Shared Arc<dyn TargetIsa> across parallel worker threads, eliminated per-file ISA re-initialization, and LSP caching.',
    highlights: ['Multi-threaded Cranelift backend', 'track-lsp released with AST caching', 'yard lint & yard clean commands', 'Extended std/fs, std/os, std/process'],
  },
  {
    version: 'v0.4.0',
    title: 'Advanced Pattern Matching & Anonymous Tuples',
    status: 'completed',
    date: 'July 2026',
    description: 'First-class anonymous tuples (T1, T2), tuple destructuring let (a, b) = expr, nested patterns, and match arm guards.',
    highlights: ['Anonymous tuple types (i64, Str, bool)', 'Tuple destructuring syntax', 'Nested variant pattern matching', 'Pattern guards (match x { n if n > 10 => ... })'],
  },
  {
    version: 'v0.5.0',
    title: 'Explicit Stack-Based Error Handling',
    status: 'completed',
    date: 'August 2026',
    description: 'Formalized the official zero-wrapper error philosophy: no Option/Result, status codes, tuple returns, out-params, and abort(msg).',
    highlights: ['No Result/Option or ? operator', '(T, i32) multi-value return conventions', 'abort(msg) fatal exit primitive', 'Predicate companions (str_is_int, env_exists)'],
  },
  {
    version: 'v0.6.0',
    title: 'Monomorphized Generics',
    status: 'completed',
    date: 'August 2026',
    description: 'Generic function templates (fn name<T, U>), type parameter inference at call sites, and specialization pass in src/mono.rs.',
    highlights: ['Generic functions with compile-time monomorphization', 'Zero vtable / zero dynamic dispatch overhead', 'Recursive nested generic call resolution', 'docs/generics.md guide'],
  },
  {
    version: 'v0.7.0',
    title: 'Self-Hosted Track Lexer & Yard Test Engine',
    status: 'completed',
    date: 'August 2026',
    description: '100% native tokenizer written in Track (compiler/src/lexer.trk) with 50+ union variants and native yard test orchestrator.',
    highlights: ['Ported logos lexer into 100% native Track', 'compiler/ is an isolated Yard package', 'yard test discovers & runs src/*_test.trk', 'Copy variant constructors fix in linear checker'],
  },
  {
    version: 'v0.8.0',
    title: 'Self-Hosted Parser, Linear Checker & C-Emitter',
    status: 'in-progress',
    date: 'Late 2026',
    description: 'Native Track parser (compiler/parser.trk), native linear ownership checker, and portable C backend emitter.',
    highlights: ['Native recursive descent parser in Track', 'Self-hosted linear ownership checker', 'Portable C code emitter', 'Compiler span error diagnostics'],
  },
  {
    version: 'v0.9.0',
    title: 'Full 3-Stage Compiler Self-Bootstrapping',
    status: 'milestone',
    date: 'Milestone Gate',
    description: 'Verification of 3-stage bootstrapping: Stage 0 (Rust) -> Stage 1 (Track C) -> Stage 2 (Track) -> Byte-for-byte identity gate.',
    highlights: ['Stage 0 builds Stage 1 compiler', 'Stage 1 compiles Stage 2 compiler', 'Stage 2 compiles Stage 3 compiler', 'cmp track_stage2 track_stage3 byte verification'],
  },
];
