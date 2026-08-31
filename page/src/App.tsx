import { useState } from 'react';
import { Navbar, PageTab } from './components/Navbar';
import { Footer } from './components/Footer';
import { SearchModal, SearchItem } from './components/SearchModal';
import { HomePage } from './pages/HomePage';
import { DocsPage } from './pages/DocsPage';
import { StdlibPage } from './pages/StdlibPage';
import { PlaygroundPage } from './pages/PlaygroundPage';
import { BlogPage } from './pages/BlogPage';
import { RoadmapPage } from './pages/RoadmapPage';
import { docsChapters } from './data/docsData';
import { stdlibModules } from './data/stdlibData';
import { blogPosts } from './data/blogData';
import { codeExamples } from './data/examplesData';

export function App() {
  const [activeTab, setActiveTab] = useState<PageTab>('home');
  const [isSearchOpen, setIsSearchOpen] = useState(false);
  const [playgroundCode, setPlaygroundCode] = useState<string | undefined>(undefined);
  const [selectedDocChapter, setSelectedDocChapter] = useState<string>('intro');
  const [selectedBlogPost, setSelectedBlogPost] = useState<string | undefined>(undefined);

  // Unified search index
  const searchItems: SearchItem[] = [
    ...docsChapters.map(c => ({
      id: `doc-${c.id}`,
      title: c.title,
      category: 'docs' as const,
      section: c.category,
      snippet: c.subtitle,
      path: `docs#${c.id}`,
    })),
    ...stdlibModules.flatMap(m =>
      m.functions.map(fn => ({
        id: `stdlib-${m.name}-${fn.name}`,
        title: fn.name,
        category: 'stdlib' as const,
        section: m.name,
        snippet: `${fn.signature} — ${fn.description}`,
        path: `stdlib#${fn.name}`,
      }))
    ),
    ...blogPosts.map(p => ({
      id: `blog-${p.id}`,
      title: p.title,
      category: 'blog' as const,
      section: p.author.name,
      snippet: p.summary,
      path: `blog#${p.id}`,
    })),
    ...codeExamples.map(e => ({
      id: `example-${e.id}`,
      title: e.title,
      category: 'example' as const,
      section: e.category,
      snippet: e.description,
      path: `examples#${e.id}`,
    })),
  ];

  const handleSelectTab = (tab: PageTab) => {
    setActiveTab(tab);
    window.scrollTo({ top: 0, behavior: 'smooth' });
  };

  const handleOpenPlaygroundWithCode = (code: string) => {
    setPlaygroundCode(code);
    setActiveTab('playground');
    window.scrollTo({ top: 0, behavior: 'smooth' });
  };

  const handleSearchSelect = (item: SearchItem) => {
    if (item.category === 'docs') {
      const chapterId = item.id.replace('doc-', '');
      setSelectedDocChapter(chapterId);
      setActiveTab('docs');
    } else if (item.category === 'stdlib') {
      setActiveTab('stdlib');
    } else if (item.category === 'blog') {
      const postId = item.id.replace('blog-', '');
      setSelectedBlogPost(postId);
      setActiveTab('blog');
    } else if (item.category === 'example') {
      setActiveTab('docs');
    }
    window.scrollTo({ top: 0, behavior: 'smooth' });
  };

  return (
    <div className="min-h-screen bg-[#090a0d] text-zinc-200 flex flex-col font-sans selection:bg-amber-500/20 selection:text-amber-300">
      {/* Top Header */}
      <Navbar
        activeTab={activeTab}
        onSelectTab={handleSelectTab}
        onOpenSearch={() => setIsSearchOpen(true)}
      />

      {/* Main Content View */}
      <main className="flex-1">
        {activeTab === 'home' && (
          <HomePage
            onSelectTab={handleSelectTab}
            onOpenPlaygroundWithCode={handleOpenPlaygroundWithCode}
          />
        )}
        {activeTab === 'docs' && (
          <DocsPage
            onSelectTab={handleSelectTab}
            onOpenPlaygroundWithCode={handleOpenPlaygroundWithCode}
            initialChapterId={selectedDocChapter}
          />
        )}
        {activeTab === 'stdlib' && (
          <StdlibPage
            onSelectTab={handleSelectTab}
            onOpenPlaygroundWithCode={handleOpenPlaygroundWithCode}
          />
        )}
        {activeTab === 'playground' && (
          <PlaygroundPage initialCode={playgroundCode} />
        )}
        {activeTab === 'blog' && (
          <BlogPage
            onSelectTab={handleSelectTab}
            onOpenPlaygroundWithCode={handleOpenPlaygroundWithCode}
            initialPostId={selectedBlogPost}
          />
        )}
        {activeTab === 'roadmap' && (
          <RoadmapPage onSelectTab={handleSelectTab} />
        )}
      </main>

      {/* Search Modal */}
      <SearchModal
        isOpen={isSearchOpen}
        onClose={() => setIsSearchOpen(false)}
        onSelect={handleSearchSelect}
        searchItems={searchItems}
      />

      {/* Clean Footer */}
      <Footer onSelectTab={handleSelectTab} />
    </div>
  );
}

export default App;
