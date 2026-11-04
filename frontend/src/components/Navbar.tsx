import React, { useState, useEffect } from 'react';
import {
  LayoutDashboard,
  Music,
  AlarmClock,
  Activity,
  Sliders,
  Menu,
  X,
  Wifi,
  WifiOff,
} from 'lucide-react';
import { Badge } from './Badge';

export type NavPage = 'dashboard' | 'music' | 'alarms' | 'diagnostics' | 'settings';

interface NavbarProps {
  currentPage: NavPage;
  onNavigate: (page: NavPage) => void;
  isBackendConnected: boolean;
  version?: string;
}

export const Navbar: React.FC<NavbarProps> = ({
  currentPage,
  onNavigate,
  isBackendConnected,
}) => {
  const [mobileMenuOpen, setMobileMenuOpen] = useState(false);
  const [currentTime, setCurrentTime] = useState<string>('');

  useEffect(() => {
    const updateTime = () => {
      const now = new Date();
      setCurrentTime(
        now.toLocaleTimeString([], { hour: '2-digit', minute: '2-digit', second: '2-digit' })
      );
    };
    updateTime();
    const timer = setInterval(updateTime, 1000);
    return () => clearInterval(timer);
  }, []);

  const navItems = [
    { id: 'dashboard' as NavPage, label: 'Главная', icon: LayoutDashboard },
    { id: 'music' as NavPage, label: 'Музыка', icon: Music },
    { id: 'alarms' as NavPage, label: 'Будильники', icon: AlarmClock },
    { id: 'diagnostics' as NavPage, label: 'Диагностика', icon: Activity },
    { id: 'settings' as NavPage, label: 'Настройки', icon: Sliders },
  ];

  const handleNavClick = (page: NavPage) => {
    onNavigate(page);
    setMobileMenuOpen(false);
  };

  return (
    <header className="sticky top-0 z-40 w-full border-b border-slate-800 bg-slate-950/90 backdrop-blur-md">
      <div className="max-w-7xl mx-auto px-4 sm:px-6 lg:px-8">
        <div className="flex items-center justify-between h-16">
          {/* Logo & Brand */}
          <div className="flex items-center gap-3">
            <div>
              <div className="flex items-center gap-2">
                <span className="font-semibold text-base tracking-tight text-white">Камыш</span>
              </div>
            </div>
          </div>

          {/* Desktop Navigation */}
          <nav className="hidden md:flex items-center gap-1">
            {navItems.map((item) => {
              const Icon = item.icon;
              const isActive = currentPage === item.id;
              return (
                <button
                  key={item.id}
                  onClick={() => handleNavClick(item.id)}
                  className={`flex items-center gap-2 px-3.5 py-2 rounded-xl text-sm font-medium transition-colors cursor-pointer ${
                    isActive
                      ? 'bg-slate-800/90 text-sky-400 border border-slate-700/60 shadow-sm'
                      : 'text-slate-400 hover:text-slate-200 hover:bg-slate-900'
                  }`}
                >
                  <Icon className={`w-4 h-4 ${isActive ? 'text-sky-400' : 'text-slate-400'}`} />
                  <span>{item.label}</span>
                </button>
              );
            })}
          </nav>

          {/* Status & Clock Right Widget */}
          <div className="hidden sm:flex items-center gap-3">
            <div className="font-mono text-xs px-2.5 py-1 rounded-lg bg-slate-900 border border-slate-800 text-slate-300">
              {currentTime || '--:--:--'}
            </div>

            <Badge variant={isBackendConnected ? 'success' : 'danger'} size="sm">
              {isBackendConnected ? (
                <>
                  <Wifi className="w-3 h-3" />
                  <span>В сети</span>
                </>
              ) : (
                <>
                  <WifiOff className="w-3 h-3" />
                  <span>Не в сети</span>
                </>
              )}
            </Badge>
          </div>

          {/* Mobile menu button */}
          <div className="flex md:hidden items-center gap-2">
            <Badge variant={isBackendConnected ? 'success' : 'danger'} size="sm">
              <span className="w-1.5 h-1.5 rounded-full bg-current"></span>
            </Badge>
            <button
              onClick={() => setMobileMenuOpen(!mobileMenuOpen)}
              className="p-2 rounded-xl text-slate-400 hover:text-white hover:bg-slate-900 focus:outline-none"
              aria-label="Toggle menu"
            >
              {mobileMenuOpen ? <X className="w-6 h-6" /> : <Menu className="w-6 h-6" />}
            </button>
          </div>
        </div>
      </div>

      {/* Mobile dropdown */}
      {mobileMenuOpen && (
        <div className="md:hidden border-b border-slate-800 bg-slate-950 px-4 pt-2 pb-4 space-y-1">
          {navItems.map((item) => {
            const Icon = item.icon;
            const isActive = currentPage === item.id;
            return (
              <button
                key={item.id}
                onClick={() => handleNavClick(item.id)}
                className={`w-full flex items-center gap-3 px-3.5 py-2.5 rounded-xl text-sm font-medium transition-colors ${
                  isActive
                    ? 'bg-slate-800 text-sky-400 border border-slate-700/60'
                    : 'text-slate-400 hover:text-slate-200 hover:bg-slate-900'
                }`}
              >
                <Icon className={`w-4 h-4 ${isActive ? 'text-sky-400' : 'text-slate-400'}`} />
                <span>{item.label}</span>
              </button>
            );
          })}
          <div className="pt-2 flex items-center justify-between text-xs text-slate-400 px-3">
            <span>Время: {currentTime}</span>
            <span>{isBackendConnected ? 'Ядро подключено' : 'Связь потеряна'}</span>
          </div>
        </div>
      )}
    </header>
  );
};
