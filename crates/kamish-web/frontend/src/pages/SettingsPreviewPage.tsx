import React, { useState, useEffect, useCallback } from 'react';
import {
  Wifi,
  Shield,
  Mic,
  Volume2,
  Bluetooth,
  RefreshCw,
  Check,
  UploadCloud,
  Terminal,
  Copy,
  CheckCircle2,
  AlertTriangle,
  Layers,
  Lock,
  Bot,
} from 'lucide-react';
import { Card } from '../components/Card';
import { Badge } from '../components/Badge';
import { Button } from '../components/Button';
import { Tabs } from '../components/Tabs';
import { SpeechManagerPanel } from '../components/SpeechManagerPanel';
import {
  getBluetoothStatus,
  scanBluetooth,
  pairBluetooth,
  connectBluetooth,
  disconnectBluetooth,
  getNetworkStatus,
  scanWifi,
  connectWifi,
  disconnectWifi,
  startApMode,
  stopApMode,
  getVpnStatus,
  connectVpn,
  disconnectVpn,
  configureVpn,
  getHardwareTelemetry,
  setAudioVolume,
  setVolumeCeiling,
  setAudioRoute,
  getUpdateStatus,
  uploadUpdateBundle,
  installUpdate,
  getSshStatus,
  toggleSsh,
  getAiStatus,
  updateAiConfig,
  probeAi,
} from '../api';
import type {
  BluetoothStatus,
  BluetoothDevice,
  NetworkStatus,
  WifiNetwork,
  VpnStatus,
  HardwareTelemetry,
  AudioRoute,
  UpdateStatus,
  UpdateBundleInfo,
  SshStatus,
  AiReadiness,
} from '../types';

export const SettingsPreviewPage: React.FC = () => {
  const [activeTab, setActiveTab] = useState<string>('audio');

  // Audio & Volume Ceiling state
  const [telemetry, setTelemetry] = useState<HardwareTelemetry | null>(null);
  const [volume, setVolumeState] = useState<number>(65);
  const [ceiling, setCeilingState] = useState<number>(85);
  const [selectedRoute, setSelectedRoute] = useState<AudioRoute>('gt112_default');
  const [audioSaving, setAudioSaving] = useState(false);
  const [audioMessage, setAudioMessage] = useState<string | null>(null);

  // Network state
  const [networkStatus, setNetworkStatus] = useState<NetworkStatus | null>(null);
  const [wifiNetworks, setWifiNetworks] = useState<WifiNetwork[]>([]);
  const [isScanningWifi, setIsScanningWifi] = useState(false);
  const [selectedSsid, setSelectedSsid] = useState<string>('');
  const [wifiPassword, setWifiPassword] = useState<string>('');
  const [isConnectingWifi, setIsConnectingWifi] = useState(false);
  const [networkMessage, setNetworkMessage] = useState<string | null>(null);

  // Bluetooth state
  const [btStatus, setBtStatus] = useState<BluetoothStatus | null>(null);
  const [btDevices, setBtDevices] = useState<BluetoothDevice[]>([]);
  const [isScanningBt, setIsScanningBt] = useState(false);
  const [btActionLoading, setBtActionLoading] = useState(false);
  const [btMessage, setBtMessage] = useState<string | null>(null);

  // VPN state
  const [vpnStatus, setVpnStatus] = useState<VpnStatus | null>(null);
  const [vlessUri, setVlessUri] = useState<string>('');
  const [isVpnLoading, setIsVpnLoading] = useState(false);
  const [vpnMessage, setVpnMessage] = useState<string | null>(null);

  // KAMISH OS: OTA Updates state
  const [updateStatus, setUpdateStatus] = useState<UpdateStatus | null>(null);
  const [selectedFile, setSelectedFile] = useState<File | null>(null);
  const [bundleInfo, setBundleInfo] = useState<UpdateBundleInfo | null>(null);
  const [isUploadingBundle, setIsUploadingBundle] = useState(false);
  const [isInstallingBundle, setIsInstallingBundle] = useState(false);
  const [updateMessage, setUpdateMessage] = useState<string | null>(null);
  const [otaError, setOtaError] = useState<string | null>(null);

  // KAMISH OS: Dropbear SSH state
  const [sshStatus, setSshStatus] = useState<SshStatus | null>(null);
  const [sshLoading, setSshLoading] = useState(false);
  const [tempPassword, setTempPassword] = useState<string | null>(null);
  const [ed25519Key, setEd25519Key] = useState<string>('');
  const [copiedPass, setCopiedPass] = useState(false);
  const [sshMessage, setSshMessage] = useState<string | null>(null);

  // AI Provider state (Antigravity CLI vs Offline Mock)
  const [aiStatus, setAiStatus] = useState<AiReadiness | null>(null);
  const [aiMode, setAiMode] = useState<'cli' | 'mock'>('cli');
  const [cliPath, setCliPath] = useState<string>('/opt/kamish/bin/agy');
  const [aiLoading, setAiLoading] = useState(false);
  const [aiMessage, setAiMessage] = useState<string | null>(null);
  const [aiError, setAiError] = useState<string | null>(null);

  // Load Audio & Telemetry
  const refreshAudio = useCallback(async () => {
    try {
      const data = await getHardwareTelemetry();
      setTelemetry(data);
      if (data.audio_routing) {
        setVolumeState(data.audio_routing.volume);
        setCeilingState(data.audio_routing.safe_volume_ceiling);
      }
    } catch (err) {
      console.warn('Failed to load telemetry:', err);
    }
  }, []);

  // Load Network
  const refreshNetwork = useCallback(async () => {
    try {
      const data = await getNetworkStatus();
      setNetworkStatus(data);
    } catch (err) {
      console.warn('Failed to load network status:', err);
    }
  }, []);

  // Load Bluetooth
  const refreshBluetooth = useCallback(async () => {
    try {
      const data = await getBluetoothStatus();
      setBtStatus(data);
    } catch (err) {
      console.warn('Failed to load Bluetooth status:', err);
    }
  }, []);

  // Load VPN
  const refreshVpn = useCallback(async () => {
    try {
      const data = await getVpnStatus();
      setVpnStatus(data);
    } catch (err) {
      console.warn('Failed to load VPN status:', err);
    }
  }, []);

  // Load OTA Update Status
  const refreshUpdate = useCallback(async () => {
    try {
      const data = await getUpdateStatus();
      setUpdateStatus(data);
    } catch (err) {
      console.warn('Failed to load update status:', err);
    }
  }, []);

  // Load SSH Status
  const refreshSsh = useCallback(async () => {
    try {
      const data = await getSshStatus();
      setSshStatus(data);
    } catch (err) {
      console.warn('Failed to load SSH status:', err);
    }
  }, []);

  // Load AI Provider Status
  const refreshAi = useCallback(async () => {
    try {
      const data = await getAiStatus();
      setAiStatus(data);
      setAiMode(data.mode);
      setCliPath(data.cli_path);
    } catch (err) {
      console.warn('Failed to load AI status:', err);
    }
  }, []);

  useEffect(() => {
    refreshAudio();
    refreshNetwork();
    refreshBluetooth();
    refreshVpn();
    refreshUpdate();
    refreshSsh();
    refreshAi();
  }, [refreshAudio, refreshNetwork, refreshBluetooth, refreshVpn, refreshUpdate, refreshSsh, refreshAi]);

  // Audio handlers
  const handleApplyVolume = async (val: number) => {
    setAudioSaving(true);
    setAudioMessage(null);
    try {
      const res = await setAudioVolume(val);
      setVolumeState(res.applied_volume);
      setAudioMessage(`Громкость установлена: ${res.applied_volume}% (ограничение ${res.safe_ceiling}%)`);
    } catch (err: any) {
      setAudioMessage(`Ошибка: ${err.message}`);
    } finally {
      setAudioSaving(false);
    }
  };

  const handleApplyCeiling = async () => {
    setAudioSaving(true);
    setAudioMessage(null);
    try {
      const res = await setVolumeCeiling(ceiling);
      setCeilingState(res.safe_ceiling);
      setAudioMessage(`Безопасный лимитер GT-112 установлен на ${res.safe_ceiling}%`);
    } catch (err: any) {
      setAudioMessage(`Ошибка: ${err.message}`);
    } finally {
      setAudioSaving(false);
    }
  };

  const handleSelectRoute = async (route: AudioRoute) => {
    setSelectedRoute(route);
    try {
      const res = await setAudioRoute(route);
      setAudioMessage(`Маршрут переключен: ${res.description}`);
    } catch (err: any) {
      setAudioMessage(`Ошибка переключения: ${err.message}`);
    }
  };

  // Wi-Fi handlers
  const handleScanWifi = async () => {
    setIsScanningWifi(true);
    setNetworkMessage(null);
    try {
      const res = await scanWifi();
      setWifiNetworks(res.networks);
      setNetworkMessage(`Найдено сетей: ${res.count}`);
    } catch (err: any) {
      setNetworkMessage(`Ошибка сканирования: ${err.message}`);
    } finally {
      setIsScanningWifi(false);
    }
  };

  const handleConnectWifi = async (e: React.FormEvent) => {
    e.preventDefault();
    if (!selectedSsid) return;
    setIsConnectingWifi(true);
    setNetworkMessage(null);
    try {
      const res = await connectWifi({ ssid: selectedSsid, password: wifiPassword });
      setNetworkMessage(`Успешное подключение к ${res.ssid} (${res.ip_address})`);
      setWifiPassword('');
      refreshNetwork();
    } catch (err: any) {
      setNetworkMessage(`Ошибка подключения: ${err.message}`);
    } finally {
      setIsConnectingWifi(false);
    }
  };

  const handleToggleAp = async () => {
    if (!networkStatus) return;
    setNetworkMessage(null);
    try {
      if (networkStatus.ap_mode.active) {
        await stopApMode();
        setNetworkMessage('Режим точки доступа отключён');
      } else {
        const res = await startApMode();
        setNetworkMessage(`Точка доступа ${res.ssid} активирована (${res.ip})`);
      }
      refreshNetwork();
    } catch (err: any) {
      setNetworkMessage(`Ошибка изменения AP режима: ${err.message}`);
    }
  };

  // Bluetooth handlers
  const handleScanBt = async () => {
    setIsScanningBt(true);
    setBtMessage(null);
    try {
      const res = await scanBluetooth(5);
      setBtDevices(res.devices_found);
      setBtMessage(`Найдено устройств: ${res.devices_found.length}`);
    } catch (err: any) {
      setBtMessage(`Ошибка поиска устройств: ${err.message}`);
    } finally {
      setIsScanningBt(false);
    }
  };

  const handlePairBt = async (mac: string) => {
    setBtActionLoading(true);
    setBtMessage(null);
    try {
      await pairBluetooth(mac);
      setBtMessage(`Устройство ${mac} сопряжено`);
      refreshBluetooth();
    } catch (err: any) {
      setBtMessage(`Ошибка сопряжения: ${err.message}`);
    } finally {
      setBtActionLoading(false);
    }
  };

  const handleConnectBt = async (mac: string) => {
    setBtActionLoading(true);
    setBtMessage(null);
    try {
      await connectBluetooth(mac);
      setBtMessage(`Аудиопоток подключён к ${mac}`);
      refreshBluetooth();
    } catch (err: any) {
      setBtMessage(`Ошибка подключения: ${err.message}`);
    } finally {
      setBtActionLoading(false);
    }
  };

  const handleDisconnectBt = async () => {
    setBtActionLoading(true);
    setBtMessage(null);
    try {
      await disconnectBluetooth();
      setBtMessage('Bluetooth отключён');
      refreshBluetooth();
    } catch (err: any) {
      setBtMessage(`Ошибка отключения: ${err.message}`);
    } finally {
      setBtActionLoading(false);
    }
  };

  // VPN handlers
  const handleToggleVpn = async () => {
    if (!vpnStatus) return;
    setIsVpnLoading(true);
    setVpnMessage(null);
    try {
      if (vpnStatus.enabled) {
        await disconnectVpn();
        setVpnMessage('VPN отключён');
      } else {
        const res = await connectVpn();
        setVpnMessage(`VPN подключён (пинг ${res.ping_ms || 25} мс)`);
      }
      refreshVpn();
    } catch (err: any) {
      setVpnMessage(`Ошибка VPN: ${err.message}`);
    } finally {
      setIsVpnLoading(false);
    }
  };

  const handleSaveVpnConfig = async (e: React.FormEvent) => {
    e.preventDefault();
    if (!vlessUri.trim()) return;
    setIsVpnLoading(true);
    setVpnMessage(null);
    try {
      const res = await configureVpn({ vless_uri: vlessUri });
      setVpnMessage(`Конфигурация VLESS Reality обновлена: сервер ${res.server}`);
      setVlessUri('');
      refreshVpn();
    } catch (err: any) {
      setVpnMessage(`Ошибка конфигурации: ${err.message}`);
    } finally {
      setIsVpnLoading(false);
    }
  };

  // KAMISH OS: OTA Update Handlers
  const handleFileChange = (e: React.ChangeEvent<HTMLInputElement>) => {
    if (e.target.files && e.target.files[0]) {
      const file = e.target.files[0];
      setSelectedFile(file);
      setBundleInfo(null);
      setOtaError(null);
      setUpdateMessage(null);
    }
  };

  const handleUploadBundle = async () => {
    if (!selectedFile) return;
    setIsUploadingBundle(true);
    setOtaError(null);
    setUpdateMessage(null);
    try {
      const info = await uploadUpdateBundle(selectedFile);
      setBundleInfo(info);
      setUpdateMessage(`Пакет прошивки v${info.version} проверен: цифровая подпись валидна.`);
    } catch (err: any) {
      setOtaError(err.message || 'Ошибка проверки бандла прошивки');
    } finally {
      setIsUploadingBundle(false);
    }
  };

  const handleInstallBundle = async () => {
    setIsInstallingBundle(true);
    setOtaError(null);
    try {
      const res = await installUpdate();
      setUpdateMessage(`Прошивка успешно записана в Слот ${res.target_slot}. Колонка готова к перезагрузке.`);
      refreshUpdate();
    } catch (err: any) {
      setOtaError(err.message || 'Ошибка установки обновления');
    } finally {
      setIsInstallingBundle(false);
    }
  };

  // KAMISH OS: Dropbear SSH Handlers
  const handleToggleSsh = async (enable: boolean) => {
    setSshLoading(true);
    setSshMessage(null);
    try {
      const res = await toggleSsh({
        enable,
        ssh_key: ed25519Key.trim() ? ed25519Key.trim() : null,
      });
      setSshStatus((prev) => prev ? { ...prev, enabled: res.enabled } : null);
      if (res.temp_password) {
        setTempPassword(res.temp_password);
      } else {
        setTempPassword(null);
      }
      setSshMessage(res.message);
      refreshSsh();
    } catch (err: any) {
      setSshMessage(`Ошибка SSH: ${err.message}`);
    } finally {
      setSshLoading(false);
    }
  };

  const handleCopyPassword = () => {
    if (!tempPassword) return;
    navigator.clipboard.writeText(tempPassword);
    setCopiedPass(true);
    setTimeout(() => setCopiedPass(false), 2000);
  };

  // AI Provider Handlers
  const handleSaveAiConfig = async (e: React.FormEvent) => {
    e.preventDefault();
    setAiLoading(true);
    setAiMessage(null);
    setAiError(null);
    try {
      const res = await updateAiConfig({
        mode: aiMode,
        cli_path: cliPath.trim(),
      });
      setAiStatus(res);
      if (res.ready) {
        setAiMessage(
          res.mode === 'cli'
            ? `Конфигурация сохранена: Antigravity CLI проверен (${res.cli_version || 'готов'}).`
            : 'Включён режим локальной офлайн-заглушки (Mock). Система готова.'
        );
      } else {
        setAiError(`Настройка не завершена: ${res.error_reason || 'бинарник CLI недоступен'}`);
      }
    } catch (err: any) {
      setAiError(err.message || 'Ошибка обновления конфигурации ИИ');
    } finally {
      setAiLoading(false);
    }
  };

  const handleProbeAi = async () => {
    setAiLoading(true);
    setAiMessage(null);
    setAiError(null);
    try {
      const res = await probeAi();
      setAiStatus(res);
      if (res.ready) {
        setAiMessage(`Проверка успешна: ${res.cli_version || 'готов к вызовам'}`);
      } else {
        setAiError(`Сбой проверки CLI: ${res.error_reason}`);
      }
    } catch (err: any) {
      setAiError(err.message || 'Ошибка запуска probe CLI');
    } finally {
      setAiLoading(false);
    }
  };

  const tabs = [
    {
      id: 'audio',
      label: 'Аудио и GT-112',
      icon: <Volume2 className="w-4 h-4" />,
      badge: <Badge variant="success" size="sm">Section 9</Badge>,
    },
    {
      id: 'ai',
      label: 'ИИ-провайдер',
      icon: <Bot className="w-4 h-4" />,
      badge: (
        <Badge variant={aiStatus?.ready ? 'success' : 'danger'} size="sm">
          {aiStatus?.ready ? (aiStatus.mode === 'cli' ? 'CLI Готов' : 'Заглушка') : 'Не настроен'}
        </Badge>
      ),
    },
    {
      id: 'speech',
      label: 'Речь и STT/TTS',
      icon: <Mic className="w-4 h-4" />,
      badge: <Badge variant="default" size="sm">Stage 6</Badge>,
    },
    {
      id: 'network',
      label: 'Wi-Fi и Сеть',
      icon: <Wifi className="w-4 h-4" />,
      badge: <Badge variant="default" size="sm">Stage 7</Badge>,
    },
    {
      id: 'bluetooth',
      label: 'Bluetooth A2DP',
      icon: <Bluetooth className="w-4 h-4" />,
      badge: <Badge variant="default" size="sm">Stage 7</Badge>,
    },
    {
      id: 'vpn',
      label: 'VPN (Happ/VLESS)',
      icon: <Shield className="w-4 h-4" />,
      badge: <Badge variant="default" size="sm">Stage 7</Badge>,
    },
    {
      id: 'ota',
      label: 'Обновление A/B',
      icon: <UploadCloud className="w-4 h-4" />,
      badge: <Badge variant="default" size="sm">RAUC</Badge>,
    },
    {
      id: 'ssh',
      label: 'SSH и отладка',
      icon: <Terminal className="w-4 h-4" />,
      badge: <Badge variant={sshStatus?.enabled ? 'warning' : 'default'} size="sm">Dropbear</Badge>,
    },
  ];

  return (
    <div className="space-y-6">
      {/* Header */}
      <div>
        <h1 className="text-2xl font-bold text-slate-100 tracking-tight">Настройки системы</h1>
        <p className="text-sm text-slate-400 mt-0.5">
          Управление периферией устройства, акустикой GT-112, беспроводными интерфейсами и безопасным туннелем.
        </p>
      </div>

      {/* Tabs */}
      <Tabs tabs={tabs} activeTab={activeTab} onChange={setActiveTab} />

      {/* TAB 1: AUDIO & GT-112 */}
      {activeTab === 'audio' && (
        <div className="space-y-6">
          {audioMessage && (
            <div className="p-3 rounded-xl bg-slate-800 border border-slate-700 text-sm text-slate-200 flex items-center gap-2">
              <Check className="w-4 h-4 text-sky-400 shrink-0" />
              <span>{audioMessage}</span>
            </div>
          )}

          <div className="grid grid-cols-1 md:grid-cols-2 gap-6">
            {/* Volume & Safe Ceiling Card */}
            <Card title="Акустическая система GT-112">
              <div className="space-y-5">
                <div>
                  <div className="flex justify-between items-center text-sm font-medium mb-1.5">
                    <span className="text-slate-300">Текущая громкость</span>
                    <span className="text-sky-400 font-semibold">{volume}%</span>
                  </div>
                  <input
                    type="range"
                    min="0"
                    max="100"
                    value={volume}
                    onChange={(e) => setVolumeState(Number(e.target.value))}
                    onMouseUp={(e) => handleApplyVolume(Number((e.target as HTMLInputElement).value))}
                    className="w-full accent-sky-500 bg-slate-800 rounded-lg cursor-pointer h-2"
                  />
                  <div className="flex justify-between text-xs text-slate-500 mt-1">
                    <span>0% (Тишина)</span>
                    <span>Безопасный лимит: {ceiling}%</span>
                  </div>
                  {telemetry?.audio_routing && (
                    <div className="text-xs text-slate-400 mt-2">
                      Устройство: <span className="text-slate-200">{telemetry.audio_routing.output_device}</span> ({telemetry.audio_routing.mixer_status})
                    </div>
                  )}
                </div>

                <div className="pt-4 border-t border-slate-800">
                  <div className="flex justify-between items-center text-sm font-medium mb-1.5">
                    <span className="text-slate-300">Безопасный потолок (Safe Ceiling)</span>
                    <span className="text-amber-400 font-semibold">{ceiling}%</span>
                  </div>
                  <p className="text-xs text-slate-400 mb-2">
                    Предотвращает клиппинг и тепловой пробой катушек динамиков GT-112 (Раздел 9 ТЗ).
                  </p>
                  <div className="flex items-center gap-3">
                    <input
                      type="range"
                      min="50"
                      max="100"
                      value={ceiling}
                      onChange={(e) => setCeilingState(Number(e.target.value))}
                      className="flex-1 accent-amber-500 bg-slate-800 rounded-lg cursor-pointer h-2"
                    />
                    <Button
                      size="sm"
                      variant="secondary"
                      onClick={handleApplyCeiling}
                      disabled={audioSaving}
                    >
                      Сохранить
                    </Button>
                  </div>
                </div>
              </div>
            </Card>

            {/* Audio Route Selector */}
            <Card title="Маршрутизация аудиовыхода">
              <div className="space-y-3">
                {[
                  { id: 'gt112_default' as AudioRoute, label: 'GT-112 Line Out (ALSA default)', desc: 'Основной аналоговый усилитель' },
                  { id: 'bluetooth_sink' as AudioRoute, label: 'Bluetooth A2DP Sink', desc: 'Беспроводная трансляция со смартфона' },
                  { id: 'headphones' as AudioRoute, label: 'Наушники (3.5mm)', desc: 'Прямой выход на разъём' },
                  { id: 'system_default' as AudioRoute, label: 'Системный аудиовыход', desc: 'PipeWire / PulseAudio fallback' },
                ].map((item) => (
                  <button
                    key={item.id}
                    onClick={() => handleSelectRoute(item.id)}
                    className={`w-full p-3 rounded-xl border text-left transition-colors cursor-pointer flex items-center justify-between ${
                      selectedRoute === item.id
                        ? 'bg-slate-800 border-sky-500/50 text-slate-100'
                        : 'bg-slate-900 border-slate-800 text-slate-400 hover:border-slate-700'
                    }`}
                  >
                    <div>
                      <div className="text-sm font-medium text-slate-200">{item.label}</div>
                      <div className="text-xs text-slate-400">{item.desc}</div>
                    </div>
                    {selectedRoute === item.id && (
                      <span className="w-2 h-2 rounded-full bg-sky-400" />
                    )}
                  </button>
                ))}
              </div>
            </Card>
          </div>
        </div>
      )}

      {/* TAB 2: AI PROVIDER & ANTIGRAVITY CLI */}
      {activeTab === 'ai' && (
        <div className="space-y-6">
          {aiMessage && (
            <div className="p-3 rounded-xl bg-slate-800 border border-slate-700 text-sm text-slate-200 flex items-center gap-2">
              <CheckCircle2 className="w-4 h-4 text-emerald-400 shrink-0" />
              <span>{aiMessage}</span>
            </div>
          )}

          {aiError && (
            <div className="p-3 rounded-xl bg-rose-500/10 border border-rose-500/30 text-sm text-rose-300 flex items-center gap-2">
              <AlertTriangle className="w-4 h-4 text-rose-400 shrink-0" />
              <span>{aiError}</span>
            </div>
          )}

          <div className="grid grid-cols-1 md:grid-cols-2 gap-6">
            {/* Status & Architectural Policy */}
            <Card title="Состояние ИИ-провайдера">
              <div className="space-y-4 text-sm">
                <div className="p-3 rounded-xl bg-slate-900 border border-slate-800 text-xs text-slate-400">
                  <span className="font-semibold text-white block mb-1">Архитектурный инвариант:</span>
                  Внешние облачные REST API отключены в ядре. Вся работа с ИИ ведётся исключительно через локальный процесс <span className="font-mono text-slate-300">Antigravity CLI (agy)</span> через stdin/stdout либо через встроенную локальную заглушку.
                </div>

                <div className="flex justify-between items-center py-2 border-b border-slate-800">
                  <span className="text-slate-400">Текущий режим</span>
                  <Badge variant={aiStatus?.mode === 'cli' ? 'default' : 'default'}>
                    {aiStatus?.mode === 'cli' ? 'Antigravity CLI' : 'Встроенная заглушка (Mock)'}
                  </Badge>
                </div>

                <div className="flex justify-between items-center py-2 border-b border-slate-800">
                  <span className="text-slate-400">Статус готовности</span>
                  <Badge variant={aiStatus?.ready ? 'success' : 'danger'}>
                    {aiStatus?.ready ? 'Готов к работе' : 'Настройка не завершена'}
                  </Badge>
                </div>

                <div className="flex justify-between items-center py-2 border-b border-slate-800">
                  <span className="text-slate-400">Путь к CLI</span>
                  <span className="font-mono text-slate-200 text-xs truncate max-w-[200px]" title={aiStatus?.cli_path}>
                    {aiStatus?.cli_path || '/opt/kamish/bin/agy'}
                  </span>
                </div>

                {aiStatus?.cli_version && (
                  <div className="flex justify-between items-center py-2 border-b border-slate-800">
                    <span className="text-slate-400">Версия CLI</span>
                    <span className="font-mono text-emerald-400 text-xs">{aiStatus.cli_version}</span>
                  </div>
                )}

                {aiStatus?.error_reason && (
                  <div className="p-3 rounded-xl bg-rose-500/10 border border-rose-500/20 text-xs text-rose-300 space-y-1">
                    <span className="font-semibold block">Причина сбоя готовности:</span>
                    <p className="font-mono">{aiStatus.error_reason}</p>
                    <span className="text-[11px] text-rose-400 block mt-1">
                      Колонка блокирует голосовые ответы, пока настройка не завершена.
                    </span>
                  </div>
                )}

                <div className="pt-2">
                  <Button
                    size="sm"
                    variant="secondary"
                    onClick={handleProbeAi}
                    isLoading={aiLoading}
                    className="w-full"
                  >
                    Выполнить проверку бинарника (Health Probe)
                  </Button>
                </div>
              </div>
            </Card>

            {/* Configuration Form */}
            <Card title="Конфигурация Antigravity CLI">
              <form onSubmit={handleSaveAiConfig} className="space-y-4">
                <div>
                  <label className="block text-xs font-medium text-slate-400 mb-2">
                    Выберите режим работы ИИ
                  </label>
                  <div className="grid grid-cols-2 gap-2">
                    <button
                      type="button"
                      onClick={() => setAiMode('cli')}
                      className={`p-3 rounded-xl border text-left text-xs transition-colors cursor-pointer ${
                        aiMode === 'cli'
                          ? 'bg-slate-800 border-slate-600 text-white'
                          : 'bg-slate-900 border-slate-800 text-slate-400 hover:border-slate-700'
                      }`}
                    >
                      <span className="font-semibold block text-white mb-0.5">Antigravity CLI</span>
                      <span>Внешний процесс agy через stdin/stdout</span>
                    </button>

                    <button
                      type="button"
                      onClick={() => setAiMode('mock')}
                      className={`p-3 rounded-xl border text-left text-xs transition-colors cursor-pointer ${
                        aiMode === 'mock'
                          ? 'bg-sky-600/20 border-sky-500 text-sky-200'
                          : 'bg-slate-900 border-slate-800 text-slate-400 hover:border-slate-700'
                      }`}
                    >
                      <span className="font-semibold block text-white mb-0.5">Офлайн-заглушка</span>
                      <span>Встроенный парсер 14 локальных инструментов</span>
                    </button>
                  </div>
                </div>

                {aiMode === 'cli' && (
                  <div className="space-y-2 pt-2 border-t border-slate-800">
                    <label className="block text-xs font-medium text-slate-400">
                      Путь к исполняемому файлу CLI (AI_CLI_PATH)
                    </label>
                    <input
                      type="text"
                      value={cliPath}
                      onChange={(e) => setCliPath(e.target.value)}
                      placeholder="/opt/kamish/bin/agy или agy"
                      className="w-full bg-slate-900 border border-slate-700 rounded-lg px-3 py-2 text-xs font-mono text-white focus:outline-none focus:border-slate-500"
                    />

                    {/* Presets */}
                    <div className="flex flex-wrap gap-2 pt-1">
                      {[
                        { label: 'Тестовый эмулятор', path: 'scripts/agy-mock.sh' },
                        { label: 'Штатный путь OS', path: '/opt/kamish/bin/agy' },
                        { label: 'Системный PATH', path: 'agy' },
                      ].map((preset) => (
                        <button
                          key={preset.label}
                          type="button"
                          onClick={() => setCliPath(preset.path)}
                          className="px-2 py-1 rounded bg-slate-800 hover:bg-slate-700 text-[11px] font-mono text-slate-300 border border-slate-700 transition-colors"
                        >
                          {preset.label}: {preset.path}
                        </button>
                      ))}
                    </div>
                  </div>
                )}

                <div className="pt-3">
                  <Button
                    size="sm"
                    variant="primary"
                    type="submit"
                    isLoading={aiLoading}
                    className="w-full"
                  >
                    Сохранить настройки и применить
                  </Button>
                </div>
              </form>
            </Card>
          </div>
        </div>
      )}

      {/* TAB 2: SPEECH & VOICE */}
      {activeTab === 'speech' && <SpeechManagerPanel />}

      {/* TAB 3: NETWORK & WI-FI */}
      {activeTab === 'network' && (
        <div className="space-y-6">
          {networkMessage && (
            <div className="p-3 rounded-xl bg-slate-800 border border-slate-700 text-sm text-slate-200 flex items-center gap-2">
              <Check className="w-4 h-4 text-sky-400 shrink-0" />
              <span>{networkMessage}</span>
            </div>
          )}

          <div className="grid grid-cols-1 md:grid-cols-2 gap-6">
            {/* Current Network Status */}
            <Card title="Сетевые интерфейсы">
              <div className="space-y-3 text-sm">
                <div className="flex justify-between py-1 border-b border-slate-800">
                  <span className="text-slate-400">Основной интерфейс</span>
                  <span className="text-slate-200">{networkStatus?.primary_interface || 'eth0'}</span>
                </div>
                <div className="flex justify-between py-1 border-b border-slate-800">
                  <span className="text-slate-400">IP-адрес</span>
                  <span className="text-sky-400 font-mono">{networkStatus?.ip_address || '192.168.1.150'}</span>
                </div>
                <div className="flex justify-between py-1 border-b border-slate-800">
                  <span className="text-slate-400">Шлюз</span>
                  <span className="text-slate-200 font-mono">{networkStatus?.gateway || '192.168.1.1'}</span>
                </div>
                <div className="flex justify-between py-1 border-b border-slate-800">
                  <span className="text-slate-400">DNS-серверы</span>
                  <span className="text-slate-200 font-mono">{networkStatus?.dns.join(', ') || '1.1.1.1'}</span>
                </div>
                <div className="flex justify-between py-1 border-b border-slate-800">
                  <span className="text-slate-400">Режим точки доступа (AP)</span>
                  <Badge variant={networkStatus?.ap_mode.active ? 'warning' : 'outline'}>
                    {networkStatus?.ap_mode.active
                      ? `Активна (${networkStatus?.ap_mode.ssid || 'Kamish-Setup'})`
                      : 'Отключена'}
                  </Badge>
                </div>

                <div className="pt-2 flex gap-3">
                  <Button
                    size="sm"
                    variant={networkStatus?.ap_mode.active ? 'danger' : 'secondary'}
                    onClick={handleToggleAp}
                  >
                    {networkStatus?.ap_mode.active ? 'Остановить AP' : 'Включить Setup AP'}
                  </Button>
                  <Button size="sm" variant="ghost" onClick={refreshNetwork}>
                    <RefreshCw className="w-3.5 h-3.5 mr-1" /> Обновить
                  </Button>
                </div>
              </div>
            </Card>

            {/* Wi-Fi Scanner & Connection */}
            <Card title="Подключение к Wi-Fi">
              <form onSubmit={handleConnectWifi} className="space-y-4">
                <div className="flex items-center justify-between">
                  <span className="text-xs text-slate-400">Доступные беспроводные сети</span>
                  <Button
                    size="sm"
                    variant="secondary"
                    type="button"
                    onClick={handleScanWifi}
                    isLoading={isScanningWifi}
                  >
                    Сканировать
                  </Button>
                </div>

                {wifiNetworks.length > 0 ? (
                  <div className="max-h-48 overflow-y-auto space-y-1 pr-1 border border-slate-800 rounded-xl p-2 bg-slate-950/40">
                    {wifiNetworks.map((net) => (
                      <button
                        key={`${net.bssid}-${net.ssid}`}
                        type="button"
                        onClick={() => setSelectedSsid(net.ssid)}
                        className={`w-full p-2 rounded-lg text-left text-xs transition-colors cursor-pointer flex items-center justify-between ${
                          selectedSsid === net.ssid
                            ? 'bg-sky-950/40 border border-sky-500/30 text-sky-200'
                            : 'hover:bg-slate-800 text-slate-300'
                        }`}
                      >
                        <span className="font-medium truncate">{net.ssid}</span>
                        <span className="text-slate-400">{net.signal_percent}% ({net.security})</span>
                      </button>
                    ))}
                  </div>
                ) : (
                  <p className="text-xs text-slate-500 italic">Нажмите «Сканировать» для поиска сетей</p>
                )}

                <div>
                  <label className="block text-xs text-slate-400 mb-1">SSID сети</label>
                  <input
                    type="text"
                    value={selectedSsid}
                    onChange={(e) => setSelectedSsid(e.target.value)}
                    placeholder="Имя сети"
                    className="w-full px-3 py-2 bg-slate-800 border border-slate-700 rounded-lg text-sm text-slate-100 focus:outline-none focus:border-sky-500"
                  />
                </div>

                <div>
                  <label className="block text-xs text-slate-400 mb-1">Пароль (WPA2/WPA3)</label>
                  <input
                    type="password"
                    value={wifiPassword}
                    onChange={(e) => setWifiPassword(e.target.value)}
                    placeholder="Пароль от Wi-Fi"
                    className="w-full px-3 py-2 bg-slate-800 border border-slate-700 rounded-lg text-sm text-slate-100 focus:outline-none focus:border-sky-500"
                  />
                </div>

                <div className="flex gap-2">
                  <Button
                    size="sm"
                    variant="primary"
                    type="submit"
                    disabled={!selectedSsid}
                    isLoading={isConnectingWifi}
                  >
                    Подключиться
                  </Button>
                  <Button
                    size="sm"
                    variant="ghost"
                    type="button"
                    onClick={() => disconnectWifi().then(() => refreshNetwork())}
                  >
                    Отключить Wi-Fi
                  </Button>
                </div>
              </form>
            </Card>
          </div>
        </div>
      )}

      {/* TAB 4: BLUETOOTH A2DP */}
      {activeTab === 'bluetooth' && (
        <div className="space-y-6">
          {btMessage && (
            <div className="p-3 rounded-xl bg-slate-800 border border-slate-700 text-sm text-slate-200 flex items-center gap-2">
              <Check className="w-4 h-4 text-sky-400 shrink-0" />
              <span>{btMessage}</span>
            </div>
          )}

          <div className="grid grid-cols-1 md:grid-cols-2 gap-6">
            <Card title="Состояние Bluetooth A2DP Sink">
              <div className="space-y-3 text-sm">
                <div className="flex justify-between py-1 border-b border-slate-800">
                  <span className="text-slate-400">Адаптер</span>
                  <Badge variant={btStatus?.adapter_enabled ? 'success' : 'danger'}>
                    {btStatus?.adapter_enabled ? 'Включён' : 'Выключен'}
                  </Badge>
                </div>
                <div className="flex justify-between py-1 border-b border-slate-800">
                  <span className="text-slate-400">Режим видимости</span>
                  <span className="text-slate-200">{btStatus?.discoverable ? 'Видим для устройств' : 'Скрыт'}</span>
                </div>
                <div className="flex justify-between py-1 border-b border-slate-800">
                  <span className="text-slate-400">Подключённое устройство</span>
                  <span className="text-sky-400">
                    {btStatus?.connected_device ? btStatus.connected_device.name : 'Нет подключений'}
                  </span>
                </div>
                {btStatus?.connected_device && (
                  <div className="pt-2">
                    <Button
                      size="sm"
                      variant="danger"
                      onClick={handleDisconnectBt}
                      isLoading={btActionLoading}
                    >
                      Отключить устройство
                    </Button>
                  </div>
                )}
              </div>
            </Card>

            <Card title="Поиск и сопряжение устройств">
              <div className="space-y-4">
                <div className="flex justify-between items-center">
                  <span className="text-xs text-slate-400">Окружающие устройства</span>
                  <Button
                    size="sm"
                    variant="secondary"
                    onClick={handleScanBt}
                    isLoading={isScanningBt}
                  >
                    Поиск устройств
                  </Button>
                </div>

                <div className="space-y-2 max-h-60 overflow-y-auto">
                  {btDevices.length > 0 ? (
                    btDevices.map((dev) => (
                      <div
                        key={dev.mac}
                        className="p-2.5 rounded-xl border border-slate-800 bg-slate-950/40 flex items-center justify-between text-xs"
                      >
                        <div>
                          <div className="font-medium text-slate-200">{dev.name}</div>
                          <div className="text-slate-500 font-mono">{dev.mac}</div>
                        </div>
                        <div className="flex gap-2">
                          <Button
                            size="sm"
                            variant="secondary"
                            onClick={() => handlePairBt(dev.mac)}
                            disabled={btActionLoading}
                          >
                            Сопрячь
                          </Button>
                          <Button
                            size="sm"
                            variant="primary"
                            onClick={() => handleConnectBt(dev.mac)}
                            disabled={btActionLoading}
                          >
                            Подключить
                          </Button>
                        </div>
                      </div>
                    ))
                  ) : (
                    <p className="text-xs text-slate-500 italic">Нажмите «Поиск устройств» для обнаружения телефонов/ПК</p>
                  )}
                </div>
              </div>
            </Card>
          </div>
        </div>
      )}

      {/* TAB 5: VPN & PROXY (VLESS) */}
      {activeTab === 'vpn' && (
        <div className="space-y-6">
          {vpnMessage && (
            <div className="p-3 rounded-xl bg-slate-800 border border-slate-700 text-sm text-slate-200 flex items-center gap-2">
              <Check className="w-4 h-4 text-sky-400 shrink-0" />
              <span>{vpnMessage}</span>
            </div>
          )}

          <div className="grid grid-cols-1 md:grid-cols-2 gap-6">
            <Card title="Туннель Happ / VLESS Reality">
              <div className="space-y-3 text-sm">
                <div className="flex justify-between py-1 border-b border-slate-800">
                  <span className="text-slate-400">Статус туннеля</span>
                  <Badge variant={vpnStatus?.enabled ? 'success' : 'outline'}>
                    {vpnStatus?.enabled ? 'Подключён (VLESS)' : 'Отключён'}
                  </Badge>
                </div>
                <div className="flex justify-between py-1 border-b border-slate-800">
                  <span className="text-slate-400">Сервер</span>
                  <span className="text-slate-200 font-mono">{vpnStatus?.server_address || 'vless.kamish.internal'}</span>
                </div>
                <div className="flex justify-between py-1 border-b border-slate-800">
                  <span className="text-slate-400">Задержка (Ping)</span>
                  <span className="text-emerald-400 font-semibold">{vpnStatus?.ping_ms || 24} мс</span>
                </div>
                <div className="flex justify-between py-1 border-b border-slate-800">
                  <span className="text-slate-400">Передано / Получено</span>
                  <span className="text-slate-300 font-mono">
                    {((vpnStatus?.bytes_sent || 0) / (1024 * 1024)).toFixed(1)} MB / {((vpnStatus?.bytes_received || 0) / (1024 * 1024)).toFixed(1)} MB
                  </span>
                </div>

                <div className="pt-2">
                  <Button
                    size="sm"
                    variant={vpnStatus?.enabled ? 'danger' : 'primary'}
                    onClick={handleToggleVpn}
                    isLoading={isVpnLoading}
                  >
                    {vpnStatus?.enabled ? 'Отключить VPN' : 'Подключить VPN'}
                  </Button>
                </div>
              </div>
            </Card>

            <Card title="Конфигурация VLESS URI">
              <form onSubmit={handleSaveVpnConfig} className="space-y-4">
                <p className="text-xs text-slate-400">
                  Вставьте строку vless:// с поддержкой Reality и XTLS для обхода региональных блокировок музыкальных стримов.
                </p>
                <div>
                  <textarea
                    rows={4}
                    value={vlessUri}
                    onChange={(e) => setVlessUri(e.target.value)}
                    placeholder="vless://uuid@server:443?security=reality&sni=example.com..."
                    className="w-full p-2.5 bg-slate-800 border border-slate-700 rounded-lg text-xs font-mono text-slate-100 focus:outline-none focus:border-sky-500"
                  />
                </div>
                <Button
                  size="sm"
                  variant="primary"
                  type="submit"
                  disabled={!vlessUri.trim()}
                  isLoading={isVpnLoading}
                >
                  Применить конфигурацию
                </Button>
              </form>
            </Card>
          </div>
        </div>
      )}

      {/* TAB 6: OTA & A/B SYSTEM UPDATES (RAUC) */}
      {activeTab === 'ota' && (
        <div className="space-y-6">
          {updateMessage && (
            <div className="p-3 rounded-xl bg-slate-800 border border-slate-700 text-sm text-slate-200 flex items-center gap-2">
              <CheckCircle2 className="w-4 h-4 text-emerald-400 shrink-0" />
              <span>{updateMessage}</span>
            </div>
          )}

          {otaError && (
            <div className="p-3 rounded-xl bg-rose-500/10 border border-rose-500/30 text-sm text-rose-300 flex items-center gap-2">
              <AlertTriangle className="w-4 h-4 text-rose-400 shrink-0" />
              <span>{otaError}</span>
            </div>
          )}

          <div className="grid grid-cols-1 md:grid-cols-2 gap-6">
            {/* Slot & System Status */}
            <Card title="Состояние слотов A/B (KAMISH OS)">
              <div className="space-y-3 text-sm">
                <div className="flex justify-between py-1 border-b border-slate-800">
                  <span className="text-slate-400">Текущий активный слот</span>
                  <div className="flex items-center gap-1.5">
                    <Layers className="w-3.5 h-3.5 text-slate-400" />
                    <Badge variant="default">Слот {updateStatus?.active_slot || 'A'}</Badge>
                  </div>
                </div>
                <div className="flex justify-between py-1 border-b border-slate-800">
                  <span className="text-slate-400">Слот для обновления</span>
                  <span className="text-slate-200 font-mono">
                    Слот {updateStatus?.active_slot === 'A' ? 'B' : 'A'} (неактивный раздел)
                  </span>
                </div>
                <div className="flex justify-between py-1 border-b border-slate-800">
                  <span className="text-slate-400">Установленная версия прошивки</span>
                  <span className="text-slate-200 font-mono">v{updateStatus?.installed_version || '0.1.0'}</span>
                </div>
                <div className="flex justify-between py-1 border-b border-slate-800">
                  <span className="text-slate-400">Механизм отката</span>
                  <span className="text-emerald-400 text-xs">U-Boot Watchdog + Healthcheck (mark-good)</span>
                </div>
                <div className="flex justify-between py-1">
                  <span className="text-slate-400">Целевая платформа</span>
                  <span className="text-slate-300 font-mono text-xs">kamish (ARM64)</span>
                </div>
              </div>
            </Card>

            {/* Firmware Upload & Flash */}
            <Card title="Установка обновления (.raucb)">
              <div className="space-y-4">
                <p className="text-xs text-slate-400">
                  Выберите файл новой прошивки с расширением <span className="font-mono text-slate-200">.raucb</span>. Перед записью система выполнит проверку подлинности цифровой подписи X.509 разработчика.
                </p>

                <div className="p-3 rounded-lg border border-dashed border-slate-700 bg-slate-900/60 flex flex-col items-center justify-center text-center">
                  <UploadCloud className="w-8 h-8 text-slate-400 mb-2" />
                  <input
                    type="file"
                    accept=".raucb,.tar.gz"
                    onChange={handleFileChange}
                    className="text-xs text-slate-300 file:mr-3 file:py-1 file:px-2.5 file:rounded-md file:border-0 file:text-xs file:bg-slate-800 file:text-slate-200 hover:file:bg-slate-700 cursor-pointer"
                  />
                  {selectedFile && (
                    <span className="text-[11px] text-slate-400 mt-2 font-mono">
                      Файл: {selectedFile.name} ({(selectedFile.size / (1024 * 1024)).toFixed(1)} МБ)
                    </span>
                  )}
                </div>

                {selectedFile && !bundleInfo && (
                  <Button
                    size="sm"
                    variant="primary"
                    onClick={handleUploadBundle}
                    isLoading={isUploadingBundle}
                    className="w-full"
                  >
                    Проверить цифровую подпись бандла
                  </Button>
                )}

                {bundleInfo && (
                  <div className="p-3 rounded-xl bg-slate-900 border border-slate-800 space-y-2 text-xs">
                    <div className="flex justify-between items-center">
                      <span className="text-slate-400">Совместимость:</span>
                      <span className="text-slate-200 font-mono">{bundleInfo.compatible}</span>
                    </div>
                    <div className="flex justify-between items-center">
                      <span className="text-slate-400">Версия прошивки:</span>
                      <span className="text-slate-300 font-semibold font-mono">v{bundleInfo.version}</span>
                    </div>
                    <div className="flex justify-between items-center">
                      <span className="text-slate-400">Цифровая подпись:</span>
                      <span className="inline-flex items-center gap-1 text-emerald-400 font-medium">
                        <CheckCircle2 className="w-3.5 h-3.5" /> Подлинная (X.509 CMS)
                      </span>
                    </div>

                    <div className="pt-2">
                      <Button
                        size="sm"
                        variant="primary"
                        onClick={handleInstallBundle}
                        isLoading={isInstallingBundle}
                        className="w-full"
                      >
                        {isInstallingBundle
                          ? 'Установка в неактивный слот...'
                          : `Записать обновление в Слот ${updateStatus?.active_slot === 'A' ? 'B' : 'A'}`}
                      </Button>
                    </div>
                  </div>
                )}
              </div>
            </Card>
          </div>
        </div>
      )}

      {/* TAB 7: DROPBEAR SSH & DEBUG */}
      {activeTab === 'ssh' && (
        <div className="space-y-6">
          {sshMessage && (
            <div className="p-3 rounded-xl bg-slate-800 border border-slate-700 text-sm text-slate-200 flex items-center gap-2">
              <CheckCircle2 className="w-4 h-4 text-emerald-400 shrink-0" />
              <span>{sshMessage}</span>
            </div>
          )}

          <div className="grid grid-cols-1 md:grid-cols-2 gap-6">
            {/* Control & Security Policy */}
            <Card title="Служба Dropbear SSH">
              <div className="space-y-4 text-sm">
                <div className="p-3 rounded-xl bg-amber-500/10 border border-amber-500/20 text-xs text-amber-300 flex items-start gap-2.5">
                  <Lock className="w-4 h-4 shrink-0 mt-0.5" />
                  <div>
                    <span className="font-semibold block">Политика безопасности:</span>
                    По умолчанию SSH выключен. Прямой вход под пользователем root строго запрещён. Вход разрешён только для пользователя <span className="font-mono text-white">kamish</span> по одноразовому паролю или ключу Ed25519.
                  </div>
                </div>

                <div className="flex justify-between items-center py-2 border-b border-slate-800">
                  <div>
                    <span className="text-slate-200 block font-medium">Состояние службы SSH</span>
                    <span className="text-slate-400 text-xs">Порт 22 / Dropbear</span>
                  </div>
                  <Badge variant={sshStatus?.enabled ? 'warning' : 'outline'}>
                    {sshStatus?.enabled ? 'Включён' : 'Выключен'}
                  </Badge>
                </div>

                <div className="flex justify-between items-center py-2 border-b border-slate-800">
                  <span className="text-slate-400">Пользователь для подключения</span>
                  <span className="text-slate-200 font-mono">kamish</span>
                </div>

                <div className="flex justify-between items-center py-2 border-b border-slate-800">
                  <span className="text-slate-400">Активные SSH сессии</span>
                  <span className="text-slate-200 font-mono">{sshStatus?.active_sessions || 0}</span>
                </div>

                <div className="pt-2">
                  {sshStatus?.enabled ? (
                    <Button
                      size="sm"
                      variant="danger"
                      onClick={() => handleToggleSsh(false)}
                      isLoading={sshLoading}
                      className="w-full"
                    >
                      Выключить Dropbear SSH
                    </Button>
                  ) : (
                    <Button
                      size="sm"
                      variant="primary"
                      onClick={() => handleToggleSsh(true)}
                      isLoading={sshLoading}
                      className="w-full"
                    >
                      Включить SSH и сгенерировать временный пароль
                    </Button>
                  )}
                </div>
              </div>
            </Card>

            {/* Credentials & Key Setup */}
            <Card title="Учётные данные и ключи">
              <div className="space-y-4">
                {tempPassword && (
                  <div className="p-3.5 rounded-xl bg-slate-900 border border-slate-700 space-y-2">
                    <span className="text-xs text-slate-400 block font-medium">
                      Временный автоматически сгенерированный пароль:
                    </span>
                    <div className="flex items-center justify-between bg-slate-950 p-2.5 rounded-lg border border-slate-800">
                      <span className="font-mono text-emerald-400 text-sm tracking-wider font-semibold select-all">
                        {tempPassword}
                      </span>
                      <button
                        type="button"
                        onClick={handleCopyPassword}
                        className="px-2 py-1 rounded bg-slate-800 hover:bg-slate-700 text-slate-300 text-xs flex items-center gap-1 transition-colors"
                      >
                        {copiedPass ? <Check className="w-3.5 h-3.5 text-emerald-400" /> : <Copy className="w-3.5 h-3.5" />}
                        {copiedPass ? 'Скопировано' : 'Копировать'}
                      </button>
                    </div>
                    <div className="text-[11px] text-slate-400 font-mono mt-1">
                      Команда: <span className="text-slate-200">ssh kamish@kamish.local</span>
                    </div>
                  </div>
                )}

                <div>
                  <label className="block text-xs font-medium text-slate-400 mb-1">
                    Открытый SSH-ключ (Ed25519) для авторизации без пароля
                  </label>
                  <textarea
                    rows={3}
                    value={ed25519Key}
                    onChange={(e) => setEd25519Key(e.target.value)}
                    placeholder="ssh-ed25519 AAAAC3NzaC1lZDI1NTE5AAAAI... developer@host"
                    className="w-full p-2.5 bg-slate-900 border border-slate-700 rounded-lg text-xs font-mono text-slate-200 focus:outline-none focus:border-sky-500"
                  />
                  <span className="text-[11px] text-slate-500 block mt-1">
                    Ключ будет записан в ~/.ssh/authorized_keys пользователя kamish при включении службы.
                  </span>
                </div>
              </div>
            </Card>
          </div>
        </div>
      )}
    </div>
  );
};
