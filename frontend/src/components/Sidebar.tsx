import React from 'react';
import {
  LayoutDashboard,
  Cpu,
  Volume2,
  Bot,
  Wifi,
  Music,
  AlarmClock,
  Activity,
  HardDriveDownload,
  X,
  LogOut,
} from 'lucide-react';

export type NavPage =
  | 'dashboard'
  | 'system'
  | 'voice'
  | 'ai'
  | 'network'
  | 'multimedia'
  | 'alarms'
  | 'diagnostics'
  | 'updates';

interface SidebarProps {
  currentPage: NavPage;
  onNavigate: (page: NavPage) => void;
  isOpen: boolean;
  onClose: () => void;
  onLogout: () => void;
  version?: string;
}

export const Sidebar: React.FC<SidebarProps> = ({
  currentPage,
  onNavigate,
  isOpen,
  onClose,
  onLogout,
  version = 'v0.1.0',
}) => {
  const navItems: Array<{ id: NavPage; label: string; icon: React.ComponentType<{ className?: string }> }> = [
    { id: 'dashboard', label: 'Обзор', icon: LayoutDashboard },
    { id: 'system', label: 'Система', icon: Cpu },
    { id: 'voice', label: 'Голос и звук', icon: Volume2 },
    { id: 'ai', label: 'ИИ и модели', icon: Bot },
    { id: 'network', label: 'Сеть', icon: Wifi },
    { id: 'multimedia', label: 'Мультимедиа', icon: Music },
    { id: 'alarms', label: 'Будильники и напоминания', icon: AlarmClock },
    { id: 'diagnostics', label: 'Диагностика', icon: Activity },
    { id: 'updates', label: 'Обновления', icon: HardDriveDownload },
  ];

  const handleItemClick = (id: NavPage) => {
    onNavigate(id);
    onClose();
  };

  return (
    <>
      {/* Mobile backdrop */}
      {isOpen && (
        <div
          className="fixed inset-0 bg-slate-900/40 z-40 lg:hidden transition-opacity backdrop-blur-xs"
          onClick={onClose}
          aria-hidden="true"
        />
      )}

      {/* Fixed Sidebar */}
      <aside
        className={`fixed top-0 bottom-0 left-0 z-50 w-64 bg-white border-r border-slate-200 flex flex-col transition-transform duration-200 ease-in-out lg:translate-x-0 ${
          isOpen ? 'translate-x-0' : '-translate-x-full'
        }`}
      >
        {/* Hardware Header / Brand: Clean text branding without speaker icon box */}
        <div className="h-16 px-6 border-b border-slate-100 flex items-center justify-between">
          <div>
            <div className="flex items-center gap-2">
              <span className="font-bold text-sm tracking-tight text-slate-900">
                KAMISH
              </span>
            </div>
          </div>

          <button
            type="button"
            onClick={onClose}
            className="lg:hidden p-1.5 rounded-lg text-slate-400 hover:text-slate-700 hover:bg-slate-100 transition-colors"
            aria-label="Close menu"
          >
            <X className="w-4 h-4" />
          </button>
        </div>

        {/* Navigation list */}
        <nav className="flex-1 px-3 py-4 space-y-1 overflow-y-auto">
          {navItems.map((item) => {
            const Icon = item.icon;
            const isActive = currentPage === item.id;
            return (
              <button
                key={item.id}
                type="button"
                onClick={() => handleItemClick(item.id)}
                className={`w-full flex items-center gap-3 px-3.5 py-2.5 text-xs font-medium rounded-lg transition-colors text-left cursor-pointer ${
                  isActive
                    ? 'bg-slate-100 text-slate-900 font-semibold'
                    : 'text-slate-600 hover:bg-slate-50 hover:text-slate-900'
                }`}
              >
                <Icon
                  className={`w-4 h-4 shrink-0 transition-colors ${
                    isActive ? 'text-slate-900' : 'text-slate-400'
                  }`}
                />
                <span className="truncate">{item.label}</span>
              </button>
            );
          })}
        </nav>

        {/* Sidebar Footer: Logout Button */}
        <div className="p-3 border-t border-slate-100 bg-white">
          <button
            type="button"
            onClick={onLogout}
            className="w-full flex items-center gap-3 px-3.5 py-2.5 text-xs font-medium text-slate-600 hover:text-rose-700 hover:bg-rose-50 border border-transparent hover:border-rose-100 rounded-lg transition-colors cursor-pointer group"
          >
            <LogOut className="w-4 h-4 text-slate-400 group-hover:text-rose-600 transition-colors" />
            <span>Выход</span>
          </button>
        </div>
      </aside>
    </>
  );
};
