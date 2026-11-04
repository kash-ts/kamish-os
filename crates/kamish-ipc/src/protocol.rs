use serde::{Deserialize, Serialize};

pub const DEFAULT_SOCKET_PATH: &str = "/run/kamish/core.sock";
pub const FALLBACK_SOCKET_PATH: &str = "/tmp/kamish_core.sock";

/// Returns the active socket path depending on environment and directory accessibility
pub fn resolve_socket_path() -> String {
    if let Ok(custom) = std::env::var("KAMISH_CORE_SOCK") {
        return custom;
    }
    if std::path::Path::new("/run/kamish").is_dir() {
        DEFAULT_SOCKET_PATH.to_string()
    } else {
        FALLBACK_SOCKET_PATH.to_string()
    }
}

/// Wire envelope exchanged over the Unix domain socket
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", content = "data")]
pub enum IpcMessage {
    Request { id: String, request: CoreRequest },
    Response { id: String, response: CoreResponse },
    Event { event: CoreEvent },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "action", content = "payload")]
pub enum CoreRequest {
    // Status & Telemetry
    GetStatus,
    GetHardwareTelemetry,

    // Audio & Playback
    GetAudioStatus,
    SetVolume {
        volume: u8,
    },
    PlayTrack {
        title: String,
        uri: Option<String>,
        artist: Option<String>,
    },
    Pause,
    Resume,
    Stop,
    Next,
    Previous,
    GetPlaybackState,
    SearchMusic {
        query: String,
        source: Option<String>,
        limit: usize,
    },
    GetMusicHistory {
        limit: usize,
    },
    GetRecommendations {
        limit: usize,
    },
    GetYouTubeAuthStatus,
    StartYouTubeAuth,
    PollYouTubeAuth,

    // Scheduler
    ListAlarms,
    CreateAlarm {
        time: String,
        days: Vec<u8>,
        sound: String,
        enabled: bool,
        label: Option<String>,
    },
    UpdateAlarm {
        id: String,
        time: Option<String>,
        days: Option<Vec<u8>>,
        sound: Option<String>,
        enabled: Option<bool>,
        label: Option<String>,
    },
    DeleteAlarm {
        id: String,
    },
    ListReminders,
    CreateReminder {
        text: String,
        trigger_at: String,
    },
    DeleteReminder {
        id: String,
    },

    // Speech & VAD
    GetSpeechStatus,
    SetWakeWordDetection {
        enabled: bool,
    },
    SetVadThreshold {
        threshold: f32,
    },
    StartCapture,
    StopCapture,
    SynthesizeSpeech {
        text: String,
    },

    // AI
    ExecutePrompt {
        prompt: String,
        session_id: Option<String>,
    },
    GetAgyStatus,
    ClearAiSession {
        session_id: String,
    },

    // Network & Wi-Fi
    GetNetworkStatus,
    ScanWifi,
    ConnectWifi {
        ssid: String,
        password: Option<String>,
    },
    StartSetupAp,
    StopSetupAp,

    // Bluetooth
    GetBluetoothStatus,
    StartBluetoothScan,
    ConnectBluetoothDevice {
        address: String,
    },
    DisconnectBluetoothDevice {
        address: String,
    },

    // VPN
    GetVpnStatus,
    SaveVpnConfig {
        vless_url: String,
    },
    ConnectVpn,
    DisconnectVpn,

    // Setup & Diagnostics
    GetSetupStatus,
    RunDiagnostics,
    CompleteSetupStep {
        step: u8,
        data: serde_json::Value,
    },

    // System Operations
    TriggerReboot,
    TriggerShutdown,
    CheckUpdates,
    PerformUpdate,

    // Raw/Generic Extensibility
    RawCommand {
        command: String,
        payload: serde_json::Value,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "status", content = "data")]
pub enum CoreResponse {
    Ok(serde_json::Value),
    Error { code: String, message: String },
}

impl CoreResponse {
    pub fn ok<T: Serialize>(value: T) -> Self {
        match serde_json::to_value(value) {
            Ok(v) => CoreResponse::Ok(v),
            Err(e) => CoreResponse::Error {
                code: "serialization_error".to_string(),
                message: e.to_string(),
            },
        }
    }

    pub fn err<C: Into<String>, M: Into<String>>(code: C, message: M) -> Self {
        CoreResponse::Error {
            code: code.into(),
            message: message.into(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "event", content = "data")]
pub enum CoreEvent {
    VolumeChanged {
        level: u8,
    },
    PlaybackStateChanged {
        state: String,
        track: Option<serde_json::Value>,
    },
    AlarmTriggered {
        id: String,
        time: String,
        label: Option<String>,
    },
    AlarmToggled {
        id: String,
        enabled: bool,
    },
    ReminderTriggered {
        id: String,
        text: String,
    },
    SpeechRecognized {
        text: String,
        final_result: bool,
    },
    WakeWordDetected {
        score: f32,
    },
    HardwareWarning {
        cpu_temp_c: f32,
        throttled: bool,
    },
    NetworkStateChanged {
        connected: bool,
        ip: Option<String>,
        ssid: Option<String>,
    },
    BluetoothStateChanged {
        connected: bool,
        device: Option<String>,
    },
    VpnStateChanged {
        connected: bool,
    },
    Generic {
        event_name: String,
        payload: serde_json::Value,
    },
}
