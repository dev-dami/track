export interface LanguageComparisonRow {
  feature: string;
  track: string;
  rust: string;
  zig: string;
  c: string;
  go: string;
  advantage: 'track' | 'neutral';
}

export const languageComparisons: LanguageComparisonRow[] = [
  {
    feature: 'Memory Safety Model',
    track: 'Linear Ownership + Lexical Lenses (Compile-time verified)',
    rust: 'Borrow Checker with Lifetime Parameters',
    zig: 'Manual Allocators (Runtime safety checks)',
    c: 'Manual (Unchecked, undefined behavior)',
    go: 'Tracing Garbage Collector (Stop-the-world)',
    advantage: 'track',
  },
  {
    feature: 'Lifetime Annotations',
    track: 'None (Zero lifetime syntax or annotations)',
    rust: "Infectious syntax ('a, 'b: 'a, elision rules)",
    zig: 'N/A (No lifetime analysis)',
    c: 'N/A',
    go: 'N/A',
    advantage: 'track',
  },
  {
    feature: 'Runtime / GC Overhead',
    track: 'Zero Runtime, Zero GC, Deterministic latency',
    rust: 'Zero Runtime, Zero GC',
    zig: 'Zero Runtime, Zero GC',
    c: 'Zero Runtime, Zero GC',
    go: 'Heavy runtime (~2-10ms GC pauses)',
    advantage: 'neutral',
  },
  {
    feature: 'Error Handling Model',
    track: 'Explicit Stack Tuples (T, i32) & Predicates',
    rust: 'Monadic Result<T, E> & ? Operator',
    zig: 'Error Sets & try operator',
    c: 'Status codes / global errno',
    go: '(value, error) multi-return',
    advantage: 'track',
  },
  {
    feature: 'Compiler Backend',
    track: 'Cranelift Standalone (Sub-ms native compilation)',
    rust: 'LLVM (Large memory & build overhead)',
    zig: 'LLVM / C Emitter / Self-hosted x86',
    c: 'GCC / Clang (LLVM)',
    go: 'Custom Go Plan9 backend',
    advantage: 'track',
  },
  {
    feature: 'Package Manager',
    track: 'Yard (Built-in test orchestrator & builder)',
    rust: 'Cargo',
    zig: 'Zig Build system',
    c: 'CMake / Make / Meson (Fragmented)',
    go: 'Go Modules',
    advantage: 'neutral',
  },
  {
    feature: 'Pattern Matching & Guards',
    track: 'Full Nested Pattern Matching + Arm Guards',
    rust: 'Full Pattern Matching + Guards',
    zig: 'Basic switch expressions',
    c: 'Integer switch statements only',
    go: 'Type switches only',
    advantage: 'track',
  },
  {
    feature: 'Self-Hosting Status',
    track: 'v0.7.0 Native Lexer -> v0.9.0 3-Stage Byte Verify',
    rust: 'Self-hosted via LLVM',
    zig: 'Self-hosted',
    c: 'Self-hosted (GCC/Clang)',
    go: 'Self-hosted',
    advantage: 'neutral',
  },
];
