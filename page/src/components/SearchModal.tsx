import React, { useState, useEffect, useRef } from 'react';
import { Search, FileText, BookOpen, Code, Layers, Sparkles, X, ArrowRight } from 'lucide-react';
import { cn } from '../utils/cn';

export interface SearchItem {
  id: string;
  title: string;
  category: 'docs' | 'stdlib' | 'blog' | 'example' | 'spec';
  section?: string;
  snippet: string;
  path: string;
}

interface SearchModalProps {
  isOpen: boolean;
  onClose: () => void;
  onSelect: (item: SearchItem) => void;
  searchItems: SearchItem[];
}

export const SearchModal: React.FC<SearchModalProps> = ({
  isOpen,
  onClose,
  onSelect,
  searchItems,
}) => {
  const [query, setQuery] = useState('');
  const [selectedIndex, setSelectedIndex] = useState(0);
  const [activeCategory, setActiveCategory] = useState<string>('all');
  const inputRef = useRef<HTMLInputElement>(null);

  useEffect(() => {
    if (isOpen) {
      setTimeout(() => inputRef.current?.focus(), 50);
      setQuery('');
      setSelectedIndex(0);
    }
  }, [isOpen]);

  // Global keybindings (Cmd+K / Ctrl+K and Esc)
  useEffect(() => {
    const handleKeyDown = (e: KeyboardEvent) => {
      if ((e.metaKey || e.ctrlKey) && e.key === 'k') {
        e.preventDefault();
        if (isOpen) onClose();
      }
      if (e.key === 'Escape' && isOpen) {
        onClose();
      }
    };
    window.addEventListener('keydown', handleKeyDown);
    return () => window.removeEventListener('keydown', handleKeyDown);
  }, [isOpen, onClose]);

  const filteredItems = searchItems.filter(item => {
    const matchesCategory = activeCategory === 'all' || item.category === activeCategory;
    const matchesQuery =
      item.title.toLowerCase().includes(query.toLowerCase()) ||
      item.snippet.toLowerCase().includes(query.toLowerCase()) ||
      (item.section && item.section.toLowerCase().includes(query.toLowerCase()));
    return matchesCategory && matchesQuery;
  });

  const handleKeyDown = (e: React.KeyboardEvent) => {
    if (e.key === 'ArrowDown') {
      e.preventDefault();
      setSelectedIndex(prev => (prev < filteredItems.length - 1 ? prev + 1 : prev));
    } else if (e.key === 'ArrowUp') {
      e.preventDefault();
      setSelectedIndex(prev => (prev > 0 ? prev - 1 : prev));
    } else if (e.key === 'Enter' && filteredItems[selectedIndex]) {
      e.preventDefault();
      onSelect(filteredItems[selectedIndex]);
      onClose();
    }
  };

  if (!isOpen) return null;

  const categoryIcons = {
    docs: BookOpen,
    stdlib: Code,
    blog: FileText,
    example: Layers,
    spec: Sparkles,
  };

  return (
    <div className="fixed inset-0 z-50 flex items-start justify-center p-4 pt-16 md:pt-24 bg-black/70 backdrop-blur-md animate-in fade-in duration-150">
      <div
        className="w-full max-w-2xl overflow-hidden rounded-xl border border-zinc-700/80 bg-zinc-950 shadow-2xl ring-1 ring-white/10"
        onClick={e => e.stopPropagation()}
      >
        {/* Search Input Bar */}
        <div className="relative flex items-center border-b border-zinc-800/80 px-4 py-3 bg-zinc-900/60">
          <Search className="h-4 w-4 text-zinc-400 shrink-0 mr-3" />
          <input
            ref={inputRef}
            type="text"
            value={query}
            onChange={e => {
              setQuery(e.target.value);
              setSelectedIndex(0);
            }}
            onKeyDown={handleKeyDown}
            placeholder="Search Track docs, stdlib, specs, memory model..."
            className="w-full bg-transparent font-sans text-sm text-zinc-100 placeholder-zinc-500 focus:outline-none"
          />
          {query && (
            <button
              onClick={() => setQuery('')}
              className="text-zinc-500 hover:text-zinc-300 p-1 mr-2"
            >
              <X className="h-4 w-4" />
            </button>
          )}
          <kbd className="hidden sm:inline-block rounded border border-zinc-700 bg-zinc-800/80 px-1.5 py-0.5 text-[10px] font-mono text-zinc-400">
            ESC
          </kbd>
        </div>

        {/* Category filter pills */}
        <div className="flex items-center gap-1.5 border-b border-zinc-800/60 bg-zinc-900/40 px-4 py-2 text-xs font-mono">
          {['all', 'docs', 'stdlib', 'blog', 'example', 'spec'].map(cat => (
            <button
              key={cat}
              onClick={() => {
                setActiveCategory(cat);
                setSelectedIndex(0);
              }}
              className={cn(
                'rounded px-2 py-0.5 capitalize transition-colors',
                activeCategory === cat
                  ? 'bg-amber-500/20 text-amber-300 font-semibold'
                  : 'text-zinc-400 hover:bg-zinc-800/60 hover:text-zinc-200'
              )}
            >
              {cat}
            </button>
          ))}
        </div>

        {/* Results List */}
        <div className="max-h-96 overflow-y-auto p-2 divide-y divide-zinc-900/60">
          {filteredItems.length === 0 ? (
            <div className="py-12 text-center text-xs text-zinc-500 font-mono">
              No results found for "{query}". Try searching for <span className="text-amber-400">"lens"</span>, <span className="text-cyan-400">"yard"</span>, or <span className="text-emerald-400">"net_socket"</span>.
            </div>
          ) : (
            filteredItems.map((item, idx) => {
              const Icon = categoryIcons[item.category] || BookOpen;
              const isSelected = idx === selectedIndex;
              return (
                <div
                  key={item.id}
                  onClick={() => {
                    onSelect(item);
                    onClose();
                  }}
                  onMouseEnter={() => setSelectedIndex(idx)}
                  className={cn(
                    'group flex items-start gap-3 rounded-lg p-3 cursor-pointer transition-all',
                    isSelected
                      ? 'bg-zinc-800/90 text-zinc-100 ring-1 ring-inset ring-amber-500/30'
                      : 'text-zinc-300 hover:bg-zinc-900/80'
                  )}
                >
                  <div
                    className={cn(
                      'rounded-md p-2 mt-0.5 shrink-0 border border-zinc-800',
                      isSelected ? 'bg-amber-500/20 text-amber-300' : 'bg-zinc-900 text-zinc-400'
                    )}
                  >
                    <Icon className="h-4 w-4" />
                  </div>
                  <div className="min-w-0 flex-1">
                    <div className="flex items-center gap-2">
                      <span className="font-semibold text-sm text-zinc-100 font-sans">
                        {item.title}
                      </span>
                      {item.section && (
                        <span className="text-[11px] font-mono text-zinc-500">
                          / {item.section}
                        </span>
                      )}
                      <span className="ml-auto text-[10px] font-mono uppercase tracking-wider rounded border border-zinc-800 px-1.5 py-0.5 text-zinc-400">
                        {item.category}
                      </span>
                    </div>
                    <p className="text-xs text-zinc-400 line-clamp-2 mt-1 font-sans">
                      {item.snippet}
                    </p>
                  </div>
                  <ArrowRight
                    className={cn(
                      'h-4 w-4 shrink-0 self-center text-zinc-500 transition-transform',
                      isSelected && 'translate-x-0.5 text-amber-400'
                    )}
                  />
                </div>
              );
            })
          )}
        </div>

        {/* Footer shortcuts */}
        <div className="flex items-center justify-between border-t border-zinc-800/80 bg-zinc-900/60 px-4 py-2 text-[11px] font-mono text-zinc-500">
          <div className="flex items-center gap-3">
            <span>
              <kbd className="rounded bg-zinc-800 px-1 py-0.5 text-zinc-400 mr-1">↑↓</kbd>
              Navigate
            </span>
            <span>
              <kbd className="rounded bg-zinc-800 px-1 py-0.5 text-zinc-400 mr-1">↵</kbd>
              Select
            </span>
          </div>
          <span>Track v0.7.0 Documentation Index</span>
        </div>
      </div>
    </div>
  );
};
