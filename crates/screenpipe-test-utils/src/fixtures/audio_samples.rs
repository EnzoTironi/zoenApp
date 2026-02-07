//! Embedded audio sample fixtures
//!
//! These audio samples are embedded at compile time using `include_bytes!`
//! for zero-overhead access in tests.

/// 440Hz sine wave, 5 seconds, 16kHz sample rate, mono, f32 little-endian
///
/// This is a pure 440Hz tone (A4 note) useful for testing:
/// - Frequency detection algorithms
/// - Audio processing pipelines
/// - Volume/gain calculations
///
/// # Specifications
/// - Duration: 5 seconds
/// - Sample rate: 16000 Hz
/// - Channels: 1 (mono)
/// - Format: 32-bit float, little-endian
/// - Total samples: 80000
/// - File size: 320,000 bytes
///
/// # Example
///
/// ```rust
/// use screenpipe_test_utils::fixtures::audio_samples::SINE_440HZ_5S;
/// use screenpipe_test_utils::mocks::MockAudioDevice;
///
/// let device = MockAudioDevice::new("test")
///     .with_sample_data_bytes(SINE_440HZ_5S)
///     .with_sample_rate(16000);
/// ```
pub const SINE_440HZ_5S: &[u8] = include_bytes!("../../fixtures/audio/sine_440hz_5s.raw");

/// Simulated speech pattern, 3 seconds, 16kHz sample rate, mono, f32 little-endian
///
/// This fixture contains a synthesized speech-like pattern with:
/// - Variable amplitude envelope (simulating syllables)
/// - Frequency modulation (simulating formants)
/// - Brief pauses between "words"
///
/// Useful for testing:
/// - Voice Activity Detection (VAD)
/// - Speech recognition preprocessing
/// - Audio segmentation algorithms
///
/// # Specifications
/// - Duration: 3 seconds
/// - Sample rate: 16000 Hz
/// - Channels: 1 (mono)
/// - Format: 32-bit float, little-endian
/// - Total samples: 48000
/// - File size: 192,000 bytes
///
/// # Example
///
/// ```rust
/// use screenpipe_test_utils::fixtures::audio_samples::SPEECH_SAMPLE;
///
/// // Load as f32 samples
/// let samples: Vec<f32> = SPEECH_SAMPLE
///     .chunks_exact(4)
///     .map(|chunk| f32::from_le_bytes(chunk.try_into().unwrap()))
///     .collect();
/// ```
pub const SPEECH_SAMPLE: &[u8] = include_bytes!("../../fixtures/audio/speech_sample.raw");

/// Helper functions for working with audio fixtures
pub mod helpers {
    /// Converts raw bytes to f32 samples
    ///
    /// # Example
    ///
    /// ```rust
    /// use screenpipe_test_utils::fixtures::audio_samples::{SINE_440HZ_5S, helpers};
    ///
    /// let samples = helpers::bytes_to_f32_samples(SINE_440HZ_5S);
    /// assert_eq!(samples.len(), SINE_440HZ_5S.len() / 4);
    /// ```
    pub fn bytes_to_f32_samples(bytes: &[u8]) -> Vec<f32> {
        bytes
            .chunks_exact(4)
            .map(|chunk| {
                let bytes: [u8; 4] = chunk.try_into().unwrap();
                f32::from_le_bytes(bytes)
            })
            .collect()
    }

    /// Converts f32 samples to raw bytes
    pub fn f32_samples_to_bytes(samples: &[f32]) -> Vec<u8> {
        samples.iter().flat_map(|s| s.to_le_bytes()).collect()
    }

    /// Calculates the RMS (Root Mean Square) of audio samples
    ///
    /// This is a measure of the signal's power/amplitude.
    pub fn calculate_rms(samples: &[f32]) -> f32 {
        if samples.is_empty() {
            return 0.0;
        }

        let sum_squares: f32 = samples.iter().map(|s| s * s).sum();
        (sum_squares / samples.len() as f32).sqrt()
    }

    /// Calculates the peak amplitude of audio samples
    pub fn calculate_peak(samples: &[f32]) -> f32 {
        samples.iter().map(|s| s.abs()).fold(0.0, f32::max)
    }

    /// Detects if audio contains silence (below threshold)
    pub fn is_silence(samples: &[f32], threshold_db: f32) -> bool {
        let rms = calculate_rms(samples);
        let rms_db = 20.0 * rms.log10();
        rms_db < threshold_db
    }

    /// Generates a test tone at the specified frequency
    ///
    /// # Arguments
    /// * `frequency` - Frequency in Hz
    /// * `duration_secs` - Duration in seconds
    /// * `sample_rate` - Sample rate in Hz
    pub fn generate_tone(frequency: f32, duration_secs: f32, sample_rate: u32) -> Vec<f32> {
        let num_samples = (duration_secs * sample_rate as f32) as usize;
        let mut samples = Vec::with_capacity(num_samples);

        for i in 0..num_samples {
            let t = i as f32 / sample_rate as f32;
            let sample = (2.0 * std::f32::consts::PI * frequency * t).sin();
            samples.push(sample);
        }

        samples
    }

    /// Applies a simple fade in/out to samples
    pub fn apply_fade(samples: &mut [f32], fade_samples: usize) {
        let fade_len = fade_samples.min(samples.len() / 2);

        // Fade in
        for i in 0..fade_len {
            let factor = i as f32 / fade_len as f32;
            samples[i] *= factor;
        }

        // Fade out
        let len = samples.len();
        for i in 0..fade_len {
            let factor = i as f32 / fade_len as f32;
            samples[len - 1 - i] *= factor;
        }
    }

    /// Mixes two audio signals together
    pub fn mix(a: &[f32], b: &[f32]) -> Vec<f32> {
        let len = a.len().max(b.len());
        let mut result = vec![0.0; len];

        for i in 0..a.len() {
            result[i] += a[i];
        }
        for i in 0..b.len() {
            result[i] += b[i];
        }

        // Normalize to prevent clipping
        let peak = result.iter().map(|s| s.abs()).fold(0.0, f32::max);
        if peak > 1.0 {
            for sample in &mut result {
                *sample /= peak;
            }
        }

        result
    }
}

#[cfg(test)]
mod tests {
    use super::helpers::*;
    use super::*;

    #[test]
    fn test_sine_fixture_exists() {
        // The fixture should be embedded
        assert!(!SINE_440HZ_5S.is_empty());
        // Should be 5 seconds * 16000 samples/second * 4 bytes/sample
        assert_eq!(SINE_440HZ_5S.len(), 5 * 16000 * 4);
    }

    #[test]
    fn test_speech_fixture_exists() {
        assert!(!SPEECH_SAMPLE.is_empty());
        // Should be 3 seconds * 16000 samples/second * 4 bytes/sample
        assert_eq!(SPEECH_SAMPLE.len(), 3 * 16000 * 4);
    }

    #[test]
    fn test_bytes_to_f32_samples() {
        let samples = bytes_to_f32_samples(SINE_440HZ_5S);
        assert_eq!(samples.len(), SINE_440HZ_5S.len() / 4);

        // All samples should be in valid range
        for sample in &samples {
            assert!(sample.abs() <= 1.0);
        }
    }

    #[test]
    fn test_f32_samples_to_bytes() {
        let samples = vec![0.5f32, -0.5, 1.0, -1.0];
        let bytes = f32_samples_to_bytes(&samples);
        assert_eq!(bytes.len(), samples.len() * 4);
    }

    #[test]
    fn test_calculate_rms() {
        // RMS of a sine wave should be approximately 0.707 (1/sqrt(2))
        let samples = generate_tone(440.0, 0.1, 16000);
        let rms = calculate_rms(&samples);
        assert!(rms > 0.6 && rms < 0.8);

        // RMS of silence should be 0
        let silence = vec![0.0f32; 1000];
        assert_eq!(calculate_rms(&silence), 0.0);
    }

    #[test]
    fn test_calculate_peak() {
        let samples = vec![0.5f32, -0.8, 0.3, -0.2];
        assert_eq!(calculate_peak(&samples), 0.8);
    }

    #[test]
    fn test_is_silence() {
        let silence = vec![0.0f32; 1000];
        assert!(is_silence(&silence, -60.0));

        let tone = generate_tone(440.0, 0.1, 16000);
        assert!(!is_silence(&tone, -60.0));
    }

    #[test]
    fn test_generate_tone() {
        let tone = generate_tone(440.0, 1.0, 16000);
        assert_eq!(tone.len(), 16000);

        // All samples should be in valid range
        for sample in &tone {
            assert!(sample.abs() <= 1.0);
        }
    }

    #[test]
    fn test_apply_fade() {
        let mut samples = vec![1.0f32; 100];
        apply_fade(&mut samples, 10);

        // First sample should be near 0
        assert!(samples[0] < 0.5);
        // Last sample should be near 0
        assert!(samples[99] < 0.5);
        // Middle samples should be near 1.0
        assert!(samples[50] > 0.9);
    }

    #[test]
    fn test_mix() {
        let a = vec![0.5f32; 100];
        let b = vec![0.5f32; 100];
        let mixed = mix(&a, &b);

        // Mixed signal should be normalized
        assert!(mixed.iter().all(|s| s.abs() <= 1.0));
    }
}
