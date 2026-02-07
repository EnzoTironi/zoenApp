//! Mock audio capture implementations
//!
//! Provides mock audio devices and streams for testing audio processing
//! without requiring actual audio hardware.

use crate::mocks::{next_mock_id, ErrorSimulation, MockComponent, MockStats};
use async_trait::async_trait;
use screenpipe_audio::core::device::{AudioDevice, DeviceType};
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::broadcast;
use tracing::{debug, trace};

/// A mock audio device that returns pre-recorded or generated samples
///
/// # Example
///
/// ```rust
/// use screenpipe_test_utils::mocks::MockAudioDevice;
/// use screenpipe_test_utils::fixtures::audio_samples::SINE_440HZ_5S;
///
/// let device = MockAudioDevice::new("test-microphone")
///     .with_sample_data(SINE_440HZ_5S)
///     .with_sample_rate(16000);
///
/// assert_eq!(device.name(), "test-microphone");
/// assert_eq!(device.sample_rate(), 16000);
/// ```
#[derive(Clone)]
pub struct MockAudioDevice {
    id: u64,
    name: String,
    device_type: DeviceType,
    sample_data: Arc<Vec<f32>>,
    sample_rate: u32,
    channels: u16,
    is_running: Arc<AtomicBool>,
    stats: Arc<std::sync::Mutex<MockStats>>,
    error_simulation: ErrorSimulation,
    loop_samples: bool,
}

impl MockAudioDevice {
    /// Creates a new mock audio device with the given name
    pub fn new(name: impl Into<String>) -> Self {
        let name = name.into();
        Self {
            id: next_mock_id(),
            name: name.clone(),
            device_type: DeviceType::Input,
            sample_data: Arc::new(Vec::new()),
            sample_rate: 16000,
            channels: 1,
            is_running: Arc::new(AtomicBool::new(false)),
            stats: Arc::new(std::sync::Mutex::new(MockStats::new())),
            error_simulation: ErrorSimulation::None,
            loop_samples: true,
        }
    }

    /// Sets the sample data to be returned by this device
    pub fn with_sample_data(mut self, data: &[f32]) -> Self {
        self.sample_data = Arc::new(data.to_vec());
        self
    }

    /// Sets the sample data from raw bytes (interleaved f32 little-endian)
    pub fn with_sample_data_bytes(mut self, bytes: &[u8]) -> Self {
        let samples: Vec<f32> = bytes
            .chunks_exact(4)
            .map(|chunk| {
                let bytes: [u8; 4] = chunk.try_into().unwrap();
                f32::from_le_bytes(bytes)
            })
            .collect();
        self.sample_data = Arc::new(samples);
        self
    }

    /// Sets the sample rate
    pub fn with_sample_rate(mut self, rate: u32) -> Self {
        self.sample_rate = rate;
        self
    }

    /// Sets the number of channels
    pub fn with_channels(mut self, channels: u16) -> Self {
        self.channels = channels;
        self
    }

    /// Sets the device type (input/output)
    pub fn with_device_type(mut self, device_type: DeviceType) -> Self {
        self.device_type = device_type;
        self
    }

    /// Configures error simulation
    pub fn with_error_simulation(mut self, simulation: ErrorSimulation) -> Self {
        self.error_simulation = simulation;
        self
    }

    /// Sets whether to loop samples when reaching the end
    pub fn with_loop(mut self, loop_samples: bool) -> Self {
        self.loop_samples = loop_samples;
        self
    }

    /// Returns the device name
    pub fn name(&self) -> &str {
        &self.name
    }

    /// Returns the sample rate
    pub fn sample_rate(&self) -> u32 {
        self.sample_rate
    }

    /// Returns the number of channels
    pub fn channels(&self) -> u16 {
        self.channels
    }

    /// Returns the device type
    pub fn device_type(&self) -> &DeviceType {
        &self.device_type
    }

    /// Returns the sample data
    pub fn sample_data(&self) -> &[f32] {
        &self.sample_data
    }

    /// Starts the mock device and returns a stream
    pub async fn start(&self) -> anyhow::Result<MockAudioStream> {
        if self.is_running.load(Ordering::SeqCst) {
            return Err(anyhow::anyhow!("Device is already running"));
        }

        self.is_running.store(true, Ordering::SeqCst);
        self.stats.lock().unwrap().start();

        let (tx, _rx) = broadcast::channel::<Vec<f32>>(100);

        MockAudioStream::new(
            self.id,
            self.name.clone(),
            tx,
            self.sample_data.clone(),
            self.sample_rate,
            self.channels,
            self.is_running.clone(),
            self.loop_samples,
        )
    }

    /// Stops the mock device
    pub fn stop(&self) {
        self.is_running.store(false, Ordering::SeqCst);
        debug!("Mock audio device {} stopped", self.name);
    }

    /// Returns current statistics
    pub fn stats(&self) -> MockStats {
        self.stats.lock().unwrap().clone()
    }

    /// Converts to an AudioDevice struct
    pub fn to_audio_device(&self) -> AudioDevice {
        AudioDevice::new(self.name.clone(), self.device_type.clone())
    }
}

impl MockComponent for MockAudioDevice {
    fn mock_id(&self) -> u64 {
        self.id
    }

    fn is_initialized(&self) -> bool {
        !self.sample_data.is_empty()
    }

    fn reset(&mut self) {
        self.is_running.store(false, Ordering::SeqCst);
        self.stats = Arc::new(std::sync::Mutex::new(MockStats::new()));
    }
}

impl std::fmt::Debug for MockAudioDevice {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("MockAudioDevice")
            .field("id", &self.id)
            .field("name", &self.name)
            .field("sample_rate", &self.sample_rate)
            .field("channels", &self.channels)
            .field("is_running", &self.is_running.load(Ordering::SeqCst))
            .field("sample_count", &self.sample_data.len())
            .finish()
    }
}

/// A mock audio stream that broadcasts sample data
///
/// This simulates a real-time audio stream by broadcasting chunks of audio data
/// at appropriate intervals based on the sample rate.
#[derive(Clone)]
pub struct MockAudioStream {
    device_id: u64,
    device_name: String,
    transmitter: broadcast::Sender<Vec<f32>>,
    sample_data: Arc<Vec<f32>>,
    sample_rate: u32,
    channels: u16,
    position: Arc<AtomicU64>,
    is_running: Arc<AtomicBool>,
    loop_samples: bool,
}

impl MockAudioStream {
    fn new(
        device_id: u64,
        device_name: String,
        transmitter: broadcast::Sender<Vec<f32>>,
        sample_data: Arc<Vec<f32>>,
        sample_rate: u32,
        channels: u16,
        is_running: Arc<AtomicBool>,
        loop_samples: bool,
    ) -> anyhow::Result<Self> {
        let stream = Self {
            device_id,
            device_name,
            transmitter: transmitter.clone(),
            sample_data,
            sample_rate,
            channels,
            position: Arc::new(AtomicU64::new(0)),
            is_running,
            loop_samples,
        };

        // Start the streaming task
        stream.start_streaming_task(transmitter);

        Ok(stream)
    }

    fn start_streaming_task(&self, tx: broadcast::Sender<Vec<f32>>) {
        let sample_data = self.sample_data.clone();
        let sample_rate = self.sample_rate;
        let channels = self.channels;
        let position = self.position.clone();
        let is_running = self.is_running.clone();
        let loop_samples = self.loop_samples;

        // Calculate chunk size for ~100ms of audio
        let chunk_size = (sample_rate as usize / 10) * channels as usize;

        tokio::spawn(async move {
            let mut interval = tokio::time::interval(Duration::from_millis(100));

            while is_running.load(Ordering::SeqCst) {
                interval.tick().await;

                let pos = position.load(Ordering::SeqCst) as usize;

                if sample_data.is_empty() {
                    continue;
                }

                let chunk = if pos + chunk_size <= sample_data.len() {
                    sample_data[pos..pos + chunk_size].to_vec()
                } else if loop_samples {
                    // Wrap around
                    let remaining = sample_data.len() - pos;
                    let mut chunk = sample_data[pos..].to_vec();
                    let needed = chunk_size - remaining;
                    chunk.extend_from_slice(&sample_data[..needed.min(sample_data.len())]);
                    position.store(needed as u64, Ordering::SeqCst);
                    chunk
                } else {
                    // Send remaining samples and stop
                    let chunk = sample_data[pos..].to_vec();
                    is_running.store(false, Ordering::SeqCst);
                    chunk
                };

                if !chunk.is_empty() {
                    trace!("Sending audio chunk with {} samples", chunk.len());
                    let _ = tx.send(chunk);
                    position.fetch_add(chunk_size as u64, Ordering::SeqCst);
                }
            }
        });
    }

    /// Subscribe to the audio stream
    pub fn subscribe(&self) -> broadcast::Receiver<Vec<f32>> {
        self.transmitter.subscribe()
    }

    /// Returns the device name
    pub fn device_name(&self) -> &str {
        &self.device_name
    }

    /// Returns the sample rate
    pub fn sample_rate(&self) -> u32 {
        self.sample_rate
    }

    /// Returns the number of channels
    pub fn channels(&self) -> u16 {
        self.channels
    }

    /// Returns true if the stream is still running
    pub fn is_running(&self) -> bool {
        self.is_running.load(Ordering::SeqCst)
    }

    /// Stops the stream
    pub fn stop(&self) {
        self.is_running.store(false, Ordering::SeqCst);
    }

    /// Seeks to a specific position in the sample data
    pub fn seek(&self, position: u64) {
        self.position.store(position, Ordering::SeqCst);
    }

    /// Returns the current playback position
    pub fn position(&self) -> u64 {
        self.position.load(Ordering::SeqCst)
    }
}

impl MockComponent for MockAudioStream {
    fn mock_id(&self) -> u64 {
        self.device_id
    }

    fn is_initialized(&self) -> bool {
        true
    }

    fn reset(&mut self) {
        self.position.store(0, Ordering::SeqCst);
        self.is_running.store(false, Ordering::SeqCst);
    }
}

impl std::fmt::Debug for MockAudioStream {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("MockAudioStream")
            .field("device_id", &self.device_id)
            .field("device_name", &self.device_name)
            .field("sample_rate", &self.sample_rate)
            .field("channels", &self.channels)
            .field("is_running", &self.is_running())
            .field("position", &self.position())
            .finish()
    }
}

/// Generates a sine wave at the specified frequency
///
/// # Arguments
/// * `frequency` - The frequency of the sine wave in Hz
/// * `duration_secs` - The duration of the wave in seconds
/// * `sample_rate` - The sample rate in Hz
///
/// # Example
///
/// ```rust
/// use screenpipe_test_utils::mocks::audio::generate_sine_wave;
///
/// let samples = generate_sine_wave(440.0, 1.0, 16000);
/// assert_eq!(samples.len(), 16000);
/// ```
pub fn generate_sine_wave(frequency: f32, duration_secs: f32, sample_rate: u32) -> Vec<f32> {
    let num_samples = (duration_secs * sample_rate as f32) as usize;
    let mut samples = Vec::with_capacity(num_samples);

    for i in 0..num_samples {
        let t = i as f32 / sample_rate as f32;
        let sample = (2.0 * std::f32::consts::PI * frequency * t).sin();
        samples.push(sample);
    }

    samples
}

/// Generates silence (zeros)
///
/// # Arguments
/// * `duration_secs` - The duration of silence in seconds
/// * `sample_rate` - The sample rate in Hz
pub fn generate_silence(duration_secs: f32, sample_rate: u32) -> Vec<f32> {
    let num_samples = (duration_secs * sample_rate as f32) as usize;
    vec![0.0; num_samples]
}

/// Generates white noise
///
/// # Arguments
/// * `duration_secs` - The duration of noise in seconds
/// * `sample_rate` - The sample rate in Hz
/// * `amplitude` - The amplitude of the noise (0.0 to 1.0)
pub fn generate_white_noise(duration_secs: f32, sample_rate: u32, amplitude: f32) -> Vec<f32> {
    use rand::Rng;

    let num_samples = (duration_secs * sample_rate as f32) as usize;
    let mut rng = rand::thread_rng();
    let mut samples = Vec::with_capacity(num_samples);

    for _ in 0..num_samples {
        let noise: f32 = rng.gen_range(-1.0..1.0) * amplitude;
        samples.push(noise);
    }

    samples
}

/// Applies a simple fade in/out to audio samples
pub fn apply_fade(samples: &mut [f32], fade_samples: usize) {
    let len = samples.len();
    let fade_len = fade_samples.min(len / 2);

    // Fade in
    for i in 0..fade_len {
        let factor = i as f32 / fade_len as f32;
        samples[i] *= factor;
    }

    // Fade out
    for i in 0..fade_len {
        let factor = i as f32 / fade_len as f32;
        samples[len - 1 - i] *= factor;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_mock_audio_device_creation() {
        let device = MockAudioDevice::new("test-mic").with_sample_rate(44100);
        assert_eq!(device.name(), "test-mic");
        assert_eq!(device.sample_rate(), 44100);
        assert_eq!(device.channels(), 1);
    }

    #[test]
    fn test_mock_audio_device_with_data() {
        let samples = vec![0.1, 0.2, 0.3, 0.4];
        let device = MockAudioDevice::new("test")
            .with_sample_data(&samples)
            .with_sample_rate(16000);

        assert_eq!(device.sample_data().len(), 4);
        assert!(device.is_initialized());
    }

    #[tokio::test]
    async fn test_mock_audio_stream() {
        let samples = generate_sine_wave(440.0, 0.5, 16000);
        let device = MockAudioDevice::new("test")
            .with_sample_data(&samples)
            .with_sample_rate(16000);

        let stream = device.start().await.unwrap();
        let mut rx = stream.subscribe();

        // Wait a bit and receive some samples
        tokio::time::sleep(Duration::from_millis(150)).await;

        let received = rx.try_recv();
        assert!(received.is_ok());

        stream.stop();
    }

    #[test]
    fn test_generate_sine_wave() {
        let samples = generate_sine_wave(440.0, 1.0, 16000);
        assert_eq!(samples.len(), 16000);

        // Check that values are in valid range [-1, 1]
        for sample in &samples {
            assert!(sample.abs() <= 1.0);
        }
    }

    #[test]
    fn test_generate_silence() {
        let samples = generate_silence(1.0, 16000);
        assert_eq!(samples.len(), 16000);
        assert!(samples.iter().all(|&s| s == 0.0));
    }

    #[test]
    fn test_apply_fade() {
        let mut samples = vec![1.0; 100];
        apply_fade(&mut samples, 10);

        // First and last samples should be near 0
        assert!(samples[0] < 0.5);
        assert!(samples[99] < 0.5);

        // Middle samples should be near 1.0
        assert!(samples[50] > 0.9);
    }
}
