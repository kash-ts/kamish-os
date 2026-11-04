use super::traits::VadDetector;

/// Energy and Zero-Crossing Rate (ZCR) based Voice Activity Detector in pure Rust.
///
/// Combines RMS energy thresholding, zero-crossing frequency metrics,
/// adaptive noise floor tracking, and a speech hangover filter to avoid clipping
/// during pauses between words or syllables.
#[derive(Debug, Clone)]
pub struct EnergyVadDetector {
    /// Energy threshold above noise floor to qualify as speech
    pub energy_threshold: f32,
    /// Minimum zero-crossing rate for speech candidate
    pub min_zcr: f32,
    /// Maximum zero-crossing rate (higher usually indicates high-frequency hiss/noise)
    pub max_zcr: f32,
    /// Dynamic background noise floor estimate
    pub noise_floor: f32,
    /// Noise floor smoothing factor (0.0 - 1.0)
    pub alpha_noise: f32,
    /// Hangover frames remaining to maintain active state across short pauses
    hangover_frames: usize,
    /// Maximum hangover frame count (e.g. 8 frames ~ 160-240ms)
    pub max_hangover: usize,
}

impl Default for EnergyVadDetector {
    fn default() -> Self {
        Self::new()
    }
}

impl EnergyVadDetector {
    pub fn new() -> Self {
        Self {
            energy_threshold: 0.012, // normalized RMS energy threshold (~ -38 dB)
            min_zcr: 0.015,          // minimum zero-crossing rate
            max_zcr: 0.70,           // maximum zero-crossing rate
            noise_floor: 0.003,      // initial noise floor estimate
            alpha_noise: 0.95,       // slow adaptation to ambient background
            hangover_frames: 0,
            max_hangover: 8, // 8 frames hangover smoothing
        }
    }

    /// Compute root-mean-square (RMS) energy normalized to [0.0, 1.0]
    pub fn calculate_rms(frame: &[i16]) -> f32 {
        if frame.is_empty() {
            return 0.0;
        }

        let sum_sq: f64 = frame
            .iter()
            .map(|&s| {
                let norm = s as f64 / 32768.0;
                norm * norm
            })
            .sum();

        ((sum_sq / frame.len() as f64) as f32).sqrt()
    }

    /// Compute zero-crossing rate normalized to [0.0, 1.0]
    pub fn calculate_zcr(frame: &[i16]) -> f32 {
        if frame.len() < 2 {
            return 0.0;
        }

        let mut crossings = 0;
        for i in 1..frame.len() {
            let prev = frame[i - 1];
            let curr = frame[i];
            if (prev >= 0 && curr < 0) || (prev < 0 && curr >= 0) {
                crossings += 1;
            }
        }

        crossings as f32 / (frame.len() - 1) as f32
    }
}

impl VadDetector for EnergyVadDetector {
    fn is_speech(&mut self, pcm_frame: &[i16]) -> bool {
        if pcm_frame.is_empty() {
            return false;
        }

        let rms = Self::calculate_rms(pcm_frame);
        let zcr = Self::calculate_zcr(pcm_frame);

        // Check if current frame exceeds threshold
        let energy_active = rms > (self.noise_floor + self.energy_threshold);
        let zcr_valid = zcr >= self.min_zcr && zcr <= self.max_zcr;

        let frame_is_speech = energy_active && zcr_valid;

        if frame_is_speech {
            self.hangover_frames = self.max_hangover;
            true
        } else {
            // Adaptive noise floor update on quiet frames
            if rms < self.noise_floor * 2.0 {
                self.noise_floor =
                    self.alpha_noise * self.noise_floor + (1.0 - self.alpha_noise) * rms;
            }

            if self.hangover_frames > 0 {
                self.hangover_frames -= 1;
                true
            } else {
                false
            }
        }
    }

    fn reset(&mut self) {
        self.hangover_frames = 0;
        self.noise_floor = 0.003;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_vad_silence_detection() {
        let mut vad = EnergyVadDetector::new();
        let silence_frame = vec![0i16; 320];

        // Should return false for silence
        assert!(!vad.is_speech(&silence_frame));
        assert!(!vad.is_speech(&silence_frame));
    }

    #[test]
    fn test_vad_speech_detection_and_hangover() {
        let mut vad = EnergyVadDetector::new();

        // Generate synthetic speech-like tone (voiced vowel 250 Hz, 16kHz sampling)
        let sample_rate = 16000.0;
        let freq = 250.0;
        let frame_len = 320; // 20 ms
        let mut speech_frame = Vec::with_capacity(frame_len);

        for i in 0..frame_len {
            let t = i as f32 / sample_rate;
            let val = (2.0 * std::f32::consts::PI * freq * t).sin();
            speech_frame.push((val * 16000.0) as i16);
        }

        // Must detect speech
        assert!(vad.is_speech(&speech_frame));

        // When silence follows, hangover filter holds speech true for a few frames
        let silence_frame = vec![0i16; frame_len];
        assert!(vad.is_speech(&silence_frame)); // hangover 1
        assert!(vad.is_speech(&silence_frame)); // hangover 2

        // Reset clears hangover
        vad.reset();
        assert!(!vad.is_speech(&silence_frame));
    }
}
