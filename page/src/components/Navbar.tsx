import React, { useState } from 'react';
import { 
  Search, 
  Menu, 
  X,
  Github
} from 'lucide-react';
import { cn } from '../utils/cn';

export type PageTab = 'home' | 'docs' | 'stdlib' | 'playground' | 'blog' | 'roadmap';

interface NavbarProps {
  activeTab: PageTab;
  onSelectTab: (tab: PageTab) => void;
  onOpenSearch: () => void;
}

export const Navbar: React.FC<NavbarProps> = ({
  activeTab,
  onSelectTab,
  onOpenSearch,
}) => {
  const [mobileMenuOpen, setMobileMenuOpen] = useState(false);

  const navItems: { id: PageTab; label: string }[] = [
    { id: 'docs', label: 'Docs' },
    { id: 'stdlib', label: 'Standard Library' },
    { id: 'playground', label: 'Playground' },
    { id: 'blog', label: 'Blog' },
    { id: 'roadmap', label: 'Roadmap' },
  ];

  return (
    <header className="sticky top-0 z-40 w-full border-b border-zinc-800 bg-[#090a0d]/95 backdrop-blur">
      <div className="mx-auto flex max-w-6xl items-center justify-between px-4 sm:px-6 h-14">
        {/* Brand / Logo */}
        <div className="flex items-center gap-6">
          <button
            onClick={() => onSelectTab('home')}
            className="flex items-center gap-2.5 text-left focus:outline-none"
          >
            <span className="font-mono text-sm font-bold tracking-tight text-zinc-100 hover:text-amber-400 transition-colors">
              TRACK
            </span>
            <span className="rounded border border-zinc-800 bg-zinc-900 px-1.5 py-0.5 font-mono text-[10px] text-zinc-400">
              v0.7.0
            </span>
          </button>

          {/* Desktop Navigation Links */}
          <nav className="hidden md:flex items-center gap-1">
            {navItems.map(item => {
              const isActive = activeTab === item.id;
              return (
                <button
                  key={item.id}
                  onClick={() => onSelectTab(item.id)}
                  className={cn(
                    'px-3 py-1 text-xs font-mono transition-colors rounded',
                    isActive
                      ? 'text-amber-400 bg-zinc-900 font-semibold'
                      : 'text-zinc-400 hover:text-zinc-200 hover:bg-zinc-900/50'
                  )}
                >
                  {item.label}
                </button>
              );
            })}
          </nav>
        </div>

        {/* Right Actions */}
        <div className="flex items-center gap-3">
          {/* Quick Search */}
          <button
            onClick={onOpenSearch}
            className="flex items-center gap-2 rounded border border-zinc-800 bg-zinc-900/60 px-2.5 py-1 text-xs font-mono text-zinc-400 hover:text-zinc-200 hover:border-zinc-700 transition-colors"
          >
            <Search className="h-3 w-3" />
            <span className="hidden sm:inline">Search</span>
            <kbd className="text-[10px] text-zinc-500 font-mono">⌘K</kbd>
          </button>

          {/* GitHub link */}
          <a
            href="https://github.com/dev-dami/track"
            target="_blank"
            rel="noreferrer"
            className="text-zinc-400 hover:text-zinc-200 transition-colors p-1"
            title="GitHub Repository"
          >
            <Github className="h-4 w-4" />
          </a>

          {/* Mobile hamburger */}
          <button
            onClick={() => setMobileMenuOpen(!mobileMenuOpen)}
            className="md:hidden text-zinc-400 hover:text-zinc-200 p-1"
          >
            {mobileMenuOpen ? <X className="h-4 w-4" /> : <Menu className="h-4 w-4" />}
          </button>
        </div>
      </div>

      {/* Mobile Drawer */}
      {mobileMenuOpen && (
        <div className="md:hidden border-t border-zinc-800 bg-[#090a0d] px-4 py-3 space-y-1">
          {navItems.map(item => (
            <button
              key={item.id}
              onClick={() => {
                onSelectTab(item.id);
                setMobileMenuOpen(false);
              }}
              className={cn(
                'block w-full text-left px-2 py-1.5 text-xs font-mono rounded',
                activeTab === item.id
                  ? 'text-amber-400 bg-zinc-900 font-semibold'
                  : 'text-zinc-400 hover:text-zinc-200'
              )}
            >
              {item.label}
            </button>
          ))}
        </div>
      )}
    </header>
  );
};
