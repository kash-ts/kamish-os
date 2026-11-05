import React from 'react';

interface CardProps {
  children: React.ReactNode;
  className?: string;
  title?: React.ReactNode;
  subtitle?: React.ReactNode;
  badge?: React.ReactNode;
  action?: React.ReactNode;
  footer?: React.ReactNode;
}

export const Card: React.FC<CardProps> = ({
  children,
  className = '',
  title,
  subtitle,
  badge,
  action,
  footer,
}) => {
  return (
    <div
      className={`bg-white border border-slate-200/80 rounded-lg shadow-xs ${className}`}
    >
      {(title || badge || action) && (
        <div className="flex items-center justify-between px-6 py-4 border-b border-slate-100">
          <div>
            <div className="flex items-center gap-2">
              {typeof title === 'string' ? (
                <h3 className="font-semibold text-slate-900 text-sm sm:text-base">{title}</h3>
              ) : (
                title
              )}
              {badge}
            </div>
            {subtitle && <p className="text-xs text-slate-500 mt-0.5">{subtitle}</p>}
          </div>
          {action && <div className="flex items-center gap-2">{action}</div>}
        </div>
      )}
      <div className="p-6">{children}</div>
      {footer && (
        <div className="px-6 py-3.5 border-t border-slate-100 bg-slate-50/60 rounded-b-lg">
          {footer}
        </div>
      )}
    </div>
  );
};
