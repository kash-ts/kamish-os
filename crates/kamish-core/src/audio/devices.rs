//! ALSA device enumeration and runtime selection of the active output / input device.
//!
//! Devices are discovered with `aplay -l` (playback) and `arecord -l` (capture). When the
//! utilities are not installed (dev hosts) the lists are empty — no devices are invented.

use serde::{Deserialize, Serialize};
use std::sync::Mutex;

/// Identifier meaning "let ALSA pick the default device".
pub const DEFAULT_DEVICE_ID: &str = "default";

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AudioDevice {
    /// ALSA device string usable with `-D`, e.g. `plughw:1,0`.
    pub id: String,
    /// Human-readable name.
    pub name: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AudioDevicesResponse {
    pub outputs: Vec<AudioDevice>,
    pub inputs: Vec<AudioDevice>,
    pub selected_output: String,
    pub selected_input: String,
}

static SELECTED_OUTPUT: Mutex<Option<String>> = Mutex::new(None);
static SELECTED_INPUT: Mutex<Option<String>> = Mutex::new(None);

/// Currently selected ALSA output device (`None` = system default).
pub fn selected_output() -> Option<String> {
    SELECTED_OUTPUT.lock().ok().and_then(|g| g.clone())
}

/// Currently selected ALSA capture device (`None` = environment / default).
pub fn selected_input() -> Option<String> {
    SELECTED_INPUT.lock().ok().and_then(|g| g.clone())
}

/// Select the active device. `kind` is `"output"` or `"input"`.
/// `default` clears the override. Returns `false` for an unknown kind.
pub fn set_selected(kind: &str, id: &str) -> bool {
    let value = if id.is_empty() || id == DEFAULT_DEVICE_ID {
        None
    } else {
        Some(id.to_string())
    };
    let slot = match kind {
        "output" => &SELECTED_OUTPUT,
        "input" => &SELECTED_INPUT,
        _ => return false,
    };
    if let Ok(mut guard) = slot.lock() {
        *guard = value;
    }
    true
}

/// Parse the output of `aplay -l` / `arecord -l` into devices.
///
/// Line format: `card 1: Device [USB Audio Device], device 0: USB Audio [USB Audio]`
pub fn parse_alsa_list(text: &str) -> Vec<AudioDevice> {
    let mut devices = Vec::new();
    for line in text.lines() {
        let line = line.trim();
        let Some(rest) = line.strip_prefix("card ") else {
            continue;
        };
        let Some((card_num, after_card)) = rest.split_once(':') else {
            continue;
        };
        let Some((_, after_dev)) = after_card.split_once(", device ") else {
            continue;
        };
        let Some((dev_num, dev_rest)) = after_dev.split_once(':') else {
            continue;
        };
        let card_name = bracket_name(after_card).unwrap_or_else(|| after_card.trim().to_string());
        let dev_name = bracket_name(dev_rest).unwrap_or_else(|| dev_rest.trim().to_string());
        let (card_num, dev_num) = (card_num.trim(), dev_num.trim());
        if card_num.parse::<u32>().is_err() || dev_num.parse::<u32>().is_err() {
            continue;
        }
        let name = if card_name == dev_name {
            card_name
        } else {
            format!("{card_name} ({dev_name})")
        };
        devices.push(AudioDevice {
            id: format!("plughw:{card_num},{dev_num}"),
            name,
        });
    }
    devices
}

fn bracket_name(s: &str) -> Option<String> {
    let start = s.find('[')?;
    let end = s[start..].find(']')? + start;
    Some(s[start + 1..end].to_string())
}

async fn run_list(bin: &str) -> Vec<AudioDevice> {
    match tokio::process::Command::new(bin).arg("-l").output().await {
        Ok(out) if out.status.success() => parse_alsa_list(&String::from_utf8_lossy(&out.stdout)),
        _ => Vec::new(),
    }
}

/// Enumerate playback and capture devices together with the current selection.
pub async fn list_devices() -> AudioDevicesResponse {
    AudioDevicesResponse {
        outputs: run_list("aplay").await,
        inputs: run_list("arecord").await,
        selected_output: selected_output().unwrap_or_else(|| DEFAULT_DEVICE_ID.to_string()),
        selected_input: selected_input().unwrap_or_else(|| DEFAULT_DEVICE_ID.to_string()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_aplay_list() {
        let text = "**** List of PLAYBACK Hardware Devices ****\n\
card 0: audiocodec [audiocodec], device 0: 2032000.codec-sun8i [2032000.codec-sun8i]\n  Subdevices: 1/1\n\
card 1: Device [USB Audio Device], device 0: USB Audio [USB Audio]\n";
        let devs = parse_alsa_list(text);
        assert_eq!(devs.len(), 2);
        assert_eq!(devs[0].id, "plughw:0,0");
        assert_eq!(devs[1].id, "plughw:1,0");
        assert_eq!(devs[1].name, "USB Audio Device (USB Audio)");
    }

    #[test]
    fn selection_roundtrip() {
        assert!(set_selected("output", "plughw:1,0"));
        assert_eq!(selected_output().as_deref(), Some("plughw:1,0"));
        assert!(set_selected("output", "default"));
        assert_eq!(selected_output(), None);
        assert!(!set_selected("bogus", "x"));
    }
}
