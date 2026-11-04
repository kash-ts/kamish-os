import React, { useState, useEffect } from 'react';
import {
  RotateCcw,
  Send,
  Download,
  KeyRound,
  ExternalLink,
  CheckCircle2,
  AlertCircle,
} from 'lucide-react';
import { Card } from '../components/Card';
import { Button } from '../components/Button';
import {
  getAiStatus,
  getAgyStatus,
  getAiTools,
  probeAi,
  sendAiPrompt,
  installAgy,
  beginAgyLogin,
  submitAgyCode,
} from '../api';
import type {
  AgyLoginChallenge,
  AgyStatus,
  AiReadiness,
  ToolDeclaration,
  AiResponse,
} from '../types';

export const AiModelsPage: React.FC = () => {
  const [aiStatus, setAiStatus] = useState<AiReadiness | null>(null);
  const [tools, setTools] = useState<ToolDeclaration[]>([]);
  const [isProbing, setIsProbing] = useState(false);
  const [promptInput, setPromptInput] = useState('Какая сейчас температура и громкость?');
  const [isPromptLoading, setIsPromptLoading] = useState(false);
  const [aiResponse, setAiResponse] = useState<AiResponse | null>(null);
  const [message, setMessage] = useState<{ text: string; type: 'success' | 'error' } | null>(null);

  // Antigravity CLI install / authorization state
  const [agyStatus, setAgyStatus] = useState<AgyStatus | null>(null);
  const [challenge, setChallenge] = useState<AgyLoginChallenge | null>(null);
  const [authCodeInput, setAuthCodeInput] = useState('');
  const [isInstalling, setIsInstalling] = useState(false);
  const [isBeginningLogin, setIsBeginningLogin] = useState(false);
  const [isSubmittingCode, setIsSubmittingCode] = useState(false);

  const loadData = async () => {
    try {
      const [statusRes, toolsRes, agyRes] = await Promise.all([
        getAiStatus().catch(() => null),
        getAiTools().catch(() => ({ tools: [] })),
        getAgyStatus().catch(() => null),
      ]);
      if (statusRes) setAiStatus(statusRes);
      if (toolsRes) setTools(toolsRes.tools || []);
      if (agyRes) setAgyStatus(agyRes);
    } catch (err) {
      console.warn('Failed to load AI data:', err);
    }
  };

  useEffect(() => {
    loadData();
  }, []);

  const handleInstallAgy = async () => {
    setIsInstalling(true);
    setMessage(null);
    try {
      const res = await installAgy();
      setAgyStatus(res);
      setMessage({ text: 'Antigravity CLI установлен', type: 'success' });
      await loadData();
    } catch (err: any) {
      setMessage({
        text: err.message || 'Не удалось скачать Antigravity CLI',
        type: 'error',
      });
    } finally {
      setIsInstalling(false);
    }
  };

  const handleBeginLogin = async () => {
    setIsBeginningLogin(true);
    setMessage(null);
    setChallenge(null);
    try {
      const res = await beginAgyLogin();
      setChallenge(res);
      if (res.auth_code) setAuthCodeInput(res.auth_code);
      setMessage({
        text: 'Откройте ссылку в браузере и введите код ниже',
        type: 'success',
      });
    } catch (err: any) {
      setMessage({
        text: err.message || 'Не удалось начать авторизацию',
        type: 'error',
      });
    } finally {
      setIsBeginningLogin(false);
    }
  };

  const handleSubmitCode = async (e: React.FormEvent) => {
    e.preventDefault();
    if (!authCodeInput.trim() || isSubmittingCode) return;
    setIsSubmittingCode(true);
    setMessage(null);
    try {
      const res = await submitAgyCode(authCodeInput.trim());
      setMessage({ text: res.message, type: res.authorized ? 'success' : 'error' });
      if (res.authorized) {
        setChallenge(null);
        setAuthCodeInput('');
        await loadData();
      }
    } catch (err: any) {
      setMessage({ text: err.message || 'Код не принят', type: 'error' });
    } finally {
      setIsSubmittingCode(false);
    }
  };

  const handleProbe = async () => {
    setIsProbing(true);
    setMessage(null);
    try {
      const res = await probeAi();
      setAiStatus(res);
      setMessage({
        text: res.ready
          ? 'Проверка успешна: локальный процесс готов к работе'
          : `Ошибка проверки: ${res.error_reason || 'Процесс не отвечает'}`,
        type: res.ready ? 'success' : 'error',
      });
    } catch (err: any) {
      setMessage({ text: err.message || 'Ошибка выполнения проверки CLI', type: 'error' });
    } finally {
      setIsProbing(false);
    }
  };

  const handleSendPrompt = async (e: React.FormEvent) => {
    e.preventDefault();
    if (!promptInput.trim() || isPromptLoading) return;
    setIsPromptLoading(true);
    setAiResponse(null);
    setMessage(null);

    try {
      const res = await sendAiPrompt(promptInput.trim());
      setAiResponse(res);
      setMessage({ text: 'Запрос успешно обработан', type: 'success' });
    } catch (err: any) {
      setMessage({ text: err.message || 'Ошибка обработки запроса', type: 'error' });
    } finally {
      setIsPromptLoading(false);
    }
  };

  return (
    <div className="space-y-6">
      {/* Alert Notification */}
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

      {/* Antigravity CLI: install & authorization */}
      <Card
        title="Antigravity CLI (agy)"
        subtitle="Скачивание и авторизация без терминала"
        badge={
          agyStatus?.authorized ? (
            <span className="inline-flex items-center gap-1.5 text-xs font-medium text-emerald-700">
              <CheckCircle2 className="w-3.5 h-3.5" />
              <span>Авторизован</span>
            </span>
          ) : agyStatus?.installed ? (
            <span className="inline-flex items-center gap-1.5 text-xs font-medium text-amber-700">
              <AlertCircle className="w-3.5 h-3.5" />
              <span>Требуется вход</span>
            </span>
          ) : (
            <span className="inline-flex items-center gap-1.5 text-xs font-medium text-slate-600">
              <AlertCircle className="w-3.5 h-3.5" />
              <span>Не установлен</span>
            </span>
          )
        }
      >
        <div className="space-y-4">
          <div className="divide-y divide-slate-100 text-xs sm:text-sm">
            <div className="py-2.5 flex items-center justify-between">
              <span className="text-slate-500">Статус:</span>
              <span className="font-medium text-slate-900">
                {agyStatus?.installed
                  ? agyStatus.authorized
                    ? 'Установлен, авторизован'
                    : 'Установлен, не авторизован'
                  : 'Не установлен'}
              </span>
            </div>
            <div className="py-2.5 flex items-center justify-between">
              <span className="text-slate-500">Путь:</span>
              <span className="font-mono text-slate-900">{agyStatus?.path || '—'}</span>
            </div>
            <div className="py-2.5 flex items-center justify-between">
              <span className="text-slate-500">Версия:</span>
              <span className="font-mono text-slate-900">{agyStatus?.version ?? '—'}</span>
            </div>
            <div className="py-2.5 flex items-center justify-between">
              <span className="text-slate-500">Архитектура:</span>
              <span className="font-mono text-slate-900">{agyStatus?.architecture || '—'}</span>
            </div>
          </div>

          {agyStatus?.problem && (
            <p className="text-xs text-amber-700 bg-amber-50 p-3 rounded-lg">{agyStatus.problem}</p>
          )}

          <div className="flex flex-wrap gap-2">
            <Button
              variant="outline"
              size="sm"
              onClick={handleInstallAgy}
              isLoading={isInstalling}
              leftIcon={<Download className="w-3.5 h-3.5" />}
            >
              {agyStatus?.installed ? 'Переустановить' : 'Скачать и установить'}
            </Button>

            {agyStatus?.installed && (
              <Button
                variant="primary"
                size="sm"
                onClick={handleBeginLogin}
                isLoading={isBeginningLogin}
                leftIcon={<KeyRound className="w-3.5 h-3.5" />}
              >
                Войти в аккаунт
              </Button>
            )}
          </div>

          {challenge && (
            <div className="p-4 bg-slate-50 border border-slate-200/80 rounded-lg space-y-3">
              {challenge.auth_url && (
                <div>
                  <span className="text-xs font-medium text-slate-500 block mb-1">
                    Ссылка для авторизации:
                  </span>
                  <a
                    href={challenge.auth_url}
                    target="_blank"
                    rel="noopener noreferrer"
                    className="inline-flex items-center gap-1.5 text-xs sm:text-sm text-slate-900 underline break-all"
                  >
                    {challenge.auth_url}
                    <ExternalLink className="w-3.5 h-3.5 shrink-0" />
                  </a>
                </div>
              )}

              {challenge.auth_code && (
                <div>
                  <span className="text-xs font-medium text-slate-500 block mb-1">
                    Код (введите на странице):
                  </span>
                  <code className="text-sm font-mono bg-white border border-slate-200 px-3 py-1.5 rounded-md inline-block">
                    {challenge.auth_code}
                  </code>
                </div>
              )}

              <form onSubmit={handleSubmitCode} className="pt-1">
                <label className="block text-xs font-medium text-slate-700 mb-1.5">
                  Код из браузера
                </label>
                <div className="flex gap-2">
                  <input
                    type="text"
                    value={authCodeInput}
                    onChange={(e) => setAuthCodeInput(e.target.value)}
                    placeholder="XXXX-XXXX"
                    disabled={isSubmittingCode}
                    className="flex-1 text-xs sm:text-sm font-mono bg-white border border-slate-200 rounded-lg px-3 py-2 text-slate-900 focus:outline-none focus:ring-2 focus:ring-slate-900/10 focus:border-slate-800"
                  />
                  <Button
                    type="submit"
                    variant="primary"
                    size="sm"
                    disabled={!authCodeInput.trim() || isSubmittingCode}
                    isLoading={isSubmittingCode}
                  >
                    Подтвердить
                  </Button>
                </div>
              </form>

              {challenge.instructions && (
                <details className="text-xs text-slate-500">
                  <summary className="cursor-pointer">Вывод CLI</summary>
                  <pre className="mt-2 p-2 bg-white border border-slate-200 rounded text-[11px] whitespace-pre-wrap break-all">
                    {challenge.instructions}
                  </pre>
                </details>
              )}
            </div>
          )}
        </div>
      </Card>

      {/* AI Orchestrator Status Card */}
      <Card
        title="Голосовой ассистент (Локально)"
        subtitle="Локальный процесс без внешних сетевых зависимостей"
        badge={
          <span className="inline-flex items-center gap-1.5 text-xs font-medium text-slate-700">
            <span
              className={`w-1.5 h-1.5 rounded-full ${
                aiStatus?.ready ? 'bg-emerald-500' : 'bg-amber-500'
              }`}
            />
            <span>{aiStatus?.ready ? 'Готов к работе' : 'Ожидание проверки'}</span>
          </span>
        }
        action={
          <Button
            variant="outline"
            size="sm"
            onClick={handleProbe}
            isLoading={isProbing}
            leftIcon={<RotateCcw className={`w-3.5 h-3.5 ${isProbing ? 'animate-spin' : ''}`} />}
          >
            Тест процесса
          </Button>
        }
      >
        <div className="space-y-4">
          <div className="divide-y divide-slate-100 text-xs sm:text-sm">
            <div className="py-2.5 flex items-center justify-between">
              <span className="text-slate-500">Режим работы:</span>
              <span className="font-medium text-slate-900">
                {aiStatus?.mode === 'cli' ? 'Antigravity CLI (agy)' : 'Автономный Mock'}
              </span>
            </div>
            <div className="py-2.5 flex items-center justify-between">
              <span className="text-slate-500">Путь к бинарнику:</span>
              <span className="font-mono text-slate-900">
                {aiStatus?.cli_path || '/usr/local/bin/agy'}
              </span>
            </div>
            <div className="py-2.5 flex items-center justify-between">
              <span className="text-slate-500">Версия CLI:</span>
              <span className="font-mono text-slate-900">
                {aiStatus?.cli_version ?? 'Не известно'}
              </span>
            </div>
          </div>
        </div>
      </Card>

      {/* Interactive Prompt Console */}
      <Card
        title="Диагностическая консоль команд"
        subtitle="Проверка распознавания намерений и вызова системных инструментов"
      >
        <form onSubmit={handleSendPrompt} className="space-y-4">
          <div>
            <label className="block text-xs font-medium text-slate-700 mb-1.5">
              Тестовый запрос к ассистенту
            </label>
            <div className="flex gap-2">
              <input
                type="text"
                value={promptInput}
                onChange={(e) => setPromptInput(e.target.value)}
                placeholder="Введите команду (например: 'поставь будильник на 08:00')..."
                disabled={isPromptLoading}
                className="flex-1 text-xs sm:text-sm bg-white border border-slate-200 rounded-lg px-3 py-2 text-slate-900 focus:outline-none focus:ring-2 focus:ring-slate-900/10 focus:border-slate-800"
              />
              <Button
                type="submit"
                variant="primary"
                size="sm"
                disabled={!promptInput.trim() || isPromptLoading}
                isLoading={isPromptLoading}
                leftIcon={<Send className="w-3.5 h-3.5" />}
              >
                Отправить
              </Button>
            </div>
          </div>

          {aiResponse && (
            <div className="space-y-3 pt-2">
              <div className="p-4 bg-slate-50 border border-slate-200/80 rounded-lg">
                <span className="text-xs font-medium text-slate-500 block mb-1">
                  Ответ ассистента:
                </span>
                <p className="text-xs sm:text-sm text-slate-900 whitespace-pre-wrap">{aiResponse.response}</p>
              </div>

              {aiResponse.tool_calls && aiResponse.tool_calls.length > 0 && (
                <div className="p-4 bg-slate-50 border border-slate-200/80 rounded-lg space-y-2">
                  <span className="text-xs font-medium text-slate-500 block">
                    Выполненные инструменты ({aiResponse.tool_calls.length}):
                  </span>
                  <div className="space-y-1.5">
                    {aiResponse.tool_calls.map((tc, idx) => (
                      <div key={idx} className="p-3 bg-white border border-slate-200 rounded-lg text-xs font-mono">
                        <div className="flex items-center justify-between font-semibold">
                          <span className="text-slate-900">{tc.tool}</span>
                          <span className={tc.success ? 'text-emerald-700' : 'text-rose-700'}>
                            {tc.success ? 'Успешно' : 'Ошибка'}
                          </span>
                        </div>
                        <div className="text-slate-500 mt-1">Параметры: {JSON.stringify(tc.arguments)}</div>
                        <div className="text-slate-700 mt-0.5">Результат: {JSON.stringify(tc.result)}</div>
                      </div>
                    ))}
                  </div>
                </div>
              )}
            </div>
          )}
        </form>
      </Card>

      {/* Tool Registry */}
      <Card
        title={`Реестр инструментов (${tools.length})`}
        subtitle="Типизированный API интерфейс для вызова функций колонки"
      >
        <div className="divide-y divide-slate-100">
          {tools.length === 0 ? (
            <div className="py-6 text-center text-xs text-slate-500">
              Инструменты регистрируются ядром при инициализации...
            </div>
          ) : (
            tools.map((t, i) => (
              <div key={i} className="py-3 flex flex-col sm:flex-row sm:items-center justify-between gap-2">
                <div className="space-y-0.5">
                  <span className="text-xs font-mono font-semibold text-slate-900">{t.name}</span>
                  <p className="text-xs text-slate-500">{t.description}</p>
                </div>
                <span className="text-[11px] font-mono text-slate-400 bg-slate-100 px-2 py-0.5 rounded-md self-start sm:self-auto">
                  strict typed
                </span>
              </div>
            ))
          )}
        </div>
      </Card>
    </div>
  );
};
