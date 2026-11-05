import React, { useState, useEffect, useCallback } from 'react';
import {
  Mic,
  Volume2,
  Play,
  CheckCircle2,
  AlertCircle,
  Wand2,
} from 'lucide-react';
import { Card } from './Card';
import { Badge } from './Badge';
import { Button } from './Button';
import { Toggle } from './Toggle';
import {
  getSpeechStatus,
  getSpeechVoices,
  updateSpeechConfig,
  synthesizeSpeech,
  synthesizeSpeechBlob,
  recognizeSpeech,
  simulateWakeWord,
} from '../api';
import type {
  SpeechStatus,
  TtsVoice,
  SimulateWakeWordResponse,
  RecognizeResponse,
} from '../types';

export const SpeechManagerPanel: React.FC = () => {
  const [status, setStatus] = useState<SpeechStatus | null>(null);
  const [voices, setVoices] = useState<TtsVoice[]>([]);
  const [errorMsg, setErrorMsg] = useState<string | null>(null);
  const [successMsg, setSuccessMsg] = useState<string | null>(null);

  // Synthesizer tester state
  const [synthText, setSynthText] = useState('Привет! Я голосовой ассистент умной колонки.');
  const [synthVoice, setSynthVoice] = useState<string>('offline_assistant');
  const [synthSpeed, setSynthSpeed] = useState<number>(1.0);
  const [isPlayingBrowser, setIsPlayingBrowser] = useState(false);
  const [isSpeakingSpeaker, setIsSpeakingSpeaker] = useState(false);

  // Wake Word & Recognition tester state
  const [simKeyword, setSimKeyword] = useState<'колонка' | 'ассистент' | 'слушай'>('колонка');
  const [simCommand, setSimCommand] = useState('поставь будильник на 07:30');
  const [simDispatchAi, setSimDispatchAi] = useState(true);
  const [simResult, setSimResult] = useState<SimulateWakeWordResponse | null>(null);
  const [isSimulating, setIsSimulating] = useState(false);

  // Manual STT tester state
  const [sttPrompt, setSttPrompt] = useState('какой статус системы?');
  const [sttResult, setSttResult] = useState<RecognizeResponse | null>(null);
  const [isRecognizing, setIsRecognizing] = useState(false);

  const fetchSpeechData = useCallback(async () => {
    try {
      const [statusRes, voicesRes] = await Promise.all([
        getSpeechStatus().catch(() => null),
        getSpeechVoices().catch(() => ({ voices: [] })),
      ]);

      if (statusRes) {
        setStatus(statusRes);
        setSynthVoice(statusRes.active_voice || (voicesRes?.voices?.[0]?.id) || 'offline_assistant');
      }
      if (voicesRes) {
        setVoices(voicesRes.voices);
      }
    } catch (err: any) {
      console.warn('Speech data load warning:', err);
    }
  }, []);

  useEffect(() => {
    fetchSpeechData();
  }, [fetchSpeechData]);

  // Handle configuration changes
  const handleConfigUpdate = async (field: string, value: any) => {
    setErrorMsg(null);
    try {
      const updated = await updateSpeechConfig({ [field]: value });
      setStatus(updated);
      setSuccessMsg(`Конфигурация речи обновлена: ${field}`);
      setTimeout(() => setSuccessMsg(null), 3000);
    } catch (err: any) {
      setErrorMsg(err.message || 'Ошибка обновления конфигурации речи');
    }
  };

  // Preview synthesis in browser
  const handlePreviewInBrowser = async () => {
    if (!synthText.trim()) return;
    setIsPlayingBrowser(true);
    setErrorMsg(null);
    try {
      const blob = await synthesizeSpeechBlob({
        text: synthText,
        voice: synthVoice,
        speed: synthSpeed,
      });

      const audioUrl = URL.createObjectURL(blob);
      const audio = new Audio(audioUrl);
      audio.onended = () => {
        setIsPlayingBrowser(false);
        URL.revokeObjectURL(audioUrl);
      };
      audio.onerror = () => {
        setIsPlayingBrowser(false);
        setErrorMsg('Ошибка воспроизведения аудио в браузере');
      };
      await audio.play();
    } catch (err: any) {
      setErrorMsg(err.message || 'Ошибка синтеза речи');
      setIsPlayingBrowser(false);
    }
  };

  // Play on device speaker
  const handlePlayOnSpeaker = async () => {
    if (!synthText.trim()) return;
    setIsSpeakingSpeaker(true);
    setErrorMsg(null);
    try {
      await synthesizeSpeech({
        text: synthText,
        voice: synthVoice,
        speed: synthSpeed,
        play_on_speaker: true,
      });
      setSuccessMsg('Аудио отправлено на динамик колонки');
      setTimeout(() => setSuccessMsg(null), 3000);
    } catch (err: any) {
      setErrorMsg(err.message || 'Ошибка вывода на динамик');
    } finally {
      setIsSpeakingSpeaker(false);
    }
  };

  // Trigger wake word simulation
  const handleSimulateWakeWord = async () => {
    setIsSimulating(true);
    setErrorMsg(null);
    try {
      const res = await simulateWakeWord({
        keyword: simKeyword,
        command: simCommand,
        dispatch_ai: simDispatchAi,
      });
      setSimResult(res);
      fetchSpeechData();
    } catch (err: any) {
      setErrorMsg(err.message || 'Сбой симуляции ключевого слова');
    } finally {
      setIsSimulating(false);
    }
  };

  // Trigger STT recognition
  const handleTestStt = async () => {
    if (!sttPrompt.trim()) return;
    setIsRecognizing(true);
    setErrorMsg(null);
    try {
      // Simulate base64 audio payload with prompt
      const payload = btoa(`CMD:${sttPrompt}`);
      const res = await recognizeSpeech({
        audio_base64: payload,
        language: 'ru',
        dispatch_ai: true,
      });
      setSttResult(res);
    } catch (err: any) {
      setErrorMsg(err.message || 'Ошибка теста распознавания STT');
    } finally {
      setIsRecognizing(false);
    }
  };

  return (
    <div className="space-y-6">
      {/* Overview Banner */}
      <div className="p-4 rounded-xl bg-slate-900 border border-slate-800 flex flex-col md:flex-row items-start md:items-center justify-between gap-4">
        <div className="flex items-center gap-3">
          <div className="p-2.5 rounded-lg bg-slate-800 text-slate-300 border border-slate-700">
            <Mic className="w-5 h-5" />
          </div>
          <div>
            <h4 className="text-sm font-semibold text-white">Речевой модуль</h4>
            <p className="text-xs text-slate-400">
              STT (Vosk/Sherpa-onnx), TTS (Piper/RHVoice/Offline), детекция активационной фразы и Rust VAD.
            </p>
          </div>
        </div>
        <div className="flex items-center gap-2">
          <Badge variant={status?.microphone_available ? 'success' : 'danger'}>
            Микрофон: {status?.microphone_available ? 'Активен' : 'Отключён'}
          </Badge>
          <Badge variant="default">Модуль готов</Badge>
        </div>
      </div>

      {/* Status Messages */}
      {errorMsg && (
        <div className="p-3 rounded-xl bg-rose-500/10 border border-rose-500/30 text-xs text-rose-300 flex items-center gap-2">
          <AlertCircle className="w-4 h-4 shrink-0" />
          <span>{errorMsg}</span>
        </div>
      )}
      {successMsg && (
        <div className="p-3 rounded-xl bg-emerald-500/10 border border-emerald-500/30 text-xs text-emerald-300 flex items-center gap-2">
          <CheckCircle2 className="w-4 h-4 shrink-0" />
          <span>{successMsg}</span>
        </div>
      )}

      {/* Subsystem Configuration & Engine Controls */}
      <div className="grid grid-cols-1 md:grid-cols-2 gap-6">
        {/* Engine Settings */}
        <Card title="Настройки речевого движка и маршрутизация">
          <div className="space-y-4 text-xs">
            {/* Active STT Provider */}
            <div>
              <label className="block text-slate-400 mb-1 font-medium">
                Провайдер распознавания речи (STT)
              </label>
              <select
                value={status?.active_stt_provider || 'mock'}
                onChange={(e) => handleConfigUpdate('active_stt_provider', e.target.value)}
                className="w-full bg-slate-900 border border-slate-700 rounded-lg px-3 py-2 text-white focus:outline-none focus:border-slate-500"
              >
                <option value="mock">Эмуляция STT (тесты / автономно)</option>
                <option value="vosk">Vosk STT (локальный процесс)</option>
                <option value="sherpa_onnx">Sherpa-onnx (нейросетевой ONNX)</option>
              </select>
            </div>

            {/* Active TTS Provider */}
            <div>
              <label className="block text-slate-400 mb-1 font-medium">
                Провайдер синтеза речи (TTS)
              </label>
              <select
                value={status?.active_tts_provider || 'offline_fallback'}
                onChange={(e) => handleConfigUpdate('active_tts_provider', e.target.value)}
                className="w-full bg-slate-900 border border-slate-700 rounded-lg px-3 py-2 text-white focus:outline-none focus:border-slate-500"
              >
                <option value="offline_fallback">Автономный синтезатор (чистый Rust форманты)</option>
                <option value="piper">Piper (нейросетевой локальный голос)</option>
                <option value="rhvoice">RHVoice (лёгкий движок ARM)</option>
              </select>
            </div>

            {/* Active Voice */}
            <div>
              <label className="block text-slate-400 mb-1 font-medium">
                Голос по умолчанию
              </label>
              <select
                value={status?.active_voice || synthVoice || 'offline_assistant'}
                onChange={(e) => {
                  setSynthVoice(e.target.value);
                  handleConfigUpdate('active_voice', e.target.value);
                }}
                className="w-full bg-slate-900 border border-slate-700 rounded-lg px-3 py-2 text-white focus:outline-none focus:border-slate-500"
              >
                {voices.length > 0 ? (
                  voices.map((v) => (
                    <option key={v.id} value={v.id}>
                      {v.name} ({v.language.toUpperCase()}) — [{v.provider}]
                    </option>
                  ))
                ) : (
                  <option value="offline_assistant">Автономный ассистент (RU)</option>
                )}
              </select>
            </div>

            {/* Toggles */}
            <div className="pt-2 border-t border-slate-800 space-y-3">
              <div className="flex justify-between items-center">
                <div>
                  <span className="text-white block font-medium">Детектор речевой активности (VAD)</span>
                  <span className="text-slate-400 text-[11px]">Фильтрация тишины по энергии и Zero-Crossing</span>
                </div>
                <Toggle
                  checked={status?.vad_enabled ?? true}
                  onChange={(checked) => handleConfigUpdate('vad_enabled', checked)}
                />
              </div>

              <div className="flex justify-between items-center">
                <div>
                  <span className="text-white block font-medium">Активационная фраза (Wake Word)</span>
                  <span className="text-slate-400 text-[11px]">Ключевые слова: «колонка», «камыш», «слушай»</span>
                </div>
                <Toggle
                  checked={status?.wakeword_enabled ?? true}
                  onChange={(checked) => handleConfigUpdate('wakeword_enabled', checked)}
                />
              </div>
            </div>
          </div>
        </Card>

        {/* Hardware & Diagnostic Info */}
        <Card title="Акустический тракт и оборудование">
          <div className="space-y-3 text-xs">
            <div className="flex justify-between py-2 border-b border-slate-800">
              <span className="text-slate-400">Микрофонный массив</span>
              <span className="font-mono text-slate-200">Seeed ReSpeaker XVF3800</span>
            </div>
            <div className="flex justify-between py-2 border-b border-slate-800">
              <span className="text-slate-400">Аппаратная обработка</span>
              <span className="font-mono text-emerald-400">AEC, Beamforming, подавление шума</span>
            </div>
            <div className="flex justify-between py-2 border-b border-slate-800">
              <span className="text-slate-400">Частота / Формат</span>
              <span className="font-mono text-slate-200">16,000 Гц, 16-бит PCM Mono</span>
            </div>
            <div className="flex justify-between py-2 border-b border-slate-800">
              <span className="text-slate-400">Устройство вывода звука</span>
              <span className="font-mono text-slate-200">Усилитель GT-112 / ALSA</span>
            </div>
            <div className="flex justify-between py-2">
              <span className="text-slate-400">Последнее распознавание фразы</span>
              <span className="font-mono text-slate-300">
                {status?.last_wake_word ? `«${status.last_wake_word.keyword}» (${(status.last_wake_word.score * 100).toFixed(0)}%)` : 'Ещё не зафиксировано'}
              </span>
            </div>
          </div>
        </Card>
      </div>

      {/* Voice Synthesizer (TTS) Tester */}
      <Card title="Тестирование синтезатора речи (TTS)">
        <div className="space-y-4 text-xs">
          <div>
            <label className="block text-slate-400 mb-1 font-medium">Текст для озвучивания</label>
            <textarea
              rows={2}
              value={synthText}
              onChange={(e) => setSynthText(e.target.value)}
              placeholder="Введите текст для озвучивания..."
              className="w-full bg-slate-900 border border-slate-700 rounded-lg p-3 text-white focus:outline-none focus:border-slate-500 font-sans"
            />
          </div>

          {/* Quick preset chips */}
          <div className="flex flex-wrap gap-2">
            {[
              'Привет! Я колонка Камыш.',
              'Поставь будильник на 07:30',
              'Напоминание: пора сделать перерыв.',
              'Все системы умной колонки работают штатно.',
            ].map((preset) => (
              <button
                key={preset}
                type="button"
                onClick={() => setSynthText(preset)}
                className="px-2.5 py-1 rounded-md bg-slate-800 hover:bg-slate-700 text-[11px] text-slate-300 border border-slate-700 transition-colors"
              >
                {preset}
              </button>
            ))}
          </div>

          <div className="grid grid-cols-1 sm:grid-cols-2 gap-4 pt-2">
            <div>
              <label className="block text-slate-400 mb-1">Голос синтезатора</label>
              <select
                value={synthVoice}
                onChange={(e) => setSynthVoice(e.target.value)}
                className="w-full bg-slate-900 border border-slate-700 rounded-lg px-3 py-2 text-white"
              >
                {voices.map((v) => (
                  <option key={v.id} value={v.id}>
                    {v.name}
                  </option>
                ))}
              </select>
            </div>

            <div>
              <div className="flex justify-between mb-1">
                <span className="text-slate-400">Скорость речи: {synthSpeed}x</span>
              </div>
              <input
                type="range"
                min="0.5"
                max="1.75"
                step="0.05"
                value={synthSpeed}
                onChange={(e) => setSynthSpeed(parseFloat(e.target.value))}
                className="w-full h-1.5 bg-slate-800 rounded-lg appearance-none cursor-pointer accent-slate-400 mt-2"
              />
            </div>
          </div>

          <div className="flex flex-wrap items-center gap-3 pt-3 border-t border-slate-800">
            <Button
              variant="primary"
              size="sm"
              onClick={handlePreviewInBrowser}
              disabled={isPlayingBrowser}
            >
              <Play className="w-4 h-4 mr-1.5" />
              {isPlayingBrowser ? 'Воспроизведение в браузере...' : 'Прослушать в браузере'}
            </Button>

            <Button
              variant="outline"
              size="sm"
              onClick={handlePlayOnSpeaker}
              disabled={isSpeakingSpeaker}
            >
              <Volume2 className="w-4 h-4 mr-1.5" />
              {isSpeakingSpeaker ? 'Отправка на динамик...' : 'Озвучить на динамике'}
            </Button>
          </div>
        </div>
      </Card>

      {/* Wake Word & Voice Recognition Simulator */}
      <div className="grid grid-cols-1 md:grid-cols-2 gap-6">
        {/* Wake Word Spotter Simulator */}
        <Card title="Симуляция ключевого слова и ИИ-ассистента">
          <div className="space-y-4 text-xs">
            <div>
              <label className="block text-slate-400 mb-1 font-medium">Активационное слово</label>
              <div className="grid grid-cols-3 gap-2">
                {(['колонка', 'ассистент', 'слушай'] as const).map((kw) => (
                  <button
                    key={kw}
                    type="button"
                    onClick={() => setSimKeyword(kw)}
                    className={`py-2 rounded-lg text-center font-mono border transition-all ${
                      simKeyword === kw
                        ? 'bg-slate-800 border-slate-600 text-white'
                        : 'bg-slate-900 border-slate-700 text-slate-400 hover:text-white'
                    }`}
                  >
                    «{kw}»
                  </button>
                ))}
              </div>
            </div>

            <div>
              <label className="block text-slate-400 mb-1 font-medium">Голосовая команда</label>
              <input
                type="text"
                value={simCommand}
                onChange={(e) => setSimCommand(e.target.value)}
                placeholder="Например: поставь будильник на 08:00"
                className="w-full bg-slate-900 border border-slate-700 rounded-lg px-3 py-2 text-white"
              />
            </div>

            <div className="flex items-center gap-2">
              <input
                type="checkbox"
                id="dispatch-ai-check"
                checked={simDispatchAi}
                onChange={(e) => setSimDispatchAi(e.target.checked)}
                className="rounded bg-slate-800 border-slate-700 text-slate-900 focus:ring-slate-700"
              />
              <label htmlFor="dispatch-ai-check" className="text-slate-300 select-none cursor-pointer">
                Автоматически передать команду ассистенту и озвучить ответ через TTS
              </label>
            </div>

            <Button
              variant="primary"
              size="sm"
              onClick={handleSimulateWakeWord}
              disabled={isSimulating}
              className="w-full"
            >
              <Wand2 className="w-4 h-4 mr-1.5" />
              {isSimulating ? 'Обработка фразы...' : `Симулировать слово «${simKeyword}»`}
            </Button>

            {simResult && (
              <div className="p-3 rounded-xl bg-slate-900 border border-slate-800 space-y-2 mt-2">
                <div className="flex justify-between items-center text-[11px]">
                  <span className="font-semibold text-slate-300">Событие детекции</span>
                  <span className="font-mono text-slate-400">Уверенность: {(simResult.event.score * 100).toFixed(0)}%</span>
                </div>
                {simResult.ai_response && (
                  <div className="pt-2 border-t border-slate-800">
                    <span className="text-slate-400 block text-[10px] uppercase font-mono">Ответ ИИ</span>
                    <p className="text-white font-medium text-xs mt-0.5">{simResult.ai_response.response}</p>
                    {simResult.spoken_response && (
                      <span className="inline-flex items-center gap-1 text-[11px] text-emerald-400 mt-1">
                        <Volume2 className="w-3.5 h-3.5" /> Синтезировано и озвучено
                      </span>
                    )}
                  </div>
                )}
              </div>
            )}
          </div>
        </Card>

        {/* Speech Recognition (STT) Direct Tester */}
        <Card title="Тестирование распознавания речи (STT)">
          <div className="space-y-4 text-xs">
            <div>
              <label className="block text-slate-400 mb-1 font-medium">Текст или фраза для распознавания</label>
              <input
                type="text"
                value={sttPrompt}
                onChange={(e) => setSttPrompt(e.target.value)}
                placeholder="Что сказать умной колонке..."
                className="w-full bg-slate-900 border border-slate-700 rounded-lg px-3 py-2 text-white"
              />
            </div>

            <div className="flex flex-wrap gap-2">
              {['статус устройства', 'включи музыку', 'какая громкость', 'поставь будильник на 08:30'].map((phrase) => (
                <button
                  key={phrase}
                  type="button"
                  onClick={() => setSttPrompt(phrase)}
                  className="px-2.5 py-1 rounded-md bg-slate-800 text-[11px] text-slate-300 hover:text-white border border-slate-700"
                >
                  {phrase}
                </button>
              ))}
            </div>

            <Button
              variant="primary"
              size="sm"
              onClick={handleTestStt}
              disabled={isRecognizing}
              className="w-full"
            >
              <Mic className="w-4 h-4 mr-1.5" />
              {isRecognizing ? 'Распознавание речи...' : 'Распознать и передать ИИ'}
            </Button>

            {sttResult && (
              <div className="p-3 rounded-xl bg-slate-900 border border-slate-800 space-y-2 mt-2">
                <div className="flex justify-between items-center text-[11px]">
                  <span className="text-slate-400 font-mono">Распознано ({sttResult.stt.provider}):</span>
                  <span className="text-emerald-400 font-mono">{(sttResult.stt.confidence * 100).toFixed(0)}% точность</span>
                </div>
                <p className="text-white font-semibold">«{sttResult.stt.text}»</p>
                {sttResult.ai_response && (
                  <div className="pt-2 border-t border-slate-800">
                    <span className="text-slate-400 block text-[10px] uppercase font-mono">Результат выполнения ИИ</span>
                    <p className="text-slate-200 text-xs mt-0.5">{sttResult.ai_response.response}</p>
                  </div>
                )}
              </div>
            )}
          </div>
        </Card>
      </div>
    </div>
  );
};
