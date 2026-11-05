import React, { useState } from 'react';
import { changePassword } from '../api';

interface ChangePasswordPageProps {
  onPasswordChanged: () => void;
}

export const ChangePasswordPage: React.FC<ChangePasswordPageProps> = ({ onPasswordChanged }) => {
  const [oldPassword, setOldPassword] = useState('');
  const [newPassword, setNewPassword] = useState('');
  const [confirmPassword, setConfirmPassword] = useState('');
  const [error, setError] = useState<string | null>(null);
  const [isLoading, setIsLoading] = useState(false);
  const [isSuccess, setIsSuccess] = useState(false);

  const handleSubmit = async (e: React.FormEvent) => {
    e.preventDefault();

    if (!oldPassword) {
      setError('Введите текущий пароль');
      return;
    }
    if (!newPassword) {
      setError('Введите новый пароль');
      return;
    }
    if (newPassword.length < 4) {
      setError('Новый пароль должен содержать не менее 4 символов');
      return;
    }
    if (newPassword === oldPassword) {
      setError('Новый пароль должен отличаться от текущего пароля');
      return;
    }
    if (newPassword !== confirmPassword) {
      setError('Новые пароли не совпадают');
      return;
    }

    setIsLoading(true);
    setError(null);

    try {
      await changePassword({
        old_password: oldPassword,
        new_password: newPassword,
      });
      setIsSuccess(true);
      setTimeout(() => {
        onPasswordChanged();
      }, 1000);
    } catch (err: any) {
      console.error('Password change error:', err);
      setError(err.message || 'Ошибка смены пароля. Проверьте текущий пароль.');
      setIsLoading(false);
    }
  };

  return (
    <div className="min-h-screen bg-slate-50 flex flex-col justify-center items-center px-4 py-12">
      <div className="w-full max-w-sm">
        <div className="bg-white border border-slate-200/80 rounded-xl shadow-xs overflow-hidden">
          <div className="px-6 py-5 border-b border-slate-100">
            <h2 className="text-base font-semibold text-slate-900">
              Смена временного пароля
            </h2>
            <p className="text-xs text-slate-500 mt-0.5">
              Для безопасности требуется заменить стандартный заводской пароль администратора.
            </p>
          </div>

          <form onSubmit={handleSubmit} className="p-6 space-y-4">
            {error && (
              <div className="p-3 rounded-lg bg-rose-50 text-rose-700 text-xs">
                {error}
              </div>
            )}

            {isSuccess && (
              <div className="p-3 rounded-lg bg-emerald-50 text-emerald-700 text-xs">
                Пароль успешно обновлен. Переход к настройке...
              </div>
            )}

            <div>
              <label className="block text-xs font-medium text-slate-700 mb-1.5">
                Новый пароль
              </label>
              <input
                type="password"
                value={newPassword}
                onChange={(e) => setNewPassword(e.target.value)}
                placeholder="Минимум 4 символа"
                disabled={isLoading || isSuccess}
                className="w-full px-3 py-2 text-sm bg-white border border-slate-200 rounded-lg text-slate-900 focus:outline-none focus:ring-2 focus:ring-slate-900/10 focus:border-slate-800 transition-colors"
              />
            </div>

            <div>
              <label className="block text-xs font-medium text-slate-700 mb-1.5">
                Подтверждение нового пароля
              </label>
              <input
                type="password"
                value={confirmPassword}
                onChange={(e) => setConfirmPassword(e.target.value)}
                placeholder="Повторите новый пароль"
                disabled={isLoading || isSuccess}
                className="w-full px-3 py-2 text-sm bg-white border border-slate-200 rounded-lg text-slate-900 focus:outline-none focus:ring-2 focus:ring-slate-900/10 focus:border-slate-800 transition-colors"
              />
            </div>

            <button
              type="submit"
              disabled={isLoading || isSuccess}
              className="w-full mt-2 flex items-center justify-center px-4 py-2.5 bg-slate-900 hover:bg-slate-800 text-white text-sm font-medium rounded-lg transition-colors disabled:opacity-50 disabled:cursor-not-allowed cursor-pointer shadow-xs"
            >
              <span>Установить пароль</span>
            </button>
          </form>
        </div>
      </div>
    </div>
  );
};
