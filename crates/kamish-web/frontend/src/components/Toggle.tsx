import React from 'react';

interface ToggleProps {
  checked: boolean;
  onChange: (checked: boolean) => void;
  disabled?: boolean;
  label?: string;
  id?: string;
}

export const Toggle: React.FC<ToggleProps> = ({
  checked,
  onChange,
  disabled = false,
  label,
  id,
}) => {
  const toggleId = id || (label ? `toggle-${label.replace(/\s+/g, '-').toLowerCase()}` : undefined);

  return (
    <label
      htmlFor={toggleId}
      className={`inline-flex items-center gap-3 select-none ${
        disabled ? 'opacity-50 cursor-not-allowed' : 'cursor-pointer'
      }`}
    >
      <div className="relative inline-flex items-center">
        <input
          type="checkbox"
          id={toggleId}
          checked={checked}
          disabled={disabled}
          onChange={(e) => onChange(e.target.checked)}
          className="sr-only peer"
        />
        <div className="w-9 h-5 bg-slate-200 peer-focus:outline-none peer-focus:ring-2 peer-focus:ring-slate-900/10 rounded-full peer peer-checked:after:translate-x-4 after:content-[''] after:absolute after:top-[2px] after:left-[2px] after:bg-white after:rounded-full after:h-4 after:w-4 after:shadow-xs after:transition-all peer-checked:bg-slate-900 transition-colors" />
      </div>
      {label && <span className="text-sm font-medium text-slate-700">{label}</span>}
    </label>
  );
};
