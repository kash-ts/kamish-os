import React, { useState, useEffect, useCallback } from 'react';
import { Sidebar, type NavPage } from './components/Sidebar';
import { TopHeader } from './components/TopHeader';
import { ConnectionLostModal } from './components/ConnectionLostModal';
import { LoginPage } from './pages/LoginPage';
import { ChangePasswordPage } from './pages/ChangePasswordPage';
import { SetupWizardPage } from './pages/SetupWizardPage';
import { DashboardPage } from './pages/DashboardPage';
import { SystemPage } from './pages/SystemPage';
import { VoiceSoundPage } from './pages/VoiceSoundPage';
import { AiModelsPage } from './pages/AiModelsPage';
import { NetworkPage } from './pages/NetworkPage';
import { MusicPage } from './pages/MusicPage';
import { AlarmsRemindersPage } from './pages/AlarmsRemindersPage';
import { DiagnosticsPage } from './pages/DiagnosticsPage';
import { UpdatesPage } from './pages/UpdatesPage';

import {
  getStatus,
  getSession,
  logout,
  getSetupStatus,
  getAlarms,
  createAlarm,
  toggleAlarm,
  deleteAlarm,
  getReminders,
  createReminder,
  deleteReminder,
} from './api';

import type {
  StatusResponse,
  AuthUser,
  SetupStatusResponse,
  Alarm,
  Reminder,
  CreateAlarmRequest,
  CreateReminderRequest,
} from './types';

export const App: React.FC = () => {
  // Auth state
  const [currentUser, setCurrentUser] = useState<AuthUser | null>(null);
  const [isAuthenticated, setIsAuthenticated] = useState<boolean>(false);
  const [mustChangePassword, setMustChangePassword] = useState<boolean>(false);
  const [isAuthChecking, setIsAuthChecking] = useState<boolean>(true);

  // Setup wizard state
  const [setupStatus, setSetupStatus] = useState<SetupStatusResponse | null>(null);
  const [showSetupWizard, setShowSetupWizard] = useState<boolean>(false);

  // Navigation & UI state
  const [currentPage, setCurrentPage] = useState<NavPage>('dashboard');
  const [isMobileMenuOpen, setIsMobileMenuOpen] = useState<boolean>(false);

  // Speaker core data
  const [status, setStatus] = useState<StatusResponse | null>(null);
  const [alarms, setAlarms] = useState<Alarm[]>([]);
  const [reminders, setReminders] = useState<Reminder[]>([]);
  const [isCoreConnected, setIsCoreConnected] = useState<boolean>(true);
  const [isLoading, setIsLoading] = useState<boolean>(false);

  // Modal triggers for alarms
  const [isCreateAlarmOpen, setIsCreateAlarmOpen] = useState(false);
  const [isCreateReminderOpen, setIsCreateReminderOpen] = useState(false);

  // 1. Initial auth and session validation
  const checkSession = useCallback(async () => {
    setIsAuthChecking(true);
    try {
      const res = await getSession();
      if (res && res.user) {
        setCurrentUser(res.user);
        setIsAuthenticated(true);
        setMustChangePassword(res.user.must_change_password);
      } else {
        setIsAuthenticated(false);
        setCurrentUser(null);
      }
    } catch {
      setIsAuthenticated(false);
      setCurrentUser(null);
    } finally {
      setIsAuthChecking(false);
    }
  }, []);

  // 2. Refresh device status, setup state, alarms, reminders
  const refreshAll = useCallback(async () => {
    setIsLoading(true);
    try {
      const [statusRes, setupRes, alarmsRes, remindersRes] = await Promise.all([
        getStatus().catch(() => null),
        getSetupStatus().catch(() => null),
        getAlarms().catch(() => [] as Alarm[]),
        getReminders().catch(() => [] as Reminder[]),
      ]);

      if (statusRes) {
        setStatus(statusRes);
        setIsCoreConnected(true);
      } else {
        setIsCoreConnected(false);
      }

      if (setupRes) {
        setSetupStatus(setupRes);
        if (!setupRes.is_completed || setupRes.degraded_components.length > 0) {
          setShowSetupWizard(true);
        } else {
          setShowSetupWizard(false);
        }
      }

      setAlarms(alarmsRes);
      setReminders(remindersRes);
    } catch {
      setIsCoreConnected(false);
    } finally {
      setIsLoading(false);
    }
  }, []);

  useEffect(() => {
    checkSession();
  }, [checkSession]);

  useEffect(() => {
    if (isAuthenticated && !mustChangePassword) {
      refreshAll();
      const interval = setInterval(refreshAll, 10000);

      // WebSocket Real-time Event Bus
      let ws: WebSocket | null = null;
      let reconnectTimeout: ReturnType<typeof setTimeout> | null = null;

      const connectWs = () => {
        const protocol = window.location.protocol === 'https:' ? 'wss:' : 'ws:';
        const wsUrl = `${protocol}//${window.location.host}/api/v1/events`;
        try {
          ws = new WebSocket(wsUrl);
          ws.onopen = () => {
            setIsCoreConnected(true);
          };
          ws.onmessage = () => {
            refreshAll();
          };
          ws.onerror = () => {
            setIsCoreConnected(false);
          };
          ws.onclose = () => {
            setIsCoreConnected(false);
            reconnectTimeout = setTimeout(connectWs, 2000);
          };
        } catch {
          setIsCoreConnected(false);
          reconnectTimeout = setTimeout(connectWs, 3000);
        }
      };

      connectWs();

      const handleOnline = () => {
        refreshAll();
      };
      const handleOffline = () => {
        setIsCoreConnected(false);
      };

      window.addEventListener('online', handleOnline);
      window.addEventListener('offline', handleOffline);

      return () => {
        clearInterval(interval);
        if (reconnectTimeout) clearTimeout(reconnectTimeout);
        if (ws) ws.close();
        window.removeEventListener('online', handleOnline);
        window.removeEventListener('offline', handleOffline);
      };
    }
  }, [isAuthenticated, mustChangePassword, refreshAll]);

  // Logout handler
  const handleLogout = async () => {
    try {
      await logout();
    } catch {
      // Ignored, proceed to clear local state
    }
    localStorage.removeItem('kamish_auth_token');
    setIsAuthenticated(false);
    setCurrentUser(null);
    setMustChangePassword(false);
    setCurrentPage('dashboard');
  };

  const handleSetupComplete = useCallback(() => {
    setShowSetupWizard(false);
    refreshAll();
  }, [refreshAll]);

  const handleForcedPasswordRequired = useCallback(() => {
    setMustChangePassword(true);
  }, []);

  // Alarms Handlers
  const handleCreateAlarm = async (data: CreateAlarmRequest) => {
    const newAlarm = await createAlarm(data);
    setAlarms((prev) => [newAlarm, ...prev]);
    getStatus().then(setStatus).catch(() => {});
  };

  const handleToggleAlarm = async (id: string, enabled?: boolean) => {
    const updated = await toggleAlarm(id, enabled);
    setAlarms((prev) => prev.map((a) => (a.id === id ? updated : a)));
    getStatus().then(setStatus).catch(() => {});
  };

  const handleDeleteAlarm = async (id: string) => {
    await deleteAlarm(id);
    setAlarms((prev) => prev.filter((a) => a.id !== id));
    getStatus().then(setStatus).catch(() => {});
  };

  // Reminders Handlers
  const handleCreateReminder = async (data: CreateReminderRequest) => {
    const newReminder = await createReminder(data);
    setReminders((prev) => [newReminder, ...prev]);
    getStatus().then(setStatus).catch(() => {});
  };

  const handleDeleteReminder = async (id: string) => {
    await deleteReminder(id);
    setReminders((prev) => prev.filter((r) => r.id !== id));
    getStatus().then(setStatus).catch(() => {});
  };

  // 1. Loading state while checking auth
  if (isAuthChecking) {
    return (
      <div className="min-h-screen bg-slate-50 flex flex-col items-center justify-center p-4">
        <div className="w-8 h-8 border-2 border-slate-900 border-t-transparent rounded-full animate-spin mb-4" />
        <span className="text-xs font-mono text-slate-400">
          Загрузка...
        </span>
      </div>
    );
  }

  // 2. Unauthenticated: show Login card
  if (!isAuthenticated || !currentUser) {
    return (
      <LoginPage
        onLoginSuccess={(user) => {
          setCurrentUser(user);
          setIsAuthenticated(true);
          setMustChangePassword(user.must_change_password);
        }}
      />
    );
  }

  // 3. Authenticated but initial password change required
  if (mustChangePassword) {
    return (
      <ChangePasswordPage
        onPasswordChanged={() => {
          setMustChangePassword(false);
          checkSession();
          refreshAll();
        }}
      />
    );
  }

  // 4. Initial Setup Wizard (if not yet completed or recovering degraded component)
  if (showSetupWizard) {
    return (
      <>
        <SetupWizardPage
          onSetupComplete={handleSetupComplete}
          onForcedPasswordRequired={handleForcedPasswordRequired}
        />
        {!isCoreConnected && (
          <ConnectionLostModal onRetry={refreshAll} isRetrying={isLoading} />
        )}
      </>
    );
  }

  // 5. Main Administration Console
  const hasDegraded = (setupStatus?.degraded_components.length ?? 0) > 0;

  return (
    <div className="flex h-screen bg-slate-50 overflow-hidden font-sans text-slate-800">
      {/* Fixed Sidebar */}
      <div className="hidden lg:block w-64 shrink-0">
        <Sidebar
          currentPage={currentPage}
          onNavigate={(page) => {
            setCurrentPage(page);
            setIsMobileMenuOpen(false);
          }}
          isOpen={isMobileMenuOpen}
          onClose={() => setIsMobileMenuOpen(false)}
          onLogout={handleLogout}
          version={status?.version || 'v0.1.0'}
        />
      </div>

      <div className="lg:hidden">
        <Sidebar
          currentPage={currentPage}
          onNavigate={(page) => {
            setCurrentPage(page);
            setIsMobileMenuOpen(false);
          }}
          isOpen={isMobileMenuOpen}
          onClose={() => setIsMobileMenuOpen(false)}
          onLogout={handleLogout}
          version={status?.version || 'v0.1.0'}
        />
      </div>

      {/* Main Content Pane */}
      <div className="flex-1 flex flex-col min-w-0 overflow-hidden">
        {/* Top Header */}
        <TopHeader
          hasDegradedComponents={hasDegraded}
        />

        {/* Dynamic Page Content */}
        <main className="flex-1 overflow-y-auto p-4 sm:p-6 lg:p-8 bg-slate-50">
          <div className="max-w-6xl mx-auto">
            {currentPage === 'dashboard' && (
              <DashboardPage
                status={status}
                alarms={alarms}
                reminders={reminders}
                isLoading={isLoading}
                onRefresh={refreshAll}
                onNavigate={setCurrentPage}
                onOpenCreateAlarm={() => {
                  setCurrentPage('alarms');
                  setIsCreateAlarmOpen(true);
                }}
                onOpenCreateReminder={() => {
                  setCurrentPage('alarms');
                  setIsCreateReminderOpen(true);
                }}
              />
            )}

            {currentPage === 'system' && (
              <SystemPage />
            )}

            {currentPage === 'voice' && (
              <VoiceSoundPage />
            )}

            {currentPage === 'ai' && (
              <AiModelsPage />
            )}

            {currentPage === 'network' && (
              <NetworkPage />
            )}

            {currentPage === 'multimedia' && (
              <MusicPage />
            )}

            {currentPage === 'alarms' && (
              <AlarmsRemindersPage
                alarms={alarms}
                reminders={reminders}
                onCreateAlarm={handleCreateAlarm}
                onToggleAlarm={handleToggleAlarm}
                onDeleteAlarm={handleDeleteAlarm}
                onCreateReminder={handleCreateReminder}
                onDeleteReminder={handleDeleteReminder}
                isCreateAlarmOpen={isCreateAlarmOpen}
                setIsCreateAlarmOpen={setIsCreateAlarmOpen}
                isCreateReminderOpen={isCreateReminderOpen}
                setIsCreateReminderOpen={setIsCreateReminderOpen}
              />
            )}

            {currentPage === 'diagnostics' && (
              <DiagnosticsPage
                status={status}
                isLoading={isLoading}
                onRefresh={refreshAll}
              />
            )}

            {currentPage === 'updates' && (
              <UpdatesPage />
            )}
          </div>
        </main>
      </div>

      {/* Offline / Connection Lost Modal */}
      {!isCoreConnected && (
        <ConnectionLostModal onRetry={refreshAll} isRetrying={isLoading} />
      )}
    </div>
  );
};

export default App;
