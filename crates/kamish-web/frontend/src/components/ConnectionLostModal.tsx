import React from 'react';
import { WifiOff, RefreshCw } from 'lucide-react';

interface ConnectionLostModalProps {
  onRetry?: () => void;
  isRetrying?: boolean;
}

export const ConnectionLostModal: React.FC<ConnectionLostModalProps> = ({
  onRetry,
  isRetrying = false,
}) => {
  return (
    <div className="fixed inset-0 z-50 bg-slate-900/60 backdrop-blur-xs flex items-center justify-center p-4 animate-in fade-in duration-200">
      <div className="bg-white rounded-2xl border border-slate-200/90 shadow-2xl max-w-sm w-full p-6 text-center space-y-4">
        <div className="space-y-1.5">
          <h2 className="text-sm font-semibold text-slate-900">
            Связь с устройством потеряна
          </h2>
          <p className="text-xs text-slate-500 leading-relaxed">
            Потеряно соединение с ядром умной колонки. Ожидание восстановления связи...
          </p>
        </div>
      </div>
    </div>
  );
};
