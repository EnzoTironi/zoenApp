//! Mock video capture implementations
//!
//! Provides mock video capture and frame generation for testing vision
//! processing without requiring actual screen capture hardware.

use crate::mocks::{next_mock_id, ErrorSimulation, MockComponent, MockStats};
use chrono::{DateTime, Utc};
use image::{DynamicImage, GenericImageView, ImageBuffer, Rgba};
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::mpsc;
use tracing::{debug, trace};

/// A mock video frame
///
/// Represents a captured frame with metadata similar to what would be
/// captured from a real screen capture.
#[derive(Clone, Debug)]
pub struct MockFrame {
    /// The frame image data
    pub image: DynamicImage,
    /// Frame sequence number
    pub frame_number: u64,
    /// Timestamp when the frame was "captured"
    pub timestamp: Instant,
    /// Wall-clock timestamp
    pub captured_at: DateTime<Utc>,
    /// Optional window name (for window captures)
    pub window_name: Option<String>,
    /// Optional application name
    pub app_name: Option<String>,
}

impl MockFrame {
    /// Creates a new mock frame from an image
    pub fn new(image: DynamicImage, frame_number: u64) -> Self {
        Self {
            image,
            frame_number,
            timestamp: Instant::now(),
            captured_at: Utc::now(),
            window_name: None,
            app_name: None,
        }
    }

    /// Creates a solid color frame
    pub fn solid_color(width: u32, height: u32, color: Rgba<u8>, frame_number: u64) -> Self {
        let image = ImageBuffer::from_pixel(width, height, color);
        Self::new(DynamicImage::ImageRgba8(image), frame_number)
    }

    /// Creates a gradient frame
    pub fn gradient(width: u32, height: u32, frame_number: u64) -> Self {
        let mut image = ImageBuffer::new(width, height);

        for (x, y, pixel) in image.enumerate_pixels_mut() {
            let r = ((x as f32 / width as f32) * 255.0) as u8;
            let g = ((y as f32 / height as f32) * 255.0) as u8;
            let b = 128;
            *pixel = Rgba([r, g, b, 255]);
        }

        Self::new(DynamicImage::ImageRgba8(image), frame_number)
    }

    /// Creates a test pattern frame (checkerboard)
    pub fn test_pattern(width: u32, height: u32, frame_number: u64) -> Self {
        let mut image = ImageBuffer::new(width, height);
        let checker_size = 32;

        for (x, y, pixel) in image.enumerate_pixels_mut() {
            let checker_x = x / checker_size;
            let checker_y = y / checker_size;
            let is_white = (checker_x + checker_y) % 2 == 0;

            if is_white {
                *pixel = Rgba([255, 255, 255, 255]);
            } else {
                *pixel = Rgba([0, 0, 0, 255]);
            }
        }

        Self::new(DynamicImage::ImageRgba8(image), frame_number)
    }

    /// Creates a frame with text rendered on it
    pub fn with_text(
        width: u32,
        height: u32,
        text: &str,
        frame_number: u64,
    ) -> anyhow::Result<Self> {
        // Create a white background
        let mut image = ImageBuffer::from_pixel(width, height, Rgba([255, 255, 255, 255]));

        // Simple text representation using pixel patterns
        // In a real implementation, you might use a text rendering library
        let text_color = Rgba([0, 0, 0, 255]);
        let start_x = 50;
        let start_y = height / 2;

        // Draw a simple line to represent text
        for (i, _) in text.chars().enumerate() {
            let x = start_x + (i as u32 * 10);
            if x < width - 10 {
                for dy in 0..20 {
                    for dx in 0..8 {
                        image.put_pixel(x + dx, start_y + dy, text_color);
                    }
                }
            }
        }

        Ok(Self::new(DynamicImage::ImageRgba8(image), frame_number))
    }

    /// Sets the window name
    pub fn with_window_name(mut self, name: impl Into<String>) -> Self {
        self.window_name = Some(name.into());
        self
    }

    /// Sets the application name
    pub fn with_app_name(mut self, name: impl Into<String>) -> Self {
        self.app_name = Some(name.into());
        self
    }

    /// Returns the dimensions of the frame
    pub fn dimensions(&self) -> (u32, u32) {
        self.image.dimensions()
    }

    /// Converts to bytes (RGBA)
    pub fn to_bytes(&self) -> Vec<u8> {
        self.image.to_rgba8().into_raw()
    }
}

/// A mock video capture that generates synthetic frames
///
/// # Example
///
/// ```rust
/// use screenpipe_test_utils::mocks::MockVideoCapture;
/// use std::time::Duration;
///
/// #[tokio::main]
/// async fn main() {
///     let capture = MockVideoCapture::new()
///         .with_resolution(1920, 1080)
///         .with_fps(30.0);
///
///     // Start capturing frames
///     let rx = capture.start().await.unwrap();
///
///     // Receive frames...
/// }
/// ```
#[derive(Clone)]
pub struct MockVideoCapture {
    id: u64,
    width: u32,
    height: u32,
    fps: f32,
    is_running: Arc<AtomicBool>,
    frame_counter: Arc<AtomicU64>,
    stats: Arc<std::sync::Mutex<MockStats>>,
    error_simulation: ErrorSimulation,
    frame_generator: Arc<dyn Fn(u64) -> MockFrame + Send + Sync>,
}

impl MockVideoCapture {
    /// Creates a new mock video capture with default settings (1920x1080, 30fps)
    pub fn new() -> Self {
        Self::with_defaults(1920, 1080, 30.0)
    }

    /// Creates a new mock video capture with specified parameters
    pub fn with_defaults(width: u32, height: u32, fps: f32) -> Self {
        let width_clone = width;
        let height_clone = height;

        Self {
            id: next_mock_id(),
            width,
            height,
            fps,
            is_running: Arc::new(AtomicBool::new(false)),
            frame_counter: Arc::new(AtomicU64::new(0)),
            stats: Arc::new(std::sync::Mutex::new(MockStats::new())),
            error_simulation: ErrorSimulation::None,
            frame_generator: Arc::new(move |frame_num| {
                MockFrame::gradient(width_clone, height_clone, frame_num)
            }),
        }
    }

    /// Sets the resolution
    pub fn with_resolution(mut self, width: u32, height: u32) -> Self {
        self.width = width;
        self.height = height;
        // Update frame generator with new dimensions
        self.frame_generator =
            Arc::new(move |frame_num| MockFrame::gradient(width, height, frame_num));
        self
    }

    /// Sets the frame rate
    pub fn with_fps(mut self, fps: f32) -> Self {
        self.fps = fps;
        self
    }

    /// Sets a custom frame generator
    pub fn with_frame_generator<F>(mut self, generator: F) -> Self
    where
        F: Fn(u64) -> MockFrame + Send + Sync + 'static,
    {
        self.frame_generator = Arc::new(generator);
        self
    }

    /// Configures error simulation
    pub fn with_error_simulation(mut self, simulation: ErrorSimulation) -> Self {
        self.error_simulation = simulation;
        self
    }

    /// Starts the mock capture and returns a receiver for frames
    pub async fn start(&self) -> anyhow::Result<mpsc::Receiver<MockFrame>> {
        if self.is_running.load(Ordering::SeqCst) {
            return Err(anyhow::anyhow!("Capture is already running"));
        }

        self.is_running.store(true, Ordering::SeqCst);
        self.stats.lock().unwrap().start();

        let (tx, rx) = mpsc::channel::<MockFrame>(10);

        self.start_capture_task(tx);

        Ok(rx)
    }

    fn start_capture_task(&self, tx: mpsc::Sender<MockFrame>) {
        let is_running = self.is_running.clone();
        let frame_counter = self.frame_counter.clone();
        let stats = self.stats.clone();
        let frame_generator = self.frame_generator.clone();
        let fps = self.fps;
        let error_simulation = self.error_simulation.clone();

        tokio::spawn(async move {
            let interval_duration = Duration::from_secs_f32(1.0 / fps);
            let mut interval = tokio::time::interval(interval_duration);

            while is_running.load(Ordering::SeqCst) {
                interval.tick().await;

                let frame_num = frame_counter.fetch_add(1, Ordering::SeqCst) + 1;

                // Check error simulation
                if error_simulation.should_fail(frame_num) {
                    stats.lock().unwrap().record_error();
                    continue;
                }

                let frame = frame_generator(frame_num);
                trace!("Generated frame {}", frame_num);

                stats.lock().unwrap().record_call();

                if tx.send(frame).await.is_err() {
                    debug!("Frame receiver dropped, stopping capture");
                    break;
                }
            }

            is_running.store(false, Ordering::SeqCst);
        });
    }

    /// Stops the mock capture
    pub fn stop(&self) {
        self.is_running.store(false, Ordering::SeqCst);
        debug!("Mock video capture {} stopped", self.id);
    }

    /// Returns true if the capture is running
    pub fn is_running(&self) -> bool {
        self.is_running.load(Ordering::SeqCst)
    }

    /// Returns the current frame count
    pub fn frame_count(&self) -> u64 {
        self.frame_counter.load(Ordering::SeqCst)
    }

    /// Returns current statistics
    pub fn stats(&self) -> MockStats {
        self.stats.lock().unwrap().clone()
    }

    /// Returns the resolution
    pub fn resolution(&self) -> (u32, u32) {
        (self.width, self.height)
    }

    /// Returns the frame rate
    pub fn fps(&self) -> f32 {
        self.fps
    }

    /// Captures a single frame immediately
    pub fn capture_single_frame(&self) -> MockFrame {
        let frame_num = self.frame_counter.fetch_add(1, Ordering::SeqCst) + 1;
        (self.frame_generator)(frame_num)
    }
}

impl Default for MockVideoCapture {
    fn default() -> Self {
        Self::new()
    }
}

impl MockComponent for MockVideoCapture {
    fn mock_id(&self) -> u64 {
        self.id
    }

    fn is_initialized(&self) -> bool {
        self.width > 0 && self.height > 0 && self.fps > 0.0
    }

    fn reset(&mut self) {
        self.is_running.store(false, Ordering::SeqCst);
        self.frame_counter.store(0, Ordering::SeqCst);
        self.stats = Arc::new(std::sync::Mutex::new(MockStats::new()));
    }
}

impl std::fmt::Debug for MockVideoCapture {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("MockVideoCapture")
            .field("id", &self.id)
            .field("width", &self.width)
            .field("height", &self.height)
            .field("fps", &self.fps)
            .field("is_running", &self.is_running())
            .field("frame_count", &self.frame_count())
            .finish()
    }
}

/// Predefined frame generators for common test scenarios
pub mod frame_generators {
    use super::*;

    /// Creates a generator that produces solid color frames
    pub fn solid_color(width: u32, height: u32, color: Rgba<u8>) -> impl Fn(u64) -> MockFrame {
        move |frame_num| MockFrame::solid_color(width, height, color, frame_num)
    }

    /// Creates a generator that produces gradient frames
    pub fn gradient(width: u32, height: u32) -> impl Fn(u64) -> MockFrame {
        move |frame_num| MockFrame::gradient(width, height, frame_num)
    }

    /// Creates a generator that produces test pattern frames
    pub fn test_pattern(width: u32, height: u32) -> impl Fn(u64) -> MockFrame {
        move |frame_num| MockFrame::test_pattern(width, height, frame_num)
    }

    /// Creates a generator that cycles through different colors
    pub fn color_cycle(width: u32, height: u32) -> impl Fn(u64) -> MockFrame {
        move |frame_num| {
            let colors = [
                Rgba([255, 0, 0, 255]),   // Red
                Rgba([0, 255, 0, 255]),   // Green
                Rgba([0, 0, 255, 255]),   // Blue
                Rgba([255, 255, 0, 255]), // Yellow
                Rgba([255, 0, 255, 255]), // Magenta
                Rgba([0, 255, 255, 255]), // Cyan
            ];
            let color = colors[frame_num as usize % colors.len()];
            MockFrame::solid_color(width, height, color, frame_num)
        }
    }

    /// Creates a generator that simulates a blinking cursor
    pub fn blinking_cursor(width: u32, height: u32) -> impl Fn(u64) -> MockFrame {
        move |frame_num| {
            let mut frame =
                MockFrame::solid_color(width, height, Rgba([255, 255, 255, 255]), frame_num);

            // Add a "cursor" that blinks every 30 frames
            if frame_num % 60 < 30 {
                // Draw cursor (simplified as a black rectangle)
                if let DynamicImage::ImageRgba8(ref mut img) = frame.image {
                    for y in 400..600 {
                        for x in 900..920 {
                            img.put_pixel(x, y, Rgba([0, 0, 0, 255]));
                        }
                    }
                }
            }

            frame
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_mock_frame_creation() {
        let frame = MockFrame::solid_color(100, 100, Rgba([255, 0, 0, 255]), 1);
        assert_eq!(frame.dimensions(), (100, 100));
        assert_eq!(frame.frame_number, 1);
    }

    #[test]
    fn test_mock_frame_gradient() {
        let frame = MockFrame::gradient(100, 100, 1);
        assert_eq!(frame.dimensions(), (100, 100));
    }

    #[test]
    fn test_mock_frame_test_pattern() {
        let frame = MockFrame::test_pattern(64, 64, 1);
        assert_eq!(frame.dimensions(), (64, 64));
    }

    #[test]
    fn test_mock_video_capture_creation() {
        let capture = MockVideoCapture::new();
        assert_eq!(capture.resolution(), (1920, 1080));
        assert_eq!(capture.fps(), 30.0);
    }

    #[test]
    fn test_mock_video_capture_custom() {
        let capture = MockVideoCapture::with_defaults(1280, 720, 60.0);
        assert_eq!(capture.resolution(), (1280, 720));
        assert_eq!(capture.fps(), 60.0);
    }

    #[tokio::test]
    async fn test_mock_video_capture_start_stop() {
        let capture = MockVideoCapture::new().with_fps(10.0);

        assert!(!capture.is_running());

        let mut rx = capture.start().await.unwrap();
        assert!(capture.is_running());

        // Receive a frame
        let frame = rx.recv().await;
        assert!(frame.is_some());

        capture.stop();
        tokio::time::sleep(Duration::from_millis(100)).await;
        assert!(!capture.is_running());
    }

    #[test]
    fn test_frame_generators() {
        let gen = frame_generators::solid_color(100, 100, Rgba([255, 0, 0, 255]));
        let frame = gen(1);
        assert_eq!(frame.dimensions(), (100, 100));

        let gen = frame_generators::color_cycle(100, 100);
        let frame1 = gen(1);
        let frame2 = gen(2);
        // Different frames should have different colors
        assert_ne!(frame1.to_bytes()[0..4], frame2.to_bytes()[0..4]);
    }

    #[test]
    fn test_capture_single_frame() {
        let capture = MockVideoCapture::new();
        let frame = capture.capture_single_frame();
        assert_eq!(frame.frame_number, 1);

        let frame = capture.capture_single_frame();
        assert_eq!(frame.frame_number, 2);
    }
}
