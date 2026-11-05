import React, { useState, useEffect } from 'react';
import {
  RotateCcw,
  Power,
} from 'lucide-react';
import { Card } from '../components/Card';
import { Button } from '../components/Button';
import {
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
} from '../api';
import type { NetworkStatus, WifiNetwork, VpnStatus } from '../types';

export const NetworkPage: React.FC = () => {
  const [networkStatus, setNetworkStatus] = useState<NetworkStatus | null>(null);
  const [wifiNetworks, setWifiNetworks] = useState<WifiNetwork[]>([]);
  const [isScanning, setIsScanning] = useState(false);
  const [selectedSsid, setSelectedSsid] = useState('');
  const [wifiPassword, setWifiPassword] = useState('');
  const [isConnecting, setIsConnecting] = useState(false);

  // AP state
  const [isApLoading, setIsApLoading] = useState(false);

  // VPN state
  const [vpnStatus, setVpnStatus] = useState<VpnStatus | null>(null);
  const [vlessConfig, setVlessConfig] = useState('');
  const [isVpnLoading, setIsVpnLoading] = useState(false);

  const [message, setMessage] = useState<{ text: string; type: 'success' | 'error' } | null>(null);

  const refreshAll = async () => {
    try {
      const [netRes, vpnRes] = await Promise.all([
        getNetworkStatus().catch(() => null),
        getVpnStatus().catch(() => null),
      ]);
      if (netRes) setNetworkStatus(netRes);
      if (vpnRes) setVpnStatus(vpnRes);
    } catch (err) {
      console.warn('Network refresh error:', err);
    }
  };

  useEffect(() => {
    refreshAll();
  }, []);

  const handleScan = async () => {
    setIsScanning(true);
    setMessage(null);
    try {
      const res = await scanWifi();
      setWifiNetworks(res.networks || []);
      setMessage({ text: `Найдено сетей: ${res.networks?.length || 0}`, type: 'success' });
    } catch (err: any) {
      setMessage({ text: err.message || 'Ошибка сканирования Wi-Fi', type: 'error' });
    } finally {
      setIsScanning(false);
    }
  };

  const handleConnect = async (e: React.FormEvent) => {
    e.preventDefault();
    if (!selectedSsid) return;
    setIsConnecting(true);
    setMessage(null);
    try {
      await connectWifi({ ssid: selectedSsid, password: wifiPassword });
      setMessage({ text: `Подключено к сети ${selectedSsid}`, type: 'success' });
      setWifiPassword('');
      refreshAll();
    } catch (err: any) {
      setMessage({ text: err.message || 'Ошибка подключения к Wi-Fi', type: 'error' });
    } finally {
      setIsConnecting(false);
    }
  };

  const handleDisconnect = async () => {
    setIsConnecting(true);
    setMessage(null);
    try {
      await disconnectWifi();
      setMessage({ text: 'Отключено от Wi-Fi', type: 'success' });
      refreshAll();
    } catch (err: any) {
      setMessage({ text: err.message || 'Ошибка отключения', type: 'error' });
    } finally {
      setIsConnecting(false);
    }
  };

  const handleToggleAp = async () => {
    setIsApLoading(true);
    setMessage(null);
    try {
      if (networkStatus?.ap_mode?.active) {
        await stopApMode();
        setMessage({ text: 'Точка доступа остановлена', type: 'success' });
      } else {
        const res = await startApMode();
        setMessage({ text: `Точка доступа ${res.ssid} запущена на ${res.ip}`, type: 'success' });
      }
      refreshAll();
    } catch (err: any) {
      setMessage({ text: err.message || 'Ошибка управления точкой доступа', type: 'error' });
    } finally {
      setIsApLoading(false);
    }
  };

  const isVpnConnected = vpnStatus?.status === 'connected';

  const handleToggleVpn = async () => {
    setIsVpnLoading(true);
    setMessage(null);
    try {
      if (isVpnConnected) {
        await disconnectVpn();
        setMessage({ text: 'VLESS туннель отключен', type: 'success' });
      } else {
        await connectVpn();
        setMessage({ text: 'VLESS туннель подключен', type: 'success' });
      }
      refreshAll();
    } catch (err: any) {
      setMessage({ text: err.message || 'Ошибка управления VPN', type: 'error' });
    } finally {
      setIsVpnLoading(false);
    }
  };

  const handleSaveVpnConfig = async (e: React.FormEvent) => {
    e.preventDefault();
    if (!vlessConfig.trim()) return;
    setIsVpnLoading(true);
    setMessage(null);
    try {
      await configureVpn({
        vless_uri: vlessConfig.trim(),
      });
      setMessage({ text: 'Конфигурация VLESS Reality сохранена', type: 'success' });
      setVlessConfig('');
      refreshAll();
    } catch (err: any) {
      setMessage({ text: err.message || 'Ошибка сохранения конфигурации VPN', type: 'error' });
    } finally {
      setIsVpnLoading(false);
    }
  };

  return (
    <div className="space-y-6">
      {/* Alert Message */}
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

      {/* Network Overview Card */}
      <Card
        title="Сетевые интерфейсы"
        subtitle="Параметры сетевых подключений устройства"
        action={
          <Button
            variant="outline"
            size="sm"
            onClick={refreshAll}
            leftIcon={<RotateCcw className="w-3.5 h-3.5" />}
          >
            Обновить
          </Button>
        }
      >
        <div className="divide-y divide-slate-100 text-xs sm:text-sm">
          <div className="py-2.5 flex items-center justify-between">
            <span className="text-slate-500">IP-адрес:</span>
            <span className="font-mono text-slate-900 font-medium">
              {networkStatus?.ip_address || '—'}
            </span>
          </div>
          <div className="py-2.5 flex items-center justify-between">
            <span className="text-slate-500">Шлюз по умолчанию:</span>
            <span className="font-mono text-slate-900">
              {networkStatus?.gateway || '—'}
            </span>
          </div>
          <div className="py-2.5 flex items-center justify-between">
            <span className="text-slate-500">Сетевое подключение:</span>
            <span className="text-slate-900 font-medium">
              {networkStatus?.wifi?.connected
                ? `Wi-Fi: ${networkStatus.wifi.ssid}${networkStatus.wifi.signal_level_percent ? ` (${networkStatus.wifi.signal_level_percent}%)` : ''}`
                : networkStatus?.ethernet?.connected
                ? `Ethernet${networkStatus.ethernet.speed_mbps ? ` (${networkStatus.ethernet.speed_mbps} Мбит/с)` : ''}`
                : 'Не подключено'}
            </span>
          </div>
          <div className="py-2.5 flex items-center justify-between">
            <span className="text-slate-500">Точка доступа (Setup AP):</span>
            <span className="text-slate-900 font-medium">
              {networkStatus?.ap_mode?.active ? `Активна (${networkStatus.ap_mode.ssid || '192.168.4.1'})` : 'Выключена'}
            </span>
          </div>
        </div>
      </Card>

      {/* Wi-Fi Manager Card */}
      <Card
        title="Wi-Fi подключение"
        subtitle="Поиск сетей и подключение к беспроводной сети"
        action={
          <Button
            variant="outline"
            size="sm"
            onClick={handleScan}
            isLoading={isScanning}
            leftIcon={<RotateCcw className={`w-3.5 h-3.5 ${isScanning ? 'animate-spin' : ''}`} />}
          >
            Сканировать сети
          </Button>
        }
      >
        <div className="space-y-4">
          {networkStatus?.wifi?.connected && (
            <div className="p-3.5 bg-slate-50 border border-slate-200/80 rounded-lg flex items-center justify-between">
              <div className="text-xs font-medium text-slate-800">
                Подключено к: <span className="font-semibold">{networkStatus.wifi.ssid}</span>
              </div>
              <Button
                variant="outline"
                size="sm"
                onClick={handleDisconnect}
                isLoading={isConnecting}
              >
                Отключиться
              </Button>
            </div>
          )}

          <form onSubmit={handleConnect} className="space-y-3">
            <div className="grid grid-cols-1 sm:grid-cols-2 gap-3">
              <div>
                <label className="block text-xs font-medium text-slate-700 mb-1">
                  Доступные сети
                </label>
                <select
                  value={selectedSsid}
                  onChange={(e) => setSelectedSsid(e.target.value)}
                  className="w-full text-xs bg-white border border-slate-200 rounded-lg px-3 py-2 text-slate-900"
                >
                  <option value="">
                    {wifiNetworks.length === 0
                      ? isScanning
                        ? 'Поиск сетей...'
                        : 'Сети не найдены (нажмите «Сканировать сети»)'
                      : '-- Выберите сеть из списка --'}
                  </option>
                  {wifiNetworks.map((n, i) => (
                    <option key={i} value={n.ssid}>
                      {n.ssid} (Сигнал: {n.signal_percent}%, {n.security})
                    </option>
                  ))}
                </select>
              </div>

              <div>
                <label className="block text-xs font-medium text-slate-700 mb-1">
                  Пароль сети
                </label>
                <input
                  type="password"
                  value={wifiPassword}
                  onChange={(e) => setWifiPassword(e.target.value)}
                  placeholder="Введите пароль"
                  className="w-full text-xs bg-white border border-slate-200 rounded-lg px-3 py-2 text-slate-900"
                />
              </div>
            </div>

            <Button
              type="submit"
              variant="primary"
              size="sm"
              disabled={!selectedSsid || isConnecting}
              isLoading={isConnecting}
            >
              Подключиться
            </Button>
          </form>
        </div>
      </Card>

      {/* Access Point (Setup AP) Card */}
      <Card
        title="Точка доступа"
        subtitle="Локальная сеть для настройки устройства без интернета"
        badge={
          <span className="inline-flex items-center gap-1.5 text-xs font-medium text-slate-700">
            <span
              className={`w-1.5 h-1.5 rounded-full ${
                networkStatus?.ap_mode?.active ? 'bg-emerald-500' : 'bg-slate-400'
              }`}
            />
            <span>{networkStatus?.ap_mode?.active ? 'Включена' : 'Выключена'}</span>
          </span>
        }
        action={
          <Button
            variant={networkStatus?.ap_mode?.active ? 'danger' : 'secondary'}
            size="sm"
            onClick={handleToggleAp}
            isLoading={isApLoading}
          >
            {networkStatus?.ap_mode?.active ? 'Остановить' : 'Запустить'}
          </Button>
        }
      >
        <div className="text-xs text-slate-500 leading-relaxed">
          В режиме точки доступа умная колонка раздает сеть{' '}
          <span className="font-mono text-slate-900 font-medium">
            {networkStatus?.ap_mode?.ssid || 'KAMISH-Setup'}
          </span>{' '}
          с IP-адресом <span className="font-mono text-slate-900 font-medium">192.168.4.1</span>.
        </div>
      </Card>

      {/* VPN VLESS Reality Tunnel Card */}
      <Card
        title="Туннель VLESS Reality"
        subtitle="Шифрованный туннель для сетевых сервисов"
        badge={
          <span className="inline-flex items-center gap-1.5 text-xs font-medium text-slate-700">
            <span
              className={`w-1.5 h-1.5 rounded-full ${
                isVpnConnected ? 'bg-emerald-500' : 'bg-slate-400'
              }`}
            />
            <span>{isVpnConnected ? 'Подключен' : 'Отключен'}</span>
          </span>
        }
        action={
          <Button
            variant={isVpnConnected ? 'danger' : 'primary'}
            size="sm"
            onClick={handleToggleVpn}
            isLoading={isVpnLoading}
            leftIcon={<Power className="w-3.5 h-3.5" />}
          >
            {isVpnConnected ? 'Отключить' : 'Подключить'}
          </Button>
        }
      >
        <form onSubmit={handleSaveVpnConfig} className="space-y-4">
          <div>
            <label className="block text-xs font-medium text-slate-700 mb-1">
              Строка подключения (URI)
            </label>
            <input
              type="text"
              value={vlessConfig}
              onChange={(e) => setVlessConfig(e.target.value)}
              placeholder="vless://... (секреты маскируются)"
              className="w-full text-xs font-mono bg-white border border-slate-200 rounded-lg px-3 py-2 text-slate-900"
            />
            <p className="text-xs text-slate-400 mt-1">
              Учетные данные сохраняются в локальной базе данных устройства.
            </p>
          </div>

          <Button
            type="submit"
            variant="secondary"
            size="sm"
            disabled={!vlessConfig.trim() || isVpnLoading}
            isLoading={isVpnLoading}
          >
            Сохранить конфигурацию
          </Button>
        </form>
      </Card>
    </div>
  );
};
