import React, { useState } from 'react';
import { Eye, EyeOff } from 'lucide-react';
import { login } from '../api';
import type { AuthUser } from '../types';

interface LoginPageProps {
  onLoginSuccess: (user: AuthUser) => void;
}

export const LoginPage: React.FC<LoginPageProps> = ({ onLoginSuccess }) => {
  const [username, setUsername] = useState('');
  const [password, setPassword] = useState('');
  const [showPassword, setShowPassword] = useState(false);
  const [rememberMe, setRememberMe] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [isLoading, setIsLoading] = useState(false);

  const handleClear = () => {
    setUsername('');
    setPassword('');
    setError(null);
  };

  const handleSubmit = async (e: React.FormEvent) => {
    e.preventDefault();
    if (!username.trim()) {
      setError('Введите имя пользователя');
      return;
    }
    if (!password) {
      setError('Введите пароль');
      return;
    }

    setIsLoading(true);
    setError(null);

    try {
      const res = await login({ username: username.trim(), password });
      onLoginSuccess(res.user);
    } catch (err: any) {
      console.error('Login error:', err);
      if (err.status === 401 || err.message?.includes('401') || err.message?.includes('INVALID_CREDENTIALS')) {
        setError('Неверное имя пользователя или пароль');
      } else {
        setError(err.message || 'Ошибка подключения к серверу авторизации');
      }
    } finally {
      setIsLoading(false);
    }
  };

  return (
    <div className="min-h-screen bg-white flex flex-col justify-center items-center px-4 py-12">
      <div className="w-full max-w-[320px]">
        {/* Card: sharp corners (rounded-none), pure white background, and deep diffuse drop shadow */}
        <div className="bg-white rounded-none shadow-[0_15px_35px_rgba(0,0,0,0.35)] overflow-hidden">
          {/* Header */}
          <div className="px-5 pt-4 pb-3">
            <h2 className="text-base font-normal text-slate-800">
              Авторизация
            </h2>
          </div>
          {/* Accent red separator line */}
          <div className="h-[2px] w-full bg-[#ff0f3e]" />

          <form onSubmit={handleSubmit} className="p-5 space-y-4">
            {error && (
              <div className="p-2.5 rounded-none bg-rose-50 text-rose-700 text-xs border border-rose-200">
                {error}
              </div>
            )}

            <div>
              <label className="block text-xs font-normal text-slate-500 mb-1">
                Имя пользователя<span className="text-[#ff0f3e] ml-0.5">*</span>
              </label>
              <input
                type="text"
                value={username}
                onChange={(e) => setUsername(e.target.value)}
                disabled={isLoading}
                autoComplete="username"
                className="w-full px-0 py-1 text-sm bg-transparent border-0 border-b border-slate-300 text-slate-900 focus:outline-none focus:border-[#ff0f3e] focus:ring-0 transition-colors"
              />
            </div>

            <div className="relative">
              <label className="block text-xs font-normal text-slate-500 mb-1">
                Пароль<span className="text-[#ff0f3e] ml-0.5">*</span>
              </label>
              <div className="relative">
                <input
                  type={showPassword ? 'text' : 'password'}
                  value={password}
                  onChange={(e) => setPassword(e.target.value)}
                  disabled={isLoading}
                  autoComplete="current-password"
                  className="w-full px-0 py-1 pr-7 text-sm bg-transparent border-0 border-b border-slate-300 text-slate-900 focus:outline-none focus:border-[#ff0f3e] focus:ring-0 transition-colors"
                />
                <button
                  type="button"
                  onClick={() => setShowPassword(!showPassword)}
                  className="absolute right-0 top-1/2 -translate-y-1/2 text-slate-400 hover:text-slate-600 focus:outline-none cursor-pointer"
                  tabIndex={-1}
                  aria-label={showPassword ? 'Скрыть пароль' : 'Показать пароль'}
                >
                  {showPassword ? (
                    <EyeOff className="w-4 h-4" />
                  ) : (
                    <Eye className="w-4 h-4" />
                  )}
                </button>
              </div>
            </div>

            {/* Remember Me switch */}
            <div className="flex items-center gap-2.5 pt-1">
              <label className="relative inline-flex items-center cursor-pointer">
                <input
                  type="checkbox"
                  checked={rememberMe}
                  onChange={(e) => setRememberMe(e.target.checked)}
                  className="sr-only peer"
                />
                <div className="w-8 h-4 bg-slate-300 peer-focus:outline-none rounded-full peer peer-checked:after:translate-x-full peer-checked:after:border-white after:content-[''] after:absolute after:top-[2px] after:left-[2px] after:bg-white after:border-slate-300 after:border after:rounded-full after:h-3 after:w-3 after:transition-all peer-checked:bg-[#ff0f3e]"></div>
              </label>
              <span className="text-xs text-slate-700 select-none">
                Оставаться в системе
              </span>
            </div>

            {/* Forgot password link */}
            <div>
              <button
                type="button"
                className="text-xs text-[#ff0f3e] hover:underline cursor-pointer bg-transparent border-0 p-0"
              >
                Забыли пароль?
              </button>
            </div>

            {/* Actions: ВХОД and ОЧИСТИТЬ */}
            <div className="flex items-center gap-6 pt-2">
              <button
                type="submit"
                disabled={isLoading}
                className="text-sm font-bold tracking-wider text-[#ff0f3e] hover:text-[#d90429] uppercase disabled:opacity-50 cursor-pointer bg-transparent border-0 p-0 transition-colors"
              >
                {isLoading ? 'Вход...' : 'ВХОД'}
              </button>
              <button
                type="button"
                onClick={handleClear}
                disabled={isLoading}
                className="text-sm font-bold tracking-wider text-slate-900 hover:text-black uppercase disabled:opacity-50 cursor-pointer bg-transparent border-0 p-0 transition-colors"
              >
                ОЧИСТИТЬ
              </button>
            </div>
          </form>
        </div>
      </div>
    </div>
  );
};
