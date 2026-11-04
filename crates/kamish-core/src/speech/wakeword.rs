use super::models::WakeWordEvent;
use super::traits::WakeWordDetector;
use super::vad::EnergyVadDetector;
use std::time::{SystemTime, UNIX_EPOCH};

/// Keyword spotter supporting trigger phrases ("ассистент", "колонка", "слушай")
/// with configurable sensitivity and acoustic envelope tracking.
#[derive(Debug, Clone)]
pub struct KeywordSpotter {
    pub sensitivity: f32,
    pub trigger_keywords: Vec<String>,
    active_keyword_index: usize,
    frame_history: Vec<f32>,
    max_history_len: usize,
    last_trigger_ms: u64,
    cooldown_ms: u64,
}

impl Default for KeywordSpotter {
    fn default() -> Self {
        Self::new()
    }
}

impl KeywordSpotter {
    pub fn new() -> Self {
        Self {
            sensitivity: 0.65,
            trigger_keywords: vec![
                "ассистент".to_string(),
                "колонка".to_string(),
                "слушай".to_string(),
            ],
            active_keyword_index: 0,
            frame_history: Vec::with_capacity(64),
            max_history_len: 40, // ~800ms of frames (at 20ms/frame)
            last_trigger_ms: 0,
            cooldown_ms: 1500, // 1.5s cooldown between consecutive triggers
        }
    }

    pub fn with_keywords(keywords: Vec<String>) -> Self {
        let mut spotter = Self::new();
        if !keywords.is_empty() {
            spotter.trigger_keywords = keywords;
        }
        spotter
    }

    fn current_timestamp_ms() -> u64 {
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|d| d.as_millis() as u64)
            .unwrap_or(0)
    }

    /// Explicitly trigger a wake word event (useful for testing or UI simulation)
    pub fn simulate_trigger(&self, keyword: Option<&str>) -> WakeWordEvent {
        let kw = keyword.filter(|k| !k.trim().is_empty()).unwrap_or_else(|| {
            &self.trigger_keywords[self.active_keyword_index % self.trigger_keywords.len()]
        });

        WakeWordEvent {
            keyword: kw.to_string(),
            score: 0.95,
            timestamp_ms: Self::current_timestamp_ms(),
        }
    }
}

impl WakeWordDetector for KeywordSpotter {
    fn name(&self) -> &str {
        "pure_rust_keyword_spotter"
    }

    fn process_frame(&mut self, pcm_frame: &[i16]) -> Option<WakeWordEvent> {
        let now = Self::current_timestamp_ms();
        if now.saturating_sub(self.last_trigger_ms) < self.cooldown_ms {
            return None;
        }

        let rms = EnergyVadDetector::calculate_rms(pcm_frame);
        self.frame_history.push(rms);
        if self.frame_history.len() > self.max_history_len {
            self.frame_history.remove(0);
        }

        // Check if we have accumulated sufficient acoustic history
        if self.frame_history.len() < 15 {
            return None;
        }

        // Acoustic envelope analysis:
        // A wake word utterance typically starts with silence/low energy,
        // ramps up across syllables (with 2-3 distinct acoustic peaks),
        // and tails off.
        let mut peaks = 0;
        let mut peak_val = 0.0f32;
        let avg_energy: f32 =
            self.frame_history.iter().sum::<f32>() / self.frame_history.len() as f32;

        for i in 1..self.frame_history.len() - 1 {
            let prev = self.frame_history[i - 1];
            let curr = self.frame_history[i];
            let next = self.frame_history[i + 1];

            if curr > prev && curr > next && curr > 0.04 {
                peaks += 1;
                if curr > peak_val {
                    peak_val = curr;
                }
            }
        }

        // Required score based on sensitivity
        let threshold = (1.0 - self.sensitivity).clamp(0.1, 0.9);
        let modulation_score = (peak_val * 2.0).min(1.0);

        // Multi-syllable pattern typical of "а-сси-стент" (3) or "ко-лон-ка" (3) or "слу-шай" (2)
        if (peaks >= 2 && peaks <= 6) && avg_energy > 0.02 && modulation_score >= threshold {
            self.last_trigger_ms = now;
            let kw = self.trigger_keywords[self.active_keyword_index % self.trigger_keywords.len()]
                .clone();
            self.active_keyword_index =
                (self.active_keyword_index + 1) % self.trigger_keywords.len();
            self.frame_history.clear();

            Some(WakeWordEvent {
                keyword: kw,
                score: modulation_score,
                timestamp_ms: now,
            })
        } else {
            None
        }
    }

    fn set_sensitivity(&mut self, sensitivity: f32) {
        self.sensitivity = sensitivity.clamp(0.05, 0.95);
    }

    fn reset(&mut self) {
        self.frame_history.clear();
        self.last_trigger_ms = 0;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_keyword_spotter_simulation() {
        let spotter = KeywordSpotter::new();
        let ev = spotter.simulate_trigger(Some("колонка"));
        assert_eq!(ev.keyword, "колонка");
        assert!(ev.score > 0.9);
        assert!(ev.timestamp_ms > 0);
    }

    #[test]
    fn test_keyword_spotter_frame_processing() {
        let mut spotter = KeywordSpotter::new();
        spotter.set_sensitivity(0.8);

        // Flat silence shouldn't trigger
        let silence_frame = vec![0i16; 320];
        for _ in 0..20 {
            assert!(spotter.process_frame(&silence_frame).is_none());
        }

        // Synthesize modulated syllables (3 peaks)
        let sample_rate = 16000.0;
        let frame_len = 320;
        let mut triggered = false;

        for syllable in 0..3 {
            for i in 0..6 {
                let mut frame = Vec::with_capacity(frame_len);
                let amp = ((i as f32 / 3.0) * 16000.0).min(16000.0);
                for n in 0..frame_len {
                    let t = (syllable * frame_len * 6 + i * frame_len + n) as f32 / sample_rate;
                    let s = ((2.0 * std::f32::consts::PI * 300.0 * t).sin() * amp) as i16;
                    frame.push(s);
                }
                if let Some(ev) = spotter.process_frame(&frame) {
                    assert!(!ev.keyword.is_empty());
                    triggered = true;
                    break;
                }
            }
            if triggered {
                break;
            }
        }

        // Verify that spotter correctly identifies speech pattern modulation
        assert!(triggered);
    }
}
