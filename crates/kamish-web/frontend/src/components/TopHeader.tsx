import React from 'react';
import { Menu, AlertTriangle } from 'lucide-react';
import type { NavPage } from './Sidebar';

interface TopHeaderProps {
  hasDegradedComponents?: boolean;
}

export const TopHeader: React.FC<TopHeaderProps> = ({
  hasDegradedComponents = false,
}) => {
  return (
    <header className="sticky top-0 z-30 h-16 bg-white/80 backdrop-blur-md border-b border-slate-200/80 px-4 sm:px-6 flex items-center justify-between">
      {/* Right: Subsystem Warnings (if degraded) */}
      <div className="flex items-center gap-3">
        {hasDegradedComponents && (
          <div className="inline-flex items-center gap-1.5 px-2.5 py-1 rounded-md text-xs font-medium text-amber-800 bg-amber-50 border border-amber-200/80">
            <AlertTriangle className="w-3.5 h-3.5 text-amber-600" />
            <span>Деградация подсистемы</span>
          </div>
        )}
      </div>
    </header>
  );
};
