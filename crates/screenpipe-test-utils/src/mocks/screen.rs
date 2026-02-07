//! Mock ScreenCaptureKit implementation for macOS
//!
//! Provides a cross-platform mock of macOS ScreenCaptureKit functionality
//! for testing screen capture without requiring actual macOS APIs.

use crate::mocks::video::{MockFrame, MockVideoCapture};
use crate::mocks::{next_mock_id, ErrorSimulation, MockComponent, MockStats};
use std::collections::HashMap;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::mpsc;
use tracing::{debug, trace, warn};

/// Represents a mock display/monitor
#[derive(Clone, Debug)]
pub struct MockDisplay {
    /// Unique display identifier
    pub id: u32,
    /// Display name
    pub name: String,
    /// Width in pixels
    pub width: u32,
    /// Height in pixels
    pub height: u32,
    /// Refresh rate
    pub refresh_rate: f32,
    /// Is this the main display
    pub is_main: bool,
    /// Scale factor (e.g., 2.0 for Retina)
    pub scale_factor: f32,
}

impl MockDisplay {
    /// Creates a new mock display
    pub fn new(id: u32, name: impl Into<String>, width: u32, height: u32) -> Self {
        Self {
            id,
            name: name.into(),
            width,
            height,
            refresh_rate: 60.0,
            is_main: false,
            scale_factor: 1.0,
        }
    }

    /// Sets this as the main display
    pub fn set_main(mut self, is_main: bool) -> Self {
        self.is_main = is_main;
        self
    }

    /// Sets the refresh rate
    pub fn with_refresh_rate(mut self, rate: f32) -> Self {
        self.refresh_rate = rate;
        self
    }

    /// Sets the scale factor (for Retina displays)
    pub fn with_scale_factor(mut self, factor: f32) -> Self {
        self.scale_factor = factor;
        self
    }

    /// Returns the resolution scaled for the display
    pub fn scaled_resolution(&self) -> (u32, u32) {
        (
            (self.width as f32 * self.scale_factor) as u32,
            (self.height as f32 * self.scale_factor) as u32,
        )
    }
}

/// Represents a mock window for window capture
#[derive(Clone, Debug)]
pub struct MockWindow {
    /// Unique window identifier
    pub id: u32,
    /// Window title
    pub title: String,
    /// Owning application name
    pub app_name: String,
    /// Window position and size
    pub x: i32,
    pub y: i32,
    pub width: u32,
    pub height: u32,
    /// Is this window currently focused
    pub is_focused: bool,
    /// Is this window visible
    pub is_visible: bool,
}

impl MockWindow {
    /// Creates a new mock window
    pub fn new(id: u32, title: impl Into<String>, app_name: impl Into<String>) -> Self {
        Self {
            id,
            title: title.into(),
            app_name: app_name.into(),
            x: 100,
            y: 100,
            width: 800,
            height: 600,
            is_focused: false,
            is_visible: true,
        }
    }

    /// Sets the window position
    pub fn at_position(mut self, x: i32, y: i32) -> Self {
        self.x = x;
        self.y = y;
        self
    }

    /// Sets the window size
    pub fn with_size(mut self, width: u32, height: u32) -> Self {
        self.width = width;
        self.height = height;
        self
    }

    /// Sets the focused state
    pub fn focused(mut self, focused: bool) -> Self {
        self.is_focused = focused;
        self
    }

    /// Sets the visibility
    pub fn visible(mut self, visible: bool) -> Self {
        self.is_visible = visible;
        self
    }
}

/// Mock implementation of macOS ScreenCaptureKit
///
/// This provides a cross-platform way to test screen capture functionality
/// that would normally require macOS ScreenCaptureKit APIs.
///
/// # Example
///
/// ```rust
/// use screenpipe_test_utils::mocks::MockScreenCaptureKit;
///
/// let sck = MockScreenCaptureKit::new()
///     .with_display(1, "Built-in Display", 1920, 1080)
///     .with_window(1, "Test Window", "TestApp");
///
/// // List available displays
/// let displays = sck.list_displays();
/// assert_eq!(displays.len(), 1);
/// ```
#[derive(Clone)]
pub struct MockScreenCaptureKit {
    id: u64,
    displays: Arc<std::sync::Mutex<HashMap<u32, MockDisplay>>>,
    windows: Arc<std::sync::Mutex<HashMap<u32, MockWindow>>>,
    is_running: Arc<AtomicBool>,
    stats: Arc<std::sync::Mutex<MockStats>>,
    error_simulation: ErrorSimulation,
    capture_delay: Duration,
}

impl MockScreenCaptureKit {
    /// Creates a new mock ScreenCaptureKit with default displays
    pub fn new() -> Self {
        let mut displays = HashMap::new();
        displays.insert(
            1,
            MockDisplay::new(1, "Built-in Retina Display", 1920, 1080)
                .set_main(true)
                .with_scale_factor(2.0),
        );

        Self {
            id: next_mock_id(),
            displays: Arc::new(std::sync::Mutex::new(displays)),
            windows: Arc::new(std::sync::Mutex::new(HashMap::new())),
            is_running: Arc::new(AtomicBool::new(false)),
            stats: Arc::new(std::sync::Mutex::new(MockStats::new())),
            error_simulation: ErrorSimulation::None,
            capture_delay: Duration::from_millis(16), // ~60fps
        }
    }

    /// Adds a display to the mock
    pub fn with_display(
        mut self,
        id: u32,
        name: impl Into<String>,
        width: u32,
        height: u32,
    ) -> Self {
        self.displays
            .lock()
            .unwrap()
            .insert(id, MockDisplay::new(id, name, width, height));
        self
    }

    /// Adds a window to the mock
    pub fn with_window(
        mut self,
        id: u32,
        title: impl Into<String>,
        app_name: impl Into<String>,
    ) -> Self {
        self.windows
            .lock()
            .unwrap()
            .insert(id, MockWindow::new(id, title, app_name));
        self
    }

    /// Configures error simulation
    pub fn with_error_simulation(mut self, simulation: ErrorSimulation) -> Self {
        self.error_simulation = simulation;
        self
    }

    /// Sets the simulated capture delay
    pub fn with_capture_delay(mut self, delay: Duration) -> Self {
        self.capture_delay = delay;
        self
    }

    /// Lists all available displays
    pub fn list_displays(&self) -> Vec<MockDisplay> {
        self.displays.lock().unwrap().values().cloned().collect()
    }

    /// Lists all available windows
    pub fn list_windows(&self) -> Vec<MockWindow> {
        self.windows
            .lock()
            .unwrap()
            .values()
            .filter(|w| w.is_visible)
            .cloned()
            .collect()
    }

    /// Gets a specific display by ID
    pub fn get_display(&self, id: u32) -> Option<MockDisplay> {
        self.displays.lock().unwrap().get(&id).cloned()
    }

    /// Gets a specific window by ID
    pub fn get_window(&self, id: u32) -> Option<MockWindow> {
        self.windows.lock().unwrap().get(&id).cloned()
    }

    /// Gets the main display
    pub fn get_main_display(&self) -> Option<MockDisplay> {
        self.displays
            .lock()
            .unwrap()
            .values()
            .find(|d| d.is_main)
            .cloned()
    }

    /// Gets the focused window
    pub fn get_focused_window(&self) -> Option<MockWindow> {
        self.windows
            .lock()
            .unwrap()
            .values()
            .find(|w| w.is_focused)
            .cloned()
    }

    /// Starts capturing a display
    pub async fn start_display_capture(
        &self,
        display_id: u32,
    ) -> anyhow::Result<mpsc::Receiver<MockFrame>> {
        let display = self
            .get_display(display_id)
            .ok_or_else(|| anyhow::anyhow!("Display {} not found", display_id))?;

        if self.is_running.load(Ordering::SeqCst) {
            return Err(anyhow::anyhow!("Capture already running"));
        }

        self.is_running.store(true, Ordering::SeqCst);
        self.stats.lock().unwrap().start();

        let (tx, rx) = mpsc::channel::<MockFrame>(10);

        self.run_display_capture_loop(tx, display);

        debug!("Started mock display capture for display {}", display_id);
        Ok(rx)
    }

    /// Starts capturing a window
    pub async fn start_window_capture(
        &self,
        window_id: u32,
    ) -> anyhow::Result<mpsc::Receiver<MockFrame>> {
        let window = self
            .get_window(window_id)
            .ok_or_else(|| anyhow::anyhow!("Window {} not found", window_id))?;

        if !window.is_visible {
            return Err(anyhow::anyhow!("Window {} is not visible", window_id));
        }

        let (tx, rx) = mpsc::channel::<MockFrame>(10);

        self.run_window_capture_loop(tx, window);

        debug!("Started mock window capture for window {}", window_id);
        Ok(rx)
    }

    fn run_display_capture_loop(&self, tx: mpsc::Sender<MockFrame>, display: MockDisplay) {
        let is_running = self.is_running.clone();
        let stats = self.stats.clone();
        let error_simulation = self.error_simulation.clone();
        let delay = self.capture_delay;
        let mut frame_counter: u64 = 0;

        tokio::spawn(async move {
            let mut interval = tokio::time::interval(delay);

            while is_running.load(Ordering::SeqCst) {
                interval.tick().await;
                frame_counter += 1;

                // Check error simulation
                if error_simulation.should_fail(frame_counter) {
                    stats.lock().unwrap().record_error();
                    continue;
                }

                // Generate a frame representing the display
                let frame = MockFrame::gradient(display.width, display.height, frame_counter)
                    .with_app_name("MockScreenCaptureKit");

                trace!("Captured display frame {}", frame_counter);
                stats.lock().unwrap().record_call();

                if tx.send(frame).await.is_err() {
                    debug!("Display capture receiver dropped");
                    break;
                }
            }

            is_running.store(false, Ordering::SeqCst);
        });
    }

    fn run_window_capture_loop(&self, tx: mpsc::Sender<MockFrame>, window: MockWindow) {
        let is_running = self.is_running.clone();
        let stats = self.stats.clone();
        let error_simulation = self.error_simulation.clone();
        let delay = self.capture_delay;
        let mut frame_counter: u64 = 0;

        tokio::spawn(async move {
            let mut interval = tokio::time::interval(delay);

            while is_running.load(Ordering::SeqCst) {
                interval.tick().await;
                frame_counter += 1;

                // Check error simulation
                if error_simulation.should_fail(frame_counter) {
                    stats.lock().unwrap().record_error();
                    continue;
                }

                // Generate a frame representing the window
                let frame = MockFrame::solid_color(
                    window.width,
                    window.height,
                    image::Rgba([100, 150, 200, 255]),
                    frame_counter,
                )
                .with_window_name(&window.title)
                .with_app_name(&window.app_name);

                trace!("Captured window frame {}", frame_counter);
                stats.lock().unwrap().record_call();

                if tx.send(frame).await.is_err() {
                    debug!("Window capture receiver dropped");
                    break;
                }
            }
        });
    }

    /// Stops all capture
    pub fn stop_capture(&self) {
        self.is_running.store(false, Ordering::SeqCst);
        debug!("Mock ScreenCaptureKit capture stopped");
    }

    /// Simulates a display being connected
    pub fn connect_display(&self, display: MockDisplay) {
        let display_id = display.id;
        self.displays.lock().unwrap().insert(display_id, display);
        debug!("Display {} connected", display_id);
    }

    /// Simulates a display being disconnected
    pub fn disconnect_display(&self, display_id: u32) {
        self.displays.lock().unwrap().remove(&display_id);
        debug!("Display {} disconnected", display_id);
    }

    /// Simulates a window being opened
    pub fn open_window(&self, window: MockWindow) {
        let window_id = window.id;
        self.windows.lock().unwrap().insert(window_id, window);
        debug!("Window {} opened", window_id);
    }

    /// Simulates a window being closed
    pub fn close_window(&self, window_id: u32) {
        self.windows.lock().unwrap().remove(&window_id);
        debug!("Window {} closed", window_id);
    }

    /// Updates window properties
    pub fn update_window<F>(&self, window_id: u32, update: F)
    where
        F: FnOnce(&mut MockWindow),
    {
        if let Some(window) = self.windows.lock().unwrap().get_mut(&window_id) {
            update(window);
        }
    }

    /// Simulates a window focus change
    pub fn focus_window(&self, window_id: u32) {
        let mut windows = self.windows.lock().unwrap();

        // Unfocus all windows
        for window in windows.values_mut() {
            window.is_focused = false;
        }

        // Focus the target window
        if let Some(window) = windows.get_mut(&window_id) {
            window.is_focused = true;
            debug!("Window {} focused", window_id);
        }
    }

    /// Returns current statistics
    pub fn stats(&self) -> MockStats {
        self.stats.lock().unwrap().clone()
    }

    /// Returns true if capture is running
    pub fn is_capturing(&self) -> bool {
        self.is_running.load(Ordering::SeqCst)
    }

    /// Creates a MockVideoCapture from a display configuration
    pub fn create_video_capture(&self, display_id: u32) -> anyhow::Result<MockVideoCapture> {
        let display = self
            .get_display(display_id)
            .ok_or_else(|| anyhow::anyhow!("Display {} not found", display_id))?;

        Ok(MockVideoCapture::with_defaults(
            display.width,
            display.height,
            display.refresh_rate,
        ))
    }
}

impl Default for MockScreenCaptureKit {
    fn default() -> Self {
        Self::new()
    }
}

impl MockComponent for MockScreenCaptureKit {
    fn mock_id(&self) -> u64 {
        self.id
    }

    fn is_initialized(&self) -> bool {
        !self.displays.lock().unwrap().is_empty()
    }

    fn reset(&mut self) {
        self.stop_capture();
        self.displays.lock().unwrap().clear();
        self.windows.lock().unwrap().clear();
        self.stats = Arc::new(std::sync::Mutex::new(MockStats::new()));
    }
}

impl std::fmt::Debug for MockScreenCaptureKit {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("MockScreenCaptureKit")
            .field("id", &self.id)
            .field("displays", &self.list_displays().len())
            .field("windows", &self.list_windows().len())
            .field("is_capturing", &self.is_capturing())
            .finish()
    }
}

/// Predefined display configurations for common scenarios
pub mod preset_displays {
    use super::*;

    /// MacBook Pro 16" built-in display
    pub fn macbook_pro_16() -> MockDisplay {
        MockDisplay::new(1, "MacBook Pro 16\" Display", 1728, 1117)
            .set_main(true)
            .with_scale_factor(2.0)
            .with_refresh_rate(120.0)
    }

    /// MacBook Pro 14" built-in display
    pub fn macbook_pro_14() -> MockDisplay {
        MockDisplay::new(1, "MacBook Pro 14\" Display", 1512, 982)
            .set_main(true)
            .with_scale_factor(2.0)
            .with_refresh_rate(120.0)
    }

    /// Standard 4K external monitor
    pub fn external_4k() -> MockDisplay {
        MockDisplay::new(2, "LG UltraFine 4K", 3840, 2160)
            .with_scale_factor(2.0)
            .with_refresh_rate(60.0)
    }

    /// Standard 1080p external monitor
    pub fn external_1080p() -> MockDisplay {
        MockDisplay::new(2, "External Monitor", 1920, 1080).with_refresh_rate(60.0)
    }

    /// Ultrawide monitor
    pub fn ultrawide() -> MockDisplay {
        MockDisplay::new(2, "Ultrawide Monitor", 3440, 1440).with_refresh_rate(144.0)
    }
}

/// Predefined window configurations for common applications
pub mod preset_windows {
    use super::*;

    /// Safari browser window
    pub fn safari() -> MockWindow {
        MockWindow::new(1, "Google - Safari", "Safari")
            .with_size(1200, 800)
            .at_position(100, 100)
            .focused(true)
    }

    /// Terminal window
    pub fn terminal() -> MockWindow {
        MockWindow::new(2, "Terminal — zsh", "Terminal")
            .with_size(800, 600)
            .at_position(50, 50)
    }

    /// VS Code window
    pub fn vscode() -> MockWindow {
        MockWindow::new(3, "main.rs — screenpipe", "Visual Studio Code")
            .with_size(1400, 900)
            .at_position(200, 100)
    }

    /// Slack window
    pub fn slack() -> MockWindow {
        MockWindow::new(4, "Slack — screenpipe", "Slack")
            .with_size(1000, 700)
            .at_position(300, 200)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_mock_display_creation() {
        let display = MockDisplay::new(1, "Test Display", 1920, 1080)
            .set_main(true)
            .with_scale_factor(2.0);

        assert_eq!(display.id, 1);
        assert_eq!(display.name, "Test Display");
        assert_eq!(display.scaled_resolution(), (3840, 2160));
        assert!(display.is_main);
    }

    #[test]
    fn test_mock_window_creation() {
        let window = MockWindow::new(1, "Test Window", "TestApp")
            .with_size(800, 600)
            .at_position(100, 200)
            .focused(true);

        assert_eq!(window.id, 1);
        assert_eq!(window.title, "Test Window");
        assert_eq!(window.width, 800);
        assert_eq!(window.x, 100);
        assert!(window.is_focused);
    }

    #[test]
    fn test_mock_screen_capture_kit_creation() {
        let sck = MockScreenCaptureKit::new();
        let displays = sck.list_displays();
        assert!(!displays.is_empty());

        let main = sck.get_main_display();
        assert!(main.is_some());
    }

    #[test]
    fn test_mock_screen_capture_kit_with_display() {
        let sck = MockScreenCaptureKit::new().with_display(2, "External", 2560, 1440);

        let displays = sck.list_displays();
        assert_eq!(displays.len(), 2);

        let external = sck.get_display(2);
        assert!(external.is_some());
        assert_eq!(external.unwrap().width, 2560);
    }

    #[test]
    fn test_mock_screen_capture_kit_window_management() {
        let sck = MockScreenCaptureKit::new();

        let window = MockWindow::new(1, "Test", "App");
        sck.open_window(window);

        let windows = sck.list_windows();
        assert_eq!(windows.len(), 1);

        sck.close_window(1);
        let windows = sck.list_windows();
        assert!(windows.is_empty());
    }

    #[test]
    fn test_focus_window() {
        let sck = MockScreenCaptureKit::new()
            .with_window(1, "Window 1", "App1")
            .with_window(2, "Window 2", "App2");

        sck.focus_window(2);

        let focused = sck.get_focused_window();
        assert!(focused.is_some());
        assert_eq!(focused.unwrap().id, 2);
    }

    #[tokio::test]
    async fn test_display_capture() {
        let sck = MockScreenCaptureKit::new();
        let mut rx = sck.start_display_capture(1).await.unwrap();

        // Receive a frame
        let frame = rx.recv().await;
        assert!(frame.is_some());

        sck.stop_capture();
    }

    #[test]
    fn test_preset_displays() {
        let display = preset_displays::macbook_pro_16();
        assert!(display.is_main);
        assert_eq!(display.scale_factor, 2.0);

        let display = preset_displays::external_4k();
        assert_eq!(display.width, 3840);
    }

    #[test]
    fn test_preset_windows() {
        let window = preset_windows::safari();
        assert_eq!(window.app_name, "Safari");

        let window = preset_windows::vscode();
        assert_eq!(window.app_name, "Visual Studio Code");
    }

    #[test]
    fn test_create_video_capture() {
        let sck = MockScreenCaptureKit::new();
        let capture = sck.create_video_capture(1);
        assert!(capture.is_ok());

        let capture = capture.unwrap();
        assert_eq!(capture.resolution(), (1920, 1080));
    }
}
