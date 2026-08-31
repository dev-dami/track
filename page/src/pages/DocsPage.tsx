import React, { useState } from 'react';
import { 
  Search, 
  ArrowLeft, 
  ArrowRight
} from 'lucide-react';
import { docsChapters } from '../data/docsData';
import { CodeBlock } from '../components/CodeBlock';
import { PageTab } from '../components/Navbar';
import { cn } from '../utils/cn';

interface DocsPageProps {
  onSelectTab: (tab: PageTab) => void;
  onOpenPlaygroundWithCode?: (code: string) => void;
  initialChapterId?: string;
}

export const DocsPage: React.FC<DocsPageProps> = ({
  onSelectTab,
  onOpenPlaygroundWithCode,
  initialChapterId = 'intro',
}) => {
  const [selectedChapterId, setSelectedChapterId] = useState<string>(initialChapterId);
  const [searchQuery, setSearchQuery] = useState<string>('');

  const currentChapter =
    docsChapters.find(c => c.id === selectedChapterId) || docsChapters[0];

  const currentIndex = docsChapters.findIndex(c => c.id === currentChapter.id);
  const prevChapter = currentIndex > 0 ? docsChapters[currentIndex - 1] : null;
  const nextChapter =
    currentIndex < docsChapters.length - 1 ? docsChapters[currentIndex + 1] : null;

  // Filter chapters by search query
  const filteredChapters = docsChapters.filter(
    c =>
      c.title.toLowerCase().includes(searchQuery.toLowerCase()) ||
      c.subtitle.toLowerCase().includes(searchQuery.toLowerCase()) ||
      c.content.toLowerCase().includes(searchQuery.toLowerCase())
  );

  // Group chapters by category
  const categories = Array.from(new Set(docsChapters.map(c => c.category)));

  return (
    <div className="mx-auto max-w-6xl px-4 py-8 sm:px-6">
      <div className="grid grid-cols-1 md:grid-cols-12 gap-8 items-start">
        {/* Left Sidebar (mdBook / Rust-style documentation sidebar) */}
        <aside className="md:col-span-4 lg:col-span-3 space-y-6 md:sticky md:top-20">
          {/* Docs Search */}
          <div className="relative">
            <Search className="absolute left-2.5 top-2.5 h-3.5 w-3.5 text-zinc-500" />
            <input
              type="text"
              value={searchQuery}
              onChange={e => setSearchQuery(e.target.value)}
              placeholder="Search docs..."
              className="w-full rounded border border-zinc-800 bg-[#0d0e12] py-1.5 pl-8 pr-3 text-xs font-mono text-zinc-200 placeholder-zinc-500 focus:border-amber-500 focus:outline-none"
            />
          </div>

          {/* Chapter Links */}
          <div className="space-y-4 max-h-[75vh] overflow-y-auto pr-1">
            {categories.map(category => {
              const chaptersInCategory = filteredChapters.filter(c => c.category === category);
              if (chaptersInCategory.length === 0) return null;

              return (
                <div key={category} className="space-y-1">
                  <div className="font-mono text-[11px] font-bold uppercase tracking-wider text-zinc-500 px-2 py-1">
                    {category}
                  </div>
                  <div className="space-y-0.5 border-l border-zinc-800 ml-2 pl-2">
                    {chaptersInCategory.map(chapter => {
                      const isActive = chapter.id === currentChapter.id;
                      return (
                        <button
                          key={chapter.id}
                          onClick={() => {
                            setSelectedChapterId(chapter.id);
                            window.scrollTo({ top: 0, behavior: 'smooth' });
                          }}
                          className={cn(
                            'block w-full text-left py-1 px-2 text-xs font-sans rounded transition-colors',
                            isActive
                              ? 'text-amber-400 font-semibold bg-zinc-900'
                              : 'text-zinc-400 hover:text-zinc-200 hover:bg-zinc-900/50'
                          )}
                        >
                          {chapter.title}
                        </button>
                      );
                    })}
                  </div>
                </div>
              );
            })}
          </div>
        </aside>

        {/* Main Document Content */}
        <main className="md:col-span-8 lg:col-span-9 space-y-8 text-left">
          {/* Header */}
          <div className="border-b border-zinc-800 pb-4">
            <div className="text-[11px] font-mono text-zinc-500 mb-1">
              Docs › {currentChapter.category}
            </div>
            <h1 className="text-2xl sm:text-3xl font-bold font-sans text-zinc-100">
              {currentChapter.title}
            </h1>
            <p className="text-xs sm:text-sm text-zinc-400 font-sans mt-1 leading-relaxed">
              {currentChapter.subtitle}
            </p>
          </div>

          {/* Document Body */}
          <div className="space-y-6 text-sm text-zinc-300 font-sans leading-relaxed">
            {currentChapter.content.split('\n\n').map((paragraph, idx) => {
              const trimmed = paragraph.trim();
              if (!trimmed) return null;

              if (trimmed.startsWith('### ')) {
                return (
                  <h3 key={idx} className="text-base font-bold font-sans text-zinc-100 mt-6 mb-2 tracking-tight">
                    {trimmed.replace('### ', '')}
                  </h3>
                );
              }
              if (trimmed.startsWith('## ')) {
                return (
                  <h2 key={idx} className="text-lg font-bold font-sans text-zinc-100 mt-8 mb-3 border-b border-zinc-800/80 pb-1">
                    {trimmed.replace('## ', '')}
                  </h2>
                );
              }

              if (trimmed.startsWith('> ')) {
                return (
                  <blockquote key={idx} className="border-l-2 border-amber-500/80 pl-4 my-4 font-mono text-xs text-zinc-300 italic">
                    {trimmed.replace('> ', '').replace(/\*/g, '')}
                  </blockquote>
                );
              }

              if (trimmed.startsWith('- ') || trimmed.startsWith('1. ')) {
                const items = trimmed.split('\n');
                return (
                  <ul key={idx} className="space-y-1.5 my-3 pl-5 list-disc marker:text-zinc-500">
                    {items.map((item, itemIdx) => (
                      <li key={itemIdx} className="text-zinc-300 text-xs sm:text-sm">
                        {item.replace(/^[-*]|\d+\.\s*/, '')}
                      </li>
                    ))}
                  </ul>
                );
              }

              if (trimmed.startsWith('```')) {
                const lines = trimmed.split('\n');
                const lang = lines[0].replace('```', '') as any;
                const code = lines.slice(1, -1).join('\n');
                return (
                  <CodeBlock
                    key={idx}
                    code={code}
                    language={lang || 'track'}
                    onRun={() => {
                      if (onOpenPlaygroundWithCode) {
                        onOpenPlaygroundWithCode(code);
                      } else {
                        onSelectTab('playground');
                      }
                    }}
                  />
                );
              }

              return (
                <p key={idx} className="text-zinc-300 text-xs sm:text-sm leading-relaxed font-sans">
                  {trimmed}
                </p>
              );
            })}
          </div>

          {/* Embedded Code Snippets */}
          {currentChapter.codeSnippets && currentChapter.codeSnippets.length > 0 && (
            <div className="space-y-3 pt-4">
              <div className="text-xs font-mono font-bold text-zinc-400">
                Code Reference:
              </div>
              {currentChapter.codeSnippets.map((snippet, idx) => (
                <CodeBlock
                  key={idx}
                  code={snippet.code}
                  language={snippet.language}
                  filename={snippet.title}
                  onRun={() => {
                    if (onOpenPlaygroundWithCode) {
                      onOpenPlaygroundWithCode(snippet.code);
                    } else {
                      onSelectTab('playground');
                    }
                  }}
                />
              ))}
            </div>
          )}

          {/* Navigation Footer */}
          <div className="flex items-center justify-between border-t border-zinc-800 pt-6 mt-10 text-xs font-mono">
            {prevChapter ? (
              <button
                onClick={() => {
                  setSelectedChapterId(prevChapter.id);
                  window.scrollTo({ top: 0, behavior: 'smooth' });
                }}
                className="flex items-center gap-1.5 text-zinc-400 hover:text-amber-400 transition-colors"
              >
                <ArrowLeft className="h-3.5 w-3.5" />
                <span>{prevChapter.title}</span>
              </button>
            ) : <div />}

            {nextChapter && (
              <button
                onClick={() => {
                  setSelectedChapterId(nextChapter.id);
                  window.scrollTo({ top: 0, behavior: 'smooth' });
                }}
                className="flex items-center gap-1.5 text-zinc-400 hover:text-amber-400 transition-colors"
              >
                <span>{nextChapter.title}</span>
                <ArrowRight className="h-3.5 w-3.5" />
              </button>
            )}
          </div>
        </main>
      </div>
    </div>
  );
};
