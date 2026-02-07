//! # screenpipe-test-utils
//!
//! Test utilities and mocks for the screenpipe ecosystem.
//!
//! This crate provides mock implementations of audio, video, screen capture, and pipe runtime
//! components to enable reliable, deterministic testing without requiring actual hardware
//! or subprocess execution.
//!
//! ## Features
//!
//! - **MockAudioDevice**: Simulates audio capture with pre-recorded samples
//! - **MockVideoCapture**: Generates synthetic video frames
//! - **MockScreenCaptureKit**: Simulates macOS ScreenCaptureKit (cross-platform)
//! - **MockPipeRuntime**: Simulates pipe execution without spawning subprocesses
//!
//! ## Example Usage
//!
//! ```rust
//! use screenpipe_test_utils::mocks::{
//!     MockAudioDevice, MockVideoCapture, MockPipeRuntime
//! };
//! use screenpipe_test_utils::fixtures::audio_samples::SINE_440HZ_5S;
//!
//! #[tokio::test]
//! async fn test_audio_processing() {
//!     // Create a mock audio device with embedded test samples
//!     let device = MockAudioDevice::new("test-mic")
//!         .with_sample_data(SINE_440HZ_5S)
//!         .with_sample_rate(16000);
//!
//!     // Use the device in your tests...
//! }
//! ```

pub mod fixtures;
pub mod mocks;

// Re-export commonly used types
pub use fixtures::*;
pub use mocks::*;

use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;

/// Global test counter for generating unique identifiers in tests
static TEST_COUNTER: AtomicU64 = AtomicU64::new(1);

/// Generates a unique test identifier
pub fn unique_test_id() -> u64 {
    TEST_COUNTER.fetch_add(1, Ordering::SeqCst)
}

/// Creates a unique test name with the given prefix
pub fn unique_test_name(prefix: &str) -> String {
    format!("{}_{}", prefix, unique_test_id())
}

/// Test helper to create a temporary directory for tests
pub fn temp_test_dir() -> std::path::PathBuf {
    let temp_dir = std::env::temp_dir().join(format!("screenpipe_test_{}", unique_test_id()));
    std::fs::create_dir_all(&temp_dir).expect("Failed to create temp directory");
    temp_dir
}

/// Cleans up a temporary test directory
pub fn cleanup_temp_dir(path: &std::path::Path) {
    if path.exists() {
        let _ = std::fs::remove_dir_all(path);
    }
}

/// Assertion helper for approximate equality in audio/video processing
pub fn assert_approx_eq(a: f32, b: f32, epsilon: f32) {
    assert!(
        (a - b).abs() < epsilon,
        "Expected {} to be approximately equal to {} (epsilon: {})",
        a,
        b,
        epsilon
    );
}

/// Test timeout wrapper - fails the test if it takes longer than the specified duration
pub async fn with_timeout<F, T>(duration: std::time::Duration, future: F) -> anyhow::Result<T>
where
    F: std::future::Future<Output = anyhow::Result<T>>,
{
    tokio::time::timeout(duration, future)
        .await
        .map_err(|_| anyhow::anyhow!("Test timed out after {:?}", duration))?
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_unique_test_id() {
        let id1 = unique_test_id();
        let id2 = unique_test_id();
        assert_ne!(id1, id2);
        assert!(id2 > id1);
    }

    #[test]
    fn test_unique_test_name() {
        let name1 = unique_test_name("test");
        let name2 = unique_test_name("test");
        assert_ne!(name1, name2);
        assert!(name1.starts_with("test_"));
    }

    #[test]
    fn test_temp_test_dir() {
        let dir = temp_test_dir();
        assert!(dir.exists());
        cleanup_temp_dir(&dir);
        assert!(!dir.exists());
    }

    #[test]
    fn test_assert_approx_eq() {
        assert_approx_eq(1.0, 1.001, 0.01);
        assert_approx_eq(0.0, 0.0, 0.0001);
    }

    #[tokio::test]
    async fn test_with_timeout_success() {
        let result = with_timeout(std::time::Duration::from_secs(1), async {
            Ok::<_, anyhow::Error>(42)
        })
        .await;
        assert_eq!(result.unwrap(), 42);
    }

    #[tokio::test]
    async fn test_with_timeout_failure() {
        let result = with_timeout(std::time::Duration::from_millis(10), async {
            tokio::time::sleep(std::time::Duration::from_secs(1)).await;
            Ok::<_, anyhow::Error>(42)
        })
        .await;
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("timed out"));
    }
}
