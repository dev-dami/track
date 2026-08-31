import React from 'react';
import { PageTab } from './Navbar';

interface FooterProps {
  onSelectTab: (tab: PageTab) => void;
}

export const Footer: React.FC<FooterProps> = ({ onSelectTab }) => {
  return (
    <footer className="border-t border-zinc-800/80 bg-[#090a0d] text-zinc-400 font-mono text-xs mt-16">
      <div className="mx-auto max-w-6xl px-4 py-8 sm:px-6 flex flex-col sm:flex-row items-center justify-between gap-4">
        <div className="flex items-center gap-4">
          <button
            onClick={() => onSelectTab('docs')}
            className="hover:text-zinc-200 transition-colors"
          >
            Docs
          </button>
          <span>•</span>
          <button
            onClick={() => onSelectTab('stdlib')}
            className="hover:text-zinc-200 transition-colors"
          >
            Stdlib
          </button>
          <span>•</span>
          <button
            onClick={() => onSelectTab('blog')}
            className="hover:text-zinc-200 transition-colors"
          >
            Blog
          </button>
          <span>•</span>
          <button
            onClick={() => onSelectTab('roadmap')}
            className="hover:text-zinc-200 transition-colors"
          >
            Roadmap
          </button>
          <span>•</span>
          <a
            href="https://github.com/dev-dami/track"
            target="_blank"
            rel="noreferrer"
            className="hover:text-zinc-200 transition-colors"
          >
            GitHub
          </a>
        </div>

        <div className="text-zinc-500 text-[11px]">
          Track Programming Language © 2026. MIT Licensed.
        </div>
      </div>
    </footer>
  );
};
