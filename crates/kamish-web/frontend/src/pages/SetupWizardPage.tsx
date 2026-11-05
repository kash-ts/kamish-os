import React, { useState, useEffect, useCallback, useRef } from 'react';
import {
  Check,
  ChevronRight,
  AlertTriangle,
  Play,
  RotateCcw,
  CheckCircle2,
  AlertCircle,
  Loader2,
} from 'lucide-react';
import {
  getSetupStatus,
  saveSetupStep,
  completeSetup,
  getNetworkStatus,
  scanWifi,
  connectWifi,
  setAudioVolume,
  setVolumeCeiling,
  playAudioTest,
  getSpeechStatus,
  synthesizeSpeech,
  getAiStatus,
  sendAiPrompt,
} from '../api';
import type {
  SetupStepId,
  SetupStatusResponse,
  NetworkStatus,
  WifiNetwork,
  SpeechStatus,
  AiReadiness,
} from '../types';

interface SetupWizardPageProps {
  onSetupComplete: () => void;
  onForcedPasswordRequired?: () => void;
}

export const SetupWizardPage: React.FC<SetupWizardPageProps> = ({
  onSetupComplete,
  onForcedPasswordRequired,
}) => {
  const [setupStatus, setSetupStatus] = useState<SetupStatusResponse | null>(null);
  const [currentStep, setCurrentStep] = useState<SetupStepId>('network');
  const initialLoadedRef = useRef<boolean>(false);
  const [isLoading, setIsLoading] = useState<boolean>(true);
  const [actionLoading, setActionLoading] = useState<boolean>(false);
  const [errorMessage, setErrorMessage] = useState<string | null>(null);
  const [successMessage, setSuccessMessage] = useState<string | null>(null);

  // Stage 2: Network state
  const [networkStatus, setNetworkStatus] = useState<NetworkStatus | null>(null);
  const [wifiNetworks, setWifiNetworks] = useState<WifiNetwork[]>([]);
  const [isScanningWifi, setIsScanningWifi] = useState(false);
  const [selectedSsid, setSelectedSsid] = useState('');
  const [wifiPassword, setWifiPassword] = useState('');

  // Stage 3: Audio state
  const [volume, setVolume] = useState<number>(60);
  const [ceiling, setCeiling] = useState<number>(85);
  const [isPlayingTestTone, setIsPlayingTestTone] = useState(false);

  // Stage 4: Voice state
  const [speechStatus, setSpeechStatus] = useState<SpeechStatus | null>(null);
  const [isTestingSpeech, setIsTestingSpeech] = useState(false);

  // Stage 5: AI state
  const [aiStatus, setAiStatus] = useState<AiReadiness | null>(null);
  const [aiTestResponse, setAiTestResponse] = useState<string | null>(null);
  const [isTestingAi, setIsTestingAi] = useState(false);

  // Load setup status from the real service
  const loadStatus = useCallback(async () => {
    try {
      const res = await getSetupStatus();
      setSetupStatus(res);
      if (res.is_completed) {
        onSetupComplete();
        return;
      }
      // Only set currentStep on the very first mount so background polling never kicks user out
      if (!initialLoadedRef.current && res.current_step) {
        initialLoadedRef.current = true;
        setCurrentStep(res.current_step);
      }
      if (res.steps?.auth?.must_change_password && onForcedPasswordRequired) {
        onForcedPasswordRequired();
      }
    } catch (err: any) {
      console.warn('Failed to load setup status, fallback to step network:', err);
    } finally {
      setIsLoading(false);
    }
  }, [onSetupComplete, onForcedPasswordRequired]);

  useEffect(() => {
    loadStatus();
  }, [loadStatus]);

  // Load step-specific data when step changes
  useEffect(() => {
    if (currentStep === 'network') {
      getNetworkStatus().then(setNetworkStatus).catch(console.warn);
      handleScanWifi();
    } else if (currentStep === 'voice') {
      getSpeechStatus().then(setSpeechStatus).catch(console.warn);
    } else if (currentStep === 'ai') {
      getAiStatus().then(setAiStatus).catch(console.warn);
    }
  }, [currentStep]);

  const handleScanWifi = async () => {
    setIsScanningWifi(true);
    try {
      const res = await scanWifi();
      setWifiNetworks(res.networks || []);
    } catch (err) {
      console.warn('Wi-Fi scan error:', err);
    } finally {
      setIsScanningWifi(false);
    }
  };

  const handleConnectWifi = async (e: React.FormEvent) => {
    e.preventDefault();
    if (!selectedSsid) return;
    setActionLoading(true);
    setErrorMessage(null);
    try {
      const res = await connectWifi({ ssid: selectedSsid, password: wifiPassword });
      setSuccessMessage(`Успешное подключение к ${res.ssid}`);
      const updatedNet = await getNetworkStatus();
      setNetworkStatus(updatedNet);
      setWifiPassword('');
    } catch (err: any) {
      setErrorMessage(err.message || 'Ошибка подключения к сети');
    } finally {
      setActionLoading(false);
    }
  };

  const handleAudioTest = async () => {
    setIsPlayingTestTone(true);
    setErrorMessage(null);
    try {
      await playAudioTest();
      setSuccessMessage('Звуковой сигнал воспроизведен на динамике');
    } catch {
      try {
        await synthesizeSpeech({
          text: 'Проверка звукового тракта умной колонки Камыш',
          play_on_speaker: true,
        });
        setSuccessMessage('Звуковой сигнал успешно воспроизведен');
      } catch (err: any) {
        setErrorMessage(err.message || 'Ошибка воспроизведения тестового сигнала');
      }
    } finally {
      setIsPlayingTestTone(false);
    }
  };

  const handleApplyAudio = async (newVol: number, newCeil: number) => {
    const safeCeil = Math.min(newCeil, 85);
    const safeVol = Math.min(newVol, safeCeil);
    setCeiling(safeCeil);
    setVolume(safeVol);
    try {
      await setVolumeCeiling(safeCeil);
      await setAudioVolume(safeVol);
    } catch (err) {
      console.warn('Failed to update audio settings:', err);
    }
  };

  const handleVoiceTest = async () => {
    setIsTestingSpeech(true);
    setErrorMessage(null);
    try {
      await synthesizeSpeech({
        text: 'Синтез речи работает штатно',
        play_on_speaker: true,
      });
      setSuccessMessage('Синтез речи проверен');
      const updated = await getSpeechStatus();
      setSpeechStatus(updated);
    } catch (err: any) {
      setErrorMessage(err.message || 'Ошибка тестирования синтеза речи');
    } finally {
      setIsTestingSpeech(false);
    }
  };

  const handleAiTest = async () => {
    setIsTestingAi(true);
    setErrorMessage(null);
    try {
      const res = await sendAiPrompt('Проверка готовности голосового ассистента');
      setAiTestResponse(res.response || 'Ответ получен');
      setSuccessMessage('Служба ИИ готова к работе');
      const updated = await getAiStatus();
      setAiStatus(updated);
    } catch (err: any) {
      setErrorMessage(err.message || 'Служба ИИ временно недоступна');
    } finally {
      setIsTestingAi(false);
    }
  };

  const handleNextStep = async (stepId: SetupStepId, nextStep: SetupStepId, stepData: Record<string, any>) => {
    setActionLoading(true);
    setErrorMessage(null);
    setSuccessMessage(null);
    try {
      await saveSetupStep(stepId, stepData);
      setCurrentStep(nextStep);
      await loadStatus();
    } catch (err: any) {
      console.warn('Error saving setup step:', err);
      setCurrentStep(nextStep);
    } finally {
      setActionLoading(false);
    }
  };

  const handleCompleteSetup = async () => {
    setActionLoading(true);
    setErrorMessage(null);
    try {
      await saveSetupStep('ai', {
        ready: aiStatus?.ready ?? true,
        mode: aiStatus?.mode ?? 'cli',
      });
      await completeSetup();
      onSetupComplete();
    } catch (err: any) {
      setErrorMessage(err.message || 'Ошибка завершения настройки');
      setActionLoading(false);
    }
  };

  const stepsList: Array<{ id: SetupStepId; title: string }> = [
    { id: 'network', title: '1. Сеть' },
    { id: 'audio', title: '2. Звук' },
    { id: 'voice', title: '3. Голос' },
    { id: 'ai', title: '4. ИИ' },
  ];

  if (isLoading) {
    return (
      <div className="min-h-screen bg-slate-50 flex items-center justify-center">
        <div className="flex items-center gap-2.5 text-slate-600 text-sm">
          <Loader2 className="w-4 h-4 animate-spin text-slate-800" />
          <span>Загрузка параметров настройки...</span>
        </div>
      </div>
    );
  }

  const degradedList = setupStatus?.degraded_components || [];

  return (
    <div className="min-h-screen bg-slate-50 flex flex-col">
      {/* Top Header */}
      <header className="bg-white border-b border-slate-200/80 py-4 px-6">
        <div className="max-w-3xl mx-auto flex items-center justify-between">
          <div>
            <h1 className="text-base font-semibold text-slate-900">
              Мастер первоначальной настройки
            </h1>
            <p className="text-xs text-slate-500 mt-0.5">
              Пошаговая конфигурация умной колонки KAMISH
            </p>
          </div>
        </div>
      </header>

      {/* Main Container */}
      <main className="flex-1 max-w-3xl w-full mx-auto px-4 py-8">
        {/* Degradation recovery banner */}
        {degradedList.length > 0 && (
          <div className="mb-6 p-4 rounded-lg bg-amber-50 border border-amber-200/80 text-amber-900">
            <div className="flex items-start justify-between gap-4">
              <div className="flex items-start gap-3">
                <AlertTriangle className="w-4 h-4 text-amber-700 shrink-0 mt-0.5" />
                <div>
                  <h4 className="text-sm font-semibold text-amber-900">
                    Обнаружена деградация компонентов
                  </h4>
                  <ul className="mt-1 space-y-1 text-xs text-amber-800">
                    {degradedList.map((deg, i) => (
                      <li key={i} className="flex items-center gap-1.5">
                        <span className="font-medium">{deg.component}:</span>
                        <span>{deg.reason}</span>
                      </li>
                    ))}
                  </ul>
                  <p className="text-xs text-amber-700 mt-1.5">
                    Конфигурация предыдущих шагов сохранена.
                  </p>
                </div>
              </div>
              {degradedList[0]?.recovery_step && (
                <button
                  type="button"
                  onClick={() => setCurrentStep(degradedList[0].recovery_step)}
                  className="shrink-0 px-3 py-1.5 bg-amber-800 hover:bg-amber-900 text-white text-xs font-medium rounded-lg transition-colors cursor-pointer"
                >
                  Восстановление
                </button>
              )}
            </div>
          </div>
        )}

        {/* Global Error/Success notification */}
        {errorMessage && (
          <div className="mb-4 p-3 rounded-lg bg-rose-50 text-rose-700 text-xs flex items-center gap-2">
            <AlertCircle className="w-4 h-4 shrink-0" />
            <span>{errorMessage}</span>
          </div>
        )}
        {successMessage && (
          <div className="mb-4 p-3 rounded-lg bg-emerald-50 text-emerald-700 text-xs flex items-center gap-2">
            <CheckCircle2 className="w-4 h-4 shrink-0" />
            <span>{successMessage}</span>
          </div>
        )}

        {/* Stepper: Clean horizontal progress tabs */}
        <div className="grid grid-cols-4 gap-2 mb-6">
          {stepsList.map((s) => {
            const isCurrent = currentStep === s.id;
            const stepInfo = setupStatus?.steps?.[s.id];
            const isOk = stepInfo?.status === 'ok';
            const isDegraded = degradedList.some((d) => d.recovery_step === s.id);

            return (
              <button
                key={s.id}
                type="button"
                onClick={() => setCurrentStep(s.id)}
                className={`p-3 rounded-lg border text-left transition-colors cursor-pointer flex flex-col justify-between h-16 ${
                  isCurrent
                    ? 'bg-slate-900 border-slate-900 text-white shadow-xs'
                    : isDegraded
                    ? 'bg-amber-50 border-amber-200 text-amber-900'
                    : isOk
                    ? 'bg-white border-slate-200/80 text-slate-800 hover:bg-slate-50'
                    : 'bg-white border-slate-200/80 text-slate-500 hover:bg-slate-50'
                }`}
              >
                <div className="flex items-center justify-between text-[11px]">
                  <span className="font-medium truncate">{s.title}</span>
                  {isOk && !isCurrent && <Check className="w-3.5 h-3.5 text-emerald-600 shrink-0" />}
                  {isDegraded && <AlertTriangle className="w-3.5 h-3.5 text-amber-600 shrink-0" />}
                </div>
                <div className="text-[10px] text-slate-400">
                  {isCurrent ? 'Текущий' : isOk ? 'Завершён' : isDegraded ? 'Внимание' : 'Ожидание'}
                </div>
              </button>
            );
          })}
        </div>

        {/* Wizard Content Card */}
        <div className="bg-white border border-slate-200/80 rounded-xl shadow-xs overflow-hidden">
          {/* STEP 1: NETWORK */}
          {currentStep === 'network' && (
            <div>
              <div className="px-6 py-4 border-b border-slate-100 flex items-center justify-between">
                <div>
                  <h2 className="text-sm font-semibold text-slate-900">
                    Этап 1: Сетевое подключение
                  </h2>
                  <p className="text-xs text-slate-500">
                    Подключение к проводной сети (Ethernet) или Wi-Fi
                  </p>
                </div>
              </div>

              <div className="p-6 space-y-5">
                <div className="divide-y divide-slate-100 text-xs">
                  <div className="py-2.5 flex items-center justify-between">
                    <span className="text-slate-500">IP-адрес:</span>
                    <span className="font-mono font-medium text-slate-900">
                      {networkStatus?.ip_address || 'Не назначен'}
                    </span>
                  </div>
                  <div className="py-2.5 flex items-center justify-between">
                    <span className="text-slate-500">Активное подключение:</span>
                    <span className="font-medium text-slate-900">
                      {networkStatus?.ethernet?.connected
                        ? `Ethernet (кабель подключен, ${networkStatus.ip_address || 'IP получен'})`
                        : networkStatus?.wifi?.connected
                        ? `Wi-Fi: ${networkStatus.wifi.ssid}`
                        : networkStatus?.ap_mode?.active
                        ? `Точка доступа (${networkStatus.ap_mode.ssid})`
                        : 'Отключено'}
                    </span>
                  </div>
                </div>

                <form onSubmit={handleConnectWifi} className="space-y-3 pt-2">
                  <div>
                    <label className="block text-xs font-medium text-slate-700 mb-1">
                      Беспроводная сеть Wi-Fi (опционально)
                    </label>
                    {wifiNetworks.length > 0 ? (
                      <select
                        value={selectedSsid}
                        onChange={(e) => setSelectedSsid(e.target.value)}
                        className="w-full text-xs bg-white border border-slate-200 rounded-lg px-3 py-2 text-slate-900 focus:outline-none focus:ring-2 focus:ring-slate-900/10 focus:border-slate-800"
                      >
                        <option value="">-- Выберите доступную сеть --</option>
                        {wifiNetworks.map((net, idx) => (
                          <option key={idx} value={net.ssid}>
                            {net.ssid} ({net.signal_percent}%, {net.security})
                          </option>
                        ))}
                      </select>
                    ) : (
                      <div className="p-3 bg-slate-50 border border-slate-200 rounded-lg text-xs text-slate-500 flex items-center justify-between">
                        <span>{isScanningWifi ? 'Выполняется сканирование эфира...' : 'Доступные Wi-Fi сети не обнаружены'}</span>
                        <button
                          type="button"
                          onClick={handleScanWifi}
                          disabled={isScanningWifi}
                          className="text-xs text-slate-700 hover:text-slate-900 underline font-medium cursor-pointer"
                        >
                          Обновить
                        </button>
                      </div>
                    )}
                  </div>

                  {selectedSsid && (
                    <div>
                      <label className="block text-xs font-medium text-slate-700 mb-1">
                        Пароль сети {selectedSsid}
                      </label>
                      <input
                        type="password"
                        value={wifiPassword}
                        onChange={(e) => setWifiPassword(e.target.value)}
                        placeholder="Введите пароль Wi-Fi"
                        className="w-full text-xs bg-white border border-slate-200 rounded-lg px-3 py-2 text-slate-900 focus:outline-none focus:ring-2 focus:ring-slate-900/10 focus:border-slate-800"
                      />
                    </div>
                  )}

                  {selectedSsid && (
                    <button
                      type="submit"
                      disabled={actionLoading}
                      className="px-3.5 py-2 bg-slate-900 hover:bg-slate-800 text-white text-xs font-medium rounded-lg transition-colors cursor-pointer shadow-xs"
                    >
                      {actionLoading ? 'Подключение...' : 'Подключиться к сети'}
                    </button>
                  )}
                </form>

                <div className="pt-4 flex justify-end items-center border-t border-slate-100">
                  <button
                    type="button"
                    onClick={() =>
                      handleNextStep('network', 'audio', {
                        connected: !!networkStatus?.ip_address,
                        ssid: networkStatus?.wifi?.ssid || 'Ethernet',
                      })
                    }
                    disabled={actionLoading}
                    className="flex items-center gap-2 px-4 py-2 bg-slate-900 hover:bg-slate-800 text-white text-xs font-medium rounded-lg transition-colors cursor-pointer shadow-xs"
                  >
                    <span>Далее: Звук</span>
                    <ChevronRight className="w-4 h-4" />
                  </button>
                </div>
              </div>
            </div>
          )}

          {/* STEP 2: AUDIO */}
          {currentStep === 'audio' && (
            <div>
              <div className="px-6 py-4 border-b border-slate-100 flex items-center justify-between">
                <div>
                  <h2 className="text-sm font-semibold text-slate-900">
                    Этап 2: Аудиосистема
                  </h2>
                  <p className="text-xs text-slate-500">
                    Настройка громкости динамика (максимум ≤ 85%)
                  </p>
                </div>
                <span className="text-xs font-medium px-2 py-0.5 rounded-md bg-slate-100 text-slate-700">
                  Потолок: {ceiling}%
                </span>
              </div>

              <div className="p-6 space-y-6">
                <div className="space-y-4">
                  <div>
                    <div className="flex justify-between text-xs font-medium text-slate-700 mb-2">
                      <span>Громкость воспроизведения</span>
                      <span className="font-mono text-slate-900">{volume}%</span>
                    </div>
                    <input
                      type="range"
                      min={0}
                      max={ceiling}
                      value={volume}
                      onChange={(e) => handleApplyAudio(Number(e.target.value), ceiling)}
                      className="w-full accent-slate-900 cursor-pointer"
                    />
                    <div className="flex justify-between text-[11px] text-slate-400 mt-1">
                      <span>0%</span>
                      <span>Ограничено {ceiling}%</span>
                    </div>
                  </div>

                  <div>
                    <div className="flex justify-between text-xs font-medium text-slate-700 mb-2">
                      <span>Защитный программный потолок</span>
                      <span className="font-mono text-slate-900">{ceiling}% (макс 85%)</span>
                    </div>
                    <input
                      type="range"
                      min={50}
                      max={85}
                      value={ceiling}
                      onChange={(e) => handleApplyAudio(volume, Number(e.target.value))}
                      className="w-full accent-slate-900 cursor-pointer"
                    />
                  </div>
                </div>

                <div className="flex items-center justify-between pt-2">
                  <span className="text-xs text-slate-600">Проверка звукового тракта:</span>
                  <button
                    type="button"
                    onClick={handleAudioTest}
                    disabled={isPlayingTestTone}
                    className="flex items-center gap-1.5 px-3 py-1.5 bg-white hover:bg-slate-50 text-slate-700 border border-slate-200 rounded-lg text-xs font-medium transition-colors cursor-pointer"
                  >
                    <Play className="w-3.5 h-3.5" />
                    <span>{isPlayingTestTone ? 'Воспроизведение...' : 'Тестовый сигнал'}</span>
                  </button>
                </div>

                <div className="pt-4 flex justify-between items-center border-t border-slate-100">
                  <button
                    type="button"
                    onClick={() => setCurrentStep('network')}
                    className="px-3.5 py-2 bg-white hover:bg-slate-50 text-slate-700 border border-slate-200 text-xs font-medium rounded-lg cursor-pointer"
                  >
                    Назад
                  </button>
                  <button
                    type="button"
                    onClick={() =>
                      handleNextStep('audio', 'voice', {
                        volume,
                        ceiling,
                      })
                    }
                    disabled={actionLoading}
                    className="flex items-center gap-2 px-4 py-2 bg-slate-900 hover:bg-slate-800 text-white text-xs font-medium rounded-lg transition-colors cursor-pointer shadow-xs"
                  >
                    <span>Далее: Голос</span>
                    <ChevronRight className="w-4 h-4" />
                  </button>
                </div>
              </div>
            </div>
          )}

          {/* STEP 3: VOICE */}
          {currentStep === 'voice' && (
            <div>
              <div className="px-6 py-4 border-b border-slate-100 flex items-center justify-between">
                <div>
                  <h2 className="text-sm font-semibold text-slate-900">
                    Этап 3: Речевой стек
                  </h2>
                  <p className="text-xs text-slate-500">
                    Проверка микрофона и синтеза речи
                  </p>
                </div>
                <span className="text-xs font-medium px-2 py-0.5 rounded-md bg-slate-100 text-slate-700">
                  {speechStatus?.active_tts_provider || 'Модуль активен'}
                </span>
              </div>

              <div className="p-6 space-y-5">
                <div className="divide-y divide-slate-100 text-xs">
                  <div className="py-2.5 flex items-center justify-between">
                    <span className="text-slate-500">Распознавание речи (STT):</span>
                    <span className="font-medium text-slate-900">
                      {speechStatus?.active_stt_provider || 'Автономный VAD/STT'}
                    </span>
                  </div>
                  <div className="py-2.5 flex items-center justify-between">
                    <span className="text-slate-500">Синтез речи (TTS):</span>
                    <span className="font-medium text-slate-900">
                      {speechStatus?.active_tts_provider || 'Локальный генератор WAV'}
                    </span>
                  </div>
                </div>

                <div className="flex items-center justify-between pt-2">
                  <span className="text-xs text-slate-600">Тестовый синтез голосовой реплики:</span>
                  <button
                    type="button"
                    onClick={handleVoiceTest}
                    disabled={isTestingSpeech}
                    className="flex items-center gap-1.5 px-3 py-1.5 bg-white hover:bg-slate-50 text-slate-700 border border-slate-200 rounded-lg text-xs font-medium transition-colors cursor-pointer"
                  >
                    <Play className="w-3.5 h-3.5" />
                    <span>{isTestingSpeech ? 'Синтез...' : 'Проверить синтез'}</span>
                  </button>
                </div>

                <div className="pt-4 flex justify-between items-center border-t border-slate-100">
                  <button
                    type="button"
                    onClick={() => setCurrentStep('audio')}
                    className="px-3.5 py-2 bg-white hover:bg-slate-50 text-slate-700 border border-slate-200 text-xs font-medium rounded-lg cursor-pointer"
                  >
                    Назад
                  </button>
                  <button
                    type="button"
                    onClick={() =>
                      handleNextStep('voice', 'ai', {
                        stt_active: true,
                        tts_active: true,
                      })
                    }
                    disabled={actionLoading}
                    className="flex items-center gap-2 px-4 py-2 bg-slate-900 hover:bg-slate-800 text-white text-xs font-medium rounded-lg transition-colors cursor-pointer shadow-xs"
                  >
                    <span>Далее: ИИ</span>
                    <ChevronRight className="w-4 h-4" />
                  </button>
                </div>
              </div>
            </div>
          )}

          {/* STEP 4: AI & MODELS */}
          {currentStep === 'ai' && (
            <div>
              <div className="px-6 py-4 border-b border-slate-100 flex items-center justify-between">
                <div>
                  <h2 className="text-sm font-semibold text-slate-900">
                    Этап 4: Голосовой ассистент (Локально)
                  </h2>
                  <p className="text-xs text-slate-500">
                    Локальный процесс ИИ и типизированные инструменты
                  </p>
                </div>
                <span className="text-xs font-medium px-2 py-0.5 rounded-md bg-slate-100 text-slate-700">
                  {aiStatus?.mode === 'cli' ? 'Antigravity CLI' : 'Автономный режим'}
                </span>
              </div>

              <div className="p-6 space-y-5">
                <div className="divide-y divide-slate-100 text-xs">
                  <div className="py-2.5 flex items-center justify-between">
                    <span className="text-slate-500">Статус службы ИИ:</span>
                    <span className="font-medium text-slate-900">
                      {aiStatus?.ready ? 'Готов к работе' : 'Ожидание проверки'}
                    </span>
                  </div>
                  <div className="py-2.5 flex items-center justify-between">
                    <span className="text-slate-500">Режим:</span>
                    <span className="font-medium text-slate-900">Локальный процесс</span>
                  </div>
                  <div className="py-2.5 flex items-center justify-between">
                    <span className="text-slate-500">Системные инструменты:</span>
                    <span className="font-medium text-slate-900">14 типизированных инструментов</span>
                  </div>
                </div>

                <div className="space-y-2 pt-2">
                  <div className="flex items-center justify-between">
                    <span className="text-xs text-slate-600">Диагностический запрос к ассистенту:</span>
                    <button
                      type="button"
                      onClick={handleAiTest}
                      disabled={isTestingAi}
                      className="px-3 py-1.5 bg-white hover:bg-slate-50 text-slate-700 border border-slate-200 rounded-lg text-xs font-medium transition-colors cursor-pointer"
                    >
                      {isTestingAi ? 'Обработка...' : 'Отправить запрос'}
                    </button>
                  </div>

                  {aiTestResponse && (
                    <div className="p-3 bg-slate-50 border border-slate-200 rounded-lg text-xs text-slate-800">
                      <span className="text-slate-400 block mb-0.5 font-medium">Ответ ассистента:</span>
                      {aiTestResponse}
                    </div>
                  )}
                </div>

                <div className="pt-4 flex justify-between items-center border-t border-slate-100">
                  <button
                    type="button"
                    onClick={() => setCurrentStep('voice')}
                    className="px-3.5 py-2 bg-white hover:bg-slate-50 text-slate-700 border border-slate-200 text-xs font-medium rounded-lg cursor-pointer"
                  >
                    Назад
                  </button>
                  <button
                    type="button"
                    onClick={handleCompleteSetup}
                    disabled={actionLoading}
                    className="flex items-center gap-2 px-5 py-2 bg-slate-900 hover:bg-slate-800 text-white text-xs font-medium rounded-lg transition-colors cursor-pointer shadow-xs"
                  >
                    <CheckCircle2 className="w-4 h-4" />
                    <span>Завершить настройку</span>
                  </button>
                </div>
              </div>
            </div>
          )}
        </div>
      </main>
    </div>
  );
};
