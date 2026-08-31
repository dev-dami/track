import React, { useState } from 'react';
import { Boxes } from 'lucide-react';
import { codeExamples } from '../data/examplesData';
import { CodeBlock } from '../components/CodeBlock';
import { Badge } from '../components/Badge';
import { PageTab } from '../components/Navbar';
import { cn } from '../utils/cn';

interface ExamplesPageProps {
  onSelectTab: (tab: PageTab) => void;
  onOpenPlaygroundWithCode?: (code: string) => void;
}

export const ExamplesPage: React.FC<ExamplesPageProps> = ({
  onSelectTab,
  onOpenPlaygroundWithCode,
}) => {
  const [activeCategory, setActiveCategory] = useState<string>('all');
  const categories = ['all', 'Ownership', 'Lenses', 'Generics', 'Networking', 'Patterns', 'Toolchain'];

  const filteredExamples = codeExamples.filter(
    ex => activeCategory === 'all' || ex.category === activeCategory
  );

  return (
    <div className="mx-auto max-w-7xl px-4 py-8 sm:px-6 lg:px-8 space-y-8">
      {/* Header */}
      <div className="border-b border-zinc-800/80 pb-6">
        <div className="inline-flex items-center gap-2 text-xs font-mono text-amber-400 mb-2">
          <Boxes className="h-4 w-4" />
          <span>Code Examples</span>
        </div>
        <h1 className="text-3xl font-extrabold tracking-tight font-sans text-zinc-100">
          Track Idioms & Reference Programs
        </h1>
        <p className="text-sm text-zinc-400 font-sans mt-2 max-w-3xl leading-relaxed">
          Tested, working programs from the Track standard test suite and self-hosted compiler repository.
        </p>
      </div>

      {/* Category Filter Pills */}
      <div className="flex items-center gap-2 overflow-x-auto pb-1">
        {categories.map(cat => (
          <button
            key={cat}
            onClick={() => setActiveCategory(cat)}
            className={cn(
              'rounded-lg px-3 py-1.5 text-xs font-mono capitalize transition-all shrink-0',
              activeCategory === cat
                ? 'bg-amber-500/20 text-amber-300 font-bold border border-amber-500/40'
                : 'text-zinc-400 hover:bg-zinc-900 hover:text-zinc-200 border border-transparent'
            )}
          >
            {cat}
          </button>
        ))}
      </div>

      {/* Examples Grid */}
      <div className="grid grid-cols-1 lg:grid-cols-2 gap-6">
        {filteredExamples.map(example => (
          <div
            key={example.id}
            className="rounded-xl border border-zinc-800/80 bg-zinc-950/80 p-5 shadow-lg backdrop-blur flex flex-col justify-between space-y-4"
          >
            <div className="space-y-3">
              <div className="flex items-center justify-between">
                <Badge variant="amber" size="sm">
                  {example.category}
                </Badge>
                <span className="font-mono text-[11px] text-zinc-500">{example.filename}</span>
              </div>

              <h3 className="text-lg font-bold font-sans text-zinc-100">
                {example.title}
              </h3>

              <p className="text-xs text-zinc-400 font-sans leading-relaxed">
                {example.description}
              </p>

              <CodeBlock
                code={example.code}
                language="track"
                filename={example.filename}
                onRun={() => {
                  if (onOpenPlaygroundWithCode) {
                    onOpenPlaygroundWithCode(example.code);
                  } else {
                    onSelectTab('playground');
                  }
                }}
              />
            </div>

            {example.expectedOutput && (
              <div className="rounded-lg border border-zinc-850 bg-zinc-900/50 p-2.5 text-[11px] font-mono text-zinc-300">
                <span className="text-zinc-500">$ yard run</span> → {example.expectedOutput}
              </div>
            )}
          </div>
        ))}
      </div>
    </div>
  );
};
