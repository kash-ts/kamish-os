import type {
  AgyLoginChallenge,
  AgyLoginResult,
  AgyStatus,
  AiPromptRequest,
  AiResponse,
  AiReadiness,
  Alarm,
  AudioRoute,
  BluetoothScanResponse,
  BluetoothStatus,
  CreateAlarmRequest,
  CreateReminderRequest,
  HardwareTelemetry,
  HistoryItem,
  NetworkStatus,
  PlaybackState,
  RadioStation,
  RecommendationItem,
  Reminder,
  SearchResult,
  RecognizeResponse,
  SetRouteResponse,
  SetVolumeResponse,
  SimulateWakeWordRequest,
  SimulateWakeWordResponse,
  SpeechStatus,
  StatusResponse,
  SynthesizeRequest,
  SynthesizeResponse,
  ToolsResponse,
  Track,
  UpdateSpeechConfigRequest,
  VoicesResponse,
  VpnConfigRequest,
  VpnConfigResponse,
  VpnConnectResponse,
  VpnDisconnectResponse,
  VpnStatus,
  WifiConnectRequest,
  WifiConnectResponse,
  WifiScanResponse,
  UpdateStatus,
  UpdateBundleInfo,
  UpdateInstallResult,
  SshStatus,
  SshToggleRequest,
  SshToggleResponse,
  LoginRequest,
  LoginResponse,
  SessionResponse,
  ChangePasswordRequest,
  ChangePasswordResponse,
  SetupStepId,
  SetupStatusResponse,
  SaveSetupStepResponse,
  CompleteSetupResponse,
  SimulateDegradationResponse,
} from '../types';

const BASE_URL = '/api/v1';

export function getStoredToken(): string | null {
  return localStorage.getItem('kamish_token');
}

export function setStoredToken(token: string): void {
  localStorage.setItem('kamish_token', token);
}

export function clearStoredToken(): void {
  localStorage.removeItem('kamish_token');
}

export function getAuthHeaders(customHeaders: Record<string, string> = {}): Record<string, string> {
  const headers: Record<string, string> = {
    Accept: 'application/json',
    ...customHeaders,
  };
  const token = getStoredToken();
  if (token) {
    headers['Authorization'] = `Bearer ${token}`;
  }
  return headers;
}

export async function apiFetch(input: RequestInfo | URL, init: RequestInit = {}): Promise<Response> {
  const customHeaders = (init.headers || {}) as Record<string, string>;
  const headers = getAuthHeaders(customHeaders);
  return fetch(input, {
    ...init,
    headers,
    credentials: 'include',
  });
}

async function handleResponse<T>(res: Response): Promise<T> {
  if (!res.ok) {
    let errorMessage = `HTTP ${res.status}: ${res.statusText}`;
    let errorCode: string | undefined;
    try {
      const errorJson = await res.json();
      if (errorJson.error) {
        errorMessage = errorJson.error;
      }
      if (errorJson.code) {
        errorCode = errorJson.code;
      }
    } catch {
      // Keep default errorMessage
    }
    const err = new Error(errorMessage) as Error & { status?: number; code?: string };
    err.status = res.status;
    err.code = errorCode;
    throw err;
  }
  return res.json() as Promise<T>;
}

export async function getStatus(): Promise<StatusResponse> {
  const res = await apiFetch(`${BASE_URL}/status`, {
    headers: { Accept: 'application/json' },
  });
  return handleResponse<StatusResponse>(res);
}

export async function getHardwareTelemetry(): Promise<HardwareTelemetry> {
  const res = await apiFetch(`${BASE_URL}/system/hardware`, {
    headers: { Accept: 'application/json' },
  });
  return handleResponse<HardwareTelemetry>(res);
}

export async function getAlarms(): Promise<Alarm[]> {
  const res = await apiFetch(`${BASE_URL}/alarms`, {
    headers: { Accept: 'application/json' },
  });
  return handleResponse<Alarm[]>(res);
}

export async function createAlarm(data: CreateAlarmRequest): Promise<Alarm> {
  const res = await apiFetch(`${BASE_URL}/alarms`, {
    method: 'POST',
    headers: {
      'Content-Type': 'application/json',
      Accept: 'application/json',
    },
    body: JSON.stringify(data),
  });
  return handleResponse<Alarm>(res);
}

export async function toggleAlarm(id: string, enabled?: boolean): Promise<Alarm> {
  const res = await apiFetch(`${BASE_URL}/alarms/${encodeURIComponent(id)}/toggle`, {
    method: 'POST',
    headers: {
      'Content-Type': 'application/json',
      Accept: 'application/json',
    },
    body: JSON.stringify(enabled !== undefined ? { enabled } : {}),
  });
  return handleResponse<Alarm>(res);
}

export async function deleteAlarm(id: string): Promise<{ status: string; id: string }> {
  const res = await apiFetch(`${BASE_URL}/alarms/${encodeURIComponent(id)}`, {
    method: 'DELETE',
    headers: { Accept: 'application/json' },
  });
  return handleResponse<{ status: string; id: string }>(res);
}

export async function getReminders(status?: string): Promise<Reminder[]> {
  const url = new URL(`${window.location.origin}${BASE_URL}/reminders`);
  if (status && status !== 'all') {
    url.searchParams.set('status', status);
  }
  const res = await apiFetch(url.toString(), {
    headers: { Accept: 'application/json' },
  });
  return handleResponse<Reminder[]>(res);
}

export async function createReminder(data: CreateReminderRequest): Promise<Reminder> {
  const res = await apiFetch(`${BASE_URL}/reminders`, {
    method: 'POST',
    headers: {
      'Content-Type': 'application/json',
      Accept: 'application/json',
    },
    body: JSON.stringify(data),
  });
  return handleResponse<Reminder>(res);
}

export async function deleteReminder(id: string): Promise<{ status: string; id: string }> {
  const res = await apiFetch(`${BASE_URL}/reminders/${encodeURIComponent(id)}`, {
    method: 'DELETE',
    headers: { Accept: 'application/json' },
  });
  return handleResponse<{ status: string; id: string }>(res);
}

export async function sendAiPrompt(prompt: string, sessionId?: string): Promise<AiResponse> {
  const payload: AiPromptRequest = {
    prompt,
    session_id: sessionId,
  };
  const res = await apiFetch(`${BASE_URL}/ai/prompt`, {
    method: 'POST',
    headers: {
      'Content-Type': 'application/json',
      Accept: 'application/json',
    },
    body: JSON.stringify(payload),
  });
  return handleResponse<AiResponse>(res);
}

export async function getAiTools(): Promise<ToolsResponse> {
  const res = await apiFetch(`${BASE_URL}/ai/tools`, {
    headers: { Accept: 'application/json' },
  });
  return handleResponse<ToolsResponse>(res);
}

export async function getAiStatus(): Promise<AiReadiness> {
  const res = await apiFetch(`${BASE_URL}/ai/status`, {
    headers: { Accept: 'application/json' },
  });
  return handleResponse<AiReadiness>(res);
}

export async function updateAiConfig(data: {
  mode: 'cli' | 'mock';
  cli_path?: string;
}): Promise<AiReadiness> {
  const res = await apiFetch(`${BASE_URL}/ai/config`, {
    method: 'POST',
    headers: {
      'Content-Type': 'application/json',
      Accept: 'application/json',
    },
    body: JSON.stringify(data),
  });
  return handleResponse<AiReadiness>(res);
}

export async function probeAi(): Promise<AiReadiness> {
  const res = await apiFetch(`${BASE_URL}/ai/probe`, {
    method: 'POST',
    headers: { Accept: 'application/json' },
  });
  return handleResponse<AiReadiness>(res);
}

// -----------------------------------------------------------------------------
// Antigravity CLI lifecycle (install + authorization from the panel)
// -----------------------------------------------------------------------------

export async function getAgyStatus(): Promise<AgyStatus> {
  const res = await apiFetch(`${BASE_URL}/ai/agy/status`, {
    headers: { Accept: 'application/json' },
  });
  return handleResponse<AgyStatus>(res);
}

export async function installAgy(): Promise<AgyStatus> {
  const res = await apiFetch(`${BASE_URL}/ai/agy/install`, {
    method: 'POST',
    headers: { Accept: 'application/json' },
  });
  return handleResponse<AgyStatus>(res);
}

export async function beginAgyLogin(): Promise<AgyLoginChallenge> {
  const res = await apiFetch(`${BASE_URL}/ai/agy/login/begin`, {
    method: 'POST',
    headers: { Accept: 'application/json' },
  });
  return handleResponse<AgyLoginChallenge>(res);
}

export async function submitAgyCode(code: string): Promise<AgyLoginResult> {
  const res = await apiFetch(`${BASE_URL}/ai/agy/login/submit`, {
    method: 'POST',
    headers: {
      'Content-Type': 'application/json',
      Accept: 'application/json',
    },
    body: JSON.stringify({ code }),
  });
  return handleResponse<AgyLoginResult>(res);
}

// -----------------------------------------------------------------------------
// Music Manager API
// -----------------------------------------------------------------------------

export async function getMusicCurrent(): Promise<PlaybackState> {
  const res = await apiFetch(`${BASE_URL}/music/current`, {
    headers: { Accept: 'application/json' },
  });
  return handleResponse<PlaybackState>(res);
}

export async function playMusic(params?: {
  track_id?: string;
  query?: string;
  uri?: string;
  title?: string;
  artist?: string;
}): Promise<{ status: string; track: Track }> {
  const res = await apiFetch(`${BASE_URL}/music/play`, {
    method: 'POST',
    headers: {
      'Content-Type': 'application/json',
      Accept: 'application/json',
    },
    body: JSON.stringify(params || {}),
  });
  return handleResponse<{ status: string; track: Track }>(res);
}

export async function pauseMusic(): Promise<{ status: string }> {
  const res = await apiFetch(`${BASE_URL}/music/pause`, {
    method: 'POST',
    headers: { Accept: 'application/json' },
  });
  return handleResponse<{ status: string }>(res);
}

export async function resumeMusic(): Promise<{ status: string }> {
  const res = await apiFetch(`${BASE_URL}/music/resume`, {
    method: 'POST',
    headers: { Accept: 'application/json' },
  });
  return handleResponse<{ status: string }>(res);
}

export async function stopMusic(): Promise<{ status: string }> {
  const res = await apiFetch(`${BASE_URL}/music/stop`, {
    method: 'POST',
    headers: { Accept: 'application/json' },
  });
  return handleResponse<{ status: string }>(res);
}

export async function skipMusic(params?: {
  track_id?: string;
  position_secs?: number;
}): Promise<{ status: string; track_id?: string }> {
  const res = await apiFetch(`${BASE_URL}/music/skip`, {
    method: 'POST',
    headers: {
      'Content-Type': 'application/json',
      Accept: 'application/json',
    },
    body: JSON.stringify(params || {}),
  });
  return handleResponse<{ status: string; track_id?: string }>(res);
}

export async function likeMusic(params?: {
  track_id?: string;
  liked?: boolean;
}): Promise<{ track_id: string; liked: boolean }> {
  const res = await apiFetch(`${BASE_URL}/music/like`, {
    method: 'POST',
    headers: {
      'Content-Type': 'application/json',
      Accept: 'application/json',
    },
    body: JSON.stringify(params || {}),
  });
  return handleResponse<{ track_id: string; liked: boolean }>(res);
}

export async function searchMusic(
  query: string,
  source = 'all'
): Promise<{ query: string; source: string; count: number; results: SearchResult[] }> {
  const url = new URL(`${window.location.origin}${BASE_URL}/music/search`);
  url.searchParams.set('q', query);
  url.searchParams.set('source', source);

  const res = await apiFetch(url.toString(), {
    headers: { Accept: 'application/json' },
  });
  return handleResponse<{ query: string; source: string; count: number; results: SearchResult[] }>(res);
}

export async function getMusicFavorites(): Promise<{ count: number; favorites: Track[] }> {
  const res = await apiFetch(`${BASE_URL}/music/favorites`, {
    headers: { Accept: 'application/json' },
  });
  return handleResponse<{ count: number; favorites: Track[] }>(res);
}

export async function getMusicRecommendations(
  limit = 10
): Promise<{ count: number; recommendations: RecommendationItem[] }> {
  const url = new URL(`${window.location.origin}${BASE_URL}/music/recommendations`);
  url.searchParams.set('limit', limit.toString());

  const res = await apiFetch(url.toString(), {
    headers: { Accept: 'application/json' },
  });
  return handleResponse<{ count: number; recommendations: RecommendationItem[] }>(res);
}

export async function getRadioStations(): Promise<{ count: number; stations: RadioStation[] }> {
  const res = await apiFetch(`${BASE_URL}/music/radio`, {
    headers: { Accept: 'application/json' },
  });
  return handleResponse<{ count: number; stations: RadioStation[] }>(res);
}

export async function getMusicHistory(
  limit = 20
): Promise<{ count: number; history: HistoryItem[] }> {
  const url = new URL(`${window.location.origin}${BASE_URL}/music/history`);
  url.searchParams.set('limit', limit.toString());

  const res = await apiFetch(url.toString(), {
    headers: { Accept: 'application/json' },
  });
  return handleResponse<{ count: number; history: HistoryItem[] }>(res);
}

// Stage 6 Speech Manager API
export async function getSpeechStatus(): Promise<SpeechStatus> {
  const res = await apiFetch(`${BASE_URL}/speech/status`, {
    headers: { Accept: 'application/json' },
  });
  return handleResponse<SpeechStatus>(res);
}

export async function getSpeechVoices(): Promise<VoicesResponse> {
  const res = await apiFetch(`${BASE_URL}/speech/voices`, {
    headers: { Accept: 'application/json' },
  });
  return handleResponse<VoicesResponse>(res);
}

export async function updateSpeechConfig(
  data: UpdateSpeechConfigRequest
): Promise<SpeechStatus> {
  const res = await apiFetch(`${BASE_URL}/speech/config`, {
    method: 'POST',
    headers: {
      'Content-Type': 'application/json',
      Accept: 'application/json',
    },
    body: JSON.stringify(data),
  });
  return handleResponse<SpeechStatus>(res);
}

export async function synthesizeSpeech(
  data: SynthesizeRequest
): Promise<SynthesizeResponse> {
  const res = await apiFetch(`${BASE_URL}/speech/synthesize?format=json`, {
    method: 'POST',
    headers: {
      'Content-Type': 'application/json',
      Accept: 'application/json',
    },
    body: JSON.stringify(data),
  });
  return handleResponse<SynthesizeResponse>(res);
}

export async function synthesizeSpeechBlob(
  data: SynthesizeRequest
): Promise<Blob> {
  const res = await apiFetch(`${BASE_URL}/speech/synthesize`, {
    method: 'POST',
    headers: {
      'Content-Type': 'application/json',
      Accept: 'audio/wav, audio/mpeg, */*',
    },
    body: JSON.stringify(data),
  });
  if (!res.ok) {
    throw new Error(`Speech synthesis failed: HTTP ${res.status}`);
  }
  return res.blob();
}

export async function recognizeSpeech(data: {
  audio_base64: string;
  language?: string;
  provider?: string;
  dispatch_ai?: boolean;
}): Promise<RecognizeResponse> {
  const res = await apiFetch(`${BASE_URL}/speech/recognize`, {
    method: 'POST',
    headers: {
      'Content-Type': 'application/json',
      Accept: 'application/json',
    },
    body: JSON.stringify(data),
  });
  return handleResponse<RecognizeResponse>(res);
}

export async function simulateWakeWord(
  data: SimulateWakeWordRequest
): Promise<SimulateWakeWordResponse> {
  const res = await apiFetch(`${BASE_URL}/speech/wakeword/simulate`, {
    method: 'POST',
    headers: {
      'Content-Type': 'application/json',
      Accept: 'application/json',
    },
    body: JSON.stringify(data),
  });
  return handleResponse<SimulateWakeWordResponse>(res);
}

// -----------------------------------------------------------------------------
// System Audio & Hardware API (Stage 8)
// -----------------------------------------------------------------------------

export async function setAudioVolume(volume: number): Promise<SetVolumeResponse> {
  const res = await apiFetch(`${BASE_URL}/system/audio/volume`, {
    method: 'POST',
    headers: {
      'Content-Type': 'application/json',
      Accept: 'application/json',
    },
    body: JSON.stringify({ volume }),
  });
  return handleResponse<SetVolumeResponse>(res);
}

export async function setVolumeCeiling(ceiling: number): Promise<SetVolumeResponse> {
  const res = await apiFetch(`${BASE_URL}/system/audio/ceiling`, {
    method: 'POST',
    headers: {
      'Content-Type': 'application/json',
      Accept: 'application/json',
    },
    body: JSON.stringify({ ceiling }),
  });
  return handleResponse<SetVolumeResponse>(res);
}

export async function setAudioRoute(route: AudioRoute): Promise<SetRouteResponse> {
  const res = await apiFetch(`${BASE_URL}/system/audio/route`, {
    method: 'POST',
    headers: {
      'Content-Type': 'application/json',
      Accept: 'application/json',
    },
    body: JSON.stringify({ route }),
  });
  return handleResponse<SetRouteResponse>(res);
}

// -----------------------------------------------------------------------------
// Bluetooth A2DP Sink API (Stage 7)
// -----------------------------------------------------------------------------

export async function getBluetoothStatus(): Promise<BluetoothStatus> {
  const res = await apiFetch(`${BASE_URL}/bluetooth/status`, {
    headers: { Accept: 'application/json' },
  });
  return handleResponse<BluetoothStatus>(res);
}

export async function scanBluetooth(durationSecs = 5): Promise<BluetoothScanResponse> {
  const res = await apiFetch(`${BASE_URL}/bluetooth/scan`, {
    method: 'POST',
    headers: {
      'Content-Type': 'application/json',
      Accept: 'application/json',
    },
    body: JSON.stringify({ duration_secs: durationSecs }),
  });
  return handleResponse<BluetoothScanResponse>(res);
}

export async function pairBluetooth(mac: string): Promise<{ status: string; mac: string }> {
  const res = await apiFetch(`${BASE_URL}/bluetooth/pair`, {
    method: 'POST',
    headers: {
      'Content-Type': 'application/json',
      Accept: 'application/json',
    },
    body: JSON.stringify({ mac }),
  });
  return handleResponse<{ status: string; mac: string }>(res);
}

export async function connectBluetooth(mac: string): Promise<{ status: string; mac: string }> {
  const res = await apiFetch(`${BASE_URL}/bluetooth/connect`, {
    method: 'POST',
    headers: {
      'Content-Type': 'application/json',
      Accept: 'application/json',
    },
    body: JSON.stringify({ mac }),
  });
  return handleResponse<{ status: string; mac: string }>(res);
}

export async function disconnectBluetooth(): Promise<{ status: string }> {
  const res = await apiFetch(`${BASE_URL}/bluetooth/disconnect`, {
    method: 'POST',
    headers: { Accept: 'application/json' },
  });
  return handleResponse<{ status: string }>(res);
}

// -----------------------------------------------------------------------------
// Network & Wi-Fi API (Stage 7)
// -----------------------------------------------------------------------------

export async function getNetworkStatus(): Promise<NetworkStatus> {
  const res = await apiFetch(`${BASE_URL}/network/status`, {
    headers: { Accept: 'application/json' },
  });
  return handleResponse<NetworkStatus>(res);
}

export async function scanWifi(): Promise<WifiScanResponse> {
  const res = await apiFetch(`${BASE_URL}/network/wifi/scan`, {
    headers: { Accept: 'application/json' },
  });
  return handleResponse<WifiScanResponse>(res);
}

export async function connectWifi(data: WifiConnectRequest): Promise<WifiConnectResponse> {
  const res = await apiFetch(`${BASE_URL}/network/wifi/connect`, {
    method: 'POST',
    headers: {
      'Content-Type': 'application/json',
      Accept: 'application/json',
    },
    body: JSON.stringify(data),
  });
  return handleResponse<WifiConnectResponse>(res);
}

export async function disconnectWifi(): Promise<{ status: string }> {
  const res = await apiFetch(`${BASE_URL}/network/wifi/disconnect`, {
    method: 'POST',
    headers: { Accept: 'application/json' },
  });
  return handleResponse<{ status: string }>(res);
}

export async function startApMode(): Promise<{ status: string; ssid: string; ip: string }> {
  const res = await apiFetch(`${BASE_URL}/network/ap/start`, {
    method: 'POST',
    headers: { Accept: 'application/json' },
  });
  return handleResponse<{ status: string; ssid: string; ip: string }>(res);
}

export async function stopApMode(): Promise<{ status: string }> {
  const res = await apiFetch(`${BASE_URL}/network/ap/stop`, {
    method: 'POST',
    headers: { Accept: 'application/json' },
  });
  return handleResponse<{ status: string }>(res);
}

// -----------------------------------------------------------------------------
// VPN & Proxy API (Stage 7)
// -----------------------------------------------------------------------------

export async function getVpnStatus(): Promise<VpnStatus> {
  const res = await apiFetch(`${BASE_URL}/vpn/status`, {
    headers: { Accept: 'application/json' },
  });
  return handleResponse<VpnStatus>(res);
}

export async function connectVpn(): Promise<VpnConnectResponse> {
  const res = await apiFetch(`${BASE_URL}/vpn/connect`, {
    method: 'POST',
    headers: { Accept: 'application/json' },
  });
  return handleResponse<VpnConnectResponse>(res);
}

export async function disconnectVpn(): Promise<VpnDisconnectResponse> {
  const res = await apiFetch(`${BASE_URL}/vpn/disconnect`, {
    method: 'POST',
    headers: { Accept: 'application/json' },
  });
  return handleResponse<VpnDisconnectResponse>(res);
}

export async function configureVpn(data: VpnConfigRequest): Promise<VpnConfigResponse> {
  const res = await apiFetch(`${BASE_URL}/vpn/config`, {
    method: 'POST',
    headers: {
      'Content-Type': 'application/json',
      Accept: 'application/json',
    },
    body: JSON.stringify(data),
  });
  return handleResponse<VpnConfigResponse>(res);
}

// -----------------------------------------------------------------------------
// KAMISH OS A/B Updates & Dropbear SSH API
// -----------------------------------------------------------------------------

export async function getUpdateStatus(): Promise<UpdateStatus> {
  const res = await apiFetch(`${BASE_URL}/system/update/status`, {
    headers: { Accept: 'application/json' },
  });
  return handleResponse<UpdateStatus>(res);
}

export async function uploadUpdateBundle(file: File): Promise<UpdateBundleInfo> {
  const formData = new FormData();
  formData.append('bundle', file);

  const res = await apiFetch(`${BASE_URL}/system/update/upload`, {
    method: 'POST',
    body: formData,
  });
  return handleResponse<UpdateBundleInfo>(res);
}

export async function installUpdate(bundlePath?: string): Promise<UpdateInstallResult> {
  const res = await apiFetch(`${BASE_URL}/system/update/install`, {
    method: 'POST',
    headers: {
      'Content-Type': 'application/json',
      Accept: 'application/json',
    },
    body: JSON.stringify({ bundle_path: bundlePath }),
  });
  return handleResponse<UpdateInstallResult>(res);
}

export async function getSshStatus(): Promise<SshStatus> {
  const res = await apiFetch(`${BASE_URL}/system/ssh/status`, {
    headers: { Accept: 'application/json' },
  });
  return handleResponse<SshStatus>(res);
}

export async function toggleSsh(data: SshToggleRequest): Promise<SshToggleResponse> {
  const res = await apiFetch(`${BASE_URL}/system/ssh/toggle`, {
    method: 'POST',
    headers: getAuthHeaders({ 'Content-Type': 'application/json' }),
    credentials: 'include',
    body: JSON.stringify(data),
  });
  return handleResponse<SshToggleResponse>(res);
}

// -----------------------------------------------------------------------------
// Authentication Endpoints (Requirement R1)
// -----------------------------------------------------------------------------

export async function login(credentials: LoginRequest): Promise<LoginResponse> {
  const res = await apiFetch(`${BASE_URL}/auth/login`, {
    method: 'POST',
    headers: getAuthHeaders({ 'Content-Type': 'application/json' }),
    credentials: 'include',
    body: JSON.stringify(credentials),
  });
  const data = await handleResponse<LoginResponse>(res);
  if (data?.token) {
    setStoredToken(data.token);
  }
  return data;
}

export async function changePassword(
  data: ChangePasswordRequest
): Promise<ChangePasswordResponse> {
  const res = await apiFetch(`${BASE_URL}/auth/change-password`, {
    method: 'POST',
    headers: getAuthHeaders({ 'Content-Type': 'application/json' }),
    credentials: 'include',
    body: JSON.stringify(data),
  });
  return handleResponse<ChangePasswordResponse>(res);
}

export async function getSession(): Promise<SessionResponse> {
  const res = await apiFetch(`${BASE_URL}/auth/session`, {
    headers: getAuthHeaders(),
    credentials: 'include',
  });
  return handleResponse<SessionResponse>(res);
}

export async function logout(): Promise<{ status: string }> {
  try {
    const res = await apiFetch(`${BASE_URL}/auth/logout`, {
      method: 'POST',
      headers: getAuthHeaders(),
      credentials: 'include',
    });
    return await handleResponse<{ status: string }>(res);
  } finally {
    clearStoredToken();
  }
}

// -----------------------------------------------------------------------------
// Setup Wizard Endpoints (Requirement R2)
// -----------------------------------------------------------------------------

export async function getSetupStatus(): Promise<SetupStatusResponse> {
  const res = await apiFetch(`${BASE_URL}/setup/status`, {
    headers: getAuthHeaders(),
    credentials: 'include',
  });
  return handleResponse<SetupStatusResponse>(res);
}

export async function saveSetupStep(
  step_id: SetupStepId,
  data: Record<string, any>
): Promise<SaveSetupStepResponse> {
  const res = await apiFetch(`${BASE_URL}/setup/step`, {
    method: 'POST',
    headers: getAuthHeaders({ 'Content-Type': 'application/json' }),
    credentials: 'include',
    body: JSON.stringify({ step: step_id, status: 'completed', data }),
  });
  return handleResponse<SaveSetupStepResponse>(res);
}

export async function completeSetup(): Promise<CompleteSetupResponse> {
  const res = await apiFetch(`${BASE_URL}/setup/complete`, {
    method: 'POST',
    headers: getAuthHeaders({ 'Content-Type': 'application/json' }),
    credentials: 'include',
  });
  return handleResponse<CompleteSetupResponse>(res);
}

export async function simulateDegradation(
  component: string,
  degraded: boolean,
  reason?: string
): Promise<SimulateDegradationResponse> {
  const res = await apiFetch(`${BASE_URL}/setup/simulate-degradation`, {
    method: 'POST',
    headers: getAuthHeaders({ 'Content-Type': 'application/json' }),
    credentials: 'include',
    body: JSON.stringify({ component, degraded, reason }),
  });
  return handleResponse<SimulateDegradationResponse>(res);
}

export async function playAudioTest(): Promise<{ status: string }> {
  const res = await apiFetch(`${BASE_URL}/system/audio/test`, {
    method: 'POST',
    headers: getAuthHeaders({ 'Content-Type': 'application/json' }),
    credentials: 'include',
  });
  return handleResponse<{ status: string }>(res);
}


