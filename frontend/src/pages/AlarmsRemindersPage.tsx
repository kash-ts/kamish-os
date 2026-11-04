import React, { useState } from 'react';
import {
  Plus,
  Trash2,
} from 'lucide-react';
import { Button } from '../components/Button';
import { Badge } from '../components/Badge';
import { Toggle } from '../components/Toggle';
import { Modal } from '../components/Modal';
import { formatDateTime, formatRelativeTime } from '../utils/formatters';
import type {
  Alarm,
  CreateAlarmRequest,
  CreateReminderRequest,
  Reminder,
} from '../types';

interface AlarmsRemindersPageProps {
  alarms: Alarm[];
  reminders: Reminder[];
  onCreateAlarm: (data: CreateAlarmRequest) => Promise<void>;
  onToggleAlarm: (id: string, enabled?: boolean) => Promise<void>;
  onDeleteAlarm: (id: string) => Promise<void>;
  onCreateReminder: (data: CreateReminderRequest) => Promise<void>;
  onDeleteReminder: (id: string) => Promise<void>;
  isCreateAlarmOpen: boolean;
  setIsCreateAlarmOpen: (open: boolean) => void;
  isCreateReminderOpen: boolean;
  setIsCreateReminderOpen: (open: boolean) => void;
}

export const AlarmsRemindersPage: React.FC<AlarmsRemindersPageProps> = ({
  alarms,
  reminders,
  onCreateAlarm,
  onToggleAlarm,
  onDeleteAlarm,
  onCreateReminder,
  onDeleteReminder,
  isCreateAlarmOpen,
  setIsCreateAlarmOpen,
  isCreateReminderOpen,
  setIsCreateReminderOpen,
}) => {
  const [activeTab, setActiveTab] = useState<'alarms' | 'reminders'>('alarms');
  const [reminderFilter, setReminderFilter] = useState<string>('all');
  const [actionLoadingId, setActionLoadingId] = useState<string | null>(null);
  const [errorMessage, setErrorMessage] = useState<string | null>(null);

  // Alarm form state
  const [alarmTime, setAlarmTime] = useState<string>('07:30');
  const [alarmLabel, setAlarmLabel] = useState<string>('Будильник');
  const [recurrencePreset, setRecurrencePreset] = useState<string>('daily');
  const [customDays, setCustomDays] = useState<string[]>(['mon', 'wed', 'fri']);
  const [alarmTz, setAlarmTz] = useState<string>(
    Intl.DateTimeFormat().resolvedOptions().timeZone || 'UTC'
  );
  const [isSubmittingAlarm, setIsSubmittingAlarm] = useState(false);

  // Reminder form state
  const [reminderText, setReminderText] = useState<string>('');
  const [reminderDate, setReminderDate] = useState<string>(() => {
    const d = new Date();
    d.setHours(d.getHours() + 1);
    d.setMinutes(0);
    return d.toISOString().slice(0, 16);
  });
  const [reminderTz, setReminderTz] = useState<string>(
    Intl.DateTimeFormat().resolvedOptions().timeZone || 'UTC'
  );
  const [isSubmittingReminder, setIsSubmittingReminder] = useState(false);

  const toggleCustomDay = (day: string) => {
    if (customDays.includes(day)) {
      if (customDays.length > 1) {
        setCustomDays(customDays.filter((d) => d !== day));
      }
    } else {
      setCustomDays([...customDays, day]);
    }
  };

  const handleCreateAlarmSubmit = async (e: React.FormEvent) => {
    e.preventDefault();
    setErrorMessage(null);
    setIsSubmittingAlarm(true);

    try {
      let finalRecurrence: string | null = null;
      if (recurrencePreset === 'daily') finalRecurrence = 'daily';
      else if (recurrencePreset === 'weekdays') finalRecurrence = 'mon-fri';
      else if (recurrencePreset === 'weekends') finalRecurrence = 'sat,sun';
      else if (recurrencePreset === 'custom') finalRecurrence = customDays.join(',');
      else finalRecurrence = null;

      await onCreateAlarm({
        time: alarmTime,
        label: alarmLabel.trim() || undefined,
        recurrence: finalRecurrence,
        timezone: alarmTz,
        enabled: true,
      });

      setIsCreateAlarmOpen(false);
      setAlarmLabel('Будильник');
    } catch (err: any) {
      setErrorMessage(err.message || 'Ошибка создания будильника');
    } finally {
      setIsSubmittingAlarm(false);
    }
  };

  const handleCreateReminderSubmit = async (e: React.FormEvent) => {
    e.preventDefault();
    if (!reminderText.trim()) return;
    setErrorMessage(null);
    setIsSubmittingReminder(true);

    try {
      const isoTime = new Date(reminderDate).toISOString();
      await onCreateReminder({
        trigger_time: isoTime,
        text: reminderText.trim(),
        timezone: reminderTz,
      });

      setIsCreateReminderOpen(false);
      setReminderText('');
    } catch (err: any) {
      setErrorMessage(err.message || 'Ошибка создания напоминания');
    } finally {
      setIsSubmittingReminder(false);
    }
  };

  const handleToggle = async (id: string, current: boolean) => {
    setActionLoadingId(id);
    try {
      await onToggleAlarm(id, !current);
    } finally {
      setActionLoadingId(null);
    }
  };

  const handleDelete = async (id: string, type: 'alarm' | 'reminder') => {
    setActionLoadingId(id);
    try {
      if (type === 'alarm') {
        await onDeleteAlarm(id);
      } else {
        await onDeleteReminder(id);
      }
    } finally {
      setActionLoadingId(null);
    }
  };

  const filteredReminders = reminders.filter((r) => {
    if (reminderFilter === 'pending') return r.status === 'pending';
    if (reminderFilter === 'completed') return r.status === 'completed';
    return true;
  });

  return (
    <div className="space-y-6">
      {/* Header bar */}
      <div className="flex flex-col sm:flex-row sm:items-center justify-between gap-4 p-5 rounded-xl bg-white border border-slate-200/80 shadow-xs">
        <div>
          <h2 className="text-base font-semibold text-slate-900 tracking-tight">
            Будильники и напоминания
          </h2>
          <p className="text-xs text-slate-500 mt-0.5">
            Автономный планировщик расписаний и локальные задачи
          </p>
        </div>

        <div className="flex items-center gap-2">
          <Button
            variant="secondary"
            size="sm"
            onClick={() => setIsCreateAlarmOpen(true)}
            leftIcon={<Plus className="w-3.5 h-3.5" />}
          >
            Новый будильник
          </Button>
          <Button
            variant="primary"
            size="sm"
            onClick={() => setIsCreateReminderOpen(true)}
            leftIcon={<Plus className="w-3.5 h-3.5" />}
          >
            Новое напоминание
          </Button>
        </div>
      </div>

      {/* Tabs */}
      <div className="flex items-center gap-1.5 border-b border-slate-200 pb-2">
        <button
          type="button"
          onClick={() => setActiveTab('alarms')}
          className={`px-4 py-1.5 rounded-lg text-xs font-medium transition-colors cursor-pointer ${
            activeTab === 'alarms'
              ? 'bg-slate-900 text-white shadow-xs'
              : 'text-slate-600 hover:bg-slate-100 hover:text-slate-900'
          }`}
        >
          Будильники ({alarms.length})
        </button>

        <button
          type="button"
          onClick={() => setActiveTab('reminders')}
          className={`px-4 py-1.5 rounded-lg text-xs font-medium transition-colors cursor-pointer ${
            activeTab === 'reminders'
              ? 'bg-slate-900 text-white shadow-xs'
              : 'text-slate-600 hover:bg-slate-100 hover:text-slate-900'
          }`}
        >
          Напоминания ({reminders.length})
        </button>
      </div>

      {/* ALARMS VIEW */}
      {activeTab === 'alarms' && (
        <div className="space-y-4">
          {alarms.length === 0 ? (
            <div className="text-center py-12 bg-white rounded-xl border border-slate-200/80 p-6 shadow-xs">
              <h3 className="text-sm font-semibold text-slate-900">Список будильников пуст</h3>
              <p className="text-xs text-slate-500 mt-1 max-w-sm mx-auto">
                Создайте регулярный или однократный будильник для воспроизведения звукового сигнала.
              </p>
              <Button
                variant="primary"
                size="sm"
                className="mt-4"
                onClick={() => setIsCreateAlarmOpen(true)}
                leftIcon={<Plus className="w-3.5 h-3.5" />}
              >
                Создать будильник
              </Button>
            </div>
          ) : (
            <div className="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-3 gap-3">
              {alarms.map((alarm) => (
                <div
                  key={alarm.id}
                  className={`p-4 rounded-xl bg-white border transition-colors shadow-xs flex flex-col justify-between ${
                    alarm.enabled ? 'border-slate-300' : 'border-slate-200/60 opacity-60'
                  }`}
                >
                  <div>
                    <div className="flex items-center justify-between mb-2">
                      <span className="text-2xl font-bold font-mono text-slate-900 tracking-tight">
                        {alarm.time}
                      </span>
                      <Toggle
                        checked={alarm.enabled}
                        disabled={actionLoadingId === alarm.id}
                        onChange={() => handleToggle(alarm.id, alarm.enabled)}
                      />
                    </div>

                    <div className="text-xs font-medium text-slate-900">{alarm.label || 'Будильник'}</div>

                    <div className="flex flex-wrap items-center gap-1.5 mt-2">
                      {alarm.recurrence ? (
                        <Badge variant="default" size="sm">
                          {alarm.recurrence}
                        </Badge>
                      ) : (
                        <Badge variant="outline" size="sm">
                          Однократно
                        </Badge>
                      )}
                      <span className="text-[11px] font-mono text-slate-400">{alarm.timezone}</span>
                    </div>
                  </div>

                  <div className="mt-4 pt-3 border-t border-slate-100 flex items-center justify-end">
                    <button
                      type="button"
                      onClick={() => handleDelete(alarm.id, 'alarm')}
                      disabled={actionLoadingId === alarm.id}
                      className="p-1.5 text-slate-400 hover:text-rose-600 hover:bg-rose-50 rounded-lg transition-colors cursor-pointer"
                      title="Удалить будильник"
                    >
                      <Trash2 className="w-4 h-4" />
                    </button>
                  </div>
                </div>
              ))}
            </div>
          )}
        </div>
      )}

      {/* REMINDERS VIEW */}
      {activeTab === 'reminders' && (
        <div className="space-y-4">
          {/* Filters */}
          <div className="flex items-center gap-1.5 text-xs">
            <span className="text-slate-400 font-medium mr-1">Фильтр:</span>
            {['all', 'pending', 'completed'].map((f) => (
              <button
                key={f}
                type="button"
                onClick={() => setReminderFilter(f)}
                className={`px-3 py-1 rounded-lg font-medium transition-colors cursor-pointer ${
                  reminderFilter === f
                    ? 'bg-slate-900 text-white shadow-xs'
                    : 'bg-white border border-slate-200 text-slate-600 hover:bg-slate-50'
                }`}
              >
                {f === 'all' ? 'Все' : f === 'pending' ? 'Ожидают' : 'Завершены'}
              </button>
            ))}
          </div>

          {filteredReminders.length === 0 ? (
            <div className="text-center py-12 bg-white rounded-xl border border-slate-200/80 p-6 shadow-xs">
              <h3 className="text-sm font-semibold text-slate-900">Нет напоминаний</h3>
              <p className="text-xs text-slate-500 mt-1 max-w-sm mx-auto">
                Создайте напоминание голосом или через форму для уведомления в назначенное время.
              </p>
              <Button
                variant="primary"
                size="sm"
                className="mt-4"
                onClick={() => setIsCreateReminderOpen(true)}
                leftIcon={<Plus className="w-3.5 h-3.5" />}
              >
                Добавить напоминание
              </Button>
            </div>
          ) : (
            <div className="bg-white rounded-xl border border-slate-200/80 shadow-xs divide-y divide-slate-100 p-4">
              {filteredReminders.map((reminder) => (
                <div
                  key={reminder.id}
                  className="py-3 flex items-center justify-between gap-4 first:pt-0 last:pb-0"
                >
                  <div className="overflow-hidden flex-1">
                    <div className="flex items-center gap-2 mb-1">
                      <span className="inline-flex items-center gap-1.5 text-xs font-medium text-slate-700">
                        <span
                          className={`w-1.5 h-1.5 rounded-full ${
                            reminder.status === 'completed'
                              ? 'bg-emerald-500'
                              : reminder.status === 'pending'
                              ? 'bg-amber-500'
                              : 'bg-slate-400'
                          }`}
                        />
                        {reminder.status === 'completed' ? 'Завершено' : 'Ожидает'}
                      </span>
                      <span className="text-xs text-slate-400">
                        {formatDateTime(reminder.trigger_time)}
                      </span>
                      <span className="text-xs text-slate-500 font-medium">
                        ({formatRelativeTime(reminder.trigger_time)})
                      </span>
                    </div>
                    <p className="text-xs font-medium text-slate-900">{reminder.text}</p>
                  </div>

                  <button
                    type="button"
                    onClick={() => handleDelete(reminder.id, 'reminder')}
                    disabled={actionLoadingId === reminder.id}
                    className="p-1.5 text-slate-400 hover:text-rose-600 hover:bg-rose-50 rounded-lg transition-colors cursor-pointer"
                    title="Удалить напоминание"
                  >
                    <Trash2 className="w-4 h-4" />
                  </button>
                </div>
              ))}
            </div>
          )}
        </div>
      )}

      {/* MODAL: CREATE ALARM */}
      <Modal
        isOpen={isCreateAlarmOpen}
        onClose={() => setIsCreateAlarmOpen(false)}
        title="Новый будильник"
        description="Задайте время срабатывания и правило повторения"
      >
        <form onSubmit={handleCreateAlarmSubmit} className="space-y-4">
          {errorMessage && (
            <div className="p-3 bg-rose-50 text-rose-700 rounded-lg text-xs">
              {errorMessage}
            </div>
          )}

          <div>
            <label className="block text-xs font-medium text-slate-700 mb-1">
              Время срабатывания (ЧЧ:ММ)
            </label>
            <input
              type="time"
              value={alarmTime}
              onChange={(e) => setAlarmTime(e.target.value)}
              required
              className="w-full text-base font-mono bg-white border border-slate-200 rounded-lg px-3 py-2 text-slate-900"
            />
          </div>

          <div>
            <label className="block text-xs font-medium text-slate-700 mb-1">
              Название
            </label>
            <input
              type="text"
              value={alarmLabel}
              onChange={(e) => setAlarmLabel(e.target.value)}
              placeholder="Например: Пробуждение"
              className="w-full text-xs bg-white border border-slate-200 rounded-lg px-3 py-2 text-slate-900"
            />
          </div>

          <div>
            <label className="block text-xs font-medium text-slate-700 mb-1">
              Повторение
            </label>
            <select
              value={recurrencePreset}
              onChange={(e) => setRecurrencePreset(e.target.value)}
              className="w-full text-xs bg-white border border-slate-200 rounded-lg px-3 py-2 text-slate-900"
            >
              <option value="once">Однократно</option>
              <option value="daily">Ежедневно</option>
              <option value="weekdays">По будням (Пн-Пт)</option>
              <option value="weekends">По выходным (Сб-Вс)</option>
              <option value="custom">Выбранные дни недели</option>
            </select>
          </div>

          {recurrencePreset === 'custom' && (
            <div className="flex gap-1.5 pt-1">
              {[
                { id: 'mon', l: 'Пн' },
                { id: 'tue', l: 'Вт' },
                { id: 'wed', l: 'Ср' },
                { id: 'thu', l: 'Чт' },
                { id: 'fri', l: 'Пт' },
                { id: 'sat', l: 'Сб' },
                { id: 'sun', l: 'Вс' },
              ].map((d) => (
                <button
                  key={d.id}
                  type="button"
                  onClick={() => toggleCustomDay(d.id)}
                  className={`flex-1 py-1.5 text-xs font-medium rounded-lg border transition-colors ${
                    customDays.includes(d.id)
                      ? 'bg-slate-900 text-white border-slate-900'
                      : 'bg-white text-slate-700 border-slate-200 hover:bg-slate-50'
                  }`}
                >
                  {d.l}
                </button>
              ))}
            </div>
          )}

          <div className="flex justify-end gap-2 pt-2">
            <Button
              type="button"
              variant="outline"
              size="sm"
              onClick={() => setIsCreateAlarmOpen(false)}
            >
              Отмена
            </Button>
            <Button
              type="submit"
              variant="primary"
              size="sm"
              isLoading={isSubmittingAlarm}
            >
              Сохранить будильник
            </Button>
          </div>
        </form>
      </Modal>

      {/* MODAL: CREATE REMINDER */}
      <Modal
        isOpen={isCreateReminderOpen}
        onClose={() => setIsCreateReminderOpen(false)}
        title="Новое напоминание"
        description="Задайте текст напоминания и время срабатывания"
      >
        <form onSubmit={handleCreateReminderSubmit} className="space-y-4">
          {errorMessage && (
            <div className="p-3 bg-rose-50 text-rose-700 rounded-lg text-xs">
              {errorMessage}
            </div>
          )}

          <div>
            <label className="block text-xs font-medium text-slate-700 mb-1">
              Текст напоминания
            </label>
            <input
              type="text"
              value={reminderText}
              onChange={(e) => setReminderText(e.target.value)}
              placeholder="Например: Выключить духовку"
              required
              className="w-full text-xs bg-white border border-slate-200 rounded-lg px-3 py-2 text-slate-900"
            />
          </div>

          <div>
            <label className="block text-xs font-medium text-slate-700 mb-1">
              Дата и время
            </label>
            <input
              type="datetime-local"
              value={reminderDate}
              onChange={(e) => setReminderDate(e.target.value)}
              required
              className="w-full text-xs bg-white border border-slate-200 rounded-lg px-3 py-2 text-slate-900"
            />
          </div>

          <div className="flex justify-end gap-2 pt-2">
            <Button
              type="button"
              variant="outline"
              size="sm"
              onClick={() => setIsCreateReminderOpen(false)}
            >
              Отмена
            </Button>
            <Button
              type="submit"
              variant="primary"
              size="sm"
              isLoading={isSubmittingReminder}
            >
              Создать напоминание
            </Button>
          </div>
        </form>
      </Modal>
    </div>
  );
};
