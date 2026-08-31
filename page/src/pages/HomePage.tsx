import React, { useState } from 'react';
import { 
  ArrowRight, 
  Terminal, 
  BookOpen, 
  Code2, 
  Cpu, 
  Milestone
} from 'lucide-react';
import { CodeBlock } from '../components/CodeBlock';
import { CommandSnippet } from '../components/CommandSnippet';
import { languageComparisons } from '../data/comparisonData';
import { codeExamples } from '../data/examplesData';
import { PageTab } from '../components/Navbar';
import { cn } from '../utils/cn';

interface HomePageProps {
  onSelectTab: (tab: PageTab) => void;
  onOpenPlaygroundWithCode?: (code: string) => void;
}

export const HomePage: React.FC<HomePageProps> = ({
  onSelectTab,
  onOpenPlaygroundWithCode,
}) => {
  const [selectedExampleId, setSelectedExampleId] = useState<string>('hello');
  const activeExample = codeExamples.find(e => e.id === selectedExampleId) || codeExamples[0];

  return (
    <div className="mx-auto max-w-5xl px-4 py-12 sm:px-6 space-y-16">
      {/* Hero Section */}
      <section className="space-y-6 pt-4 text-left">
        <div className="space-y-2">
          <div className="text-xs font-mono text-amber-500 font-semibold tracking-wide uppercase">
            Systems Programming Language
          </div>
          <h1 className="text-3xl sm:text-5xl font-extrabold font-sans text-zinc-100 tracking-tight leading-tight">
            Deterministic Memory Safety <br className="hidden sm:inline" />
            without Lifetime Annotations.
          </h1>
        </div>

        <p className="text-base text-zinc-300 font-sans leading-relaxed max-w-3xl">
          Track is a low-level systems programming language designed for real-time predictability, zero-cost abstractions, and compile-time memory safety without a garbage collector or complex lifetime parameters.
        </p>

        {/* Install command */}
        <div className="pt-2 max-w-2xl">
          <div className="text-xs font-mono text-zinc-400 mb-1.5 flex items-center gap-1.5">
            <Terminal className="h-3.5 w-3.5 text-zinc-500" />
            <span>Install standalone compiler & toolchain:</span>
          </div>
          <CommandSnippet command="curl -fsSL https://raw.githubusercontent.com/dev-dami/track/main/scripts/install.sh | bash" />
        </div>

        {/* Quick Nav Links */}
        <div className="flex flex-wrap items-center gap-4 pt-2 font-mono text-xs">
          <button
            onClick={() => onSelectTab('docs')}
            className="flex items-center gap-1.5 bg-amber-500 text-zinc-950 px-3.5 py-2 rounded font-bold hover:bg-amber-400 transition-colors"
          >
            <BookOpen className="h-3.5 w-3.5" />
            <span>Documentation</span>
            <ArrowRight className="h-3.5 w-3.5" />
          </button>
          <button
            onClick={() => onSelectTab('stdlib')}
            className="flex items-center gap-1.5 border border-zinc-800 bg-zinc-900/80 text-zinc-200 px-3.5 py-2 rounded font-medium hover:border-zinc-700 transition-colors"
          >
            <Code2 className="h-3.5 w-3.5 text-cyan-400" />
            <span>Standard Library</span>
          </button>
          <button
            onClick={() => onSelectTab('playground')}
            className="flex items-center gap-1.5 border border-zinc-800 bg-zinc-900/80 text-zinc-200 px-3.5 py-2 rounded font-medium hover:border-zinc-700 transition-colors"
          >
            <Cpu className="h-3.5 w-3.5 text-amber-400" />
            <span>Interactive Playground</span>
          </button>
          <button
            onClick={() => onSelectTab('roadmap')}
            className="flex items-center gap-1.5 text-zinc-400 hover:text-zinc-200 transition-colors px-2 py-2"
          >
            <Milestone className="h-3.5 w-3.5 text-purple-400" />
            <span>v0.9.0 Bootstrapping Roadmap</span>
          </button>
        </div>
      </section>

      {/* Code Showcase by Example */}
      <section className="space-y-4 pt-4 border-t border-zinc-800/60">
        <div className="flex flex-col sm:flex-row sm:items-center justify-between gap-2">
          <h2 className="text-xl font-bold font-sans text-zinc-100">
            Track by Example
          </h2>
          <div className="flex items-center gap-1 overflow-x-auto pb-1 font-mono text-xs">
            {codeExamples.slice(0, 6).map(ex => (
              <button
                key={ex.id}
                onClick={() => setSelectedExampleId(ex.id)}
                className={cn(
                  'px-2.5 py-1 rounded transition-colors whitespace-nowrap',
                  selectedExampleId === ex.id
                    ? 'bg-zinc-800 text-amber-400 font-semibold'
                    : 'text-zinc-400 hover:text-zinc-200 hover:bg-zinc-900'
                )}
              >
                {ex.title.split(' ')[0]}
              </button>
            ))}
          </div>
        </div>

        <div className="rounded border border-zinc-800 bg-[#0d0e12] overflow-hidden">
          <div className="flex items-center justify-between border-b border-zinc-800 bg-zinc-900/70 px-4 py-2 text-xs font-mono text-zinc-400">
            <span>{activeExample.filename}</span>
            <span className="text-zinc-500">{activeExample.description}</span>
          </div>
          <div className="p-2">
            <CodeBlock
              code={activeExample.code}
              language="track"
              showLineNumbers={true}
              onRun={() => {
                if (onOpenPlaygroundWithCode) {
                  onOpenPlaygroundWithCode(activeExample.code);
                } else {
                  onSelectTab('playground');
                }
              }}
            />
          </div>
        </div>
      </section>

      {/* Core Language Design Pillars */}
      <section className="space-y-6 pt-4 border-t border-zinc-800/60">
        <h2 className="text-xl font-bold font-sans text-zinc-100">
          Core Language Architecture
        </h2>

        <div className="grid grid-cols-1 md:grid-cols-3 gap-6 text-sm">
          <div className="space-y-2 border-l-2 border-amber-500/80 pl-4">
            <h3 className="font-mono font-bold text-zinc-100">
              Deterministic Linear Ownership
            </h3>
            <p className="text-xs text-zinc-400 leading-relaxed font-sans">
              Resource lifecycles are statically verified at compile time. Variables transition strictly through <code className="text-amber-300 font-mono">Active</code>, <code className="text-cyan-300 font-mono">Borrowed</code>, <code className="text-amber-400 font-mono">Locked</code>, and <code className="text-purple-400 font-mono">Spent</code>, with automatic destruction at spend points.
            </p>
          </div>

          <div className="space-y-2 border-l-2 border-cyan-500/80 pl-4">
            <h3 className="font-mono font-bold text-zinc-100">
              Lexical Lenses (<code className="text-cyan-400 font-mono">with</code>)
            </h3>
            <p className="text-xs text-zinc-400 leading-relaxed font-sans">
              Non-escaping lexical views replace lifetime parameters (<code className="text-zinc-300 font-mono">'a</code>). Mutation is strictly bounded to the <code className="text-cyan-300 font-mono">with</code> block, freezing the outer variable and restoring ownership on scope exit.
            </p>
          </div>

          <div className="space-y-2 border-l-2 border-emerald-500/80 pl-4">
            <h3 className="font-mono font-bold text-zinc-100">
              Standalone Cranelift Backend
            </h3>
            <p className="text-xs text-zinc-400 leading-relaxed font-sans">
              Fast, standalone native code generation for x86_64 and aarch64 without multi-gigabyte LLVM dynamic libraries. Multi-threaded worker ISA sharing ensures instant check and build cycles.
            </p>
          </div>
        </div>
      </section>

      {/* Systems Language Comparison Table */}
      <section className="space-y-4 pt-4 border-t border-zinc-800/60">
        <h2 className="text-xl font-bold font-sans text-zinc-100">
          Comparison with Other Systems Languages
        </h2>
        <p className="text-xs text-zinc-400 font-sans">
          How Track compares in memory safety guarantees, cognitive overhead, and backend architecture.
        </p>

        <div className="overflow-x-auto rounded border border-zinc-800 bg-[#0d0e12]">
          <table className="w-full text-left font-sans text-xs systems-table">
            <thead>
              <tr>
                <th>Feature</th>
                <th className="text-amber-400 bg-zinc-900/90 font-bold">Track</th>
                <th>Rust</th>
                <th>Zig</th>
                <th>C</th>
                <th>Go</th>
              </tr>
            </thead>
            <tbody>
              {languageComparisons.map((row, idx) => (
                <tr key={idx}>
                  <td className="font-mono font-medium text-zinc-300">{row.feature}</td>
                  <td className="font-mono text-amber-300 bg-zinc-900/40 font-semibold">{row.track}</td>
                  <td className="text-zinc-400">{row.rust}</td>
                  <td className="text-zinc-400">{row.zig}</td>
                  <td className="text-zinc-400">{row.c}</td>
                  <td className="text-zinc-400">{row.go}</td>
                </tr>
              ))}
            </tbody>
          </table>
        </div>
      </section>

      {/* Toolchain & Self-Hosting Status */}
      <section className="space-y-4 pt-4 border-t border-zinc-800/60">
        <h2 className="text-xl font-bold font-sans text-zinc-100">
          Toolchain & Self-Hosting Roadmap
        </h2>
        <div className="grid grid-cols-1 md:grid-cols-2 gap-4 text-xs font-mono">
          <div className="rounded border border-zinc-800 bg-[#0d0e12] p-4 space-y-2">
            <div className="text-amber-400 font-bold">Package Manager: Yard</div>
            <p className="text-zinc-400 font-sans">
              <code className="text-zinc-200">yard</code> manages dependencies, builds native packages, runs lint passes, and executes native Track test suites (<code className="text-zinc-200">yard test</code>).
            </p>
          </div>
          <div className="rounded border border-zinc-800 bg-[#0d0e12] p-4 space-y-2">
            <div className="text-cyan-400 font-bold">Language Server: track-lsp</div>
            <p className="text-zinc-400 font-sans">
              Provides real-time AST diagnostics, autocompletion, and type-checks <code className="text-zinc-200">```track</code> code blocks embedded in markdown files.
            </p>
          </div>
        </div>
      </section>
    </div>
  );
};
