import React from 'react';
import {
  RefreshCw,
  Plus,
  ArrowRight,
  Copy,
  Check,
  ExternalLink,
} from 'lucide-react';
import { Card } from '../components/Card';
import { Button } from '../components/Button';
import { Badge } from '../components/Badge';
import { AiAssistant } from '../components/AiAssistant';
import { formatBytes, formatUptime, formatDateTime, formatRelativeTime } from '../utils/formatters';
import type { Alarm, Reminder, StatusResponse } from '../types';
import type { NavPage } from '../components/Sidebar';

interface DashboardPageProps {
  status: StatusResponse | null;
  alarms: Alarm[];
  reminders: Reminder[];
  isLoading: boolean;
  onRefresh: () => void;
  onNavigate: (page: NavPage) => void;
  onOpenCreateAlarm: () => void;
  onOpenCreateReminder: () => void;
}

export const DashboardPage: React.FC<DashboardPageProps> = ({
  status,
  alarms,
  reminders,
  isLoading,
  onRefresh,
  onNavigate,
  onOpenCreateAlarm,
  onOpenCreateReminder,
}) => {
  const [copied, setCopied] = React.useState(false);
  const localUrl = status?.device?.local_url || 'http://kamish.local';

  const handleCopy = () => {
    if (navigator.clipboard) {
      navigator.clipboard.writeText(localUrl);
      setCopied(true);
      setTimeout(() => setCopied(false), 2000);
    }
  };

  const memUsed = status?.device?.memory_used_bytes || 0;
  const memTotal = status?.device?.memory_total_bytes || 1;
  const memPercent = Math.min(100, Math.round((memUsed / memTotal) * 100));

  const activeAlarms = alarms.filter((a) => a.enabled);
  const pendingReminders = reminders.filter((r) => r.status === 'pending');

  return (
    <div className="space-y-6">
      {/* Top Banner / Device Summary */}
      <div className="rounded-xl bg-white border border-slate-200/80 p-6 shadow-xs">
        <div className="flex flex-col sm:flex-row sm:items-center sm:justify-between gap-6">
          <div className="space-y-2">
            <div className="flex items-center gap-2">
              <span className="w-1.5 h-1.5 rounded-full bg-emerald-500" />
              <span className="text-xs font-medium text-slate-600">
                Устройство в сети • Модель: {status?.hardware?.model || status?.device.hardware_model || 'kamish'}
              </span>
            </div>
            <h1 className="text-xl sm:text-2xl font-bold text-slate-900 tracking-tight">
              Камыш ({status?.device.hostname || 'kamish'})
            </h1>
            <div className="flex flex-wrap items-center gap-2 pt-0.5">
              <div className="inline-flex items-center gap-2 px-3 py-1 rounded-lg bg-slate-100 text-xs font-mono text-slate-700">
                <span className="text-slate-400">mDNS:</span>
                <span className="text-slate-900 font-semibold">{localUrl}</span>
                <button
                  type="button"
                  onClick={handleCopy}
                  title="Скопировать адрес"
                  className="p-0.5 hover:text-slate-900 text-slate-400 cursor-pointer transition-colors"
                >
                  {copied ? (
                    <Check className="w-3.5 h-3.5 text-emerald-600" />
                  ) : (
                    <Copy className="w-3.5 h-3.5" />
                  )}
                </button>
                <a
                  href={localUrl}
                  target="_blank"
                  rel="noreferrer"
                  title="Открыть в новой вкладке"
                  className="p-0.5 hover:text-slate-900 text-slate-400 transition-colors"
                >
                  <ExternalLink className="w-3.5 h-3.5" />
                </a>
              </div>
              <span className="text-xs text-slate-500 font-normal hidden md:inline">
                (доступно по адресу {localUrl} в домашней сети)
              </span>
            </div>
          </div>

          <div className="flex flex-wrap items-center gap-2">
            <Button
              variant="outline"
              size="sm"
              onClick={onRefresh}
              isLoading={isLoading}
              leftIcon={<RefreshCw className={`w-3.5 h-3.5 ${isLoading ? 'animate-spin' : ''}`} />}
            >
              Обновить
            </Button>
            <Button
              variant="secondary"
              size="sm"
              onClick={onOpenCreateAlarm}
              leftIcon={<Plus className="w-3.5 h-3.5" />}
            >
              Будильник
            </Button>
            <Button
              variant="primary"
              size="sm"
              onClick={onOpenCreateReminder}
              leftIcon={<Plus className="w-3.5 h-3.5" />}
            >
              Напоминание
            </Button>
          </div>
        </div>
      </div>

      {/* Metrics Row: Pure numbers and progress bars, no decorative icon boxes */}
      <div className="grid grid-cols-1 sm:grid-cols-2 lg:grid-cols-4 gap-4">
        {/* RAM Usage */}
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
                className={`h-2 rounded-full transition-all duration-300 ${
                  memPercent > 85 ? 'bg-rose-500' : memPercent > 65 ? 'bg-amber-500' : 'bg-slate-700'
                }`}
                style={{ width: `${memPercent}%` }}
              />
            </div>
          </div>
        </Card>

        {/* CPU */}
        <Card className="flex flex-col justify-between">
          <div className="text-xs font-medium text-slate-500 mb-2">
            Процессор SoC
          </div>
          <div>
            <div className="text-2xl font-bold text-slate-900 mb-1">
              {status?.hardware?.cpu_cores ?? status?.device.cpu_count ?? 4} <span className="text-xs font-normal text-slate-500">ядер</span>
            </div>
            <div className="text-xs font-mono text-slate-500 truncate">
              {status?.hardware?.cpu_arch || status?.device.arch || '—'} • {status?.hardware?.model || status?.device.hardware_model || 'kamish'}
            </div>
          </div>
        </Card>

        {/* Uptime */}
        <Card className="flex flex-col justify-between">
          <div className="text-xs font-medium text-slate-500 mb-2">
            Время работы
          </div>
          <div>
            <div className="text-2xl font-bold text-slate-900 mb-1 font-mono text-xl">
              {status ? formatUptime(status.app_uptime_secs) : '--'}
            </div>
            <div className="text-xs text-slate-500 font-mono">
              Система: {status && status.device?.system_uptime_secs !== undefined ? formatUptime(status.device.system_uptime_secs) : '--'}
            </div>
          </div>
        </Card>

        {/* Scheduler Overview */}
        <Card className="flex flex-col justify-between">
          <div className="text-xs font-medium text-slate-500 mb-2">
            Планировщик
          </div>
          <div>
            <div className="flex items-baseline gap-2 mb-1">
              <span className="text-2xl font-bold text-slate-900">
                {activeAlarms.length}
              </span>
              <span className="text-xs text-slate-500">активных будильников</span>
            </div>
            <div className="text-xs text-slate-500">
              <span className="font-semibold text-slate-800">{pendingReminders.length}</span> напоминаний в очереди
            </div>
          </div>
        </Card>
      </div>

      {/* AI Assistant Command Panel */}
      <AiAssistant onToolExecuted={onRefresh} />

      {/* Main Grid: Active Alarms & Pending Reminders */}
      <div className="grid grid-cols-1 lg:grid-cols-2 gap-6">
        {/* Active Alarms Card */}
        <Card
          title="Активные будильники"
          badge={
            <Badge variant={activeAlarms.length > 0 ? 'success' : 'default'} size="sm">
              {activeAlarms.length} активных
            </Badge>
          }
          action={
            <Button
              variant="ghost"
              size="sm"
              onClick={() => onNavigate('alarms')}
              rightIcon={<ArrowRight className="w-3.5 h-3.5" />}
            >
              Все
            </Button>
          }
        >
          {activeAlarms.length === 0 ? (
            <div className="text-center py-6 text-slate-500">
              <p className="text-xs">Нет активных будильников</p>
              <Button
                variant="outline"
                size="sm"
                className="mt-3"
                onClick={onOpenCreateAlarm}
                leftIcon={<Plus className="w-3.5 h-3.5" />}
              >
                Поставить будильник
              </Button>
            </div>
          ) : (
            <div className="divide-y divide-slate-100">
              {activeAlarms.slice(0, 4).map((alarm) => (
                <div
                  key={alarm.id}
                  className="flex items-center justify-between py-3 first:pt-0 last:pb-0"
                >
                  <div>
                    <div className="flex items-center gap-2">
                      <span className="text-base font-bold font-mono text-slate-900 tracking-tight">
                        {alarm.time}
                      </span>
                      {alarm.recurrence ? (
                        <Badge variant="default" size="sm">
                          {alarm.recurrence}
                        </Badge>
                      ) : (
                        <Badge variant="outline" size="sm">
                          Однократно
                        </Badge>
                      )}
                    </div>
                    <p className="text-xs text-slate-500 mt-0.5">{alarm.label || 'Будильник'}</p>
                  </div>
                  <span className="inline-flex items-center gap-1.5 text-xs text-emerald-700 font-medium">
                    <span className="w-1.5 h-1.5 rounded-full bg-emerald-500" />
                    Включён
                  </span>
                </div>
              ))}
              {activeAlarms.length > 4 && (
                <p className="text-xs text-center text-slate-400 pt-3">
                  +{activeAlarms.length - 4} ещё будильников
                </p>
              )}
            </div>
          )}
        </Card>

        {/* Pending Reminders Card */}
        <Card
          title="Ближайшие напоминания"
          badge={
            <Badge variant={pendingReminders.length > 0 ? 'warning' : 'default'} size="sm">
              {pendingReminders.length} ожидают
            </Badge>
          }
          action={
            <Button
              variant="ghost"
              size="sm"
              onClick={() => onNavigate('alarms')}
              rightIcon={<ArrowRight className="w-3.5 h-3.5" />}
            >
              Все
            </Button>
          }
        >
          {pendingReminders.length === 0 ? (
            <div className="text-center py-6 text-slate-500">
              <p className="text-xs">Нет запланированных напоминаний</p>
              <Button
                variant="outline"
                size="sm"
                className="mt-3"
                onClick={onOpenCreateReminder}
                leftIcon={<Plus className="w-3.5 h-3.5" />}
              >
                Добавить напоминание
              </Button>
            </div>
          ) : (
            <div className="divide-y divide-slate-100">
              {pendingReminders.slice(0, 4).map((reminder) => (
                <div
                  key={reminder.id}
                  className="flex items-center justify-between py-3 first:pt-0 last:pb-0"
                >
                  <div className="pr-4">
                    <p className="text-xs font-medium text-slate-900 line-clamp-1">{reminder.text}</p>
                    <div className="flex items-center gap-2 mt-0.5 text-xs text-slate-400">
                      <span>{formatDateTime(reminder.trigger_time)}</span>
                      <span>•</span>
                      <span className="text-slate-600 font-medium">
                        {formatRelativeTime(reminder.trigger_time)}
                      </span>
                    </div>
                  </div>
                  <span className="inline-flex items-center gap-1.5 text-xs text-amber-700 font-medium shrink-0">
                    <span className="w-1.5 h-1.5 rounded-full bg-amber-500" />
                    Ожидает
                  </span>
                </div>
              ))}
              {pendingReminders.length > 4 && (
                <p className="text-xs text-center text-slate-400 pt-3">
                  +{pendingReminders.length - 4} ещё напоминаний
                </p>
              )}
            </div>
          )}
        </Card>
      </div>

      {/* Subsystem Health Matrix */}
      {status && (
        <Card
          title="Компоненты системы"
          subtitle="Диагностика аппаратных модулей и служб ядра"
          action={
            <Button
              variant="ghost"
              size="sm"
              onClick={() => onNavigate('diagnostics')}
              rightIcon={<ArrowRight className="w-3.5 h-3.5" />}
            >
              Подробно
            </Button>
          }
        >
          <div className="grid grid-cols-2 sm:grid-cols-4 gap-3">
            {Object.entries(status.subsystems).map(([name, subStatus]) => {
              const isReady = subStatus === 'ready' || subStatus === 'connected';
              const isIdle = subStatus === 'idle' || subStatus === 'stopped';
              const nameMap: Record<string, string> = {
                audio: 'Аудио',
                bluetooth: 'Bluetooth',
                network: 'Сеть',
                vpn: 'VPN',
                speech: 'Речь',
                ai: 'Служба ИИ',
                scheduler: 'Планировщик',
                music: 'Музыка',
              };
              const statusMap: Record<string, string> = {
                ready: 'готов',
                connected: 'в сети',
                idle: 'ожидание',
                stopped: 'остановлен',
                disabled: 'отключен',
                error: 'ошибка',
              };
              return (
                <div
                  key={name}
                  className="p-3 rounded-lg bg-slate-50/70 flex items-center justify-between"
                >
                  <span className="text-xs font-medium text-slate-700">{nameMap[name] || name}</span>
                  <span className="inline-flex items-center gap-1.5 text-xs text-slate-600">
                    <span
                      className={`w-1.5 h-1.5 rounded-full ${
                        isReady ? 'bg-emerald-500' : isIdle ? 'bg-slate-400' : 'bg-amber-500'
                      }`}
                    />
                    <span>{statusMap[subStatus] || subStatus}</span>
                  </span>
                </div>
              );
            })}
          </div>
        </Card>
      )}

      {/* Offline-First Note */}
      <div className="p-4 rounded-xl bg-white border border-slate-200/80 text-xs text-slate-500 shadow-xs">
        <strong className="text-slate-800 font-medium">Автономная работа:</strong> Будильники и напоминания выполняются локально на встроенном планировщике и базе данных SQLite без обязательного подключения к внешним облачным сервисам.
      </div>
    </div>
  );
};
