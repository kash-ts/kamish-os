export interface AudioRoutingInfo {
  input_device: string;
  input_detected: boolean;
  output_device: string;
  mixer_status: string;
  volume: number;
  safe_volume_ceiling: number;
}

export type AudioRoute =
  | 'gt112_default'
  | 'bluetooth_sink'
  | 'headphones'
  | 'system_default';

export interface SetVolumeResponse {
  success: boolean;
  requested_volume: number;
  applied_volume: number;
  safe_ceiling: number;
}

export interface SetRouteResponse {
  success: boolean;
  route: AudioRoute;
  description: string;
}

// Bluetooth Types (Stage 7)
export interface BluetoothConnectedDevice {
  mac: string;
  name: string;
  connected: boolean;
  codec?: string | null;
  volume?: number | null;
}

export interface BluetoothPairedDevice {
  mac: string;
  name: string;
  trusted: boolean;
}

export interface BluetoothDevice {
  mac: string;
  name: string;
  rssi?: number | null;
}

export interface BluetoothStatus {
  adapter_enabled: boolean;
  discoverable: boolean;
  connected_device: BluetoothConnectedDevice | null;
  paired_devices: BluetoothPairedDevice[];
}

export interface BluetoothScanResponse {
  status: string;
  devices_found: BluetoothDevice[];
}

// Network Types (Stage 7)
export interface EthernetStatus {
  connected: boolean;
  speed_mbps?: number | null;
}

export interface WifiStatus {
  connected: boolean;
  ssid?: string | null;
  bssid?: string | null;
  signal_level_percent?: number | null;
  frequency_mhz?: number | null;
}

export interface ApStatus {
  active: boolean;
  ssid: string;
}

export interface NetworkStatus {
  primary_interface: string;
  ip_address: string | null;
  subnet_mask: string | null;
  gateway: string | null;
  dns: string[];
  ethernet: EthernetStatus;
  wifi: WifiStatus | null;
  ap_mode: ApStatus;
}

export interface WifiNetwork {
  ssid: string;
  bssid: string;
  signal_percent: number;
  security: string;
  channel: number;
}

export interface WifiScanResponse {
  count: number;
  networks: WifiNetwork[];
}

export interface WifiConnectRequest {
  ssid: string;
  password?: string;
}

export interface WifiConnectResponse {
  status: string;
  ssid: string;
  ip_address: string;
}

// VPN Types (Stage 7)
export interface VpnStatus {
  enabled: boolean;
  status: string;
  protocol: string;
  server_address?: string | null;
  ping_ms?: number | null;
  bytes_sent: number;
  bytes_received: number;
  connected_since?: string | null;
}

export interface VpnConfigRequest {
  subscription_url?: string;
  vless_uri?: string;
}

export interface VpnConfigResponse {
  status: string;
  server: string;
  sni?: string | null;
}

export interface VpnConnectResponse {
  status: string;
  ping_ms?: number | null;
}

export interface VpnDisconnectResponse {
  status: string;
}

export interface RespeakerInfo {
  detected: boolean;
  card_name?: string | null;
  usb_vendor_id?: string | null;
  usb_product_id?: string | null;
  channels: number;
}

export interface HardwareTelemetry {
  model: string;
  cpu_arch: string;
  cpu_cores: number;
  cpu_temp_celsius: number | null;
  cpu_freq_mhz: number | null;
  throttled: boolean;
  memory_total_bytes: number;
  memory_used_bytes: number;
  storage_total_bytes?: number;
  storage_used_bytes?: number;
  storage_available_bytes?: number;
  respeaker: RespeakerInfo;
  audio_routing: AudioRoutingInfo;
}

export interface DeviceInfo {
  hostname: string;
  local_url?: string;
  os_name: string;
  os_version: string;
  kernel_version: string;
  arch: string;
  cpu_count: number;
  memory_total_bytes: number;
  memory_used_bytes: number;
  storage_total_bytes?: number;
  storage_used_bytes?: number;
  system_uptime_secs: number;
  cpu_temp_celsius?: number | null;
  hardware_model?: string;
}

export interface DatabaseStatus {
  connected: boolean;
  applied_migrations: number;
}

export interface SubsystemStatus {
  audio: string;
  bluetooth: string;
  network: string;
  vpn: string;
  speech: string;
  ai: string;
  scheduler: string;
  music: string;
}

export interface SchedulerStatusSummary {
  active_alarms_count: number;
  pending_reminders_count: number;
  running: boolean;
}

export interface AiReadiness {
  ready: boolean;
  mode: 'cli' | 'mock';
  cli_path: string;
  cli_version?: string | null;
  error_reason?: string | null;
}

/** State of the Antigravity CLI installation and its authorization. */
export interface AgyStatus {
  installed: boolean;
  path: string;
  version?: string | null;
  authorized: boolean;
  problem?: string | null;
  download_url?: string | null;
  download_enabled: boolean;
  architecture: string;
}

/** Authorization step shown to the user: open URL, enter code. */
export interface AgyLoginChallenge {
  auth_url?: string | null;
  auth_code?: string | null;
  instructions: string;
}

/** Outcome of submitting the one-time authorization code. */
export interface AgyLoginResult {
  authorized: boolean;
  message: string;
  raw_output?: string | null;
}

export interface SystemReadiness {
  is_ready: boolean;
  setup_complete: boolean;
  issues: string[];
  ai: AiReadiness;
}

export interface StatusResponse {
  status: string;
  version: string;
  app_uptime_secs: number;
  device: DeviceInfo;
  database: DatabaseStatus;
  subsystems: SubsystemStatus;
  scheduler: SchedulerStatusSummary;
  hardware?: HardwareTelemetry;
  readiness?: SystemReadiness;
}

export interface Alarm {
  id: string;
  time: string; // HH:MM
  timezone: string;
  recurrence: string | null;
  label: string | null;
  enabled: boolean;
  created_at: string;
  updated_at: string;
}

export interface CreateAlarmRequest {
  time: string;
  recurrence?: string | null;
  label?: string | null;
  timezone?: string;
  enabled?: boolean;
}

export interface ToggleAlarmRequest {
  enabled?: boolean;
}

export type ReminderStatus = 'pending' | 'completed' | 'cancelled';

export interface Reminder {
  id: string;
  trigger_time: string; // ISO-8601
  text: string;
  timezone: string;
  status: ReminderStatus;
  created_at: string;
}

export interface CreateReminderRequest {
  trigger_time: string;
  text: string;
  timezone?: string;
}

export interface ApiError {
  error: string;
  code?: string;
}

export interface ToolCallResult {
  tool: string;
  arguments: Record<string, any>;
  result: Record<string, any>;
  success: boolean;
  error?: string;
}

export interface AiResponse {
  response: string;
  tool_calls: ToolCallResult[];
  session_id: string;
}

export interface AiPromptRequest {
  prompt: string;
  session_id?: string;
}

export interface ToolDeclaration {
  name: string;
  description: string;
  parameters: Record<string, any>;
}

export interface ToolsResponse {
  count: number;
  tools: ToolDeclaration[];
}

export type PlaybackStatus = 'playing' | 'paused' | 'stopped';

export interface Track {
  id: string;
  title: string;
  artist: string;
  duration_secs?: number;
  source: string;
  uri?: string;
  extra_json?: string;
  created_at?: string;
}

export interface PlaybackState {
  status: PlaybackStatus;
  current_track: Track | null;
  volume: number;
  position_secs: number;
  duration_secs?: number;
}

export interface SearchResult {
  id: string;
  title: string;
  artist: string;
  duration_secs?: number;
  source: string;
  uri?: string;
  thumbnail_url?: string;
}

export interface RadioStation {
  id: string;
  name: string;
  genre: string;
  stream_uri: string;
  homepage_url?: string;
  bitrate?: number;
  favicon_url?: string;
}

export interface RecommendationItem {
  track: Track;
  score: number;
  reason: string;
}

export interface HistoryItem {
  id: number;
  track_id: string;
  track?: Track;
  played_at: string;
  playback_duration_secs: number;
  completed: boolean;
}

export interface LavalinkStatus {
  connected: boolean;
  host: string;
  port: number;
  version?: string;
  sources: string[];
  plugins: string[];
  yandex_music_configured: boolean;
}

export interface LavalinkConfigRequest {
  host?: string;
  port?: number;
  password?: string;
  yandex_music_token?: string;
}

export interface YouTubeAuthStatus {
  authorized: boolean;
  has_client: boolean;
  access_token_valid: boolean;
  expires_in?: number;
  token_path: string;
  cookies_path: string;
  cookies_ready: boolean;
  problem?: string;
}

export interface DeviceCodeChallenge {
  verification_url: string;
  user_code: string;
  expires_in?: number;
  interval?: number;
  device_code: string;
}

// Stage 6 Speech Manager types
export interface WakeWordEvent {
  keyword: string;
  score: number;
  timestamp_ms: number;
}

export interface SpeechStatus {
  microphone_available: boolean;
  microphone_listening: boolean;
  active_stt_provider: string;
  active_tts_provider: string;
  active_voice: string;
  vad_enabled: boolean;
  wakeword_enabled: boolean;
  available_stt_providers: string[];
  available_tts_providers: string[];
  last_wake_word: WakeWordEvent | null;
}

export interface TtsVoice {
  id: string;
  name: string;
  language: string;
  provider: string;
  gender?: string;
}

export interface VoicesResponse {
  voices: TtsVoice[];
}

export interface SynthesizeRequest {
  text: string;
  voice?: string;
  provider?: string;
  speed?: number;
  play_on_speaker?: boolean;
}

export interface SynthesizeResponse {
  status: string;
  provider: string;
  voice: string;
  audio_format: string;
  size_bytes: number;
  played_on_speaker: boolean;
  audio_base64?: string;
}

export interface SttResult {
  text: string;
  confidence: number;
  language: string;
  provider: string;
  duration_secs?: number;
}

export interface RecognizeResponse {
  stt: SttResult;
  ai_response?: AiResponse;
  spoken_response: boolean;
}

export interface SimulateWakeWordRequest {
  keyword?: string;
  command?: string;
  dispatch_ai?: boolean;
}

export interface SimulateWakeWordResponse {
  event: WakeWordEvent;
  command?: string;
  ai_response?: AiResponse;
  spoken_response: boolean;
}

export interface UpdateSpeechConfigRequest {
  active_stt_provider?: string;
  active_tts_provider?: string;
  active_voice?: string;
  vad_enabled?: boolean;
  wakeword_enabled?: boolean;
}

// KAMISH OS Types
export interface UpdateStatus {
  active_slot: string;
  boot_primary: string;
  installed_version: string;
  last_update?: string | null;
  is_updating: boolean;
  rauc_supported: boolean;
}

export interface UpdateBundleInfo {
  compatible: string;
  version: string;
  description: string;
  build_id: string;
  is_signed: boolean;
  signature_valid: boolean;
}

export interface UpdateInstallResult {
  status: string;
  target_slot: string;
  message: string;
}

export interface SshStatus {
  enabled: boolean;
  user: string;
  port: number;
  active_sessions: number;
  root_login_allowed: boolean;
  has_temp_password: boolean;
}

export interface SshToggleRequest {
  enable: boolean;
  ssh_key?: string | null;
}

export interface SshToggleResponse {
  enabled: boolean;
  temp_password?: string | null;
  user: string;
  port: number;
  message: string;
}

// -----------------------------------------------------------------------------
// Auth & Security Types (Requirement R1)
// -----------------------------------------------------------------------------
export interface AuthUser {
  id: number;
  username: string;
  must_change_password: boolean;
}

export interface LoginRequest {
  username: string;
  password: string;
}

export interface LoginResponse {
  token: string;
  user: AuthUser;
}

export interface SessionResponse {
  authenticated: boolean;
  user?: AuthUser;
}

export interface ChangePasswordRequest {
  old_password: string;
  new_password: string;
}

export interface ChangePasswordResponse {
  status: string;
  message: string;
}

// -----------------------------------------------------------------------------
// Setup Wizard & Degradation Recovery Types (Requirement R2)
// -----------------------------------------------------------------------------
export type SetupStepId = 'network' | 'audio' | 'voice' | 'ai';

export interface SetupStepNetwork {
  status: 'pending' | 'ok' | 'error';
  connected: boolean;
  ssid?: string;
}

export interface SetupStepAudio {
  status: 'pending' | 'ok' | 'error';
  ceiling: number;
  volume: number;
}

export interface SetupStepVoice {
  status: 'pending' | 'ok' | 'error';
  stt_active: boolean;
  tts_active: boolean;
}

export interface SetupStepAi {
  status: 'pending' | 'ok' | 'error';
  cli_ready: boolean;
}

export interface DegradedComponent {
  component: string;
  reason: string;
  recovery_step: SetupStepId;
}

export interface SetupStatusResponse {
  is_completed: boolean;
  current_step: SetupStepId;
  steps: {
    auth?: {
      status?: 'pending' | 'ok' | 'error';
      must_change_password?: boolean;
    };
    network: SetupStepNetwork;
    audio: SetupStepAudio;
    voice: SetupStepVoice;
    ai: SetupStepAi;
  };
  degraded_components: DegradedComponent[];
}

export interface SaveSetupStepRequest {
  step_id: SetupStepId;
  data: Record<string, any>;
}

export interface SaveSetupStepResponse {
  status: string;
  next_step?: SetupStepId;
}

export interface CompleteSetupResponse {
  status: string;
  is_completed: boolean;
}

export interface SimulateDegradationRequest {
  component: string;
  degraded: boolean;
  reason?: string;
}

export interface SimulateDegradationResponse {
  status: string;
  recovery_step: SetupStepId;
}

