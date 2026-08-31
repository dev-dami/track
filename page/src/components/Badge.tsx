import React from 'react';
import { cn } from '../utils/cn';

interface BadgeProps {
  children: React.ReactNode;
  variant?: 'amber' | 'cyan' | 'emerald' | 'purple' | 'zinc' | 'rose';
  size?: 'sm' | 'md';
  className?: string;
}

export const Badge: React.FC<BadgeProps> = ({
  children,
  variant = 'zinc',
  size = 'sm',
  className,
}) => {
  const variantStyles = {
    amber: 'bg-amber-500/10 text-amber-400 border-amber-500/30',
    cyan: 'bg-cyan-500/10 text-cyan-400 border-cyan-500/30',
    emerald: 'bg-emerald-500/10 text-emerald-400 border-emerald-500/30',
    purple: 'bg-purple-500/10 text-purple-400 border-purple-500/30',
    zinc: 'bg-zinc-800/80 text-zinc-300 border-zinc-700/60',
    rose: 'bg-rose-500/10 text-rose-400 border-rose-500/30',
  };

  const sizeStyles = {
    sm: 'text-xs px-2 py-0.5',
    md: 'text-xs px-2.5 py-1 font-medium',
  };

  return (
    <span
      className={cn(
        'inline-flex items-center gap-1.5 rounded-full border font-mono',
        variantStyles[variant],
        sizeStyles[size],
        className
      )}
    >
      {children}
    </span>
  );
};
