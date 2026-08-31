import React, { useState } from 'react';
import { ArrowRight, Lock, ShieldCheck, Zap, Sparkles } from 'lucide-react';
import { cn } from '../utils/cn';

interface StateDetail {
  id: 'Active' | 'Borrowed' | 'Locked' | 'Spent';
  name: string;
  badgeColor: string;
  textColor: string;
  borderColor: string;
  bgColor: string;
  icon: React.ElementType;
  description: string;
  rules: string[];
  example: string;
}

const states: StateDetail[] = [
  {
    id: 'Active',
    name: 'Active (Owned)',
    badgeColor: 'bg-emerald-500/20 text-emerald-400 border-emerald-500/40',
    textColor: 'text-emerald-400',
    borderColor: 'border-emerald-500/40',
    bgColor: 'bg-emerald-950/20',
    icon: ShieldCheck,
    description: 'The variable holds unique, exclusive ownership of a heap or stack resource. Can be moved, borrowed, or locked in a lens.',
    rules: [
      'Resource is initialized and ready for mutation or read access',
      'Can be moved (transitions to Spent)',
      'Can enter a lexical lens block (transitions to Locked)',
      'Can create shared read-only borrows (transitions to Borrowed)',
      'Freed automatically at scope exit if still Active',
    ],
    example: `let mut v: Vec = vec_init(16);\n// State: 'v' is Active`,
  },
  {
    id: 'Borrowed',
    name: 'Borrowed (&T)',
    badgeColor: 'bg-cyan-500/20 text-cyan-400 border-cyan-500/40',
    textColor: 'text-cyan-400',
    borderColor: 'border-cyan-500/40',
    bgColor: 'bg-cyan-950/20',
    icon: Sparkles,
    description: 'One or more read-only references (&T) exist. The target owner cannot be moved or mutated while borrowed.',
    rules: [
      'Allows multiple simultaneous read-only references',
      'References have Copy semantics and can be freely duplicated',
      'Target owner cannot be moved or mutably accessed',
      'Restores to Active automatically when references leave scope',
      'Escape analysis prevents returning local stack references',
    ],
    example: `let v = vec_init(16);\nlet r = &v; // 'v' transitions to Borrowed\nlet val = *r;\n// End of scope -> 'v' restores to Active`,
  },
  {
    id: 'Locked',
    name: 'Locked (Lens)',
    badgeColor: 'bg-amber-500/20 text-amber-400 border-amber-500/40',
    textColor: 'text-amber-400',
    borderColor: 'border-amber-500/40',
    bgColor: 'bg-amber-950/20',
    icon: Lock,
    description: 'An exclusive lexical lens (with u -> user) is currently active. The target variable is frozen from any outer moves or borrows.',
    rules: [
      'Lens alias has exclusive, non-escaping mutable access',
      'Zero lifetime annotations (\'a) required',
      'Outer variable is frozen in Locked state during with block',
      'Lens alias cannot escape, be assigned to outer scope, or stored in heap',
      'Restores to Active immediately upon exiting the with block',
    ],
    example: `let mut u = User { age: 30 };\nwith u -> user { // 'u' transitions to Locked\n    user.set_age(31); // 'user' is exclusive non-escaping lens\n}\n// 'u' transitions back to Active!`,
  },
  {
    id: 'Spent',
    name: 'Spent (Moved)',
    badgeColor: 'bg-purple-500/20 text-purple-400 border-purple-500/40',
    textColor: 'text-purple-400',
    borderColor: 'border-purple-500/40',
    bgColor: 'bg-purple-950/20',
    icon: Zap,
    description: 'Ownership has been moved into another variable, passed to a function, or deallocated at a spend point.',
    rules: [
      'Variable cannot be accessed, moved, or borrowed after spend point',
      'Attempted reuse triggers compile error [TK201]',
      'Moving any struct field consumes the entire struct instance atomically',
      'Zero runtime double-free or use-after-free possible',
    ],
    example: `let v = vec_init(16);\nlet y = v; // 'v' is Spent, 'y' is now Active\n// vec_push(&mut v, 1); -> ERROR: use of moved variable 'v'`,
  },
];

export const MemoryStateGraph: React.FC = () => {
  const [selectedState, setSelectedState] = useState<'Active' | 'Borrowed' | 'Locked' | 'Spent'>('Active');
  const current = states.find(s => s.id === selectedState)!;

  return (
    <div className="my-8 rounded-xl border border-zinc-800/80 bg-zinc-950/90 p-6 shadow-2xl backdrop-blur-md">
      <div className="flex flex-col md:flex-row md:items-center justify-between gap-4 border-b border-zinc-800/80 pb-5">
        <div>
          <h3 className="text-lg font-bold text-zinc-100 font-sans tracking-tight">
            Deterministic Ownership State Machine
          </h3>
          <p className="text-xs text-zinc-400 mt-1">
            Track verifies all resource lifecycles at compile time using a 4-state deterministic automaton. Click each state to inspect its constraints and invariants.
          </p>
        </div>
        <div className="flex items-center gap-1.5 self-start md:self-auto rounded-lg border border-zinc-800 bg-zinc-900/80 p-1">
          {states.map(s => (
            <button
              key={s.id}
              onClick={() => setSelectedState(s.id)}
              className={cn(
                'rounded-md px-3 py-1 text-xs font-mono transition-all',
                selectedState === s.id
                  ? cn('font-bold shadow-sm', s.badgeColor)
                  : 'text-zinc-400 hover:text-zinc-200'
              )}
            >
              {s.id}
            </button>
          ))}
        </div>
      </div>

      {/* Visual State Flow Transition Nodes */}
      <div className="grid grid-cols-1 md:grid-cols-4 gap-3 my-6">
        {states.map((s) => {
          const Icon = s.icon;
          const isSelected = s.id === selectedState;
          return (
            <div
              key={s.id}
              onClick={() => setSelectedState(s.id)}
              className={cn(
                'cursor-pointer rounded-lg border p-4 transition-all duration-200 relative overflow-hidden',
                isSelected
                  ? cn(s.borderColor, s.bgColor, 'ring-1 ring-inset ring-white/10 scale-[1.02] shadow-lg')
                  : 'border-zinc-800/80 bg-zinc-900/40 hover:border-zinc-700 hover:bg-zinc-900/70'
              )}
            >
              <div className="flex items-center justify-between mb-2">
                <span className={cn('text-xs font-mono font-bold uppercase tracking-wider', isSelected ? s.textColor : 'text-zinc-400')}>
                  {s.id}
                </span>
                <Icon className={cn('h-4 w-4', isSelected ? s.textColor : 'text-zinc-500')} />
              </div>
              <p className="text-[11px] text-zinc-400 line-clamp-2 leading-relaxed">
                {s.description}
              </p>
            </div>
          );
        })}
      </div>

      {/* State details & code preview */}
      <div className="grid grid-cols-1 lg:grid-cols-2 gap-6 rounded-lg border border-zinc-800/80 bg-zinc-900/40 p-5">
        <div>
          <div className="flex items-center gap-2 mb-3">
            <span className={cn('rounded-full border px-2.5 py-0.5 text-xs font-mono font-bold', current.badgeColor)}>
              {current.name}
            </span>
          </div>
          <p className="text-sm text-zinc-300 mb-4 leading-relaxed font-sans">
            {current.description}
          </p>

          <h4 className="text-xs font-mono font-semibold uppercase tracking-wider text-zinc-400 mb-2">
            Formal State Invariants:
          </h4>
          <ul className="space-y-1.5">
            {current.rules.map((rule, idx) => (
              <li key={idx} className="flex items-start gap-2 text-xs text-zinc-300">
                <ArrowRight className="h-3.5 w-3.5 text-amber-500 shrink-0 mt-0.5" />
                <span>{rule}</span>
              </li>
            ))}
          </ul>
        </div>

        <div>
          <div className="flex items-center justify-between mb-2">
            <span className="text-xs font-mono text-zinc-400">Track Code Example</span>
            <span className="text-[11px] font-mono text-zinc-500">examples/{current.id.toLowerCase()}.trk</span>
          </div>
          <div className="rounded-lg border border-zinc-800/90 bg-zinc-950 p-4 font-mono text-xs text-zinc-200 overflow-x-auto leading-relaxed shadow-inner">
            <pre className="whitespace-pre">{current.example}</pre>
          </div>
        </div>
      </div>
    </div>
  );
};
