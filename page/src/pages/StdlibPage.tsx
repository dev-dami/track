import React, { useState } from 'react';
import { Search } from 'lucide-react';
import { stdlibModules } from '../data/stdlibData';
import { PageTab } from '../components/Navbar';
import { cn } from '../utils/cn';

interface StdlibPageProps {
  onSelectTab: (tab: PageTab) => void;
  onOpenPlaygroundWithCode?: (code: string) => void;
}

export const StdlibPage: React.FC<StdlibPageProps> = () => {
  const [selectedModule, setSelectedModule] = useState<string>('all');
  const [searchQuery, setSearchQuery] = useState<string>('');

  const allFunctions = stdlibModules.flatMap(module =>
    module.functions.map(fn => ({ ...fn, moduleName: module.name, moduleCategory: module.category }))
  );

  const filteredFunctions = allFunctions.filter(fn => {
    const matchesModule = selectedModule === 'all' || fn.moduleName === selectedModule;
    const matchesQuery =
      fn.name.toLowerCase().includes(searchQuery.toLowerCase()) ||
      fn.signature.toLowerCase().includes(searchQuery.toLowerCase()) ||
      fn.description.toLowerCase().includes(searchQuery.toLowerCase()) ||
      fn.moduleName.toLowerCase().includes(searchQuery.toLowerCase());
    return matchesModule && matchesQuery;
  });

  return (
    <div className="mx-auto max-w-6xl px-4 py-8 sm:px-6">
      {/* Header */}
      <div className="border-b border-zinc-800 pb-4 mb-6">
        <h1 className="text-2xl sm:text-3xl font-bold font-sans text-zinc-100">
          Standard Library Reference
        </h1>
        <p className="text-xs sm:text-sm text-zinc-400 font-sans mt-1">
          Zero-overhead C runtime wrappers, memory allocators, POSIX file I/O, and TCP sockets.
        </p>
      </div>

      <div className="grid grid-cols-1 md:grid-cols-12 gap-8 items-start">
        {/* Module Sidebar */}
        <aside className="md:col-span-4 lg:col-span-3 space-y-4 md:sticky md:top-20">
          {/* Search */}
          <div className="relative">
            <Search className="absolute left-2.5 top-2.5 h-3.5 w-3.5 text-zinc-500" />
            <input
              type="text"
              value={searchQuery}
              onChange={e => setSearchQuery(e.target.value)}
              placeholder="Filter functions..."
              className="w-full rounded border border-zinc-800 bg-[#0d0e12] py-1.5 pl-8 pr-3 text-xs font-mono text-zinc-200 placeholder-zinc-500 focus:border-amber-500 focus:outline-none"
            />
          </div>

          <div className="space-y-1">
            <div className="font-mono text-[11px] font-bold uppercase tracking-wider text-zinc-500 px-2 py-1">
              Modules
            </div>
            <div className="space-y-0.5 border-l border-zinc-800 ml-2 pl-2">
              <button
                onClick={() => setSelectedModule('all')}
                className={cn(
                  'block w-full text-left py-1 px-2 text-xs font-mono rounded transition-colors',
                  selectedModule === 'all'
                    ? 'text-amber-400 font-semibold bg-zinc-900'
                    : 'text-zinc-400 hover:text-zinc-200 hover:bg-zinc-900/50'
                )}
              >
                All ({allFunctions.length})
              </button>
              {stdlibModules.map(m => (
                <button
                  key={m.name}
                  onClick={() => setSelectedModule(m.name)}
                  className={cn(
                    'block w-full text-left py-1 px-2 text-xs font-mono rounded transition-colors',
                    selectedModule === m.name
                      ? 'text-amber-400 font-semibold bg-zinc-900'
                      : 'text-zinc-400 hover:text-zinc-200 hover:bg-zinc-900/50'
                  )}
                >
                  {m.name}
                </button>
              ))}
            </div>
          </div>
        </aside>

        {/* Function List */}
        <main className="md:col-span-8 lg:col-span-9 space-y-6">
          {filteredFunctions.length === 0 ? (
            <div className="py-12 text-center text-zinc-500 font-mono text-xs">
              No matching functions found.
            </div>
          ) : (
            filteredFunctions.map((fn, idx) => (
              <div
                key={idx}
                className="rounded border border-zinc-800 bg-[#0d0e12] p-4 space-y-2.5 text-left"
              >
                <div className="flex items-center justify-between gap-2 border-b border-zinc-800/60 pb-2">
                  <span className="font-mono text-xs font-bold text-zinc-200">
                    {fn.name}
                  </span>
                  <span className="font-mono text-[11px] text-zinc-500">
                    {fn.moduleName}
                  </span>
                </div>

                <div className="font-mono text-xs text-amber-400 bg-zinc-900/90 p-2 rounded overflow-x-auto select-all">
                  {fn.signature}
                </div>

                <p className="text-xs text-zinc-300 font-sans leading-relaxed">
                  {fn.description}
                </p>

                <div className="pt-1">
                  <div className="text-[10px] font-mono text-zinc-500 mb-1">Example:</div>
                  <pre className="font-mono text-[11px] text-zinc-400 bg-[#08080a] p-2 rounded border border-zinc-900 overflow-x-auto whitespace-pre">
                    {fn.example}
                  </pre>
                </div>
              </div>
            ))
          )}
        </main>
      </div>
    </div>
  );
};
