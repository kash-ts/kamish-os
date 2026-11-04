-- ==============================================================================
-- Kamish OS Unified Database Schema
-- Version: 1 (Tracked via PRAGMA user_version)
-- ==============================================================================

PRAGMA foreign_keys = ON;

-- 1. System Settings & Configuration
CREATE TABLE IF NOT EXISTS settings (
    key TEXT PRIMARY KEY,
    value TEXT NOT NULL,
    updated_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP
);

-- 2. Device Runtime State & Setup Wizard Progress
CREATE TABLE IF NOT EXISTS device_state (
    key TEXT PRIMARY KEY,
    value TEXT NOT NULL,
    updated_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP
);

-- 3. Users & Authentication
CREATE TABLE IF NOT EXISTS users (
    id TEXT PRIMARY KEY,
    username TEXT UNIQUE NOT NULL,
    password_hash TEXT NOT NULL,
    salt TEXT,
    must_change_password BOOLEAN NOT NULL DEFAULT 1,
    created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
    updated_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP
);

-- 4. Authentication Sessions
CREATE TABLE IF NOT EXISTS auth_sessions (
    token TEXT PRIMARY KEY,
    user_id TEXT NOT NULL,
    created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
    expires_at TIMESTAMP NOT NULL,
    FOREIGN KEY (user_id) REFERENCES users(id) ON DELETE CASCADE
);

CREATE INDEX IF NOT EXISTS idx_auth_sessions_user_id ON auth_sessions(user_id);

-- 5. Scheduler Alarms
CREATE TABLE IF NOT EXISTS alarms (
    id TEXT PRIMARY KEY,
    time TEXT NOT NULL,
    timezone TEXT NOT NULL DEFAULT 'UTC',
    recurrence TEXT NOT NULL DEFAULT 'once',
    label TEXT NOT NULL DEFAULT '',
    enabled BOOLEAN NOT NULL DEFAULT 1,
    created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
    updated_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP
);

CREATE INDEX IF NOT EXISTS idx_alarms_enabled ON alarms(enabled);

-- 6. Scheduler Reminders
CREATE TABLE IF NOT EXISTS reminders (
    id TEXT PRIMARY KEY,
    trigger_time TEXT NOT NULL,
    text TEXT NOT NULL,
    timezone TEXT NOT NULL DEFAULT 'UTC',
    status TEXT NOT NULL DEFAULT 'pending',
    created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP
);

CREATE INDEX IF NOT EXISTS idx_reminders_status ON reminders(status);

-- 7. AI Assistant Sessions
CREATE TABLE IF NOT EXISTS ai_sessions (
    id TEXT PRIMARY KEY,
    created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
    metadata_json TEXT
);

-- 8. Music Catalog Tracks
CREATE TABLE IF NOT EXISTS music_tracks (
    id TEXT PRIMARY KEY,
    title TEXT NOT NULL,
    artist TEXT NOT NULL,
    duration_secs INTEGER NOT NULL DEFAULT 0,
    source TEXT NOT NULL DEFAULT 'local',
    uri TEXT NOT NULL,
    extra_json TEXT,
    created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP
);

-- 9. Music Likes
CREATE TABLE IF NOT EXISTS music_likes (
    track_id TEXT PRIMARY KEY,
    liked_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
    FOREIGN KEY (track_id) REFERENCES music_tracks(id) ON DELETE CASCADE
);

-- 10. Music Skips
CREATE TABLE IF NOT EXISTS music_skips (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    track_id TEXT NOT NULL,
    skipped_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP
);

-- 11. Music Playback History
CREATE TABLE IF NOT EXISTS music_history (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    track_id TEXT NOT NULL,
    played_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
    playback_duration_secs INTEGER NOT NULL DEFAULT 0,
    completed BOOLEAN NOT NULL DEFAULT 0
);

CREATE INDEX IF NOT EXISTS idx_music_history_played_at ON music_history(played_at);

-- Schema version stamp
PRAGMA user_version = 1;
