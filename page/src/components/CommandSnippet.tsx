import React, { useState } from 'react';
import { Copy, Check } from 'lucide-react';
import { cn } from '../utils/cn';

interface CommandSnippetProps {
  command: string;
  prefix?: string;
  className?: string;
}

export const CommandSnippet: React.FC<CommandSnippetProps> = ({
  command,
  prefix = '$',
  className,
}) => {
  const [copied, setCopied] = useState(false);

  const handleCopy = () => {
    navigator.clipboard.writeText(command);
    setCopied(true);
    setTimeout(() => setCopied(false), 2000);
  };

  return (
    <div
      className={cn(
        'group flex items-center justify-between gap-3 rounded-lg border border-zinc-800/90 bg-zinc-950/80 px-3.5 py-2.5 font-mono text-xs shadow-inner backdrop-blur transition-all hover:border-zinc-700/80',
        className
      )}
    >
      <div className="flex items-center gap-2 overflow-x-auto select-all">
        <span className="select-none text-amber-500/80 font-bold">{prefix}</span>
        <span className="text-zinc-200 whitespace-nowrap">{command}</span>
      </div>
      <button
        onClick={handleCopy}
        className="flex items-center gap-1 shrink-0 rounded border border-zinc-800 bg-zinc-900/80 px-2 py-1 text-xs text-zinc-400 transition-colors hover:border-zinc-700 hover:text-zinc-100"
        title="Copy command"
      >
        {copied ? (
          <>
            <Check className="h-3.5 w-3.5 text-emerald-400" />
            <span className="text-emerald-400 text-[11px]">Copied</span>
          </>
        ) : (
          <>
            <Copy className="h-3.5 w-3.5" />
            <span className="text-[11px]">Copy</span>
          </>
        )}
      </button>
    </div>
  );
};
