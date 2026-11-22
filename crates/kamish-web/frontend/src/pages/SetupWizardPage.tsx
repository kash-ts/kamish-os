import React, { useState, useEffect, useCallback, useRef } from 'react';
import { AlertTriangle, AlertCircle, Loader2, Eye, EyeOff } from 'lucide-react';
import {
  getSetupStatus,
  saveSetupStep,
  completeSetup,
  getNetworkStatus,
  scanWifi,
  connectWifi,
  disconnectWifi,
  setAudioVolume,
  setVolumeCeiling,
  playAudioTest,
  getAudioDevices,
  setAudioDevice,
  getSpeechStatus,
  synthesizeSpeech,
  changePassword,
} from '../api';
import type {
  SetupStepId,
  SetupStatusResponse,
  NetworkStatus,
  WifiNetwork,
  SpeechStatus,
  AudioDevicesResponse,
} from '../types';

interface SetupWizardPageProps {
  onSetupComplete: () => void;
  mustChangePassword?: boolean;
  onPasswordChanged?: () => void;
  onForcedPasswordRequired?: () => void;
}

type WizardStep = 'auth' | 'network' | 'audio' | 'voice';

const INPUT_CLASS =
  'w-full text-xs bg-white border border-slate-200 rounded-lg px-3 py-2 text-slate-900 focus:outline-none focus:ring-2 focus:ring-slate-900/10 focus:border-slate-800';
const PRIMARY_BTN =
  'px-4 py-2 bg-slate-900 hover:bg-slate-800 disabled:opacity-50 disabled:cursor-not-allowed text-white text-xs font-medium rounded-lg transition-colors cursor-pointer';
const SECONDARY_BTN =
  'px-3.5 py-2 bg-white hover:bg-slate-50 disabled:opacity-50 disabled:cursor-not-allowed text-slate-700 border border-slate-200 text-xs font-medium rounded-lg cursor-pointer';
const LINK_BTN = 'text-xs text-slate-600 hover:text-slate-900 underline font-medium cursor-pointer';

const isOpenNetwork = (security: string | undefined): boolean =>
  !security || /^(--|open|none)$/i.test(security.trim());

export const SetupWizardPage: React.FC<SetupWizardPageProps> = ({
  onSetupComplete,
  mustChangePassword = false,
  onPasswordChanged,
  onForcedPasswordRequired,
}) => {
  const [setupStatus, setSetupStatus] = useState<SetupStatusResponse | null>(null);
  const [currentStep, setCurrentStep] = useState<WizardStep>(mustChangePassword ? 'auth' : 'network');
  const initialLoadedRef = useRef<boolean>(false);
  const [isLoading, setIsLoading] = useState<boolean>(true);
  const [actionLoading, setActionLoading] = useState<boolean>(false);
  const [errorMessage, setErrorMessage] = useState<string | null>(null);

  // Stage 1: Auth / Password state
  const [oldPassword, setOldPassword] = useState('admin');
  const [newPassword, setNewPassword] = useState('');
  const [confirmPassword, setConfirmPassword] = useState('');
  const [showNewPassword, setShowNewPassword] = useState(false);
  const [showConfirmPassword, setShowConfirmPassword] = useState(false);
  const [isPasswordChangedLocally, setIsPasswordChangedLocally] = useState(!mustChangePassword);

  // Stage 2: Network state
  const [networkStatus, setNetworkStatus] = useState<NetworkStatus | null>(null);
  const [wifiNetworks, setWifiNetworks] = useState<WifiNetwork[]>([]);
  const [isScanningWifi, setIsScanningWifi] = useState(false);
  const [selectedSsid, setSelectedSsid] = useState('');
  const [manualSsid, setManualSsid] = useState(false);
  const [customSsid, setCustomSsid] = useState('');
  const [wifiPassword, setWifiPassword] = useState('');

  // Stage 3: Audio state
  const [volume, setVolume] = useState<number>(60);
  const [ceiling, setCeiling] = useState<number>(85);
  const [isPlayingTestTone, setIsPlayingTestTone] = useState(false);
  const [isAudioTested, setIsAudioTested] = useState(false);
  const [audioDevices, setAudioDevices] = useState<AudioDevicesResponse | null>(null);

  // Stage 4: Voice state
  const [speechStatus, setSpeechStatus] = useState<SpeechStatus | null>(null);
  const [isTestingSpeech, setIsTestingSpeech] = useState(false);
  const [isVoiceTested, setIsVoiceTested] = useState(false);

  const loadStatus = useCallback(async () => {
    try {
      const res = await getSetupStatus();
      setSetupStatus(res);
      if (res.is_completed) {
        onSetupComplete();
        return;
      }

      if (res.steps?.audio?.status === 'ok') {
        setIsAudioTested(true);
      }
      if (res.steps?.voice?.status === 'ok') {
        setIsVoiceTested(true);
      }

      // Only set currentStep on the very first mount
      if (!initialLoadedRef.current) {
        initialLoadedRef.current = true;
        if (mustChangePassword || res.steps?.auth?.must_change_password) {
          setCurrentStep('auth');
        } else if (res.current_step) {
          // The AI stage is not part of the wizard: fall back to the last visible stage.
          setCurrentStep(res.current_step === 'ai' ? 'voice' : res.current_step);
        }
      }

      if (res.steps?.auth?.must_change_password && onForcedPasswordRequired) {
        onForcedPasswordRequired();
      }
    } catch (err: any) {
      console.warn('Failed to load setup status:', err);
    } finally {
      setIsLoading(false);
    }
  }, [onSetupComplete, mustChangePassword, onForcedPasswordRequired]);

  useEffect(() => {
    loadStatus();
  }, [loadStatus]);

  const handleScanWifi = useCallback(async () => {
    setIsScanningWifi(true);
    try {
      const res = await scanWifi();
      setWifiNetworks(res.networks || []);
    } catch (err) {
      console.warn('Wi-Fi scan error:', err);
      setWifiNetworks([]);
    } finally {
      setIsScanningWifi(false);
    }
  }, []);

  // Load step-specific data when step changes
  useEffect(() => {
    if (currentStep === 'network') {
      getNetworkStatus()
        .then((net) => {
          setNetworkStatus(net);
          handleScanWifi();
        })
        .catch(console.warn);
    } else if (currentStep === 'audio') {
      getAudioDevices().then(setAudioDevices).catch(console.warn);
    } else if (currentStep === 'voice') {
      getSpeechStatus().then(setSpeechStatus).catch(console.warn);
      getAudioDevices().then(setAudioDevices).catch(console.warn);
    }
  }, [currentStep, handleScanWifi]);

  const handlePasswordSubmit = async (e: React.FormEvent) => {
    e.preventDefault();
    if (!newPassword) {
      setErrorMessage('Введите новый пароль');
      return;
    }
    if (newPassword.length < 4) {
      setErrorMessage('Новый пароль должен содержать не менее 4 символов');
      return;
    }
    if (newPassword === oldPassword) {
      setErrorMessage('Новый пароль должен отличаться от заводского (admin)');
      return;
    }
    if (newPassword !== confirmPassword) {
      setErrorMessage('Введенные пароли не совпадают');
      return;
    }

    setActionLoading(true);
    setErrorMessage(null);
    try {
      await changePassword({
        old_password: oldPassword,
        new_password: newPassword,
      });
      setIsPasswordChangedLocally(true);
      if (onPasswordChanged) {
        onPasswordChanged();
      }
      await saveSetupStep('auth', { changed: true });
      setCurrentStep('network');
      await loadStatus();
    } catch (err: any) {
      setErrorMessage(err.message || 'Ошибка смены пароля. Проверьте текущий пароль.');
    } finally {
      setActionLoading(false);
    }
  };

  const refreshNetwork = async () => {
    const updated = await getNetworkStatus();
    setNetworkStatus(updated);
    return updated;
  };

  const handleConnectWifi = async (e: React.FormEvent) => {
    e.preventDefault();
    const targetSsid = manualSsid ? customSsid.trim() : selectedSsid.trim();
    if (!targetSsid) return;

    setActionLoading(true);
    setErrorMessage(null);
    try {
      await connectWifi({ ssid: targetSsid, password: wifiPassword || undefined });
      await refreshNetwork();
      setWifiPassword('');
      setManualSsid(false);
      setSelectedSsid('');
    } catch (err: any) {
      setErrorMessage(err.message || 'Ошибка подключения к сети');
    } finally {
      setActionLoading(false);
    }
  };

  const handleDisconnectWifi = async () => {
    setActionLoading(true);
    setErrorMessage(null);
    try {
      await disconnectWifi();
      const updated = await refreshNetwork();
      if (!updated.wifi?.connected) {
        await handleScanWifi();
      }
    } catch (err: any) {
      setErrorMessage(err.message || 'Ошибка отключения от сети');
    } finally {
      setActionLoading(false);
    }
  };

  const handleAudioTest = async () => {
    setIsPlayingTestTone(true);
    setErrorMessage(null);
    try {
      await playAudioTest();
      setIsAudioTested(true);
    } catch {
      try {
        await synthesizeSpeech({
          text: 'Проверка звукового тракта умной колонки Камыш',
          play_on_speaker: true,
        });
        setIsAudioTested(true);
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

  const handleSelectDevice = async (kind: 'output' | 'input', id: string) => {
    setErrorMessage(null);
    try {
      await setAudioDevice(kind, id);
      setAudioDevices((prev) =>
        prev
          ? {
              ...prev,
              selected_output: kind === 'output' ? id : prev.selected_output,
              selected_input: kind === 'input' ? id : prev.selected_input,
            }
          : prev
      );
      if (kind === 'output') {
        // A different output must be verified again.
        setIsAudioTested(false);
      }
    } catch (err: any) {
      setErrorMessage(err.message || 'Не удалось выбрать устройство');
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
      setIsVoiceTested(true);
      const updated = await getSpeechStatus();
      setSpeechStatus(updated);
    } catch (err: any) {
      setErrorMessage(err.message || 'Ошибка тестирования синтеза речи');
    } finally {
      setIsTestingSpeech(false);
    }
  };

  const handleNextStep = async (stepId: SetupStepId, nextStep: WizardStep, stepData: Record<string, any>) => {
    setActionLoading(true);
    setErrorMessage(null);
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
      await saveSetupStep('voice', { stt_active: true, tts_active: true });
      await completeSetup();
      onSetupComplete();
    } catch (err: any) {
      setErrorMessage(err.message || 'Ошибка завершения настройки');
      setActionLoading(false);
    }
  };

  // Step completion criteria: the next stage stays locked until the previous one is done.
  const isAuthCompleted = isPasswordChangedLocally && !mustChangePassword;
  const isNetworkConnected = Boolean(
    networkStatus?.ip_address &&
      networkStatus.ip_address !== '127.0.0.1' &&
      (networkStatus.ethernet?.connected || networkStatus.wifi?.connected || networkStatus.ap_mode?.active)
  );
  const isAudioCompleted = isAudioTested || setupStatus?.steps?.audio?.status === 'ok';
  const isVoiceCompleted = isVoiceTested || setupStatus?.steps?.voice?.status === 'ok';

  const isStepAccessible = (stepId: WizardStep): boolean => {
    switch (stepId) {
      case 'auth':
        return true;
      case 'network':
        return isAuthCompleted;
      case 'audio':
        return isAuthCompleted && isNetworkConnected;
      case 'voice':
        return isAuthCompleted && isNetworkConnected && isAudioCompleted;
      default:
        return false;
    }
  };

  const stepsList: Array<{ id: WizardStep; title: string }> = [
    { id: 'auth', title: '1. Пароль' },
    { id: 'network', title: '2. Сеть' },
    { id: 'audio', title: '3. Звук' },
    { id: 'voice', title: '4. Голос' },
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

  const degradedList = (setupStatus?.degraded_components || []).filter((d) => d.component !== 'ai');

  const activeConnectionLabel = (): string => {
    if (networkStatus?.wifi?.connected && networkStatus.wifi.ssid) {
      return `${networkStatus.wifi.ssid} (Wi-Fi)`;
    }
    if (networkStatus?.ethernet?.connected) {
      return 'Ethernet';
    }
    if (networkStatus?.ap_mode?.active) {
      return `${networkStatus.ap_mode.ssid} (Точка доступа)`;
    }
    return 'Нет подключения';
  };

  const wifiConnected = Boolean(networkStatus?.wifi?.connected);
  const targetSsid = manualSsid ? customSsid.trim() : selectedSsid;
  const selectedNetwork = wifiNetworks.find((n) => n.ssid === selectedSsid);
  const needsPassword = manualSsid ? true : selectedNetwork ? !isOpenNetwork(selectedNetwork.security) : false;

  const outputDevices = audioDevices?.outputs ?? [];
  const inputDevices = audioDevices?.inputs ?? [];

  const renderDeviceSelect = (kind: 'output' | 'input', label: string) => {
    const devices = kind === 'output' ? outputDevices : inputDevices;
    if (devices.length < 2) return null;
    const selected = kind === 'output' ? audioDevices?.selected_output : audioDevices?.selected_input;
    return (
      <div>
        <label className="block text-xs font-medium text-slate-700 mb-1">{label}</label>
        <select
          value={selected ?? 'default'}
          onChange={(e) => handleSelectDevice(kind, e.target.value)}
          className={INPUT_CLASS}
        >
          <option value="default">Системное по умолчанию</option>
          {devices.map((d) => (
            <option key={d.id} value={d.id}>
              {d.name}
            </option>
          ))}
        </select>
      </div>
    );
  };

  return (
    <div className="min-h-screen bg-slate-50 flex flex-col">
      <header className="bg-white border-b border-slate-200/80 py-4 px-6">
        <div className="max-w-3xl mx-auto flex items-center justify-between">
          <h1 className="text-base font-semibold text-slate-900">Мастер первоначальной настройки</h1>
        </div>
      </header>

      <main className="flex-1 max-w-3xl w-full mx-auto px-4 py-8">
        {degradedList.length > 0 && (
          <div className="mb-6 p-4 rounded-lg bg-amber-50 border border-amber-200/80 text-amber-900">
            <div className="flex items-start justify-between gap-4">
              <div className="flex items-start gap-3">
                <AlertTriangle className="w-4 h-4 text-amber-700 shrink-0 mt-0.5" />
                <div>
                  <h4 className="text-sm font-semibold text-amber-900">Обнаружена деградация компонентов</h4>
                  <ul className="mt-1 space-y-1 text-xs text-amber-800">
                    {degradedList.map((deg, i) => (
                      <li key={i} className="flex items-center gap-1.5">
                        <span className="font-medium">{deg.component}:</span>
                        <span>{deg.reason}</span>
                      </li>
                    ))}
                  </ul>
                </div>
              </div>
              {degradedList[0]?.recovery_step && degradedList[0].recovery_step !== 'ai' && (
                <button
                  type="button"
                  onClick={() => setCurrentStep(degradedList[0].recovery_step as WizardStep)}
                  className="shrink-0 px-3 py-1.5 bg-amber-800 hover:bg-amber-900 text-white text-xs font-medium rounded-lg transition-colors cursor-pointer"
                >
                  Восстановление
                </button>
              )}
            </div>
          </div>
        )}

        {errorMessage && (
          <div className="mb-4 p-3 rounded-lg bg-rose-50 text-rose-700 text-xs flex items-center gap-2">
            <AlertCircle className="w-4 h-4 shrink-0" />
            <span>{errorMessage}</span>
          </div>
        )}

        <div className="grid grid-cols-4 gap-2 mb-6">
          {stepsList.map((s) => {
            const isCurrent = currentStep === s.id;
            const isAccessible = isStepAccessible(s.id);
            return (
              <button
                key={s.id}
                type="button"
                disabled={!isAccessible}
                onClick={() => {
                  setErrorMessage(null);
                  setCurrentStep(s.id);
                }}
                className={`px-3 py-3 rounded-lg border text-left text-xs font-medium transition-colors ${
                  isCurrent
                    ? 'bg-slate-900 border-slate-900 text-white cursor-default'
                    : isAccessible
                    ? 'bg-white border-slate-200/80 text-slate-700 hover:bg-slate-50 cursor-pointer'
                    : 'bg-slate-100/70 border-slate-200/60 text-slate-400 opacity-60 cursor-not-allowed'
                }`}
              >
                {s.title}
              </button>
            );
          })}
        </div>

        <div className="bg-white border border-slate-200/80 rounded-xl shadow-xs overflow-hidden">
          {/* STEP 1: PASSWORD */}
          {currentStep === 'auth' && (
            <div>
              <div className="px-6 py-4 border-b border-slate-100">
                <h2 className="text-sm font-semibold text-slate-900">Этап 1: Пароль администратора</h2>
              </div>

              <div className="p-6">
                {isAuthCompleted ? (
                  <div className="flex justify-end">
                    <button type="button" onClick={() => setCurrentStep('network')} className={PRIMARY_BTN}>
                      Далее
                    </button>
                  </div>
                ) : (
                  <form onSubmit={handlePasswordSubmit} className="space-y-4">
                    <div>
                      <label className="block text-xs font-medium text-slate-700 mb-1">
                        Текущий заводской пароль
                      </label>
                      <input
                        type="text"
                        value={oldPassword}
                        onChange={(e) => setOldPassword(e.target.value)}
                        placeholder="admin"
                        className={`${INPUT_CLASS} font-mono bg-slate-50`}
                      />
                    </div>

                    <div>
                      <label className="block text-xs font-medium text-slate-700 mb-1">Новый пароль</label>
                      <div className="relative">
                        <input
                          type={showNewPassword ? 'text' : 'password'}
                          value={newPassword}
                          onChange={(e) => setNewPassword(e.target.value)}
                          placeholder="Не менее 4 символов"
                          className={`${INPUT_CLASS} pr-9`}
                        />
                        <button
                          type="button"
                          onClick={() => setShowNewPassword(!showNewPassword)}
                          className="absolute right-2.5 top-1/2 -translate-y-1/2 text-slate-400 hover:text-slate-600"
                        >
                          {showNewPassword ? <EyeOff className="w-3.5 h-3.5" /> : <Eye className="w-3.5 h-3.5" />}
                        </button>
                      </div>
                    </div>

                    <div>
                      <label className="block text-xs font-medium text-slate-700 mb-1">
                        Подтверждение нового пароля
                      </label>
                      <div className="relative">
                        <input
                          type={showConfirmPassword ? 'text' : 'password'}
                          value={confirmPassword}
                          onChange={(e) => setConfirmPassword(e.target.value)}
                          placeholder="Повторите новый пароль"
                          className={`${INPUT_CLASS} pr-9`}
                        />
                        <button
                          type="button"
                          onClick={() => setShowConfirmPassword(!showConfirmPassword)}
                          className="absolute right-2.5 top-1/2 -translate-y-1/2 text-slate-400 hover:text-slate-600"
                        >
                          {showConfirmPassword ? (
                            <EyeOff className="w-3.5 h-3.5" />
                          ) : (
                            <Eye className="w-3.5 h-3.5" />
                          )}
                        </button>
                      </div>
                    </div>

                    <div className="pt-4 flex justify-end border-t border-slate-100">
                      <button
                        type="submit"
                        disabled={actionLoading || !newPassword || newPassword.length < 4}
                        className={PRIMARY_BTN}
                      >
                        {actionLoading ? 'Сохранение...' : 'Далее'}
                      </button>
                    </div>
                  </form>
                )}
              </div>
            </div>
          )}

          {/* STEP 2: NETWORK */}
          {currentStep === 'network' && (
            <div>
              <div className="px-6 py-4 border-b border-slate-100">
                <h2 className="text-sm font-semibold text-slate-900">Этап 2: Сеть</h2>
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
                    <span className="font-medium text-slate-900">{activeConnectionLabel()}</span>
                  </div>
                </div>

                {wifiConnected ? (
                  <div className="flex items-center justify-between pt-2">
                    <span className="text-xs text-slate-600">Чтобы выбрать другую сеть Wi-Fi, отключитесь от текущей.</span>
                    <button
                      type="button"
                      onClick={handleDisconnectWifi}
                      disabled={actionLoading}
                      className={SECONDARY_BTN}
                    >
                      {actionLoading ? 'Отключение...' : 'Отключиться'}
                    </button>
                  </div>
                ) : (
                  <form onSubmit={handleConnectWifi} className="space-y-3 pt-2">
                    <div className="flex items-center justify-between mb-1">
                      <label className="text-xs font-medium text-slate-700">Сеть Wi-Fi</label>
                      <div className="flex items-center gap-3">
                        <button
                          type="button"
                          onClick={() => {
                            setManualSsid(!manualSsid);
                            setWifiPassword('');
                          }}
                          className={LINK_BTN}
                        >
                          {manualSsid ? 'Выбрать из списка' : 'Ввести SSID вручную'}
                        </button>
                        <button
                          type="button"
                          onClick={handleScanWifi}
                          disabled={isScanningWifi}
                          className={LINK_BTN}
                        >
                          {isScanningWifi ? 'Сканирование...' : 'Обновить'}
                        </button>
                      </div>
                    </div>

                    {manualSsid ? (
                      <input
                        type="text"
                        value={customSsid}
                        onChange={(e) => setCustomSsid(e.target.value)}
                        placeholder="Имя сети (SSID)"
                        className={INPUT_CLASS}
                      />
                    ) : wifiNetworks.length > 0 ? (
                      <select
                        value={selectedSsid}
                        onChange={(e) => {
                          setSelectedSsid(e.target.value);
                          setWifiPassword('');
                        }}
                        className={INPUT_CLASS}
                      >
                        <option value="">Выберите сеть</option>
                        {wifiNetworks.map((net, idx) => (
                          <option key={`${net.ssid}-${idx}`} value={net.ssid}>
                            {net.ssid} ({net.signal_percent}%{isOpenNetwork(net.security) ? ', открытая' : ''})
                          </option>
                        ))}
                      </select>
                    ) : (
                      <div className="p-3 bg-slate-50 border border-slate-200 rounded-lg text-xs text-slate-500">
                        {isScanningWifi ? 'Сканирование...' : 'Сети Wi-Fi не найдены'}
                      </div>
                    )}

                    {targetSsid && needsPassword && (
                      <div>
                        <label className="block text-xs font-medium text-slate-700 mb-1">
                          Пароль сети {targetSsid}
                        </label>
                        <input
                          type="password"
                          value={wifiPassword}
                          onChange={(e) => setWifiPassword(e.target.value)}
                          placeholder="Пароль Wi-Fi"
                          className={INPUT_CLASS}
                        />
                      </div>
                    )}

                    {targetSsid && (
                      <button type="submit" disabled={actionLoading} className={PRIMARY_BTN}>
                        {actionLoading ? 'Подключение...' : 'Подключиться'}
                      </button>
                    )}
                  </form>
                )}

                <div className="pt-4 flex justify-between items-center border-t border-slate-100">
                  <button type="button" onClick={() => setCurrentStep('auth')} className={SECONDARY_BTN}>
                    Назад
                  </button>
                  <button
                    type="button"
                    disabled={!isNetworkConnected || actionLoading}
                    onClick={() =>
                      handleNextStep('network', 'audio', {
                        connected: true,
                        ssid: networkStatus?.wifi?.ssid || 'Ethernet',
                      })
                    }
                    className={PRIMARY_BTN}
                  >
                    Далее
                  </button>
                </div>
              </div>
            </div>
          )}

          {/* STEP 3: AUDIO */}
          {currentStep === 'audio' && (
            <div>
              <div className="px-6 py-4 border-b border-slate-100">
                <h2 className="text-sm font-semibold text-slate-900">Этап 3: Звук</h2>
              </div>

              <div className="p-6 space-y-6">
                {renderDeviceSelect('output', 'Вывод звука')}

                <div className="space-y-4">
                  <div>
                    <div className="flex justify-between text-xs font-medium text-slate-700 mb-2">
                      <span>Громкость</span>
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
                  </div>

                  <div>
                    <div className="flex justify-between text-xs font-medium text-slate-700 mb-2">
                      <span>Защитный потолок громкости</span>
                      <span className="font-mono text-slate-900">{ceiling}%</span>
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
                  <span className="text-xs text-slate-600">Проверка звука:</span>
                  <button
                    type="button"
                    onClick={handleAudioTest}
                    disabled={isPlayingTestTone}
                    className={SECONDARY_BTN}
                  >
                    {isPlayingTestTone ? 'Воспроизведение...' : 'Тестовый сигнал'}
                  </button>
                </div>

                <div className="pt-4 flex justify-between items-center border-t border-slate-100">
                  <button type="button" onClick={() => setCurrentStep('network')} className={SECONDARY_BTN}>
                    Назад
                  </button>
                  <div className="flex items-center gap-3">
                    {!isAudioCompleted && (
                      <span className="text-[11px] text-amber-700">Воспроизведите тестовый сигнал</span>
                    )}
                    <button
                      type="button"
                      disabled={!isAudioCompleted || actionLoading}
                      onClick={() => handleNextStep('audio', 'voice', { volume, ceiling })}
                      className={PRIMARY_BTN}
                    >
                      Далее
                    </button>
                  </div>
                </div>
              </div>
            </div>
          )}

          {/* STEP 4: VOICE */}
          {currentStep === 'voice' && (
            <div>
              <div className="px-6 py-4 border-b border-slate-100">
                <h2 className="text-sm font-semibold text-slate-900">Этап 4: Голос</h2>
              </div>

              <div className="p-6 space-y-5">
                {renderDeviceSelect('input', 'Микрофон')}

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
                  <span className="text-xs text-slate-600">Проверка синтеза речи:</span>
                  <button
                    type="button"
                    onClick={handleVoiceTest}
                    disabled={isTestingSpeech}
                    className={SECONDARY_BTN}
                  >
                    {isTestingSpeech ? 'Синтез...' : 'Проверить'}
                  </button>
                </div>

                <div className="pt-4 flex justify-between items-center border-t border-slate-100">
                  <button type="button" onClick={() => setCurrentStep('audio')} className={SECONDARY_BTN}>
                    Назад
                  </button>
                  <div className="flex items-center gap-3">
                    {!isVoiceCompleted && (
                      <span className="text-[11px] text-amber-700">Проверьте синтез речи</span>
                    )}
                    <button
                      type="button"
                      disabled={!isVoiceCompleted || actionLoading}
                      onClick={handleCompleteSetup}
                      className={PRIMARY_BTN}
                    >
                      Завершить
                    </button>
                  </div>
                </div>
              </div>
            </div>
          )}
        </div>
      </main>
    </div>
  );
};
