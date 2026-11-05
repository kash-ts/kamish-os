import React, { useState, useEffect } from 'react';
import { RefreshCw, AlertTriangle } from 'lucide-react';
import { Card } from '../components/Card';
import { Button } from '../components/Button';
import { getHardwareTelemetry, getStatus } from '../api';
import { formatBytes, formatUptime } from '../utils/formatters';
import type { HardwareTelemetry, StatusResponse } from '../types';

export const SystemPage: React.FC = () => {
  const [telemetry, setTelemetry] = useState<HardwareTelemetry | null>(null);
  const [status, setStatus] = useState<StatusResponse | null>(null);
  const [isLoading, setIsLoading] = useState(false);
  const [lastUpdated, setLastUpdated] = useState<string>('');

  const refreshData = async () => {
    setIsLoading(true);
    try {
      const [telemRes, statusRes] = await Promise.all([
        getHardwareTelemetry().catch(() => null),
        getStatus().catch(() => null),
      ]);
      if (telemRes) setTelemetry(telemRes);
      if (statusRes) setStatus(statusRes);
      setLastUpdated(new Date().toLocaleTimeString());
    } finally {
      setIsLoading(false);
    }
  };

  useEffect(() => {
    refreshData();
    const interval = setInterval(refreshData, 10000);
    return () => clearInterval(interval);
  }, []);

  const temp = telemetry?.cpu_temp_celsius ?? status?.device.cpu_temp_celsius ?? null;
  const hasTemp = temp !== null && temp !== undefined;
  const isThrottling = telemetry?.throttled ?? (hasTemp && temp >= 80);

  const memUsed = telemetry?.memory_used_bytes ?? status?.device.memory_used_bytes ?? 0;
  const memTotal = telemetry?.memory_total_bytes ?? status?.device.memory_total_bytes ?? (1024 * 1024 * 1024);
  const memPercent = Math.min(100, Math.round((memUsed / (memTotal || 1)) * 100));

  const storageUsed = telemetry?.storage_used_bytes ?? status?.device.storage_used_bytes ?? 0;
  const storageTotal = telemetry?.storage_total_bytes ?? status?.device.storage_total_bytes ?? 0;
  const storagePercent = storageTotal > 0 ? Math.min(100, Math.round((storageUsed / storageTotal) * 100)) : 0;

  const hardwareModel = status?.device.hardware_model || telemetry?.model || 'kamish';
  const osString = [status?.device.os_name, status?.device.os_version].filter(Boolean).join(' ') || 'Kamish Linux (Buildroot)';

  return (
    <div className="space-y-6">
      {/* Header bar */}
      <div className="flex flex-col sm:flex-row items-start sm:items-center justify-between gap-4 p-5 rounded-xl bg-white border border-slate-200/80 shadow-xs">
        <div>
          <h2 className="text-base font-semibold text-slate-900">
            Аппаратный мониторинг
          </h2>
          <p className="text-xs text-slate-500 mt-0.5">
            Платформа {hardwareModel} под управлением {osString}
          </p>
        </div>
        <div className="flex items-center gap-3">
          {lastUpdated && (
            <span className="text-xs text-slate-400 font-mono">Обновлено: {lastUpdated}</span>
          )}
          <Button
            variant="outline"
            size="sm"
            onClick={refreshData}
            isLoading={isLoading}
            leftIcon={<RefreshCw className={`w-3.5 h-3.5 ${isLoading ? 'animate-spin' : ''}`} />}
          >
            Обновить
          </Button>
        </div>
      </div>

      {/* Throttling Alert Banner */}
      {isThrottling && (
        <div className="p-4 rounded-xl bg-rose-50 border border-rose-200/80 text-rose-900 flex items-start gap-3">
          <AlertTriangle className="w-5 h-5 text-rose-600 shrink-0 mt-0.5" />
          <div>
            <h4 className="text-sm font-semibold">Аппаратный троттлинг активен</h4>
            <p className="text-xs text-rose-800 mt-0.5">
              Температура процессора достигла {temp}°C (порог ≥ 80°C). Частота процессорных ядер снижена для предотвращения перегрева {hardwareModel}.
            </p>
          </div>
        </div>
      )}

      {/* 4 Metric Cards (Clean minimal style, no decorative Lucide icons) */}
      <div className="grid grid-cols-1 sm:grid-cols-2 lg:grid-cols-4 gap-4">
        {/* CPU Temp */}
        <Card className="flex flex-col justify-between">
          <div className="text-xs font-medium text-slate-500 mb-2">
            Температура SoC
          </div>
          <div>
            <div className="flex items-baseline gap-2 mb-2">
              <span className={`text-2xl font-bold font-mono ${isThrottling ? 'text-rose-600' : 'text-slate-900'}`}>
                {hasTemp ? `${temp}°C` : '—'}
              </span>
              <span className="text-xs text-slate-400">
                {hasTemp ? (isThrottling ? 'Троттлинг' : 'Норма (<80°C)') : 'Датчик недоступен'}
              </span>
            </div>
            {hasTemp ? (
              <div className="w-full bg-slate-100 rounded-full h-2 overflow-hidden">
                <div
                  className={`h-2 rounded-full transition-all duration-300 ${
                    temp >= 80 ? 'bg-rose-500' : temp >= 65 ? 'bg-amber-500' : 'bg-emerald-500'
                  }`}
                  style={{ width: `${Math.min(100, (temp / 100) * 100)}%` }}
                />
              </div>
            ) : (
              <div className="w-full bg-slate-100 rounded-full h-2 overflow-hidden" />
            )}
          </div>
        </Card>

        {/* RAM */}
        <Card className="flex flex-col justify-between">
          <div className="text-xs font-medium text-slate-500 mb-2">
            Оперативная память
          </div>
          <div>
            <div className="flex items-baseline justify-between mb-2">
              <span className="text-2xl font-bold text-slate-900">{memPercent}%</span>
              <span className="text-xs font-mono text-slate-500">
                {formatBytes(memUsed)} / {formatBytes(memTotal)}
              </span>
            </div>
            <div className="w-full bg-slate-100 rounded-full h-2 overflow-hidden">
              <div
                className="h-2 rounded-full bg-slate-700 transition-all duration-300"
                style={{ width: `${memPercent}%` }}
              />
            </div>
          </div>
        </Card>

        {/* Storage */}
        <Card className="flex flex-col justify-between">
          <div className="text-xs font-medium text-slate-500 mb-2">
            Накопитель
          </div>
          <div>
            <div className="flex items-baseline justify-between mb-2">
              <span className="text-2xl font-bold text-slate-900">
                {storageTotal > 0 ? `${storagePercent}%` : '—'}
              </span>
              <span className="text-xs font-mono text-slate-500">
                {storageTotal > 0 ? `${formatBytes(storageUsed)} / ${formatBytes(storageTotal)}` : 'Недоступно'}
              </span>
            </div>
            <div className="w-full bg-slate-100 rounded-full h-2 overflow-hidden">
              <div
                className="h-2 rounded-full bg-slate-700 transition-all duration-300"
                style={{ width: `${storagePercent}%` }}
              />
            </div>
          </div>
        </Card>

        {/* System Uptime */}
        <Card className="flex flex-col justify-between">
          <div className="text-xs font-medium text-slate-500 mb-2">
            Время работы
          </div>
          <div>
            <div className="text-xl font-bold text-slate-900 mb-1 font-mono">
              {status ? formatUptime(status.device.system_uptime_secs) : '--'}
            </div>
            <div className="text-xs text-slate-500 font-mono">
              Процесс: {status ? formatUptime(status.app_uptime_secs) : '--'}
            </div>
          </div>
        </Card>
      </div>

      {/* Hardware Specs Key-Value Table */}
      <Card title="Спецификация аппаратного узла" subtitle={`Параметры вычислительного модуля ${hardwareModel}`}>
        <div className="divide-y divide-slate-100 text-xs sm:text-sm">
          <div className="py-3 flex items-center justify-between">
            <span className="text-slate-500">Процессор (SoC):</span>
            <span className="font-mono text-slate-900 font-medium">
              {hardwareModel} ({status?.device.cpu_count || telemetry?.cpu_cores || 4} ядра{telemetry?.cpu_freq_mhz ? ` @ ${(telemetry.cpu_freq_mhz / 1000).toFixed(1)} GHz` : ''})
            </span>
          </div>
          <div className="py-3 flex items-center justify-between">
            <span className="text-slate-500">Архитектура:</span>
            <span className="font-mono text-slate-900">{status?.device.arch || telemetry?.cpu_arch || '—'}</span>
          </div>
          <div className="py-3 flex items-center justify-between">
            <span className="text-slate-500">Операционная система:</span>
            <span className="text-slate-900 font-medium">{osString}</span>
          </div>
          <div className="py-3 flex items-center justify-between">
            <span className="text-slate-500">Версия ядра Linux:</span>
            <span className="font-mono text-slate-900">{status?.device.kernel_version || '—'}</span>
          </div>
          <div className="py-3 flex items-center justify-between">
            <span className="text-slate-500">Сетевое имя узла (Hostname):</span>
            <span className="font-mono text-slate-900 font-medium">{status?.device.hostname || '—'}</span>
          </div>
          <div className="py-3 flex items-center justify-between">
            <span className="text-slate-500">Микрофонный массив:</span>
            <span className="text-slate-900 font-medium">
              {telemetry?.respeaker.detected
                ? `${telemetry.respeaker.card_name || 'Seeed ReSpeaker XVF3800'} (USB Audio / 4ch VAD)`
                : 'Не обнаружен (Эмуляция VAD)'}
            </span>
          </div>
          <div className="py-3 flex items-center justify-between">
            <span className="text-slate-500">Акустическая система:</span>
            <span className="text-slate-900 font-medium">
              {telemetry?.audio_routing.output_device || 'Аудиовыход'} (Лимитер ≤ {telemetry?.audio_routing.safe_volume_ceiling || 85}%)
            </span>
          </div>
          <div className="py-3 flex items-center justify-between">
            <span className="text-slate-500">База данных конфигураций:</span>
            <span className="text-slate-900 font-medium">SQLite WAL</span>
          </div>
        </div>
      </Card>
    </div>
  );
};
