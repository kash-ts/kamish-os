import React, { useState, useEffect } from 'react';
import {
  HardDriveDownload,
  RotateCcw,
} from 'lucide-react';
import { Card } from '../components/Card';
import { Button } from '../components/Button';
import { Toggle } from '../components/Toggle';
import {
  getUpdateStatus,
  uploadUpdateBundle,
  installUpdate,
  getSshStatus,
  toggleSsh,
} from '../api';
import type { UpdateStatus, UpdateBundleInfo, SshStatus } from '../types';

export const UpdatesPage: React.FC = () => {
  const [updateStatus, setUpdateStatus] = useState<UpdateStatus | null>(null);
  const [bundleInfo, setBundleInfo] = useState<UpdateBundleInfo | null>(null);
  const [isUploading, setIsUploading] = useState(false);
  const [isInstalling, setIsInstalling] = useState(false);

  // SSH state
  const [sshStatus, setSshStatus] = useState<SshStatus | null>(null);
  const [isSshToggling, setIsSshToggling] = useState(false);
  const [sshKeyInput, setSshKeyInput] = useState('');
  const [tempPassword, setTempPassword] = useState<string | null>(null);

  const [message, setMessage] = useState<{ text: string; type: 'success' | 'error' } | null>(null);

  const refreshAll = async () => {
    try {
      const [upRes, sshRes] = await Promise.all([
        getUpdateStatus().catch(() => null),
        getSshStatus().catch(() => null),
      ]);
      if (upRes) setUpdateStatus(upRes);
      if (sshRes) setSshStatus(sshRes);
    } catch (err) {
      console.warn('Update page refresh error:', err);
    }
  };

  useEffect(() => {
    refreshAll();
  }, []);

  const handleFileUpload = async (e: React.ChangeEvent<HTMLInputElement>) => {
    const file = e.target.files?.[0];
    if (!file) return;
    setIsUploading(true);
    setMessage(null);
    try {
      const info = await uploadUpdateBundle(file);
      setBundleInfo(info);
      setMessage({ text: `Бандл "${file.name}" загружен. Версия: ${info.version}`, type: 'success' });
    } catch (err: any) {
      setMessage({ text: err.message || 'Ошибка загрузки бандла обновления', type: 'error' });
    } finally {
      setIsUploading(false);
    }
  };

  const handleInstall = async () => {
    setIsInstalling(true);
    setMessage(null);
    try {
      const res = await installUpdate();
      setMessage({ text: `Обновление установлено в ${res.target_slot}: ${res.message}`, type: 'success' });
      refreshAll();
    } catch (err: any) {
      setMessage({ text: err.message || 'Ошибка установки обновления', type: 'error' });
    } finally {
      setIsInstalling(false);
    }
  };

  const handleToggleSsh = async (enable: boolean) => {
    setIsSshToggling(true);
    setMessage(null);
    try {
      const res = await toggleSsh({
        enable,
        ssh_key: sshKeyInput.trim() || undefined,
      });
      setMessage({ text: res.message, type: 'success' });
      if (res.temp_password) {
        setTempPassword(res.temp_password);
      } else {
        setTempPassword(null);
      }
      refreshAll();
    } catch (err: any) {
      setMessage({ text: err.message || 'Ошибка переключения SSH', type: 'error' });
    } finally {
      setIsSshToggling(false);
    }
  };

  return (
    <div className="space-y-6">
      {/* Alert Notification */}
      {message && (
        <div
          className={`p-3 rounded-lg text-xs ${
            message.type === 'success'
              ? 'bg-emerald-50 text-emerald-700'
              : 'bg-rose-50 text-rose-700'
          }`}
        >
          {message.text}
        </div>
      )}

      {/* System Updates Card */}
      <Card
        title="Системные обновления"
        subtitle="Установка пакетов обновлений прошивки"
        badge={
          <span className="inline-flex items-center gap-1.5 text-xs font-medium text-slate-700">
            <span
              className={`w-1.5 h-1.5 rounded-full ${
                updateStatus?.rauc_supported ? 'bg-emerald-500' : 'bg-slate-400'
              }`}
            />
            <span>{updateStatus?.rauc_supported ? 'Служба активна' : 'Слот A/B'}</span>
          </span>
        }
        action={
          <Button
            variant="outline"
            size="sm"
            onClick={refreshAll}
            leftIcon={<RotateCcw className="w-3.5 h-3.5" />}
          >
            Обновить статус
          </Button>
        }
      >
        <div className="space-y-5">
          <div className="divide-y divide-slate-100 text-xs sm:text-sm">
            <div className="py-2.5 flex items-center justify-between">
              <span className="text-slate-500">Текущая версия ОС:</span>
              <span className="font-mono text-slate-900 font-medium">
                {updateStatus?.installed_version || 'v0.1.0-armbian'}
              </span>
            </div>
            <div className="py-2.5 flex items-center justify-between">
              <span className="text-slate-500">Активный системный слот:</span>
              <span className="font-mono text-slate-900 font-medium">
                {updateStatus?.active_slot || 'Slot A (rootfs0)'}
              </span>
            </div>
          </div>

          {/* Technical partition details in accordion */}
          <details className="text-xs text-slate-500 rounded-lg border border-slate-200/80 p-3 bg-slate-50/50">
            <summary className="font-medium text-slate-700 cursor-pointer select-none">
              Технические сведения о разметке слотов A/B
            </summary>
            <div className="mt-2.5 pt-2 border-t border-slate-200/60 space-y-1 font-mono text-[11px] text-slate-600">
              <div>Основной слот загрузки: {updateStatus?.boot_primary || 'Slot A'}</div>
              <div>Поддержка RAUC OTA: {updateStatus?.rauc_supported ? 'Да' : 'Эмуляция'}</div>
              <div>Конфигурация: Dual RootFS A/B failover protection</div>
            </div>
          </details>

          {/* Upload Bundle */}
          <div className="space-y-3 pt-2">
            <div>
              <h4 className="text-xs font-semibold text-slate-900">
                Загрузка бандла обновления (.raucb)
              </h4>
              <p className="text-xs text-slate-500 mt-0.5">
                Выберите файл обновления для верификации и развертывания.
              </p>
            </div>

            <div className="flex flex-col sm:flex-row items-start sm:items-center gap-3">
              <input
                type="file"
                accept=".raucb,.tar.gz,.bin"
                onChange={handleFileUpload}
                disabled={isUploading || isInstalling}
                className="text-xs text-slate-700 file:mr-3 file:py-1.5 file:px-3 file:rounded-lg file:border file:border-slate-200 file:text-xs file:font-medium file:bg-white file:text-slate-700 hover:file:bg-slate-50"
              />
              {bundleInfo && (
                <Button
                  variant="primary"
                  size="sm"
                  onClick={handleInstall}
                  isLoading={isInstalling}
                  leftIcon={<HardDriveDownload className="w-3.5 h-3.5" />}
                >
                  Установить обновление
                </Button>
              )}
            </div>

            {bundleInfo && (
              <div className="p-3.5 bg-slate-50 border border-slate-200/80 rounded-lg text-xs space-y-1 font-mono">
                <div className="text-slate-900 font-semibold">Бандл верифицирован:</div>
                <div className="text-slate-600">Версия: {bundleInfo.version}</div>
                <div className="text-slate-600">Совместимость: {bundleInfo.compatible}</div>
                <div className="text-emerald-700 font-medium">
                  Подпись: {bundleInfo.signature_valid ? 'Валидна' : 'Требуется проверка'}
                </div>
              </div>
            )}
          </div>
        </div>
      </Card>

      {/* Dropbear SSH Management Card */}
      <Card
        title="Удаленный доступ (SSH)"
        subtitle="Служба Dropbear SSH для доступа к консоли Linux"
        badge={
          <span className="inline-flex items-center gap-1.5 text-xs font-medium text-slate-700">
            <span
              className={`w-1.5 h-1.5 rounded-full ${
                sshStatus?.enabled ? 'bg-emerald-500' : 'bg-slate-400'
              }`}
            />
            <span>{sshStatus?.enabled ? 'Служба активна' : 'Служба отключена'}</span>
          </span>
        }
      >
        <div className="space-y-4">
          <div className="flex flex-col sm:flex-row sm:items-center justify-between gap-3 py-1">
            <div>
              <Toggle
                checked={sshStatus?.enabled ?? false}
                onChange={handleToggleSsh}
                disabled={isSshToggling}
                label="Активировать SSH сервер (порт 22)"
              />
              <p className="text-xs text-slate-500 mt-1 pl-12">
                Пользователь: <span className="font-mono text-slate-800">{sshStatus?.user || 'kamish'}</span> • Активных сессий: {sshStatus?.active_sessions ?? 0}
              </p>
            </div>
            {tempPassword && (
              <div className="p-2 bg-amber-50 border border-amber-200 text-xs text-amber-900 font-mono rounded-lg">
                Временный пароль: <strong className="font-bold">{tempPassword}</strong>
              </div>
            )}
          </div>

          <div>
            <label className="block text-xs font-medium text-slate-700 mb-1">
              Публичный SSH ключ (id_ed25519.pub / id_rsa.pub)
            </label>
            <textarea
              rows={2}
              value={sshKeyInput}
              onChange={(e) => setSshKeyInput(e.target.value)}
              placeholder="ssh-ed25519 AAAAC3NzaC1lZDI1NTE5... (опционально)"
              className="w-full text-xs font-mono bg-white border border-slate-200 rounded-lg p-2.5 text-slate-900 focus:outline-none focus:ring-2 focus:ring-slate-900/10 focus:border-slate-800"
            />
          </div>
        </div>
      </Card>
    </div>
  );
};
