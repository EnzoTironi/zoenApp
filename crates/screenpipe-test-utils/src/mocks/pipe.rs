//! Mock pipe runtime implementation
//!
//! Provides a mock implementation of the pipe system for testing
//! without requiring actual subprocess execution or bun runtime.

use crate::mocks::{next_mock_id, ErrorSimulation, MockComponent, MockStats};
use serde_json::Value;
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::{mpsc, oneshot, RwLock};
use tracing::{debug, error, info, trace, warn};

/// Represents the result of a pipe execution
#[derive(Clone, Debug)]
pub struct PipeExecutionResult {
    /// Whether the execution was successful
    pub success: bool,
    /// Exit code (0 for success)
    pub exit_code: i32,
    /// Standard output
    pub stdout: String,
    /// Standard error
    pub stderr: String,
    /// Execution duration
    pub duration: Duration,
    /// Pipe ID
    pub pipe_id: String,
}

impl PipeExecutionResult {
    /// Creates a successful result
    pub fn success(pipe_id: impl Into<String>, stdout: impl Into<String>) -> Self {
        Self {
            success: true,
            exit_code: 0,
            stdout: stdout.into(),
            stderr: String::new(),
            duration: Duration::default(),
            pipe_id: pipe_id.into(),
        }
    }

    /// Creates a failed result
    pub fn failure(pipe_id: impl Into<String>, exit_code: i32, stderr: impl Into<String>) -> Self {
        Self {
            success: false,
            exit_code,
            stdout: String::new(),
            stderr: stderr.into(),
            duration: Duration::default(),
            pipe_id: pipe_id.into(),
        }
    }
}

/// Represents a mock pipe process handle
#[derive(Debug)]
pub struct MockPipeHandle {
    pipe_id: String,
    is_running: Arc<AtomicBool>,
    start_time: Instant,
    port: Option<u16>,
    pid: Option<u32>,
    shutdown_tx: std::sync::Mutex<Option<oneshot::Sender<()>>>,
}

impl Clone for MockPipeHandle {
    fn clone(&self) -> Self {
        Self {
            pipe_id: self.pipe_id.clone(),
            is_running: Arc::clone(&self.is_running),
            start_time: self.start_time,
            port: self.port,
            pid: self.pid,
            shutdown_tx: std::sync::Mutex::new(None),
        }
    }
}

impl MockPipeHandle {
    /// Returns the pipe ID
    pub fn pipe_id(&self) -> &str {
        &self.pipe_id
    }

    /// Returns true if the pipe is running
    pub fn is_running(&self) -> bool {
        self.is_running.load(Ordering::SeqCst)
    }

    /// Returns the port the pipe is listening on (if any)
    pub fn port(&self) -> Option<u16> {
        self.port
    }

    /// Returns the process ID (if any)
    pub fn pid(&self) -> Option<u32> {
        self.pid
    }

    /// Returns how long the pipe has been running
    pub fn uptime(&self) -> Duration {
        self.start_time.elapsed()
    }

    /// Stops the pipe
    pub fn stop(&self) {
        self.is_running.store(false, Ordering::SeqCst);
        if let Some(tx) = self.shutdown_tx.lock().unwrap().take() {
            let _ = tx.send(());
        }
    }
}

/// Mock implementation of the pipe runtime
///
/// This simulates the pipe execution system without actually spawning
/// subprocesses, making tests faster and more reliable.
///
/// # Example
///
/// ```rust
/// use screenpipe_test_utils::mocks::MockPipeRuntime;
/// use std::path::PathBuf;
///
/// #[tokio::main]
/// async fn main() {
///     let runtime = MockPipeRuntime::new(PathBuf::from("/tmp/screenpipe"));
///
///     // Register a mock pipe
///     runtime.register_pipe("test-pipe", |config| {
///         Ok("Hello from mock pipe!".to_string())
///     }).await;
///
///     // Start the pipe
///     let handle = runtime.start_pipe("test-pipe").await.unwrap();
///     assert!(handle.is_running());
/// }
/// ```
pub struct MockPipeRuntime {
    id: u64,
    screenpipe_dir: PathBuf,
    pipes: Arc<RwLock<HashMap<String, MockPipeDefinition>>>,
    running_pipes: Arc<RwLock<HashMap<String, MockPipeHandle>>>,
    stats: Arc<std::sync::Mutex<MockStats>>,
    error_simulation: Arc<RwLock<ErrorSimulation>>,
    next_port: Arc<AtomicU64>,
    next_pid: Arc<AtomicU64>,
}

/// Internal definition of a mock pipe
#[derive(Clone)]
struct MockPipeDefinition {
    pipe_id: String,
    config: Value,
    source: String,
    is_nextjs: bool,
    handler: Arc<dyn Fn(Value) -> anyhow::Result<String> + Send + Sync>,
    cron_jobs: Vec<MockCronJob>,
}

/// Represents a mock cron job
#[derive(Clone, Debug)]
struct MockCronJob {
    path: String,
    schedule: String,
    last_run: Option<Instant>,
}

impl MockPipeRuntime {
    /// Creates a new mock pipe runtime
    pub fn new(screenpipe_dir: impl AsRef<Path>) -> Self {
        let screenpipe_dir = screenpipe_dir.as_ref().to_path_buf();

        Self {
            id: next_mock_id(),
            screenpipe_dir,
            pipes: Arc::new(RwLock::new(HashMap::new())),
            running_pipes: Arc::new(RwLock::new(HashMap::new())),
            stats: Arc::new(std::sync::Mutex::new(MockStats::new())),
            error_simulation: Arc::new(RwLock::new(ErrorSimulation::None)),
            next_port: Arc::new(AtomicU64::new(3000)),
            next_pid: Arc::new(AtomicU64::new(1000)),
        }
    }

    /// Sets the error simulation mode
    pub async fn set_error_simulation(&self, simulation: ErrorSimulation) {
        *self.error_simulation.write().await = simulation;
    }

    /// Registers a new mock pipe with a handler function
    ///
    /// The handler receives the pipe configuration and should return
    /// the stdout output as a string.
    pub async fn register_pipe<F>(&self, pipe_id: impl Into<String>, handler: F)
    where
        F: Fn(Value) -> anyhow::Result<String> + Send + Sync + 'static,
    {
        let pipe_id = pipe_id.into();
        let definition = MockPipeDefinition {
            pipe_id: pipe_id.clone(),
            config: serde_json::json!({
                "id": pipe_id.clone(),
                "enabled": false,
            }),
            source: format!("mock://{}", pipe_id),
            is_nextjs: false,
            handler: Arc::new(handler),
            cron_jobs: Vec::new(),
        };

        self.pipes.write().await.insert(pipe_id.clone(), definition);
        debug!("Registered mock pipe: {}", pipe_id);
    }

    /// Registers a Next.js-style mock pipe
    pub async fn register_nextjs_pipe<F>(&self, pipe_id: impl Into<String>, handler: F)
    where
        F: Fn(Value) -> anyhow::Result<String> + Send + Sync + 'static,
    {
        let pipe_id = pipe_id.into();
        let definition = MockPipeDefinition {
            pipe_id: pipe_id.clone(),
            config: serde_json::json!({
                "id": pipe_id,
                "enabled": false,
                "is_nextjs": true,
            }),
            source: format!("mock://{}", pipe_id),
            is_nextjs: true,
            handler: Arc::new(handler),
            cron_jobs: Vec::new(),
        };

        self.pipes.write().await.insert(pipe_id.clone(), definition);
        debug!("Registered Next.js mock pipe: {}", pipe_id);
    }

    /// Updates the configuration for a pipe
    pub async fn update_config(
        &self,
        pipe_id: impl AsRef<str>,
        new_config: Value,
    ) -> anyhow::Result<()> {
        let pipe_id = pipe_id.as_ref();
        let mut pipes = self.pipes.write().await;

        if let Some(definition) = pipes.get_mut(pipe_id) {
            // Check if enabled state changed BEFORE merging config
            let was_enabled = definition
                .config
                .get("enabled")
                .and_then(|v| v.as_bool())
                .unwrap_or(false);
            let is_enabled = new_config.get("enabled").and_then(|v| v.as_bool());

            // Merge new config with existing
            if let Some(new_obj) = new_config.as_object() {
                if let Some(existing_obj) = definition.config.as_object_mut() {
                    for (key, value) in new_obj {
                        existing_obj.insert(key.clone(), value.clone());
                    }
                }
            }

            drop(pipes); // Release lock before potential await

            if let Some(enabled) = is_enabled {
                match (was_enabled, enabled) {
                    (false, true) => {
                        self.start_pipe(pipe_id).await?;
                    }
                    (true, false) => {
                        self.stop_pipe(pipe_id).await?;
                    }
                    (true, true) => {
                        self.stop_pipe(pipe_id).await?;
                        self.start_pipe(pipe_id).await?;
                    }
                    (false, false) => {}
                }
            }

            Ok(())
        } else {
            Err(anyhow::anyhow!("Pipe '{}' not found", pipe_id))
        }
    }

    /// Starts a pipe
    pub async fn start_pipe(
        &self,
        pipe_id: impl AsRef<str>,
    ) -> anyhow::Result<Arc<MockPipeHandle>> {
        let pipe_id = pipe_id.as_ref().to_string();

        // Check if already running
        {
            let running = self.running_pipes.read().await;
            if running.contains_key(&pipe_id) {
                return Err(anyhow::anyhow!("Pipe '{}' is already running", pipe_id));
            }
        }

        let definition = {
            let pipes = self.pipes.read().await;
            pipes
                .get(&pipe_id)
                .ok_or_else(|| anyhow::anyhow!("Pipe '{}' not found", pipe_id))?
                .clone()
        };

        // Check error simulation
        let call_count = self.stats.lock().unwrap().call_count + 1;
        if self.error_simulation.read().await.should_fail(call_count) {
            self.stats.lock().unwrap().record_error();
            return Err(anyhow::anyhow!(
                "Simulated error starting pipe '{}'",
                pipe_id
            ));
        }

        let is_running = Arc::new(AtomicBool::new(true));
        let (shutdown_tx, mut shutdown_rx) = oneshot::channel();

        let port = if definition.is_nextjs {
            Some(self.next_port.fetch_add(1, Ordering::SeqCst) as u16)
        } else {
            None
        };

        let pid = Some(self.next_pid.fetch_add(1, Ordering::SeqCst) as u32);

        let handle = Arc::new(MockPipeHandle {
            pipe_id: pipe_id.clone(),
            is_running: is_running.clone(),
            start_time: Instant::now(),
            port,
            pid,
            shutdown_tx: std::sync::Mutex::new(Some(shutdown_tx)),
        });

        // Store the handle
        self.running_pipes
            .write()
            .await
            .insert(pipe_id.clone(), (*handle).clone());

        // Start the pipe execution
        let handler = definition.handler.clone();
        let config = definition.config.clone();
        let stats = self.stats.clone();
        let pipe_id_clone = pipe_id.clone();

        tokio::spawn(async move {
            info!("[{}] Mock pipe started", pipe_id_clone);
            stats.lock().unwrap().record_call();

            // Run the handler
            match handler(config) {
                Ok(output) => {
                    trace!("[{}] Pipe output: {}", pipe_id_clone, output);
                }
                Err(e) => {
                    error!("[{}] Pipe error: {}", pipe_id_clone, e);
                }
            }

            // Wait for shutdown signal
            tokio::select! {
                _ = tokio::time::sleep(Duration::from_secs(3600)) => {
                    // Timeout after 1 hour of simulated runtime
                    debug!("[{}] Pipe timed out", pipe_id_clone);
                }
                _ = &mut shutdown_rx => {
                    debug!("[{}] Pipe received shutdown signal", pipe_id_clone);
                }
            }

            is_running.store(false, Ordering::SeqCst);
            info!("[{}] Mock pipe stopped", pipe_id_clone);
        });

        self.stats.lock().unwrap().record_call();
        info!("[{}] Started mock pipe on port {:?}", pipe_id, port);

        Ok(handle)
    }

    /// Stops a running pipe
    pub async fn stop_pipe(&self, pipe_id: impl AsRef<str>) -> anyhow::Result<()> {
        let pipe_id = pipe_id.as_ref();

        let handle = {
            let mut running = self.running_pipes.write().await;
            running.remove(pipe_id)
        };

        if let Some(handle) = handle {
            handle.stop();
            info!("Stopped pipe: {}", pipe_id);
            Ok(())
        } else {
            Err(anyhow::anyhow!("Pipe '{}' is not running", pipe_id))
        }
    }

    /// Returns information about a pipe
    pub async fn get_pipe_info(&self, pipe_id: impl AsRef<str>) -> Option<Value> {
        let pipe_id = pipe_id.as_ref();
        let pipes = self.pipes.read().await;

        pipes.get(pipe_id).map(|def| {
            let running = self.running_pipes.blocking_read().contains_key(pipe_id);

            serde_json::json!({
                "id": def.pipe_id,
                "enabled": running,
                "config": def.config,
                "source": def.source,
                "port": if running {
                    self.running_pipes.blocking_read().get(pipe_id).and_then(|h| h.port())
                } else {
                    None::<u16>
                },
                "is_nextjs": def.is_nextjs,
            })
        })
    }

    /// Lists all registered pipes
    pub async fn list_pipes(&self) -> Vec<Value> {
        let pipes = self.pipes.read().await;
        let running = self.running_pipes.read().await;

        pipes
            .values()
            .map(|def| {
                let is_running = running.contains_key(&def.pipe_id);
                serde_json::json!({
                    "id": def.pipe_id,
                    "enabled": is_running,
                    "config": def.config,
                    "source": def.source,
                    "port": if is_running {
                        running.get(&def.pipe_id).and_then(|h| h.port())
                    } else {
                        None::<u16>
                    },
                    "is_nextjs": def.is_nextjs,
                })
            })
            .collect()
    }

    /// Downloads a pipe (mock implementation - just creates the directory)
    pub async fn download_pipe(&self, url: impl AsRef<str>) -> anyhow::Result<String> {
        let url = url.as_ref();
        let pipe_name = url.split('/').last().unwrap_or("unknown").to_string();

        let pipe_dir = self.screenpipe_dir.join("pipes").join(&pipe_name);
        tokio::fs::create_dir_all(&pipe_dir).await?;

        // Create a mock pipe.json
        let config = serde_json::json!({
            "id": pipe_name,
            "enabled": true,
            "source": url,
        });

        tokio::fs::write(
            pipe_dir.join("pipe.json"),
            serde_json::to_string_pretty(&config)?,
        )
        .await?;

        info!("Downloaded mock pipe: {} from {}", pipe_name, url);
        Ok(pipe_name)
    }

    /// Deletes a pipe
    pub async fn delete_pipe(&self, pipe_id: impl AsRef<str>) -> anyhow::Result<()> {
        let pipe_id = pipe_id.as_ref();

        // Stop if running
        let _ = self.stop_pipe(pipe_id).await;

        // Remove from registry
        self.pipes.write().await.remove(pipe_id);

        // Remove directory
        let pipe_dir = self.screenpipe_dir.join("pipes").join(pipe_id);
        if pipe_dir.exists() {
            tokio::fs::remove_dir_all(&pipe_dir).await?;
        }

        info!("Deleted pipe: {}", pipe_id);
        Ok(())
    }

    /// Purges all pipes
    pub async fn purge_pipes(&self) -> anyhow::Result<()> {
        // Stop all running pipes
        let running_ids: Vec<String> = {
            let running = self.running_pipes.read().await;
            running.keys().cloned().collect()
        };

        for pipe_id in running_ids {
            let _ = self.stop_pipe(&pipe_id).await;
        }

        // Clear registry
        self.pipes.write().await.clear();

        // Remove pipes directory
        let pipes_dir = self.screenpipe_dir.join("pipes");
        if pipes_dir.exists() {
            tokio::fs::remove_dir_all(&pipes_dir).await?;
        }

        info!("Purged all pipes");
        Ok(())
    }

    /// Returns true if a pipe is running
    pub async fn is_running(&self, pipe_id: impl AsRef<str>) -> bool {
        let pipe_id = pipe_id.as_ref();
        self.running_pipes.read().await.contains_key(pipe_id)
    }

    /// Returns current statistics
    pub fn stats(&self) -> MockStats {
        self.stats.lock().unwrap().clone()
    }

    /// Returns the number of running pipes
    pub async fn running_count(&self) -> usize {
        self.running_pipes.read().await.len()
    }

    /// Simulates a pipe crash
    pub async fn simulate_crash(&self, pipe_id: impl AsRef<str>) -> anyhow::Result<()> {
        let pipe_id = pipe_id.as_ref();

        if let Some(handle) = self.running_pipes.write().await.remove(pipe_id) {
            handle.stop();
            warn!("Simulated crash for pipe: {}", pipe_id);
            Ok(())
        } else {
            Err(anyhow::anyhow!("Pipe '{}' is not running", pipe_id))
        }
    }
}

impl MockComponent for MockPipeRuntime {
    fn mock_id(&self) -> u64 {
        self.id
    }

    fn is_initialized(&self) -> bool {
        self.screenpipe_dir.exists()
    }

    fn reset(&mut self) {
        // Note: This can't be async, so we just clear what we can synchronously
        // For full cleanup, use purge_pipes().await
    }
}

impl std::fmt::Debug for MockPipeRuntime {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("MockPipeRuntime")
            .field("id", &self.id)
            .field("screenpipe_dir", &self.screenpipe_dir)
            .finish()
    }
}

/// Helper functions for common pipe behaviors
pub mod pipe_behaviors {
    use super::*;

    /// Creates a handler that returns a fixed response
    pub fn fixed_response(response: impl Into<String>) -> impl Fn(Value) -> anyhow::Result<String> {
        let response = response.into();
        move |_config| Ok(response.clone())
    }

    /// Creates a handler that echoes the config
    pub fn echo_config() -> impl Fn(Value) -> anyhow::Result<String> {
        |config| Ok(serde_json::to_string_pretty(&config)?)
    }

    /// Creates a handler that simulates processing time
    pub fn with_delay(
        duration: Duration,
        inner: impl Fn(Value) -> anyhow::Result<String> + Send + Sync + 'static,
    ) -> impl Fn(Value) -> anyhow::Result<String> {
        move |config| {
            std::thread::sleep(duration);
            inner(config)
        }
    }

    /// Creates a handler that fails with a specific error
    pub fn failing(error: impl Into<String>) -> impl Fn(Value) -> anyhow::Result<String> {
        let error = error.into();
        move |_config| Err(anyhow::anyhow!("{}", error))
    }

    /// Creates a handler that randomly fails
    pub fn flaky(
        success_rate: f64,
        success: impl Fn(Value) -> anyhow::Result<String> + Send + Sync + 'static,
        failure: impl Fn(Value) -> anyhow::Result<String> + Send + Sync + 'static,
    ) -> impl Fn(Value) -> anyhow::Result<String> {
        use rand::Rng;

        move |config| {
            let mut rng = rand::thread_rng();
            if rng.gen::<f64>() < success_rate {
                success(config)
            } else {
                failure(config)
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    async fn setup() -> (MockPipeRuntime, TempDir) {
        let temp_dir = TempDir::new().unwrap();
        let runtime = MockPipeRuntime::new(temp_dir.path());
        (runtime, temp_dir)
    }

    #[tokio::test]
    async fn test_mock_pipe_runtime_creation() {
        let (runtime, _temp) = setup().await;
        assert!(runtime.is_initialized());
        assert_eq!(runtime.running_count().await, 0);
    }

    #[tokio::test]
    async fn test_register_and_start_pipe() {
        let (runtime, _temp) = setup().await;

        runtime
            .register_pipe("test-pipe", pipe_behaviors::echo_config())
            .await;

        let handle = runtime.start_pipe("test-pipe").await.unwrap();
        assert!(handle.is_running());
        assert_eq!(runtime.running_count().await, 1);

        runtime.stop_pipe("test-pipe").await.unwrap();
        tokio::time::sleep(Duration::from_millis(50)).await;
        assert!(!runtime.is_running("test-pipe").await);
    }

    #[tokio::test]
    async fn test_list_pipes() {
        let (runtime, _temp) = setup().await;

        runtime
            .register_pipe("pipe1", pipe_behaviors::fixed_response("hello"))
            .await;
        runtime
            .register_pipe("pipe2", pipe_behaviors::fixed_response("world"))
            .await;

        let pipes = runtime.list_pipes().await;
        assert_eq!(pipes.len(), 2);
    }

    #[tokio::test]
    async fn test_update_config() {
        let (runtime, _temp) = setup().await;

        runtime
            .register_pipe("test-pipe", pipe_behaviors::echo_config())
            .await;

        // Enable the pipe via config update
        runtime
            .update_config("test-pipe", serde_json::json!({ "enabled": true }))
            .await
            .unwrap();

        assert!(runtime.is_running("test-pipe").await);

        // Disable the pipe
        runtime
            .update_config("test-pipe", serde_json::json!({ "enabled": false }))
            .await
            .unwrap();

        tokio::time::sleep(Duration::from_millis(50)).await;
        assert!(!runtime.is_running("test-pipe").await);
    }

    #[tokio::test]
    async fn test_nextjs_pipe() {
        let (runtime, _temp) = setup().await;

        runtime
            .register_nextjs_pipe("web-pipe", pipe_behaviors::fixed_response("web response"))
            .await;

        let handle = runtime.start_pipe("web-pipe").await.unwrap();
        assert!(handle.port().is_some());
        assert!(handle.pid().is_some());
    }

    #[tokio::test]
    async fn test_download_pipe() {
        let (runtime, _temp) = setup().await;

        let pipe_name = runtime
            .download_pipe("https://github.com/example/pipe")
            .await
            .unwrap();

        assert_eq!(pipe_name, "pipe");
    }

    #[tokio::test]
    async fn test_purge_pipes() {
        let (runtime, _temp) = setup().await;

        runtime
            .register_pipe("pipe1", pipe_behaviors::fixed_response("1"))
            .await;
        runtime
            .register_pipe("pipe2", pipe_behaviors::fixed_response("2"))
            .await;

        runtime.start_pipe("pipe1").await.unwrap();
        runtime.start_pipe("pipe2").await.unwrap();

        assert_eq!(runtime.running_count().await, 2);

        runtime.purge_pipes().await.unwrap();

        assert_eq!(runtime.running_count().await, 0);
        assert!(runtime.list_pipes().await.is_empty());
    }

    #[tokio::test]
    async fn test_error_simulation() {
        let (runtime, _temp) = setup().await;

        runtime
            .set_error_simulation(ErrorSimulation::FailAfter(1))
            .await;

        runtime
            .register_pipe("test-pipe", pipe_behaviors::echo_config())
            .await;

        // First start should succeed
        let _ = runtime.start_pipe("test-pipe").await;
        runtime.stop_pipe("test-pipe").await.ok();

        // Second start should fail due to error simulation
        let result = runtime.start_pipe("test-pipe").await;
        assert!(result.is_err());
    }

    #[test]
    fn test_pipe_execution_result() {
        let success = PipeExecutionResult::success("test", "output");
        assert!(success.success);
        assert_eq!(success.exit_code, 0);
        assert_eq!(success.stdout, "output");

        let failure = PipeExecutionResult::failure("test", 1, "error");
        assert!(!failure.success);
        assert_eq!(failure.exit_code, 1);
        assert_eq!(failure.stderr, "error");
    }

    #[tokio::test]
    async fn test_pipe_behaviors() {
        let fixed = pipe_behaviors::fixed_response("hello");
        assert_eq!(fixed(serde_json::Value::Null).unwrap(), "hello");

        let echo = pipe_behaviors::echo_config();
        let result = echo(serde_json::json!({"key": "value"})).unwrap();
        assert!(result.contains("key"));

        let failing = pipe_behaviors::failing("error message");
        assert!(failing(serde_json::Value::Null).is_err());
    }
}
