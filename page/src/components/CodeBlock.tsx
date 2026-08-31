import React, { useState } from 'react';
import { Copy, Check, Terminal, FileCode, Play } from 'lucide-react';
import { highlightTrack, highlightBash } from '../utils/syntaxHighlight';
import { cn } from '../utils/cn';

interface CodeBlockProps {
  code: string;
  language?: 'track' | 'bash' | 'toml' | 'c' | 'rust' | 'zig' | 'plaintext';
  filename?: string;
  showLineNumbers?: boolean;
  highlightLines?: number[];
  onRun?: () => void;
  className?: string;
}

export const CodeBlock: React.FC<CodeBlockProps> = ({
  code,
  language = 'track',
  filename,
  showLineNumbers = true,
  highlightLines = [],
  onRun,
  className,
}) => {
  const [copied, setCopied] = useState(false);

  const handleCopy = () => {
    navigator.clipboard.writeText(code);
    setCopied(true);
    setTimeout(() => setCopied(false), 2000);
  };

  const getHighlightedHtml = () => {
    if (language === 'bash') {
      return highlightBash(code);
    }
    return highlightTrack(code);
  };

  const lines = code.trimEnd().split('\n');

  return (
    <div
      className={cn(
        'group relative my-4 overflow-hidden rounded-lg border border-zinc-800/80 bg-zinc-950/90 shadow-xl backdrop-blur-md transition-all hover:border-zinc-700/80',
        className
      )}
    >
      {/* Header bar if filename or language is present */}
      {(filename || language) && (
        <div className="flex items-center justify-between border-b border-zinc-800/80 bg-zinc-900/60 px-4 py-2 text-xs font-mono text-zinc-400">
          <div className="flex items-center gap-2">
            {language === 'bash' ? (
              <Terminal className="h-3.5 w-3.5 text-amber-400" />
            ) : (
              <FileCode className="h-3.5 w-3.5 text-cyan-400" />
            )}
            <span className="font-semibold text-zinc-200">{filename || language}</span>
          </div>

          <div className="flex items-center gap-2">
            {onRun && (
              <button
                onClick={onRun}
                className="flex items-center gap-1 rounded bg-amber-500/20 px-2 py-0.5 text-xs text-amber-300 transition-colors hover:bg-amber-500/30 font-mono"
                title="Run in Playground"
              >
                <Play className="h-3 w-3 fill-current" />
                <span>Run</span>
              </button>
            )}
            <button
              onClick={handleCopy}
              className="flex items-center gap-1 rounded px-2 py-0.5 text-xs text-zinc-400 transition-colors hover:bg-zinc-800 hover:text-zinc-200"
              title="Copy code"
            >
              {copied ? (
                <>
                  <Check className="h-3.5 w-3.5 text-emerald-400" />
                  <span className="text-emerald-400">Copied!</span>
                </>
              ) : (
                <>
                  <Copy className="h-3.5 w-3.5" />
                  <span>Copy</span>
                </>
              )}
            </button>
          </div>
        </div>
      )}

      {/* Code contents */}
      <div className="overflow-x-auto p-4 text-xs font-mono leading-relaxed">
        {showLineNumbers ? (
          <div className="table w-full border-collapse">
            {lines.map((line, idx) => {
              const lineNum = idx + 1;
              const isHighlighted = highlightLines.includes(lineNum);
              const highlightedLine =
                language === 'bash'
                  ? highlightBash(line)
                  : highlightTrack(line);

              return (
                <div
                  key={idx}
                  className={cn(
                    'table-row transition-colors',
                    isHighlighted ? 'bg-amber-500/10 -mx-4 px-4 block rounded' : ''
                  )}
                >
                  <span className="table-cell select-none pr-4 text-right text-zinc-600 w-8">
                    {lineNum}
                  </span>
                  <span
                    className="table-cell whitespace-pre text-zinc-200 font-mono"
                    dangerouslySetInnerHTML={{ __html: highlightedLine || '&nbsp;' }}
                  />
                </div>
              );
            })}
          </div>
        ) : (
          <pre
            className="text-zinc-200 font-mono whitespace-pre"
            dangerouslySetInnerHTML={{ __html: getHighlightedHtml() }}
          />
        )}
      </div>
    </div>
  );
};
