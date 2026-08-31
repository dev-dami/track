import React from 'react';
import { CheckCircle2, Clock, Target } from 'lucide-react';
import { roadmapMilestones } from '../data/roadmapData';
import { PageTab } from '../components/Navbar';

interface RoadmapPageProps {
  onSelectTab: (tab: PageTab) => void;
}

export const RoadmapPage: React.FC<RoadmapPageProps> = () => {
  return (
    <div className="mx-auto max-w-4xl px-4 py-8 sm:px-6 space-y-8 text-left">
      {/* Header */}
      <div className="border-b border-zinc-800 pb-4">
        <h1 className="text-2xl sm:text-3xl font-bold font-sans text-zinc-100">
          Compiler Self-Bootstrapping Roadmap
        </h1>
        <p className="text-xs sm:text-sm text-zinc-400 font-sans mt-1">
          Path from early memory primitives to verified 3-stage self-compilation in v0.9.0.
        </p>
      </div>

      {/* Milestones list */}
      <div className="space-y-4">
        {roadmapMilestones.map((m, idx) => {
          const isCompleted = m.status === 'completed';
          const isInProgress = m.status === 'in-progress';

          return (
            <div
              key={idx}
              className="rounded border border-zinc-800 bg-[#0d0e12] p-4 space-y-2 text-left"
            >
              <div className="flex items-center justify-between gap-2 border-b border-zinc-800/60 pb-2">
                <div className="flex items-center gap-2">
                  <span className="font-mono text-xs font-bold text-amber-400">
                    {m.version}
                  </span>
                  <span className="font-mono text-xs text-zinc-300 font-medium">
                    {m.title}
                  </span>
                </div>
                <div className="flex items-center gap-1.5 font-mono text-[11px]">
                  {isCompleted ? (
                    <span className="text-emerald-400 flex items-center gap-1">
                      <CheckCircle2 className="h-3.5 w-3.5" />
                      <span>Completed</span>
                    </span>
                  ) : isInProgress ? (
                    <span className="text-amber-400 flex items-center gap-1">
                      <Clock className="h-3.5 w-3.5" />
                      <span>In Progress</span>
                    </span>
                  ) : (
                    <span className="text-purple-400 flex items-center gap-1">
                      <Target className="h-3.5 w-3.5" />
                      <span>Milestone Target</span>
                    </span>
                  )}
                </div>
              </div>

              <p className="text-xs text-zinc-300 font-sans leading-relaxed">
                {m.description}
              </p>

              <div className="grid grid-cols-1 sm:grid-cols-2 gap-1.5 pt-1 font-mono text-[11px] text-zinc-400">
                {m.highlights.map((hl, hidx) => (
                  <div key={hidx} className="flex items-center gap-1.5">
                    <span className="text-zinc-600">•</span>
                    <span>{hl}</span>
                  </div>
                ))}
              </div>
            </div>
          );
        })}
      </div>
    </div>
  );
};
