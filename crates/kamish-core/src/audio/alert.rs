//! Offline alert tone generator and playback.
//!
//! Generates synthetic alert tones in WAV format and plays them through
//! available system audio backends (aplay, pw-play, paplay, mpv) with fallback
//! to headless-safe logging when no audio hardware is present.

use std::f32::consts::PI;
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};
use tracing::{debug, warn};

/// Global flag to disable audio output during tests or when requested
static MUTE_AUDIO: AtomicBool = AtomicBool::new(false);

pub fn set_audio_muted(muted: bool) {
    MUTE_AUDIO.store(muted, Ordering::Relaxed);
}

pub fn is_audio_muted() -> bool {
    // If explicitly muted via flag
    if MUTE_AUDIO.load(Ordering::Relaxed) {
        return true;
    }

    // Check environment variable
    if let Ok(val) = std::env::var("KAMISH_MUTE_AUDIO") {
        if val == "1" || val.eq_ignore_ascii_case("true") {
            return true;
        }
    }

    // Detect if running inside cargo test runner
    if let Ok(exe) = std::env::current_exe() {
        let exe_str = exe.to_string_lossy();
        if exe_str.contains("deps/") || exe_str.contains("test") {
            return true;
        }
    }

    false
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AlertType {
    Alarm,
    Reminder,
}

/// Synthesize a WAV tone in memory.
///
/// Uses 16-bit PCM, 44100 Hz, mono.
/// Alarm: energetic two-tone chime (587.33 Hz [D5] -> 880.00 Hz [A5])
/// Reminder: gentle mellow chime (523.25 Hz [C5] -> 659.25 Hz [E5])
pub fn generate_tone_wav(alert_type: AlertType) -> Vec<u8> {
    let sample_rate = 44100u32;
    let duration_secs = match alert_type {
        AlertType::Alarm => 1.5f32,
        AlertType::Reminder => 1.0f32,
    };
    let total_samples = (sample_rate as f32 * duration_secs) as usize;

    let (freq1, freq2) = match alert_type {
        AlertType::Alarm => (587.33f32, 880.00f32),
        AlertType::Reminder => (523.25f32, 659.25f32),
    };

    let mut samples: Vec<i16> = Vec::with_capacity(total_samples);

    for i in 0..total_samples {
        let t = i as f32 / sample_rate as f32;
        // Two halves of duration
        let half = duration_secs / 2.0;
        let (freq, phase_t) = if t < half {
            (freq1, t)
        } else {
            (freq2, t - half)
        };

        // Smooth decay envelope
        let env = (-4.0 * (phase_t / half)).exp();
        let wave = (2.0 * PI * freq * t).sin();
        let sample = (wave * env * 0.75 * i16::MAX as f32) as i16;
        samples.push(sample);
    }

    let data_len = (samples.len() * 2) as u32;
    let file_len = 36 + data_len;

    let mut wav = Vec::with_capacity(44 + samples.len() * 2);

    // RIFF Header
    wav.extend_from_slice(b"RIFF");
    wav.extend_from_slice(&file_len.to_le_bytes());
    wav.extend_from_slice(b"WAVE");

    // "fmt " Subchunk
    wav.extend_from_slice(b"fmt ");
    wav.extend_from_slice(&16u32.to_le_bytes()); // Subchunk1Size (16 for PCM)
    wav.extend_from_slice(&1u16.to_le_bytes()); // AudioFormat (1 = PCM)
    wav.extend_from_slice(&1u16.to_le_bytes()); // NumChannels (1 = mono)
    wav.extend_from_slice(&sample_rate.to_le_bytes()); // SampleRate
    let byte_rate = sample_rate * 2;
    wav.extend_from_slice(&byte_rate.to_le_bytes()); // ByteRate
    wav.extend_from_slice(&2u16.to_le_bytes()); // BlockAlign (NumChannels * BitsPerSample/8)
    wav.extend_from_slice(&16u16.to_le_bytes()); // BitsPerSample

    // "data" Subchunk
    wav.extend_from_slice(b"data");
    wav.extend_from_slice(&data_len.to_le_bytes());

    // Samples
    for sample in samples {
        wav.extend_from_slice(&sample.to_le_bytes());
    }

    wav
}

/// Plays an alert tone asynchronously and safely.
/// If audio hardware is unavailable or headless, it logs the event cleanly without panicking.
pub async fn play_alert_tone(alert_type: AlertType) {
    if is_audio_muted() {
        debug!("Audio alert suppressed: muted");
        return;
    }

    tokio::task::spawn_blocking(move || {
        let tone_name = match alert_type {
            AlertType::Alarm => "alarm_chime.wav",
            AlertType::Reminder => "reminder_chime.wav",
        };

        let temp_dir = std::env::temp_dir();
        let temp_path: PathBuf = temp_dir.join(tone_name);

        let wav_data = generate_tone_wav(alert_type);
        if let Err(e) = std::fs::write(&temp_path, &wav_data) {
            warn!("Failed to write alert tone to temporary file: {}", e);
            return;
        }

        // Try players in order of preference for embedded Linux / desktop
        let mut aplay_args = vec!["-q".to_string()];
        if let Some(dev) = super::devices::selected_output() {
            aplay_args.push("-D".to_string());
            aplay_args.push(dev);
        }
        aplay_args.push(temp_path.to_str().unwrap().to_string());
        let aplay_args: Vec<&str> = aplay_args.iter().map(String::as_str).collect();
        let players = [
            ("aplay", aplay_args),
            ("pw-play", vec![temp_path.to_str().unwrap()]),
            ("paplay", vec![temp_path.to_str().unwrap()]),
            (
                "mpv",
                vec![
                    "--no-terminal",
                    "--really-quiet",
                    temp_path.to_str().unwrap(),
                ],
            ),
        ];

        let mut played = false;
        for (cmd, args) in players {
            match std::process::Command::new(cmd).args(&args).status() {
                Ok(status) => {
                    if status.success() {
                        debug!("Alert tone played successfully using {}", cmd);
                        played = true;
                        break;
                    } else {
                        debug!("Player {} exited with status {}", cmd, status);
                    }
                }
                Err(err) => {
                    debug!("Player {} not available: {}", cmd, err);
                }
            }
        }

        if !played {
            debug!("Headless mode or no audio device found: alert tone simulation completed");
        }
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_generate_tone_wav() {
        let alarm_wav = generate_tone_wav(AlertType::Alarm);
        assert!(alarm_wav.len() > 44);
        assert_eq!(&alarm_wav[0..4], b"RIFF");
        assert_eq!(&alarm_wav[8..12], b"WAVE");

        let reminder_wav = generate_tone_wav(AlertType::Reminder);
        assert!(reminder_wav.len() > 44);
        assert_eq!(&reminder_wav[0..4], b"RIFF");
    }

    #[tokio::test]
    async fn test_play_alert_tone_does_not_crash() {
        set_audio_muted(true);
        play_alert_tone(AlertType::Alarm).await;
        play_alert_tone(AlertType::Reminder).await;
    }
}
