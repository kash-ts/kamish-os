import React, { useState, useEffect } from 'react';
import { RefreshCw } from 'lucide-react';
import { Card } from '../components/Card';
import { Button } from '../components/Button';
import { Toggle } from '../components/Toggle';
import { formatBytes } from '../utils/formatters';
import { simulateDegradation } from '../api';
import type { StatusResponse } from '../types';

interface DiagnosticsPageProps {
  status: StatusResponse | null;
  isLoading: boolean;
  onRefresh: () => void;
}

export const DiagnosticsPage: React.FC<DiagnosticsPageProps> = ({
  status,
  isLoading,
  onRefresh,
}) => {
  const [autoRefresh, setAutoRefresh] = useState(false);
  const [degradeLoading, setDegradeLoading] = useState(false);
  const [degradeMsg, setDegradeMsg] = useState<string | null>(null);

  useEffect(() => {
    if (!autoRefresh) return;
    const interval = setInterval(() => {
      onRefresh();
    }, 5000);
    return () => clearInterval(interval);
  }, [autoRefresh, onRefresh]);

  const memUsed = status?.device.memory_used_bytes || 0;
  const memTotal = status?.device.memory_total_bytes || 1;
  const memPercent = Math.min(100, Math.round((memUsed / memTotal) * 100));

  const cpuTemp = status?.hardware?.cpu_temp_celsius ?? status?.device.cpu_temp_celsius;
  const hardwareModel = status?.hardware?.model || status?.device.hardware_model || 'kamish';

  const subsystemDescriptions: Record<string, string> = {
    audio: 'Микрофонный массив Seeed ReSpeaker XVF3800 и акустика GT-112',
    bluetooth: 'Bluetooth A2DP Sink для приёма звука',
    network: 'Сетевые интерфейсы Wi-Fi и Ethernet',
    vpn: 'Шифрованный туннель VLESS Reality',
    speech: 'Модули распознавания (STT) и синтеза речи (TTS) с VAD',
    ai: 'Служба голосового ассистента и вызова инструментов',
    scheduler: 'Планировщик будильников и напоминаний SQLite',
    music: 'Потоковый плеер MPV и интернет-радио',
  };

  const handleSimulate = async (comp: string, isDegraded: boolean) => {
    setDegradeLoading(true);
    setDegradeMsg(null);
    try {
      const res = await simulateDegradation(
        comp,
        isDegraded,
        isDegraded ? `Имитация сбоя модуля ${comp}` : undefined
      );
      setDegradeMsg(`Статус модуля ${comp} изменен. Шаг восстановления: ${res.recovery_step}`);
      onRefresh();
    } catch (err: any) {
      setDegradeMsg(`Ошибка вызова симуляции: ${err.message}`);
    } finally {
      setDegradeLoading(false);
    }
  };

  return (
    <div className="space-y-6">
      {/* Top Header */}
      <div className="flex flex-col sm:flex-row sm:items-center justify-between gap-4 p-5 rounded-xl bg-white border border-slate-200/80 shadow-xs">
        <div>
          <h2 className="text-base font-semibold text-slate-900 tracking-tight">
            Диагностика компонентов
          </h2>
          <p className="text-xs text-slate-500 mt-0.5">
            Статус аппаратных компонентов и системных служб
          </p>
        </div>

        <div className="flex items-center gap-4">
          <Toggle
            checked={autoRefresh}
            onChange={setAutoRefresh}
            label="Автообновление (5с)"
          />
          <Button
            variant="outline"
            size="sm"
            onClick={onRefresh}
            isLoading={isLoading}
            leftIcon={<RefreshCw className={`w-3.5 h-3.5 ${isLoading ? 'animate-spin' : ''}`} />}
          >
            Обновить
          </Button>
        </div>
      </div>

      {degradeMsg && (
        <div className="p-3 bg-slate-100 rounded-lg text-xs text-slate-800 font-mono">
          {degradeMsg}
        </div>
      )}

      {/* Hardware Telemetry Card */}
      <div className="grid grid-cols-1 md:grid-cols-2 gap-4">
        <Card title="Вычислительный модуль" subtitle={hardwareModel}>
          <div className="divide-y divide-slate-100 text-xs sm:text-sm">
            <div className="py-2.5 flex items-center justify-between">
              <span className="text-slate-500">Модель платы:</span>
              <span className="font-medium text-slate-900">{hardwareModel}</span>
            </div>
            <div className="py-2.5 flex items-center justify-between">
              <span className="text-slate-500">Ядра процессора:</span>
              <span className="font-mono text-slate-900">{status?.device.cpu_count ?? 4} ядер ({status?.device.arch || '—'})</span>
            </div>
            <div className="py-2.5 flex items-center justify-between">
              <span className="text-slate-500">Температура SoC:</span>
              <span className={`font-mono font-medium ${cpuTemp && cpuTemp >= 80 ? 'text-rose-600 font-bold' : 'text-slate-900'}`}>
                {cpuTemp !== null && cpuTemp !== undefined ? `${cpuTemp}°C` : '—'} {cpuTemp && cpuTemp >= 80 ? '(Троттлинг)' : ''}
              </span>
            </div>
            <div className="py-2.5 flex items-center justify-between">
              <span className="text-slate-500">Использование RAM:</span>
              <span className="font-mono text-slate-900">
                {memPercent}% ({formatBytes(memUsed)} / {formatBytes(memTotal)})
              </span>
            </div>
          </div>
        </Card>

        <Card title="База данных и планировщик" subtitle="SQLite WAL и планировщик Tokio">
          <div className="divide-y divide-slate-100 text-xs sm:text-sm">
            <div className="py-2.5 flex items-center justify-between">
              <span className="text-slate-500">Режим базы данных:</span>
              <span className="font-mono font-medium text-slate-900">SQLite WAL</span>
            </div>
            <div className="py-2.5 flex items-center justify-between">
              <span className="text-slate-500">Применённые миграции:</span>
              <span className="font-mono text-slate-900">
                {status?.database.applied_migrations ?? 1} миграций
              </span>
            </div>
            <div className="py-2.5 flex items-center justify-between">
              <span className="text-slate-500">Статус планировщика:</span>
              <span className="inline-flex items-center gap-1.5 font-medium text-slate-700">
                <span
                  className={`w-1.5 h-1.5 rounded-full ${
                    status?.scheduler.running ? 'bg-emerald-500' : 'bg-slate-400'
                  }`}
                />
                {status?.scheduler.running ? 'Работает' : 'Остановлен'}
              </span>
            </div>
            <div className="py-2.5 flex items-center justify-between">
              <span className="text-slate-500">Активных расписаний:</span>
              <span className="font-mono text-slate-900 font-medium">
                {(status?.scheduler.active_alarms_count ?? 0) + (status?.scheduler.pending_reminders_count ?? 0)} задач
              </span>
            </div>
          </div>
        </Card>
      </div>

      {/* Subsystem Readiness Matrix */}
      <div>
        <h3 className="text-xs font-semibold text-slate-500 uppercase tracking-wider mb-3">
          Готовность подсистем ядра
        </h3>

        {status && (
          <div className="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-4 gap-3">
            {Object.entries(status.subsystems).map(([key, stateStr]) => {
              const isReady = stateStr === 'ready' || stateStr === 'connected';
              const isIdle = stateStr === 'idle' || stateStr === 'stopped';
              const desc = subsystemDescriptions[key] || '';
              const keyNameMap: Record<string, string> = {
                audio: 'Аудио',
                bluetooth: 'Bluetooth',
                network: 'Сеть',
                vpn: 'VPN',
                speech: 'Речь',
                ai: 'Служба ИИ',
                scheduler: 'Планировщик',
                music: 'Музыка',
              };

              return (
                <div key={key} className="p-4 rounded-xl bg-white border border-slate-200/80 shadow-xs flex flex-col justify-between">
                  <div className="flex items-center justify-between mb-2">
                    <span className="text-xs font-semibold text-slate-900">{keyNameMap[key] || key}</span>
                    <span className="inline-flex items-center gap-1.5 text-xs text-slate-600">
                      <span
                        className={`w-1.5 h-1.5 rounded-full ${
                          isReady ? 'bg-emerald-500' : isIdle ? 'bg-slate-400' : 'bg-amber-500'
                        }`}
                      />
                      <span>{stateStr}</span>
                    </span>
                  </div>
                  <p className="text-xs text-slate-500 leading-normal">{desc}</p>
                </div>
              );
            })}
          </div>
        )}
      </div>

      {/* Self-Healing / Degradation Simulation Tool */}
      <Card
        title="Тестирование отказоустойчивости"
        subtitle="Проверка сценариев восстановления мастера настройки при деградации компонентов"
      >
        <div className="space-y-3">
          <p className="text-xs text-slate-600 leading-relaxed">
            Имитация сбоя отправляет тестовый сигнал в ядро (<code className="font-mono text-slate-800">POST /api/v1/setup/simulate-degradation</code>). Если компонент деградирован, мастер настройки сохраняет все ранее введенные параметры и перенаправляет на шаг устранения проблемы.
          </p>
          <div className="flex flex-wrap gap-2 pt-1">
            <Button
              variant="outline"
              size="sm"
              disabled={degradeLoading}
              onClick={() => handleSimulate('ai', true)}
            >
              Сбой ИИ
            </Button>
            <Button
              variant="outline"
              size="sm"
              disabled={degradeLoading}
              onClick={() => handleSimulate('speech', true)}
            >
              Сбой речи
            </Button>
            <Button
              variant="outline"
              size="sm"
              disabled={degradeLoading}
              onClick={() => handleSimulate('network', true)}
            >
              Сбой сети
            </Button>
            <Button
              variant="secondary"
              size="sm"
              disabled={degradeLoading}
              onClick={() => {
                handleSimulate('ai', false);
                handleSimulate('speech', false);
                handleSimulate('network', false);
              }}
            >
              Сбросить сбои
            </Button>
          </div>
        </div>
      </Card>

      {/* Runtime Environment Info */}
      <Card
        title="Среда выполнения и хост"
        subtitle="Параметры сборки ядра и системы Buildroot"
      >
        <div className="bg-slate-900 text-slate-100 rounded-lg p-4 font-mono text-xs space-y-1.5 overflow-x-auto">
          <div><span className="text-slate-500">$</span> kamish --version</div>
          <div className="text-slate-300">kamish-backend {status?.version || '0.1.0'} (rustc edition 2021)</div>
          <div><span className="text-slate-500">$</span> uname -srm</div>
          <div className="text-slate-300">{status?.device.os_name} {status?.device.kernel_version} {status?.device.arch}</div>
          <div><span className="text-slate-500">$</span> tokio-runtime</div>
          <div className="text-slate-300">Многопоточный планировщик Tokio, SQLite WAL (активен)</div>
        </div>
      </Card>
    </div>
  );
};
