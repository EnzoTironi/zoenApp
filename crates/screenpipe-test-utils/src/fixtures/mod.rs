//! Test fixtures and sample data
//!
//! This module provides embedded test data that can be used across tests.
//! All fixtures are embedded using `include_bytes!` for zero-copy access.

pub mod audio_samples;
pub mod video_frames;

pub use audio_samples::*;
pub use video_frames::*;

use std::path::PathBuf;

/// Returns the path to the fixtures directory
pub fn fixtures_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("fixtures")
}

/// Returns the path to the audio fixtures directory
pub fn audio_fixtures_dir() -> PathBuf {
    fixtures_dir().join("audio")
}

/// Returns the path to the video fixtures directory
pub fn video_fixtures_dir() -> PathBuf {
    fixtures_dir().join("video")
}

/// Loads a fixture file as bytes
pub fn load_fixture_bytes(path: impl AsRef<std::path::Path>) -> anyhow::Result<&'static [u8]> {
    // This function is a placeholder - actual fixtures use include_bytes!
    // For runtime-loaded fixtures, you would read from the filesystem here
    Err(anyhow::anyhow!(
        "Use include_bytes! for compile-time fixtures, or implement runtime loading"
    ))
}

/// Information about a fixture
#[derive(Debug, Clone)]
pub struct FixtureInfo {
    /// Name of the fixture
    pub name: String,
    /// Description of the fixture contents
    pub description: String,
    /// Size in bytes
    pub size: usize,
    /// Format/extension
    pub format: String,
}

/// Lists all available fixtures
pub fn list_fixtures() -> Vec<FixtureInfo> {
    vec![
        FixtureInfo {
            name: "sine_440hz_5s.raw".to_string(),
            description: "440Hz sine wave, 5 seconds, 16kHz, mono, f32 LE".to_string(),
            size: SINE_440HZ_5S.len(),
            format: "raw f32 LE".to_string(),
        },
        FixtureInfo {
            name: "speech_sample.raw".to_string(),
            description: "Simulated speech pattern, 3 seconds, 16kHz, mono, f32 LE".to_string(),
            size: SPEECH_SAMPLE.len(),
            format: "raw f32 LE".to_string(),
        },
        FixtureInfo {
            name: "test_frame_1920x1080.png".to_string(),
            description: "1920x1080 test pattern PNG".to_string(),
            size: TEST_FRAME_1920X1080.len(),
            format: "png".to_string(),
        },
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_fixtures_dir() {
        let dir = fixtures_dir();
        assert!(dir.to_string_lossy().contains("fixtures"));
    }

    #[test]
    fn test_list_fixtures() {
        let fixtures = list_fixtures();
        assert!(!fixtures.is_empty());

        let sine = fixtures.iter().find(|f| f.name == "sine_440hz_5s.raw");
        assert!(sine.is_some());
    }

    #[test]
    fn test_fixture_sizes() {
        // Verify fixtures are not empty
        assert!(!SINE_440HZ_5S.is_empty());
        assert!(!SPEECH_SAMPLE.is_empty());
        assert!(!TEST_FRAME_1920X1080.is_empty());
    }
}
