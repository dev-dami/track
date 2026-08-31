import React from 'react';
import { Info, AlertTriangle, CheckCircle2, ShieldAlert, Sparkles } from 'lucide-react';
import { cn } from '../utils/cn';

interface CalloutProps {
  type?: 'note' | 'tip' | 'warning' | 'important' | 'research';
  title?: string;
  children: React.ReactNode;
  className?: string;
}

export const Callout: React.FC<CalloutProps> = ({
  type = 'note',
  title,
  children,
  className,
}) => {
  const configs = {
    note: {
      border: 'border-cyan-500/30 bg-cyan-950/20 text-cyan-200',
      iconColor: 'text-cyan-400',
      icon: Info,
      defaultTitle: 'NOTE',
    },
    tip: {
      border: 'border-emerald-500/30 bg-emerald-950/20 text-emerald-200',
      iconColor: 'text-emerald-400',
      icon: CheckCircle2,
      defaultTitle: 'TIP',
    },
    warning: {
      border: 'border-amber-500/30 bg-amber-950/20 text-amber-200',
      iconColor: 'text-amber-400',
      icon: AlertTriangle,
      defaultTitle: 'WARNING',
    },
    important: {
      border: 'border-rose-500/30 bg-rose-950/20 text-rose-200',
      iconColor: 'text-rose-400',
      icon: ShieldAlert,
      defaultTitle: 'CRITICAL INVARIANT',
    },
    research: {
      border: 'border-purple-500/30 bg-purple-950/20 text-purple-200',
      iconColor: 'text-purple-400',
      icon: Sparkles,
      defaultTitle: 'RESEARCH HYPOTHESIS',
    },
  };

  const config = configs[type];
  const IconComponent = config.icon;

  return (
    <div
      className={cn(
        'my-4 rounded-lg border p-4 backdrop-blur-sm',
        config.border,
        className
      )}
    >
      <div className="flex items-center gap-2 font-mono text-xs font-semibold uppercase tracking-wider mb-1.5">
        <IconComponent className={cn('w-4 h-4 shrink-0', config.iconColor)} />
        <span className={config.iconColor}>{title || config.defaultTitle}</span>
      </div>
      <div className="text-sm text-zinc-300 leading-relaxed font-sans pl-6">
        {children}
      </div>
    </div>
  );
};
