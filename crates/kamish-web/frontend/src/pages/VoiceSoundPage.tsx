import React, { useState, useEffect } from 'react';
import { Play } from 'lucide-react';
import { Card } from '../components/Card';
import { Button } from '../components/Button';
import { Toggle } from '../components/Toggle';
import {
  setAudioVolume,
  setVolumeCeiling,
  setAudioRoute,
  playAudioTest,
  getSpeechStatus,
  getSpeechVoices,
  updateSpeechConfig,
  synthesizeSpeech,
  simulateWakeWord,
} from '../api';
import type { AudioRoute, SpeechStatus } from '../types';

export const VoiceSoundPage: React.FC = () => {
  // Audio state
  const [volume, setVolumeState] = useState<number>(65);
  const [ceiling, setCeilingState] = useState<number>(85);
  const [route, setRouteState] = useState<AudioRoute>('gt112_default');
  const [isPlayingTestTone, setIsPlayingTestTone] = useState(false);

  // Speech state
  const [speechStatus, setSpeechStatus] = useState<SpeechStatus | null>(null);
  const [voices, setVoices] = useState<string[]>([]);
  const [activeVoice, setActiveVoice] = useState<string>('');
  const [vadEnabled, setVadEnabled] = useState<boolean>(true);
  const [wakewordEnabled, setWakewordEnabled] = useState<boolean>(true);

  // Test Synthesis & Simulated Wakeword
  const [testText, setTestText] = useState('Камыш готов к работе. Звуковая подсистема откалибрована.');
  const [isSynthesizing, setIsSynthesizing] = useState(false);
  const [isSimulatingWakeword, setIsSimulatingWakeword] = useState(false);
  const [wakewordResult, setWakewordResult] = useState<string | null>(null);

  const [message, setMessage] = useState<{ text: string; type: 'success' | 'error' } | null>(null);

  useEffect(() => {
    getSpeechStatus()
      .then((status) => {
        setSpeechStatus(status);
        if (status.active_voice) setActiveVoice(status.active_voice);
        setVadEnabled(status.vad_enabled);
        setWakewordEnabled(status.wakeword_enabled);
      })
      .catch(console.warn);

    getSpeechVoices()
      .then((res) => {
        const vList = res.voices.map((v) => v.id);
        setVoices(vList);
        if (vList.length > 0) {
          setActiveVoice((prev) => prev || vList[0]);
        }
      })
      .catch(console.warn);
  }, []);

  const handleApplyVolume = async (newVol: number, newCeil: number) => {
    const safeCeil = Math.min(newCeil, 85);
    const safeVol = Math.min(newVol, safeCeil);
    setCeilingState(safeCeil);
    setVolumeState(safeVol);
    try {
      await setVolumeCeiling(safeCeil);
      await setAudioVolume(safeVol);
    } catch (err: any) {
      console.warn('Volume update error:', err);
    }
  };

  const handleRouteChange = async (newRoute: AudioRoute) => {
    setRouteState(newRoute);
    try {
      await setAudioRoute(newRoute);
      setMessage({ text: `Аудиомаршрут переключен на ${newRoute}`, type: 'success' });
    } catch (err: any) {
      setMessage({ text: err.message || 'Ошибка переключения аудиомаршрута', type: 'error' });
    }
  };

  const handleAudioTest = async () => {
    setIsPlayingTestTone(true);
    setMessage(null);
    try {
      await playAudioTest();
      setMessage({ text: 'Тестовый звуковой сигнал воспроизведён', type: 'success' });
    } catch {
      try {
        await synthesizeSpeech({ text: 'Проверка динамиков', play_on_speaker: true });
        setMessage({ text: 'Тестовый звуковой сигнал воспроизведён', type: 'success' });
      } catch (err: any) {
        setMessage({ text: err.message || 'Ошибка воспроизведения теста', type: 'error' });
      }
    } finally {
      setIsPlayingTestTone(false);
    }
  };

  const handleSaveSpeechConfig = async (vad: boolean, ww: boolean, voice: string) => {
    try {
      const updated = await updateSpeechConfig({
        vad_enabled: vad,
        wakeword_enabled: ww,
        active_voice: voice,
      });
      setSpeechStatus(updated);
      setMessage({ text: 'Параметры речевого стека обновлены', type: 'success' });
    } catch (err: any) {
      setMessage({ text: err.message || 'Ошибка сохранения речевой конфигурации', type: 'error' });
    }
  };

  const handleSynthesizeTest = async () => {
    if (!testText.trim()) return;
    setIsSynthesizing(true);
    setMessage(null);
    try {
      await synthesizeSpeech({
        text: testText.trim(),
        voice: activeVoice,
        play_on_speaker: true,
      });
      setMessage({ text: 'Синтезированная речь воспроизведена на динамике', type: 'success' });
    } catch (err: any) {
      setMessage({ text: err.message || 'Ошибка синтеза речи', type: 'error' });
    } finally {
      setIsSynthesizing(false);
    }
  };

  const handleSimulateWakeword = async () => {
    setIsSimulatingWakeword(true);
    setWakewordResult(null);
    setMessage(null);
    try {
      const res = await simulateWakeWord({
        keyword: 'камыш',
        command: 'который час',
        dispatch_ai: true,
      });
      setWakewordResult(
        `Команда: "${res.command || 'который час'}". Ответ: "${res.ai_response?.response || 'OK'}"`
      );
      setMessage({ text: 'Команда успешно обработана ассистентом', type: 'success' });
    } catch (err: any) {
      setMessage({ text: err.message || 'Ошибка симуляции вейкворда', type: 'error' });
    } finally {
      setIsSimulatingWakeword(false);
    }
  };

  return (
    <div className="space-y-6">
      {/* Notifications */}
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

      {/* Audio Controls Card */}
      <Card
        title="Громкость динамика"
        subtitle="Регулировка уровня звука с ограничением (≤ 85%)"
        action={
          <Button
            variant="outline"
            size="sm"
            onClick={handleAudioTest}
            isLoading={isPlayingTestTone}
            leftIcon={<Play className="w-3.5 h-3.5" />}
          >
            Тест звука
          </Button>
        }
      >
        <div className="space-y-6">
          {/* Sliders */}
          <div className="grid grid-cols-1 md:grid-cols-2 gap-6">
            <div>
              <div className="flex justify-between text-xs font-medium text-slate-700 mb-2">
                <span>Рабочая громкость</span>
                <span className="font-mono text-slate-900">{volume}%</span>
              </div>
              <input
                type="range"
                min={0}
                max={ceiling}
                value={volume}
                onChange={(e) => handleApplyVolume(Number(e.target.value), ceiling)}
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
                onChange={(e) => handleApplyVolume(volume, Number(e.target.value))}
                className="w-full accent-slate-900 cursor-pointer"
              />
              <p className="text-xs text-slate-400 mt-1">
                Максимально допустимый уровень громкости для защиты динамика.
              </p>
            </div>
          </div>

          {/* Audio Route Selector */}
          <div>
            <label className="block text-xs font-medium text-slate-700 mb-2">
              Маршрут вывода звука
            </label>
            <div className="grid grid-cols-2 sm:grid-cols-4 gap-2">
              {[
                { id: 'gt112_default' as AudioRoute, label: 'GT-112 (Встроенный)' },
                { id: 'bluetooth_sink' as AudioRoute, label: 'Bluetooth A2DP' },
                { id: 'headphones' as AudioRoute, label: 'Наушники 3.5мм' },
                { id: 'system_default' as AudioRoute, label: 'Системный ALSA' },
              ].map((r) => (
                <button
                  key={r.id}
                  type="button"
                  onClick={() => handleRouteChange(r.id)}
                  className={`p-2.5 rounded-lg text-xs font-medium border text-left transition-colors cursor-pointer ${
                    route === r.id
                      ? 'bg-slate-900 text-white border-slate-900 shadow-xs'
                      : 'bg-white text-slate-700 border-slate-200 hover:bg-slate-50'
                  }`}
                >
                  {r.label}
                </button>
              ))}
            </div>
          </div>
        </div>
      </Card>

      {/* Speech Manager & Microphone Card */}
      <Card
        title="Параметры микрофона и речи"
        subtitle="Детекция голосовой активности и параметры синтезатора"
      >
        <div className="space-y-6">
          {/* Toggles */}
          <div className="grid grid-cols-1 sm:grid-cols-2 gap-4 py-2">
            <div>
              <Toggle
                checked={vadEnabled}
                onChange={(val) => {
                  setVadEnabled(val);
                  handleSaveSpeechConfig(val, wakewordEnabled, activeVoice);
                }}
                label="Активация по голосу (VAD)"
              />
              <p className="text-xs text-slate-500 mt-1 pl-12">
                Энергоэффективный детектор голосовой активности на Rust.
              </p>
            </div>

            <div>
              <Toggle
                checked={wakewordEnabled}
                onChange={(val) => {
                  setWakewordEnabled(val);
                  handleSaveSpeechConfig(vadEnabled, val, activeVoice);
                }}
                label="Распознавание фразы «Камыш»"
              />
              <p className="text-xs text-slate-500 mt-1 pl-12">
                Локальный поиск активационного слова без облачных запросов.
              </p>
            </div>
          </div>

          {/* Voice selection & TTS Test */}
          <div className="space-y-3 pt-2 border-t border-slate-100">
            <div className="flex flex-col sm:flex-row gap-3">
              <div className="w-full sm:w-1/3">
                <label className="block text-xs font-medium text-slate-700 mb-1">
                  Голос ассистента
                </label>
                <select
                  value={activeVoice}
                  onChange={(e) => {
                    setActiveVoice(e.target.value);
                    handleSaveSpeechConfig(vadEnabled, wakewordEnabled, e.target.value);
                  }}
                  className="w-full text-xs bg-white border border-slate-200 rounded-lg px-3 py-2 text-slate-900"
                >
                  {voices.length > 0 ? (
                    voices.map((v) => (
                      <option key={v} value={v}>
                        Голос: {v}
                      </option>
                    ))
                  ) : (
                    <option value="" disabled>
                      Локальные голоса не обнаружены
                    </option>
                  )}
                </select>
              </div>

              <div className="w-full sm:w-2/3">
                <label className="block text-xs font-medium text-slate-700 mb-1">
                  Тест синтеза речи
                </label>
                <div className="flex gap-2">
                  <input
                    type="text"
                    value={testText}
                    onChange={(e) => setTestText(e.target.value)}
                    className="flex-1 text-xs bg-white border border-slate-200 rounded-lg px-3 py-2 text-slate-900"
                  />
                  <Button
                    variant="primary"
                    size="sm"
                    onClick={handleSynthesizeTest}
                    isLoading={isSynthesizing}
                    leftIcon={<Play className="w-3 h-3" />}
                  >
                    Сказать
                  </Button>
                </div>
              </div>
            </div>
          </div>

          {/* Wake Word Simulator */}
          <div className="flex flex-col sm:flex-row items-start sm:items-center justify-between gap-3 pt-3 border-t border-slate-100">
            <div>
              <h4 className="text-xs font-semibold text-slate-900">
                Симуляция активационной фразы
              </h4>
              <p className="text-xs text-slate-500 mt-0.5">
                Программная отправка команды «Камыш, который час» в речевой стек ядра
              </p>
              {wakewordResult && (
                <p className="text-xs font-mono text-slate-800 mt-1 font-medium">{wakewordResult}</p>
              )}
            </div>
            <Button
              variant="outline"
              size="sm"
              onClick={handleSimulateWakeword}
              isLoading={isSimulatingWakeword}
            >
              Симулировать фразу
            </Button>
          </div>
        </div>
      </Card>
    </div>
  );
};
