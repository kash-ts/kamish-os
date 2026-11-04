import React, { useState, useRef, useEffect } from 'react';
import { Send, Trash2, Loader2 } from 'lucide-react';
import { Card } from './Card';
import { Button } from './Button';
import { sendAiPrompt } from '../api';

interface Message {
  id: string;
  sender: 'user' | 'assistant';
  text: string;
  timestamp: string;
}

interface AiAssistantProps {
  onToolExecuted?: () => void;
}

const QUICK_PROMPTS = [
  'Поставь будильник на 07:30',
  'Статус устройства',
  'Поставь громкость 70%',
  'Включи музыку',
  'Напомни через 15 минут выпить воды',
  'Покажи будильники',
];

export const AiAssistant: React.FC<AiAssistantProps> = ({ onToolExecuted }) => {
  const [prompt, setPrompt] = useState('');
  const [isLoading, setIsLoading] = useState(false);
  const [sessionId, setSessionId] = useState<string | undefined>();
  const [messages, setMessages] = useState<Message[]>([
    {
      id: 'welcome',
      sender: 'assistant',
      text: 'Привет! Я локальный ассистент умной колонки Камыш. Могу поставить будильник, напоминание, включить музыку, отрегулировать громкость или показать статус подсистем.',
      timestamp: new Date().toLocaleTimeString([], { hour: '2-digit', minute: '2-digit' }),
    },
  ]);

  const messagesEndRef = useRef<HTMLDivElement>(null);

  const scrollToBottom = () => {
    messagesEndRef.current?.scrollIntoView({ behavior: 'smooth' });
  };

  useEffect(() => {
    scrollToBottom();
  }, [messages, isLoading]);

  const handleSend = async (overridePrompt?: string) => {
    const textToSend = overridePrompt || prompt;
    if (!textToSend.trim() || isLoading) return;

    const userMsgId = `user-${Date.now()}`;
    const userMessage: Message = {
      id: userMsgId,
      sender: 'user',
      text: textToSend.trim(),
      timestamp: new Date().toLocaleTimeString([], { hour: '2-digit', minute: '2-digit' }),
    };

    setMessages((prev) => [...prev, userMessage]);
    if (!overridePrompt) setPrompt('');
    setIsLoading(true);

    try {
      const result = await sendAiPrompt(textToSend.trim(), sessionId);
      if (result.session_id) {
        setSessionId(result.session_id);
      }

      const assistantMessage: Message = {
        id: `ai-${Date.now()}`,
        sender: 'assistant',
        text: result.response,
        timestamp: new Date().toLocaleTimeString([], { hour: '2-digit', minute: '2-digit' }),
      };

      setMessages((prev) => [...prev, assistantMessage]);

      if (result.tool_calls && result.tool_calls.length > 0) {
        onToolExecuted?.();
      }
    } catch (err: any) {
      const errorMessage: Message = {
        id: `err-${Date.now()}`,
        sender: 'assistant',
        text: `Ошибка выполнения запроса: ${err.message || 'Не удалось связаться со службой ИИ'}`,
        timestamp: new Date().toLocaleTimeString([], { hour: '2-digit', minute: '2-digit' }),
      };
      setMessages((prev) => [...prev, errorMessage]);
    } finally {
      setIsLoading(false);
    }
  };

  const handleClearHistory = () => {
    setSessionId(undefined);
    setMessages([
      {
        id: 'welcome-reset',
        sender: 'assistant',
        text: 'История диалога очищена.',
        timestamp: new Date().toLocaleTimeString([], { hour: '2-digit', minute: '2-digit' }),
      },
    ]);
  };

  return (
    <Card
      title="Голосовой ассистент (Локально)"
      subtitle="Ввод текстовых команд и вызов функций колонки"
      action={
        messages.length > 1 && (
          <Button
            variant="ghost"
            size="sm"
            onClick={handleClearHistory}
            leftIcon={<Trash2 className="w-3.5 h-3.5" />}
          >
            Очистить
          </Button>
        )
      }
    >
      {/* Messages Scroll Area */}
      <div className="h-64 sm:h-72 overflow-y-auto pr-1 space-y-3 rounded-lg p-4 bg-slate-50 border border-slate-100">
        {messages.map((msg) => {
          const isUser = msg.sender === 'user';
          return (
            <div
              key={msg.id}
              className={`flex flex-col ${isUser ? 'items-end' : 'items-start'}`}
            >
              <span className="text-[10px] text-slate-400 mb-1 px-1 font-medium">
                {isUser ? 'Вы' : 'Ассистент'}
              </span>
              <div
                className={`max-w-[85%] sm:max-w-[75%] rounded-lg p-3 text-xs leading-relaxed ${
                  isUser
                    ? 'bg-slate-900 text-white'
                    : 'bg-white border border-slate-200/80 text-slate-800 shadow-xs'
                }`}
              >
                <p className="whitespace-pre-wrap">{msg.text}</p>
                <div
                  className={`text-[10px] mt-1 ${
                    isUser ? 'text-slate-400' : 'text-slate-400'
                  } text-right`}
                >
                  {msg.timestamp}
                </div>
              </div>
            </div>
          );
        })}

        {isLoading && (
          <div className="flex gap-2 items-center text-slate-500 text-xs italic pl-1">
            <Loader2 className="w-3.5 h-3.5 animate-spin text-slate-700" />
            <span>Обработка запроса...</span>
          </div>
        )}

        <div ref={messagesEndRef} />
      </div>

      {/* Quick Prompts */}
      <div className="flex items-center gap-1.5 overflow-x-auto py-2.5">
        <span className="text-xs text-slate-400 shrink-0 font-medium mr-1">Примеры:</span>
        {QUICK_PROMPTS.map((qp) => (
          <button
            key={qp}
            type="button"
            disabled={isLoading}
            onClick={() => handleSend(qp)}
            className="text-xs px-2.5 py-1 rounded-lg bg-white border border-slate-200 text-slate-700 hover:text-slate-900 hover:bg-slate-50 transition-colors shrink-0 disabled:opacity-50 cursor-pointer shadow-xs"
          >
            {qp}
          </button>
        ))}
      </div>

      {/* Input Box */}
      <form
        onSubmit={(e) => {
          e.preventDefault();
          handleSend();
        }}
        className="flex gap-2 items-center pt-1"
      >
        <input
          type="text"
          value={prompt}
          onChange={(e) => setPrompt(e.target.value)}
          placeholder="Введите команду (например: 'поставь будильник на 07:30')..."
          disabled={isLoading}
          className="flex-1 bg-white border border-slate-200 rounded-lg px-3 py-2 text-xs sm:text-sm text-slate-900 placeholder-slate-400 focus:outline-none focus:ring-2 focus:ring-slate-900/10 focus:border-slate-800 transition-colors disabled:opacity-50"
        />
        <Button
          type="submit"
          variant="primary"
          size="sm"
          disabled={!prompt.trim() || isLoading}
          isLoading={isLoading}
          leftIcon={<Send className="w-3.5 h-3.5" />}
        >
          Отправить
        </Button>
      </form>
    </Card>
  );
};
