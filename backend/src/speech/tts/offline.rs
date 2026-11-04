use crate::common::AppResult;
use crate::speech::models::TtsVoice;
use crate::speech::traits::TtsProvider;
use std::f32::consts::PI;

/// Offline Fallback TTS Provider.
///
/// Pure Rust vocal/formant speech synthesizer that requires zero external binaries
/// and zero internet connection. Always available as an offline fail-safe.
pub struct OfflineFallbackTtsProvider;

impl Default for OfflineFallbackTtsProvider {
    fn default() -> Self {
        Self::new()
    }
}

impl OfflineFallbackTtsProvider {
    pub fn new() -> Self {
        Self
    }

    /// Generate WAV header and PCM samples for synthesized audio
    pub fn create_wav(samples: &[i16], sample_rate: u32) -> Vec<u8> {
        let num_channels = 1u16;
        let bits_per_sample = 16u16;
        let byte_rate = sample_rate * (num_channels as u32) * (bits_per_sample as u32 / 8);
        let block_align = num_channels * (bits_per_sample / 8);
        let data_len = (samples.len() * 2) as u32;
        let file_len = 36 + data_len;

        let mut wav = Vec::with_capacity(44 + samples.len() * 2);

        // RIFF header
        wav.extend_from_slice(b"RIFF");
        wav.extend_from_slice(&file_len.to_le_bytes());
        wav.extend_from_slice(b"WAVE");

        // fmt chunk
        wav.extend_from_slice(b"fmt ");
        wav.extend_from_slice(&16u32.to_le_bytes());
        wav.extend_from_slice(&1u16.to_le_bytes()); // PCM
        wav.extend_from_slice(&num_channels.to_le_bytes());
        wav.extend_from_slice(&sample_rate.to_le_bytes());
        wav.extend_from_slice(&byte_rate.to_le_bytes());
        wav.extend_from_slice(&block_align.to_le_bytes());
        wav.extend_from_slice(&bits_per_sample.to_le_bytes());

        // data chunk
        wav.extend_from_slice(b"data");
        wav.extend_from_slice(&data_len.to_le_bytes());
        for &s in samples {
            wav.extend_from_slice(&s.to_le_bytes());
        }

        wav
    }
}

impl TtsProvider for OfflineFallbackTtsProvider {
    fn name(&self) -> &str {
        "offline_fallback"
    }

    fn is_available(&self) -> bool {
        true
    }

    fn available_voices(&self) -> Vec<TtsVoice> {
        vec![
            TtsVoice {
                id: "offline_assistant".to_string(),
                name: "Offline Speech Synthesizer".to_string(),
                language: "ru".to_string(),
                provider: "offline_fallback".to_string(),
                gender: Some("neutral".to_string()),
            },
            TtsVoice {
                id: "offline_melodic".to_string(),
                name: "Offline Melodic Chime".to_string(),
                language: "ru".to_string(),
                provider: "offline_fallback".to_string(),
                gender: Some("synthetic".to_string()),
            },
            TtsVoice {
                id: "offline_en".to_string(),
                name: "Offline English Voice".to_string(),
                language: "en".to_string(),
                provider: "offline_fallback".to_string(),
                gender: Some("neutral".to_string()),
            },
        ]
    }

    async fn synthesize(&self, text: &str, _voice: Option<&str>, speed: f32) -> AppResult<Vec<u8>> {
        let sample_rate = 22050u32;
        let speed_factor = speed.clamp(0.5, 2.0);
        let char_duration_secs = (0.075 / speed_factor).clamp(0.03, 0.2);

        let clean_text = text.trim();
        let chars: Vec<char> = clean_text.chars().collect();
        let total_chars = chars.len().max(1);

        let mut samples: Vec<i16> = Vec::new();
        let mut base_pitch = 220.0f32; // A3 base pitch

        for (idx, &c) in chars.iter().enumerate() {
            let is_vowel = matches!(
                c.to_ascii_lowercase(),
                'a' | 'e'
                    | 'i'
                    | 'o'
                    | 'u'
                    | 'y'
                    | 'а'
                    | 'е'
                    | 'ё'
                    | 'и'
                    | 'о'
                    | 'у'
                    | 'ы'
                    | 'э'
                    | 'ю'
                    | 'я'
            );

            let is_space = c.is_whitespace();
            let is_punct = matches!(c, '.' | ',' | '!' | '?' | ':' | ';');

            // Syllable duration and frequencies
            let duration = if is_space {
                char_duration_secs * 0.8
            } else if is_punct {
                char_duration_secs * 2.0
            } else if is_vowel {
                char_duration_secs * 1.3
            } else {
                char_duration_secs * 0.9
            };

            let num_samples = (sample_rate as f32 * duration) as usize;

            // Intonation curve: slight pitch variation based on position and punctuation
            let progress = idx as f32 / total_chars as f32;
            let intonation = if clean_text.ends_with('?') {
                base_pitch * (1.0 + progress * 0.35)
            } else if clean_text.ends_with('!') {
                base_pitch * 1.15
            } else {
                base_pitch * (1.0 - progress * 0.15)
            };

            // Formants: F1 and F2 simulated frequencies for phonetic distinctness
            let (f1, f2) = if is_vowel {
                (intonation, intonation * 2.5)
            } else if is_space || is_punct {
                (0.0, 0.0)
            } else {
                (intonation * 0.8, intonation * 3.2)
            };

            for s_idx in 0..num_samples {
                if is_space || is_punct {
                    samples.push(0);
                    continue;
                }

                let t = s_idx as f32 / sample_rate as f32;
                // Attack-decay envelope to prevent clicking
                let env = (-(s_idx as f32 / num_samples as f32 - 0.5).powi(2) * 8.0).exp();

                let wave1 = (2.0 * PI * f1 * t).sin();
                let wave2 = (2.0 * PI * f2 * t).sin() * 0.4;
                let sample_f = (wave1 + wave2) * env * 0.6 * i16::MAX as f32;
                samples.push(sample_f.clamp(i16::MIN as f32, i16::MAX as f32) as i16);
            }

            // Subtle pitch drift between phonemes
            base_pitch = (base_pitch + ((c as u32 % 20) as f32 - 10.0)).clamp(180.0, 320.0);
        }

        Ok(Self::create_wav(&samples, sample_rate))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_offline_tts_generates_valid_wav() {
        let provider = OfflineFallbackTtsProvider::new();
        assert!(provider.is_available());

        let wav = provider
            .synthesize("Привет, мир! Как дела?", None, 1.0)
            .await
            .expect("Offline synthesis failed");

        assert!(wav.len() > 44);
        assert_eq!(&wav[0..4], b"RIFF");
        assert_eq!(&wav[8..12], b"WAVE");
        assert_eq!(&wav[12..16], b"fmt ");
        assert_eq!(&wav[36..40], b"data");
    }
}
