import React, { useState, useEffect } from 'react';
import { 
  Play, 
  Copy, 
  Check, 
  FileCode, 
  Code
} from 'lucide-react';
import { codeExamples } from '../data/examplesData';
import { cn } from '../utils/cn';

interface PlaygroundPageProps {
  initialCode?: string;
}

export const PlaygroundPage: React.FC<PlaygroundPageProps> = ({
  initialCode,
}) => {
  const [code, setCode] = useState<string>(
    initialCode || codeExamples[1].code
  );
  const [selectedExampleId, setSelectedExampleId] = useState<string>('linear-auto-free');
  const [backend, setBackend] = useState<'cranelift' | 'c_emitter'>('cranelift');
  const [optMode, setOptMode] = useState<'debug' | 'release'>('release');
  const [targetArch, setTargetArch] = useState<'x86_64' | 'aarch64'>('x86_64');
  const [activeTab, setActiveTab] = useState<'output' | 'clif' | 'tokens' | 'diagnostics'>('output');
  const [isCompiling, setIsCompiling] = useState(false);
  const [copied, setCopied] = useState(false);
  const [executionOutput, setExecutionOutput] = useState<string>('');

  // Handle hotkeys (Cmd+Enter or Ctrl+Enter to run)
  useEffect(() => {
    const handleKeyDown = (e: KeyboardEvent) => {
      if ((e.metaKey || e.ctrlKey) && e.key === 'Enter') {
        e.preventDefault();
        handleRun();
      }
    };
    window.addEventListener('keydown', handleKeyDown);
    return () => window.removeEventListener('keydown', handleKeyDown);
  }, [code, backend, optMode]);

  const handleRun = () => {
    setIsCompiling(true);
    setTimeout(() => {
      setIsCompiling(false);
      // Simulate real execution based on code content
      if (code.includes('Hello')) {
        setExecutionOutput("Hello, Track!\n\nProgram exited with code 0.");
      } else if (code.includes('make_buffer') || code.includes('consume_buffer') || code.includes('linear')) {
        setExecutionOutput("Buffer consumed. Automatically freed at exit.\n\nProgram exited with code 0.");
      } else if (code.includes('User') || code.includes('age')) {
        setExecutionOutput("31\n150\n\nProgram exited with code 0.");
      } else if (code.includes('identity') || code.includes('pair')) {
        setExecutionOutput("42\n\nProgram exited with code 0.");
      } else if (code.includes('load_data_file') || code.includes('config')) {
        setExecutionOutput("Error: Config not found\n\nProgram exited with code 0.");
      } else if (code.includes('net_socket')) {
        setExecutionOutput("Listening on 0.0.0.0:8080...\nPacket received!\n\nProgram exited with code 0.");
      } else if (code.includes('Packet::Data')) {
        setExecutionOutput("Verified data packet length:\n256\n\nProgram exited with code 0.");
      } else {
        setExecutionOutput("Finished execution in 0.42ms.\n\nProgram exited with code 0.");
      }
    }, 180);
  };

  const handleSelectExample = (id: string) => {
    setSelectedExampleId(id);
    const ex = codeExamples.find(e => e.id === id);
    if (ex) {
      setCode(ex.code);
      setExecutionOutput('');
    }
  };

  const handleCopy = () => {
    navigator.clipboard.writeText(code);
    setCopied(true);
    setTimeout(() => setCopied(false), 2000);
  };

  const handleKeyDownInTextarea = (e: React.KeyboardEvent<HTMLTextAreaElement>) => {
    // Handle tab indent
    if (e.key === 'Tab') {
      e.preventDefault();
      const target = e.target as HTMLTextAreaElement;
      const start = target.selectionStart;
      const end = target.selectionEnd;
      const val = target.value;
      setCode(val.substring(0, start) + '    ' + val.substring(end));
      setTimeout(() => {
        target.selectionStart = target.selectionEnd = start + 4;
      }, 0);
    }
  };

  const lineCount = code.split('\n').length;
  const lineNumbers = Array.from({ length: Math.max(lineCount, 18) }, (_, i) => i + 1);

  return (
    <div className="mx-auto max-w-6xl px-4 py-6 sm:px-6 space-y-4 text-left">
      {/* Top Toolbar (like play.rust-lang.org) */}
      <div className="flex flex-wrap items-center justify-between gap-3 border-b border-zinc-800 pb-3 font-mono text-xs">
        {/* Left Actions: Run, Check, Examples */}
        <div className="flex flex-wrap items-center gap-2">
          <button
            onClick={handleRun}
            disabled={isCompiling}
            className="flex items-center gap-1.5 rounded bg-amber-500 hover:bg-amber-400 text-zinc-950 font-bold px-3 py-1.5 transition-colors disabled:opacity-50"
            title="Run code (⌘+Enter)"
          >
            <Play className="h-3.5 w-3.5 fill-current" />
            <span>{isCompiling ? 'Compiling...' : 'Run'}</span>
          </button>

          <button
            onClick={handleRun}
            className="flex items-center gap-1.5 rounded border border-zinc-800 bg-zinc-900 hover:bg-zinc-800 text-zinc-300 px-3 py-1.5 transition-colors"
            title="Typecheck without building"
          >
            <Code className="h-3.5 w-3.5 text-zinc-400" />
            <span>Check</span>
          </button>

          {/* Example Selector */}
          <div className="flex items-center gap-1 pl-2 border-l border-zinc-800">
            <span className="text-zinc-500 text-[11px]">Example:</span>
            <select
              value={selectedExampleId}
              onChange={e => handleSelectExample(e.target.value)}
              className="bg-zinc-900 border border-zinc-800 rounded px-2 py-1 text-zinc-300 font-mono text-xs focus:outline-none focus:border-zinc-700"
            >
              {codeExamples.map(ex => (
                <option key={ex.id} value={ex.id}>
                  {ex.title}
                </option>
              ))}
            </select>
          </div>
        </div>

        {/* Right Settings: Target, Backend, Mode, Share */}
        <div className="flex flex-wrap items-center gap-2">
          {/* Target Architecture */}
          <select
            value={targetArch}
            onChange={e => setTargetArch(e.target.value as any)}
            className="bg-zinc-900 border border-zinc-800 rounded px-2 py-1 text-zinc-400 font-mono text-xs focus:outline-none"
          >
            <option value="x86_64">x86_64</option>
            <option value="aarch64">aarch64</option>
          </select>

          {/* Backend */}
          <select
            value={backend}
            onChange={e => setBackend(e.target.value as any)}
            className="bg-zinc-900 border border-zinc-800 rounded px-2 py-1 text-zinc-400 font-mono text-xs focus:outline-none"
          >
            <option value="cranelift">Backend: Cranelift</option>
            <option value="c_emitter">Backend: C Emitter (Stage 1)</option>
          </select>

          {/* Optimization Mode */}
          <select
            value={optMode}
            onChange={e => setOptMode(e.target.value as any)}
            className="bg-zinc-900 border border-zinc-800 rounded px-2 py-1 text-zinc-400 font-mono text-xs focus:outline-none"
          >
            <option value="release">Release (-O3)</option>
            <option value="debug">Debug</option>
          </select>

          {/* Copy button */}
          <button
            onClick={handleCopy}
            className="flex items-center gap-1 rounded border border-zinc-800 bg-zinc-900 hover:bg-zinc-800 text-zinc-400 hover:text-zinc-200 px-2 py-1 transition-colors"
            title="Copy code"
          >
            {copied ? <Check className="h-3 w-3 text-emerald-400" /> : <Copy className="h-3 w-3" />}
            <span>{copied ? 'Copied' : 'Copy'}</span>
          </button>
        </div>
      </div>

      {/* Editor & Console Split Layout */}
      <div className="grid grid-cols-1 gap-4">
        {/* Code Editor Container */}
        <div className="rounded border border-zinc-800 bg-[#0c0d11] overflow-hidden">
          <div className="flex items-center justify-between border-b border-zinc-800/80 bg-zinc-900/80 px-4 py-1.5 text-xs font-mono text-zinc-400">
            <div className="flex items-center gap-2">
              <FileCode className="h-3.5 w-3.5 text-zinc-400" />
              <span>src/main.trk</span>
            </div>
            <div className="text-[11px] text-zinc-500">
              Track v0.7.0 • Tab indents with 4 spaces • ⌘+Enter to Run
            </div>
          </div>

          <div className="flex min-h-[380px] text-xs font-mono">
            {/* Line numbers column */}
            <div className="select-none py-3 px-3 text-right text-zinc-600 bg-zinc-950/60 border-r border-zinc-800/60 w-10 shrink-0 font-mono text-xs leading-5">
              {lineNumbers.map(n => (
                <div key={n}>{n}</div>
              ))}
            </div>

            {/* Textarea Code Input */}
            <textarea
              value={code}
              onChange={e => setCode(e.target.value)}
              onKeyDown={handleKeyDownInTextarea}
              rows={Math.max(lineCount + 2, 18)}
              className="flex-1 bg-transparent text-zinc-200 p-3 leading-5 resize-none focus:outline-none font-mono text-xs selection:bg-amber-500/20 selection:text-amber-300"
              spellCheck={false}
            />
          </div>
        </div>

        {/* Bottom Output Console (Clean Unix Terminal Output) */}
        <div className="rounded border border-zinc-800 bg-[#0a0a0d] overflow-hidden">
          {/* Console Header Tabs */}
          <div className="flex items-center justify-between border-b border-zinc-800 bg-zinc-900/60 px-3 py-1 text-xs font-mono">
            <div className="flex items-center gap-1">
              <button
                onClick={() => setActiveTab('output')}
                className={cn(
                  'px-2.5 py-1 rounded transition-colors',
                  activeTab === 'output'
                    ? 'text-amber-400 bg-zinc-800 font-semibold'
                    : 'text-zinc-400 hover:text-zinc-200'
                )}
              >
                Standard Output
              </button>
              <button
                onClick={() => setActiveTab('clif')}
                className={cn(
                  'px-2.5 py-1 rounded transition-colors',
                  activeTab === 'clif'
                    ? 'text-amber-400 bg-zinc-800 font-semibold'
                    : 'text-zinc-400 hover:text-zinc-200'
                )}
              >
                Cranelift IR
              </button>
              <button
                onClick={() => setActiveTab('tokens')}
                className={cn(
                  'px-2.5 py-1 rounded transition-colors',
                  activeTab === 'tokens'
                    ? 'text-amber-400 bg-zinc-800 font-semibold'
                    : 'text-zinc-400 hover:text-zinc-200'
                )}
              >
                Tokens (v0.7 Lexer)
              </button>
              <button
                onClick={() => setActiveTab('diagnostics')}
                className={cn(
                  'px-2.5 py-1 rounded transition-colors',
                  activeTab === 'diagnostics'
                    ? 'text-amber-400 bg-zinc-800 font-semibold'
                    : 'text-zinc-400 hover:text-zinc-200'
                )}
              >
                Diagnostics Span
              </button>
            </div>

            <div className="text-[11px] text-zinc-500 flex items-center gap-2">
              <span>Exit Code: 0</span>
              <span>•</span>
              <span>Memory: 1.2 MB</span>
            </div>
          </div>

          {/* Console Content */}
          <div className="p-4 font-mono text-xs min-h-[160px] max-h-[260px] overflow-y-auto leading-relaxed text-left">
            {activeTab === 'output' && (
              <div className="space-y-2">
                <div className="text-zinc-500">
                  $ track build --target={targetArch} --backend={backend} src/main.trk
                </div>
                <div className="text-emerald-400">
                  Compiling 1 package (main) via {backend === 'cranelift' ? 'Cranelift JIT/AOT' : 'Portable C emitter'}...
                </div>
                <div className="text-zinc-400">
                  Running target/main:
                </div>
                <div className="text-zinc-100 font-semibold whitespace-pre pt-1">
                  {executionOutput || "Click 'Run' or press ⌘+Enter to execute program."}
                </div>
              </div>
            )}

            {activeTab === 'clif' && (
              <pre className="text-zinc-300 text-xs overflow-x-auto whitespace-pre">
{`function u0:0(i64) -> i64 fast {
    gv0 = vmctx
    stack_limit = gv0

block0(v0: i64):
    v1 = iconst.i64 42
    v2 = iadd v0, v1
    v3 = load.i64 notrap aligned v0+0
    jump block1(v2)

block1(v4: i64):
    return v4
}`}
              </pre>
            )}

            {activeTab === 'tokens' && (
              <pre className="text-zinc-300 text-xs overflow-x-auto whitespace-pre">
{`Token::KwImport, Token::StringLit("std/io"), Token::Semicolon
Token::KwFn, Token::Ident("main"), Token::ParenOpen, Token::ParenClose, Token::Arrow, Token::KwVoid
Token::BraceOpen
Token::Ident("io"), Token::ColonColon, Token::Ident("print"), Token::ParenOpen, Token::StringLit("Hello, Track!"), Token::ParenClose, Token::Semicolon
Token::BraceClose
Token::Eof`}
              </pre>
            )}

            {activeTab === 'diagnostics' && (
              <div className="space-y-2">
                <div className="text-zinc-500">
                  Example compiler ownership diagnostic format:
                </div>
                <pre className="text-zinc-300 text-xs bg-zinc-950 p-2.5 rounded border border-zinc-900 overflow-x-auto whitespace-pre text-rose-300">
{`error[TK201]: use of moved/spent variable \`b\`
  --> src/main.trk:18:17
   |
14 | let b = make_buffer(512);
15 | consume_buffer(b);
   |                - value moved into function parameter here
16 |
17 | // ERROR: attempted use of moved variable
18 | io::print_int(b.len);
   |               ^ value used after move`}
                </pre>
              </div>
            )}
          </div>
        </div>
      </div>
    </div>
  );
};
