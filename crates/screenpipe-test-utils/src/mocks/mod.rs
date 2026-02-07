//! Mock implementations for screenpipe components
//!
//! This module provides mock implementations that can be used in place of real
//! hardware-dependent components for testing purposes.

pub mod audio;
pub mod pipe;
pub mod screen;
pub mod video;

pub use audio::{MockAudioDevice, MockAudioStream};
pub use pipe::{MockPipeHandle, MockPipeRuntime, PipeExecutionResult};
pub use screen::MockScreenCaptureKit;
pub use video::{MockFrame, MockVideoCapture};

use std::sync::atomic::{AtomicU64, Ordering};

/// Generates a unique mock identifier
static MOCK_ID_COUNTER: AtomicU64 = AtomicU64::new(1);

fn next_mock_id() -> u64 {
    MOCK_ID_COUNTER.fetch_add(1, Ordering::SeqCst)
}

/// Common trait for all mock components to enable identification
pub trait MockComponent {
    /// Returns the unique identifier for this mock instance
    fn mock_id(&self) -> u64;

    /// Returns true if this mock has been properly initialized
    fn is_initialized(&self) -> bool;

    /// Resets the mock to its initial state
    fn reset(&mut self);
}

/// Statistics collected during mock operation
#[derive(Debug, Clone, Default)]
pub struct MockStats {
    /// Number of times the mock was called
    pub call_count: u64,
    /// Number of errors simulated
    pub error_count: u64,
    /// Total bytes processed (if applicable)
    pub bytes_processed: u64,
    /// Start time of the mock operation
    pub start_time: Option<std::time::Instant>,
}

impl MockStats {
    /// Creates new empty stats
    pub fn new() -> Self {
        Self::default()
    }

    /// Records a call
    pub fn record_call(&mut self) {
        self.call_count += 1;
    }

    /// Records an error
    pub fn record_error(&mut self) {
        self.error_count += 1;
    }

    /// Records bytes processed
    pub fn record_bytes(&mut self, bytes: u64) {
        self.bytes_processed += bytes;
    }

    /// Starts timing
    pub fn start(&mut self) {
        self.start_time = Some(std::time::Instant::now());
    }

    /// Returns elapsed time since start
    pub fn elapsed(&self) -> Option<std::time::Duration> {
        self.start_time.map(|t| t.elapsed())
    }
}

/// Configuration for simulating errors in mocks
#[derive(Debug, Clone)]
pub enum ErrorSimulation {
    /// No errors - normal operation
    None,
    /// Fail after a specific number of calls
    FailAfter(u64),
    /// Fail with a specific probability (0.0 - 1.0)
    Random(f64),
    /// Fail on specific call numbers
    FailOn(Vec<u64>),
}

impl Default for ErrorSimulation {
    fn default() -> Self {
        ErrorSimulation::None
    }
}

impl ErrorSimulation {
    /// Determines if an error should be simulated based on the current call count
    pub fn should_fail(&self, call_count: u64) -> bool {
        match self {
            ErrorSimulation::None => false,
            ErrorSimulation::FailAfter(n) => call_count >= *n,
            ErrorSimulation::Random(prob) => {
                use rand::Rng;
                let mut rng = rand::thread_rng();
                rng.gen::<f64>() < *prob
            }
            ErrorSimulation::FailOn(calls) => calls.contains(&call_count),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_mock_stats() {
        let mut stats = MockStats::new();
        assert_eq!(stats.call_count, 0);

        stats.record_call();
        assert_eq!(stats.call_count, 1);

        stats.record_error();
        assert_eq!(stats.error_count, 1);

        stats.record_bytes(1024);
        assert_eq!(stats.bytes_processed, 1024);
    }

    #[test]
    fn test_error_simulation_none() {
        let sim = ErrorSimulation::None;
        assert!(!sim.should_fail(1));
        assert!(!sim.should_fail(100));
    }

    #[test]
    fn test_error_simulation_fail_after() {
        let sim = ErrorSimulation::FailAfter(5);
        assert!(!sim.should_fail(4));
        assert!(sim.should_fail(5));
        assert!(sim.should_fail(6));
    }

    #[test]
    fn test_error_simulation_fail_on() {
        let sim = ErrorSimulation::FailOn(vec![1, 3, 5]);
        assert!(sim.should_fail(1));
        assert!(!sim.should_fail(2));
        assert!(sim.should_fail(3));
        assert!(!sim.should_fail(4));
    }
}
