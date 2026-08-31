import React, { useState } from 'react';
import { 
  ArrowLeft, 
  ArrowRight
} from 'lucide-react';
import { blogPosts } from '../data/blogData';
import { CodeBlock } from '../components/CodeBlock';
import { PageTab } from '../components/Navbar';

interface BlogPageProps {
  onSelectTab: (tab: PageTab) => void;
  onOpenPlaygroundWithCode?: (code: string) => void;
  initialPostId?: string;
}

export const BlogPage: React.FC<BlogPageProps> = ({
  onSelectTab,
  onOpenPlaygroundWithCode,
  initialPostId,
}) => {
  const [selectedPostId, setSelectedPostId] = useState<string | null>(initialPostId || null);
  const selectedPost = blogPosts.find(p => p.id === selectedPostId || p.slug === selectedPostId);

  if (selectedPost) {
    return (
      <div className="mx-auto max-w-3xl px-4 py-8 sm:px-6 space-y-8 text-left">
        {/* Back Link */}
        <button
          onClick={() => {
            setSelectedPostId(null);
            window.scrollTo({ top: 0, behavior: 'smooth' });
          }}
          className="flex items-center gap-1.5 font-mono text-xs text-zinc-400 hover:text-amber-400 transition-colors"
        >
          <ArrowLeft className="h-3.5 w-3.5" />
          <span>Back to Articles</span>
        </button>

        {/* Header */}
        <header className="space-y-2 border-b border-zinc-800 pb-4">
          <div className="text-xs font-mono text-zinc-500">
            {selectedPost.date} • {selectedPost.readTime} • By {selectedPost.author.name}
          </div>
          <h1 className="text-2xl sm:text-3xl font-bold font-sans text-zinc-100 leading-tight">
            {selectedPost.title}
          </h1>
        </header>

        {/* Content */}
        <article className="space-y-6 text-sm text-zinc-300 font-sans leading-relaxed">
          {selectedPost.content.split('\n\n').map((para, idx) => {
            const trimmed = para.trim();
            if (!trimmed) return null;

            if (trimmed.startsWith('### ')) {
              return (
                <h3 key={idx} className="text-base font-bold font-sans text-zinc-100 mt-6 mb-2">
                  {trimmed.replace('### ', '')}
                </h3>
              );
            }

            if (trimmed.startsWith('> ')) {
              return (
                <blockquote key={idx} className="border-l-2 border-amber-500/80 pl-4 my-4 font-mono text-xs text-zinc-300 italic">
                  {trimmed.replace('> ', '').replace(/\*/g, '')}
                </blockquote>
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
              <p key={idx} className="text-zinc-300 leading-relaxed">
                {trimmed}
              </p>
            );
          })}
        </article>
      </div>
    );
  }

  return (
    <div className="mx-auto max-w-4xl px-4 py-8 sm:px-6 space-y-8 text-left">
      {/* Header */}
      <div className="border-b border-zinc-800 pb-4">
        <h1 className="text-2xl sm:text-3xl font-bold font-sans text-zinc-100">
          Engineering & Systems Journal
        </h1>
        <p className="text-xs sm:text-sm text-zinc-400 font-sans mt-1">
          Technical deep dives on compiler architecture, type theory, Cranelift code generation, and memory models.
        </p>
      </div>

      {/* Articles list */}
      <div className="divide-y divide-zinc-800/60">
        {blogPosts.map(post => (
          <div
            key={post.id}
            onClick={() => {
              setSelectedPostId(post.id);
              window.scrollTo({ top: 0, behavior: 'smooth' });
            }}
            className="py-6 group cursor-pointer space-y-2 hover:bg-zinc-900/20 -mx-4 px-4 rounded transition-colors"
          >
            <div className="text-xs font-mono text-zinc-500 flex items-center gap-2">
              <span>{post.date}</span>
              <span>•</span>
              <span>{post.readTime}</span>
            </div>

            <h2 className="text-lg font-bold font-sans text-zinc-100 group-hover:text-amber-400 transition-colors">
              {post.title}
            </h2>

            <p className="text-xs text-zinc-400 font-sans line-clamp-2 leading-relaxed">
              {post.summary}
            </p>

            <div className="pt-1">
              <span className="inline-flex items-center gap-1 text-xs font-mono text-amber-500 group-hover:text-amber-400">
                <span>Read paper</span>
                <ArrowRight className="h-3 w-3" />
              </span>
            </div>
          </div>
        ))}
      </div>
    </div>
  );
};
