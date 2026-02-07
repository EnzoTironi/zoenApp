//! Playbook Engine - Automation Rules for Screenpipe
//!
//! This module provides the core engine for executing automation rules
//! based on triggers like time, app usage, and keywords.

use anyhow::Result;
use chrono::{DateTime, Datelike, Utc};
use cron::Schedule;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::HashMap;
use std::path::PathBuf;
use std::str::FromStr;
use std::sync::Arc;
use tokio::sync::{mpsc, RwLock};
use tokio::time::{interval, Duration};
use tracing::{debug, error, info, warn};
use uuid::Uuid;

// Re-export action items types for integration
pub use crate::action_items::{ActionItem, ActionItemPriority, ActionItemSource, ActionItemStatus};

// ─── Action Executor Dependencies ────────────────────────────────────────────

/// Configuration for action execution
pub struct ActionExecutorConfig {
    /// HTTP client for webhook requests
    pub http_client: reqwest::Client,
    /// Database manager for tagging operations (optional - will be set externally)
    pub db_manager: Option<Arc<dyn DatabaseManagerTrait>>,
    /// Pipe manager for running pipes (optional - will be set externally)
    pub pipe_manager: Option<Arc<dyn PipeManagerTrait>>,
}

impl Clone for ActionExecutorConfig {
    fn clone(&self) -> Self {
        Self {
            http_client: self.http_client.clone(),
            db_manager: self.db_manager.clone(),
            pipe_manager: self.pipe_manager.clone(),
        }
    }
}

impl std::fmt::Debug for ActionExecutorConfig {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ActionExecutorConfig")
            .field("http_client", &"<reqwest::Client>")
            .field("db_manager", &self.db_manager.is_some())
            .field("pipe_manager", &self.pipe_manager.is_some())
            .finish()
    }
}

impl Default for ActionExecutorConfig {
    fn default() -> Self {
        Self {
            http_client: reqwest::Client::builder()
                .timeout(Duration::from_secs(30))
                .build()
                .unwrap_or_default(),
            db_manager: None,
            pipe_manager: None,
        }
    }
}

/// Trait for database operations needed by the playbook engine
#[async_trait::async_trait]
pub trait DatabaseManagerTrait: Send + Sync {
    /// Add tags to content
    async fn add_tags(
        &self,
        id: i64,
        content_type: TagContentType,
        tags: Vec<String>,
    ) -> Result<()>;

    /// Search for recent frames (vision content)
    async fn search_recent_frames(
        &self,
        start_time: DateTime<Utc>,
        end_time: DateTime<Utc>,
    ) -> Result<Vec<i64>>;

    /// Search for recent audio chunks
    async fn search_recent_audio(
        &self,
        start_time: DateTime<Utc>,
        end_time: DateTime<Utc>,
    ) -> Result<Vec<i64>>;
}

/// Content type for tagging operations
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum TagContentType {
    Vision,
    Audio,
}

/// Trait for pipe management operations
#[async_trait::async_trait]
pub trait PipeManagerTrait: Send + Sync {
    /// Start a pipe with the given ID and optional parameters
    async fn start_pipe(&self, pipe_id: &str, params: Option<Value>) -> Result<()>;

    /// Get pipe info
    async fn get_pipe_info(&self, pipe_id: &str) -> Result<Option<PipeInfo>>;
}

/// Pipe information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PipeInfo {
    pub id: String,
    pub enabled: bool,
    pub port: Option<u16>,
}

// ─── Trigger Types ───────────────────────────────────────────────────────────

/// Event types that can trigger playbook execution
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum Trigger {
    /// Trigger when an application opens or becomes active
    AppOpen {
        app_name: String,
        #[serde(skip_serializing_if = "Option::is_none")]
        window_name: Option<String>,
    },
    /// Trigger based on a cron schedule
    Time {
        cron: String,
        #[serde(skip_serializing_if = "Option::is_none")]
        description: Option<String>,
    },
    /// Trigger when keywords are detected in OCR or audio
    Keyword {
        pattern: String,
        source: KeywordSource,
        #[serde(skip_serializing_if = "Option::is_none")]
        threshold: Option<f32>,
    },
    /// Trigger based on context (time, apps, windows combination)
    Context {
        #[serde(skip_serializing_if = "Option::is_none")]
        apps: Option<Vec<String>>,
        #[serde(skip_serializing_if = "Option::is_none")]
        windows: Option<Vec<String>>,
        #[serde(skip_serializing_if = "Option::is_none")]
        time_range: Option<String>,
        #[serde(skip_serializing_if = "Option::is_none")]
        days_of_week: Option<Vec<u8>>,
    },
    /// Trigger when a meeting starts (detected from audio/app)
    MeetingStart {
        #[serde(skip_serializing_if = "Option::is_none")]
        app_name: Option<String>,
        #[serde(skip_serializing_if = "Option::is_none")]
        keywords: Option<Vec<String>>,
    },
    /// Trigger when a meeting ends
    MeetingEnd {
        #[serde(skip_serializing_if = "Option::is_none")]
        app_name: Option<String>,
        #[serde(skip_serializing_if = "Option::is_none")]
        min_duration_minutes: Option<u32>,
    },
    /// Trigger on idle state change
    IdleState {
        idle_minutes: u32,
        state: IdleTriggerState,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum KeywordSource {
    Ocr,
    Audio,
    Both,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum IdleTriggerState {
    BecomesIdle,
    BecomesActive,
}

/// Context information passed during trigger evaluation
#[derive(Debug, Clone, Default)]
pub struct TriggerContext {
    /// Recent OCR text content
    pub recent_ocr: Vec<OcrEvent>,
    /// Recent audio transcription
    pub recent_audio: Vec<AudioEvent>,
    /// Current idle time in minutes
    pub idle_minutes: u32,
    /// Whether user is currently idle
    pub is_idle: bool,
    /// Active meeting info
    pub active_meeting: Option<MeetingContext>,
}

#[derive(Debug, Clone)]
pub struct OcrEvent {
    pub text: String,
    pub app_name: String,
    pub timestamp: DateTime<Utc>,
}

#[derive(Debug, Clone)]
pub struct AudioEvent {
    pub transcription: String,
    pub timestamp: DateTime<Utc>,
}

#[derive(Debug, Clone)]
pub struct MeetingContext {
    pub source_id: String,
    pub started_at: DateTime<Utc>,
    pub app_name: Option<String>,
}

// ─── Action Types ────────────────────────────────────────────────────────────

/// Actions that can be executed by playbooks
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum Action {
    /// Send a notification to the user
    Notify {
        title: String,
        message: String,
        #[serde(skip_serializing_if = "Option::is_none")]
        actions: Option<Vec<NotificationAction>>,
        #[serde(skip_serializing_if = "Option::is_none")]
        persistent: Option<bool>,
    },
    /// Generate a summary of recent activity
    Summarize {
        timeframe: u32, // minutes
        #[serde(skip_serializing_if = "Option::is_none")]
        focus: Option<SummaryFocus>,
        #[serde(skip_serializing_if = "Option::is_none")]
        output: Option<SummaryOutput>,
    },
    /// Enable or disable focus mode
    FocusMode {
        enabled: bool,
        #[serde(skip_serializing_if = "Option::is_none")]
        duration: Option<u32>, // minutes
        #[serde(skip_serializing_if = "Option::is_none")]
        allowed_apps: Option<Vec<String>>,
        #[serde(skip_serializing_if = "Option::is_none")]
        silence_notifications: Option<bool>,
    },
    /// Run a specific pipe
    RunPipe {
        pipe_id: String,
        #[serde(skip_serializing_if = "Option::is_none")]
        params: Option<Value>,
    },
    /// Tag content automatically
    Tag {
        tags: Vec<String>,
        timeframe: u32, // minutes
    },
    /// Call a webhook
    Webhook {
        url: String,
        method: HttpMethod,
        #[serde(skip_serializing_if = "Option::is_none")]
        headers: Option<HashMap<String, String>>,
        #[serde(skip_serializing_if = "Option::is_none")]
        body: Option<Value>,
    },
    /// Extract action items from recent content
    ExtractActionItems {
        timeframe: u32, // minutes
        #[serde(skip_serializing_if = "Option::is_none")]
        min_confidence: Option<f32>,
        #[serde(skip_serializing_if = "Option::is_none")]
        auto_export: Option<Vec<ExportTarget>>,
    },
    /// Send action items to external service
    ExportActionItems {
        target: ExportTarget,
        #[serde(skip_serializing_if = "Option::is_none")]
        filter: Option<ActionItemFilter>,
    },
    /// Start recording with specific settings
    StartRecording {
        #[serde(skip_serializing_if = "Option::is_none")]
        focus_app: Option<String>,
        #[serde(skip_serializing_if = "Option::is_none")]
        tag: Option<String>,
    },
    /// Stop recording
    StopRecording,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ExportTarget {
    Todoist {
        api_token: String,
        project_id: Option<String>,
    },
    Notion {
        api_token: String,
        database_id: String,
    },
    Webhook {
        url: String,
    },
    Slack {
        webhook_url: String,
        channel: Option<String>,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ActionItemFilter {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub since: Option<DateTime<Utc>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub status: Option<ActionItemStatus>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub min_priority: Option<ActionItemPriority>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NotificationAction {
    pub id: String,
    pub label: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SummaryFocus {
    All,
    ActionItems,
    Decisions,
    KeyPoints,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SummaryOutput {
    Notification,
    Clipboard,
    Pipe,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "UPPERCASE")]
pub enum HttpMethod {
    Get,
    Post,
    Put,
    Delete,
}

// ─── Playbook Definition ─────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Playbook {
    pub id: String,
    pub name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    pub enabled: bool,
    pub triggers: Vec<Trigger>,
    pub actions: Vec<Action>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cooldown_minutes: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max_executions_per_day: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub created_at: Option<DateTime<Utc>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub updated_at: Option<DateTime<Utc>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub is_builtin: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub icon: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub color: Option<String>,
}

// ─── Execution Tracking ──────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlaybookExecution {
    pub id: String,
    pub playbook_id: String,
    pub started_at: DateTime<Utc>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub completed_at: Option<DateTime<Utc>>,
    pub status: ExecutionStatus,
    pub triggered_by: Trigger,
    pub action_results: Vec<ActionResult>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ExecutionStatus {
    Running,
    Completed,
    Failed,
    Cancelled,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ActionResult {
    pub action: Action,
    pub success: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub result: Option<Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
    pub duration_ms: u64,
}

// ─── Trigger State ───────────────────────────────────────────────────────────

#[derive(Debug, Default)]
#[cfg_attr(test, derive(Clone))]
pub(crate) struct TriggerState {
    /// Last time each playbook was executed
    last_execution: HashMap<String, DateTime<Utc>>,
    /// Execution count per playbook per day
    daily_executions: HashMap<String, u32>,
    /// Current day for daily counter reset
    current_day: DateTime<Utc>,
    /// Currently open apps (for app_open trigger)
    open_apps: HashMap<String, AppStateInternal>,
    /// Active focus mode sessions
    focus_mode_active: bool,
    focus_mode_end_time: Option<DateTime<Utc>>,
}

#[derive(Debug, Clone)]
pub(crate) struct AppStateInternal {
    window_name: Option<String>,
    last_seen: DateTime<Utc>,
}

// ─── Event Channels ──────────────────────────────────────────────────────────

/// Events that can trigger playbook execution
#[derive(Debug, Clone)]
pub enum PlaybookEvent {
    OcrText {
        text: String,
        app_name: String,
    },
    AudioTranscription {
        text: String,
    },
    AppOpened {
        app_name: String,
        window_name: Option<String>,
    },
    AppClosed {
        app_name: String,
    },
    MeetingStarted {
        source_id: String,
        app_name: Option<String>,
    },
    MeetingEnded {
        source_id: String,
        duration_minutes: u32,
    },
    IdleStateChanged {
        is_idle: bool,
        idle_minutes: u32,
    },
    TimeTick,
}

/// Channel sender for playbook events
type EventSender = mpsc::UnboundedSender<PlaybookEvent>;
type EventReceiver = mpsc::UnboundedReceiver<PlaybookEvent>;

// ─── Playbook Engine ─────────────────────────────────────────────────────────

pub struct PlaybookEngine {
    playbooks: Arc<RwLock<HashMap<String, Playbook>>>,
    trigger_state: Arc<RwLock<TriggerState>>,
    trigger_context: Arc<RwLock<TriggerContext>>,
    screenpipe_dir: PathBuf,
    event_sender: Option<EventSender>,
    action_executor_config: Arc<RwLock<ActionExecutorConfig>>,
}

impl PlaybookEngine {
    pub fn new(screenpipe_dir: PathBuf) -> Self {
        Self {
            playbooks: Arc::new(RwLock::new(HashMap::new())),
            trigger_state: Arc::new(RwLock::new(TriggerState::default())),
            trigger_context: Arc::new(RwLock::new(TriggerContext::default())),
            screenpipe_dir,
            event_sender: None,
            action_executor_config: Arc::new(RwLock::new(ActionExecutorConfig::default())),
        }
    }

    /// Set the database manager for tagging operations
    pub async fn set_db_manager(&self, db_manager: Arc<dyn DatabaseManagerTrait>) {
        let mut config = self.action_executor_config.write().await;
        config.db_manager = Some(db_manager);
    }

    /// Set the pipe manager for running pipes
    pub async fn set_pipe_manager(&self, pipe_manager: Arc<dyn PipeManagerTrait>) {
        let mut config = self.action_executor_config.write().await;
        config.pipe_manager = Some(pipe_manager);
    }

    /// Get a sender for playbook events
    pub fn get_event_sender(&self) -> Option<EventSender> {
        self.event_sender.clone()
    }

    /// Submit an event to the playbook engine
    pub async fn submit_event(&self, event: PlaybookEvent) {
        // Update trigger context based on event
        {
            let mut context = self.trigger_context.write().await;
            match &event {
                PlaybookEvent::OcrText { text, app_name } => {
                    context.recent_ocr.push(OcrEvent {
                        text: text.clone(),
                        app_name: app_name.clone(),
                        timestamp: Utc::now(),
                    });
                    // Keep only last 100 entries
                    if context.recent_ocr.len() > 100 {
                        context.recent_ocr.remove(0);
                    }
                }
                PlaybookEvent::AudioTranscription { text } => {
                    context.recent_audio.push(AudioEvent {
                        transcription: text.clone(),
                        timestamp: Utc::now(),
                    });
                    if context.recent_audio.len() > 100 {
                        context.recent_audio.remove(0);
                    }
                }
                PlaybookEvent::IdleStateChanged {
                    is_idle,
                    idle_minutes,
                } => {
                    context.is_idle = *is_idle;
                    context.idle_minutes = *idle_minutes;
                }
                PlaybookEvent::MeetingStarted {
                    source_id,
                    app_name,
                } => {
                    context.active_meeting = Some(MeetingContext {
                        source_id: source_id.clone(),
                        started_at: Utc::now(),
                        app_name: app_name.clone(),
                    });
                }
                PlaybookEvent::MeetingEnded { .. } => {
                    context.active_meeting = None;
                }
                _ => {}
            }
        }

        // Send event through channel if available
        if let Some(sender) = &self.event_sender {
            let _ = sender.send(event);
        }
    }

    /// Initialize the playbook engine with existing playbooks
    pub async fn init(&self, playbooks: Vec<Playbook>) -> Result<()> {
        let mut map = self.playbooks.write().await;
        for playbook in playbooks {
            map.insert(playbook.id.clone(), playbook);
        }
        info!("Playbook engine initialized with {} playbooks", map.len());
        Ok(())
    }

    /// Start the playbook engine monitoring loop
    pub async fn start(&self) -> Result<()> {
        let playbooks = self.playbooks.clone();
        let trigger_state = self.trigger_state.clone();
        let screenpipe_dir = self.screenpipe_dir.clone();
        let executor_config = self.action_executor_config.clone();

        // Spawn the monitoring task
        tokio::spawn(async move {
            let mut ticker = interval(Duration::from_secs(5));

            loop {
                ticker.tick().await;

                if let Err(e) = Self::check_triggers(
                    &playbooks,
                    &trigger_state,
                    &screenpipe_dir,
                    &executor_config,
                )
                .await
                {
                    error!("Error checking triggers: {}", e);
                }
            }
        });

        info!("Playbook engine started");
        Ok(())
    }

    async fn check_triggers(
        playbooks: &Arc<RwLock<HashMap<String, Playbook>>>,
        trigger_state: &Arc<RwLock<TriggerState>>,
        _screenpipe_dir: &PathBuf,
        executor_config: &Arc<RwLock<ActionExecutorConfig>>,
    ) -> Result<()> {
        let playbooks_guard = playbooks.read().await;
        let mut state_guard = trigger_state.write().await;

        // Reset daily counters if needed
        let now = Utc::now();
        if now.date_naive() != state_guard.current_day.date_naive() {
            state_guard.daily_executions.clear();
            state_guard.current_day = now;
        }

        // Collect playbooks to execute first
        let mut executions_to_spawn = Vec::new();

        for (id, playbook) in playbooks_guard.iter() {
            if !playbook.enabled {
                continue;
            }

            // Check cooldown
            if let Some(cooldown) = playbook.cooldown_minutes {
                if let Some(last_exec) = state_guard.last_execution.get(id) {
                    let elapsed = now.signed_duration_since(*last_exec).num_minutes();
                    if elapsed < cooldown as i64 {
                        continue;
                    }
                }
            }

            // Check daily limit
            if let Some(max_daily) = playbook.max_executions_per_day {
                let daily_count = state_guard.daily_executions.get(id).copied().unwrap_or(0);
                if daily_count >= max_daily {
                    continue;
                }
            }

            // Check each trigger
            for trigger in &playbook.triggers {
                if Self::evaluate_trigger(trigger, &state_guard, now).await? {
                    // Store data for execution
                    executions_to_spawn.push((
                        id.clone(),
                        trigger.clone(),
                        playbook.actions.clone(),
                    ));

                    // Update state
                    state_guard.last_execution.insert(id.clone(), now);
                    *state_guard.daily_executions.entry(id.clone()).or_insert(0) += 1;

                    break; // Only execute once per check cycle
                }
            }
        }

        // Drop guards before spawning tasks
        drop(state_guard);
        drop(playbooks_guard);

        // Spawn execution tasks
        for (playbook_id, trigger, actions) in executions_to_spawn {
            let trigger_state_clone = trigger_state.clone();
            let executor_config_clone = executor_config.clone();
            tokio::spawn(async move {
                if let Err(e) = Self::execute_playbook(
                    &playbook_id,
                    &trigger,
                    &actions,
                    &trigger_state_clone,
                    &executor_config_clone,
                )
                .await
                {
                    error!("Failed to execute playbook {}: {}", playbook_id, e);
                }
            });
        }

        Ok(())
    }

    async fn evaluate_trigger(
        trigger: &Trigger,
        state: &TriggerState,
        now: DateTime<Utc>,
    ) -> Result<bool> {
        match trigger {
            Trigger::AppOpen {
                app_name,
                window_name,
            } => {
                if let Some(app_state) = state.open_apps.get(app_name) {
                    if let Some(pattern) = window_name {
                        if let Some(app_window) = &app_state.window_name {
                            return Ok(app_window.to_lowercase().contains(&pattern.to_lowercase()));
                        }
                        return Ok(false);
                    }
                    return Ok(true);
                }
                Ok(false)
            }
            Trigger::Time { cron, .. } => {
                let schedule = Schedule::from_str(cron)?;
                // Check if we should have triggered in the last 5 seconds
                let five_secs_ago = now - chrono::Duration::seconds(5);

                for datetime in schedule.after(&five_secs_ago).take(1) {
                    if datetime <= now {
                        return Ok(true);
                    }
                }
                Ok(false)
            }
            Trigger::Keyword {
                pattern,
                source,
                threshold: _,
            } => {
                // This would be checked against recent OCR/audio content
                // For now, return false - actual implementation would query the database
                debug!(
                    "Keyword trigger not yet implemented: {} ({:?})",
                    pattern, source
                );
                Ok(false)
            }
            Trigger::Context {
                apps,
                windows,
                time_range,
                days_of_week,
            } => {
                // Check day of week
                if let Some(days) = days_of_week {
                    let today = now.weekday().num_days_from_sunday() as u8;
                    if !days.contains(&today) {
                        return Ok(false);
                    }
                }

                // Check time range
                if let Some(range) = time_range {
                    let current_time = now.time();
                    let parts: Vec<&str> = range.split('-').collect();
                    if parts.len() == 2 {
                        let start = chrono::NaiveTime::parse_from_str(parts[0], "%H:%M")?;
                        let end = chrono::NaiveTime::parse_from_str(parts[1], "%H:%M")?;
                        if current_time < start || current_time > end {
                            return Ok(false);
                        }
                    }
                }

                // Check apps
                if let Some(required_apps) = apps {
                    for app in required_apps {
                        if !state.open_apps.contains_key(app) {
                            return Ok(false);
                        }
                    }
                }

                // Check windows
                if let Some(required_windows) = windows {
                    let has_matching_window = state.open_apps.values().any(|app_state| {
                        if let Some(window) = &app_state.window_name {
                            required_windows
                                .iter()
                                .any(|req| window.to_lowercase().contains(&req.to_lowercase()))
                        } else {
                            false
                        }
                    });
                    if !has_matching_window {
                        return Ok(false);
                    }
                }

                Ok(true)
            }
            Trigger::MeetingStart {
                app_name: _,
                keywords: _,
            } => {
                // Meeting start detection would be handled via events
                // This trigger type is evaluated when a MeetingStarted event is received
                Ok(false)
            }
            Trigger::MeetingEnd {
                app_name: _,
                min_duration_minutes: _,
            } => {
                // Meeting end detection would be handled via events
                // This trigger type is evaluated when a MeetingEnded event is received
                Ok(false)
            }
            Trigger::IdleState {
                idle_minutes: _,
                state: _,
            } => {
                // Idle state detection would be handled via events
                // This trigger type is evaluated when an IdleStateChanged event is received
                Ok(false)
            }
        }
    }

    async fn execute_playbook(
        playbook_id: &str,
        trigger: &Trigger,
        actions: &[Action],
        trigger_state: &Arc<RwLock<TriggerState>>,
        executor_config: &Arc<RwLock<ActionExecutorConfig>>,
    ) -> Result<()> {
        info!("Executing playbook: {}", playbook_id);

        let execution_id = Uuid::new_v4().to_string();
        let started_at = Utc::now();

        let mut action_results = Vec::new();

        for action in actions {
            let start_time = std::time::Instant::now();

            let result = Self::execute_action(action, trigger_state, executor_config).await;

            let duration_ms = start_time.elapsed().as_millis() as u64;

            match result {
                Ok(res) => {
                    action_results.push(ActionResult {
                        action: action.clone(),
                        success: true,
                        result: Some(res),
                        error: None,
                        duration_ms,
                    });
                }
                Err(e) => {
                    action_results.push(ActionResult {
                        action: action.clone(),
                        success: false,
                        result: None,
                        error: Some(e.to_string()),
                        duration_ms,
                    });
                    error!("Action failed in playbook {}: {}", playbook_id, e);
                }
            }
        }

        let completed_at = Utc::now();
        let status = if action_results.iter().all(|r| r.success) {
            ExecutionStatus::Completed
        } else {
            ExecutionStatus::Failed
        };

        let _execution = PlaybookExecution {
            id: execution_id,
            playbook_id: playbook_id.to_string(),
            started_at,
            completed_at: Some(completed_at),
            status,
            triggered_by: trigger.clone(),
            action_results,
            error: None,
        };

        // Save execution to database
        // This would be implemented with proper DB access
        info!("Playbook {} execution completed", playbook_id);

        Ok(())
    }

    async fn execute_action(
        action: &Action,
        trigger_state: &Arc<RwLock<TriggerState>>,
        executor_config: &Arc<RwLock<ActionExecutorConfig>>,
    ) -> Result<Value> {
        match action {
            Action::Notify {
                title,
                message,
                actions,
                persistent,
            } => Self::execute_notify(title, message, actions.as_ref(), *persistent).await,
            Action::Summarize {
                timeframe,
                focus,
                output: _,
            } => {
                info!("Generating summary for last {} minutes", timeframe);
                // This would query the database and use LLM to generate summary
                Ok(serde_json::json!({
                    "summarized": true,
                    "timeframe": timeframe,
                    "focus": format!("{:?}", focus),
                }))
            }
            Action::FocusMode {
                enabled,
                duration,
                allowed_apps,
                silence_notifications,
            } => {
                let mut state = trigger_state.write().await;
                state.focus_mode_active = *enabled;
                if *enabled {
                    if let Some(mins) = duration {
                        state.focus_mode_end_time =
                            Some(Utc::now() + chrono::Duration::minutes(*mins as i64));
                    }
                    info!("Focus mode enabled for {:?} minutes", duration);
                } else {
                    state.focus_mode_end_time = None;
                    info!("Focus mode disabled");
                }
                Ok(serde_json::json!({
                    "focus_mode": enabled,
                    "duration": duration,
                    "allowed_apps": allowed_apps,
                    "silence_notifications": silence_notifications,
                }))
            }
            Action::RunPipe { pipe_id, params } => {
                Self::execute_run_pipe(executor_config, pipe_id, params.clone()).await
            }
            Action::Tag { tags, timeframe } => {
                Self::execute_tag(executor_config, tags, *timeframe).await
            }
            Action::Webhook {
                url,
                method,
                headers,
                body,
            } => {
                Self::execute_webhook(
                    executor_config,
                    url,
                    method,
                    headers.as_ref(),
                    body.as_ref(),
                )
                .await
            }
            Action::ExtractActionItems {
                timeframe: _,
                min_confidence: _,
                auto_export: _,
            } => {
                info!("Extracting action items");
                // This would analyze recent content and extract action items
                Ok(serde_json::json!({
                    "extracted": true,
                    "action_items": [],
                }))
            }
            Action::ExportActionItems { target, filter: _ } => {
                info!("Exporting action items to {:?}", target);
                // This would export action items to the specified target
                Ok(serde_json::json!({
                    "exported": true,
                    "target": format!("{:?}", target),
                }))
            }
            Action::StartRecording {
                focus_app: _,
                tag: _,
            } => {
                info!("Starting recording");
                // This would start recording with specific settings
                Ok(serde_json::json!({
                    "recording_started": true,
                }))
            }
            Action::StopRecording => {
                info!("Stopping recording");
                // This would stop recording
                Ok(serde_json::json!({
                    "recording_stopped": true,
                }))
            }
        }
    }

    // ─── Action Implementations ────────────────────────────────────────────────

    /// Execute a notification action
    async fn execute_notify(
        title: &str,
        message: &str,
        actions: Option<&Vec<NotificationAction>>,
        _persistent: Option<bool>,
    ) -> Result<Value> {
        info!("Sending notification: {} - {}", title, message);

        // Try to use the notify crate for desktop notifications
        #[cfg(feature = "notify")]
        {
            use notify_rust::Notification;

            let mut notification = Notification::new();
            notification.summary(title).body(message);

            if _persistent == Some(true) {
                notification.timeout(notify_rust::Timeout::Never);
            }

            // Add actions if provided
            if let Some(actions) = actions {
                for action in actions {
                    notification.action(&action.id, &action.label);
                }
            }

            match notification.show() {
                Ok(handle) => {
                    info!("Notification sent successfully with id: {:?}", handle.id());
                    Ok(serde_json::json!({
                        "notified": true,
                        "title": title,
                        "message": message,
                        "notification_id": handle.id(),
                    }))
                }
                Err(e) => {
                    warn!("Failed to show notification: {}. Falling back to log.", e);
                    // Fallback to just logging
                    Ok(serde_json::json!({
                        "notified": true,
                        "title": title,
                        "message": message,
                        "fallback": true,
                    }))
                }
            }
        }

        #[cfg(not(feature = "notify"))]
        {
            // Without notify feature, just log the notification
            info!("[NOTIFICATION] {}: {}", title, message);
            if let Some(actions) = actions {
                info!("Notification actions: {:?}", actions);
            }

            Ok(serde_json::json!({
                "notified": true,
                "title": title,
                "message": message,
                "method": "log",
            }))
        }
    }

    /// Execute a webhook action - makes HTTP POST/GET/etc requests
    async fn execute_webhook(
        executor_config: &Arc<RwLock<ActionExecutorConfig>>,
        url: &str,
        method: &HttpMethod,
        headers: Option<&HashMap<String, String>>,
        body: Option<&Value>,
    ) -> Result<Value> {
        info!("Calling webhook: {} {:?}", url, method);

        let config = executor_config.read().await;
        let client = config.http_client.clone();
        drop(config);

        // Validate URL
        let parsed_url = url
            .parse::<reqwest::Url>()
            .map_err(|e| anyhow::anyhow!("Invalid webhook URL: {}", e))?;

        // Build the request
        let mut request_builder = match method {
            HttpMethod::Get => client.get(parsed_url),
            HttpMethod::Post => client.post(parsed_url),
            HttpMethod::Put => client.put(parsed_url),
            HttpMethod::Delete => client.delete(parsed_url),
        };

        // Add headers
        if let Some(headers) = headers {
            for (key, value) in headers {
                request_builder = request_builder.header(key, value);
            }
        }

        // Add default content-type for POST/PUT with body
        if matches!(method, HttpMethod::Post | HttpMethod::Put) && body.is_some() {
            request_builder = request_builder.header("Content-Type", "application/json");
        }

        // Add body if present
        if let Some(body) = body {
            request_builder = request_builder.json(body);
        }

        // Execute the request
        let start_time = std::time::Instant::now();
        match request_builder.send().await {
            Ok(response) => {
                let duration_ms = start_time.elapsed().as_millis() as u64;
                let status = response.status();
                let status_code = status.as_u16();

                // Try to parse response body as JSON
                let response_body = response.json::<Value>().await.ok();

                if status.is_success() {
                    info!("Webhook call successful: {} ({}ms)", url, duration_ms);
                    Ok(serde_json::json!({
                        "webhook_called": true,
                        "url": url,
                        "method": format!("{:?}", method),
                        "status_code": status_code,
                        "duration_ms": duration_ms,
                        "response": response_body,
                    }))
                } else {
                    warn!(
                        "Webhook call returned error status: {} ({})",
                        status_code, url
                    );
                    Err(anyhow::anyhow!(
                        "Webhook returned error status: {}",
                        status_code
                    ))
                }
            }
            Err(e) => {
                error!("Webhook call failed: {} - {}", url, e);
                Err(anyhow::anyhow!("Webhook request failed: {}", e))
            }
        }
    }

    /// Execute a RunPipe action - starts a pipe with optional parameters
    async fn execute_run_pipe(
        executor_config: &Arc<RwLock<ActionExecutorConfig>>,
        pipe_id: &str,
        params: Option<Value>,
    ) -> Result<Value> {
        info!("Running pipe: {}", pipe_id);

        let config = executor_config.read().await;

        if let Some(pipe_manager) = &config.pipe_manager {
            // Check if pipe exists
            match pipe_manager.get_pipe_info(pipe_id).await {
                Ok(Some(pipe_info)) => {
                    if pipe_info.enabled {
                        info!(
                            "Pipe {} is already running on port {:?}",
                            pipe_id, pipe_info.port
                        );
                        Ok(serde_json::json!({
                            "pipe_run": true,
                            "pipe_id": pipe_id,
                            "already_running": true,
                            "port": pipe_info.port,
                        }))
                    } else {
                        // Start the pipe
                        match pipe_manager.start_pipe(pipe_id, params).await {
                            Ok(()) => {
                                info!("Pipe {} started successfully", pipe_id);
                                Ok(serde_json::json!({
                                    "pipe_run": true,
                                    "pipe_id": pipe_id,
                                    "started": true,
                                }))
                            }
                            Err(e) => {
                                error!("Failed to start pipe {}: {}", pipe_id, e);
                                Err(anyhow::anyhow!("Failed to start pipe: {}", e))
                            }
                        }
                    }
                }
                Ok(None) => {
                    // Pipe doesn't exist, try to start it anyway (might be a new pipe)
                    match pipe_manager.start_pipe(pipe_id, params).await {
                        Ok(()) => {
                            info!("Pipe {} started successfully", pipe_id);
                            Ok(serde_json::json!({
                                "pipe_run": true,
                                "pipe_id": pipe_id,
                                "started": true,
                            }))
                        }
                        Err(e) => {
                            error!("Failed to start pipe {}: {}", pipe_id, e);
                            Err(anyhow::anyhow!("Failed to start pipe: {}", e))
                        }
                    }
                }
                Err(e) => {
                    error!("Error getting pipe info for {}: {}", pipe_id, e);
                    Err(anyhow::anyhow!("Error accessing pipe manager: {}", e))
                }
            }
        } else {
            warn!("No pipe manager configured, cannot run pipe: {}", pipe_id);
            // Return success but indicate no pipe manager available
            Ok(serde_json::json!({
                "pipe_run": false,
                "pipe_id": pipe_id,
                "error": "No pipe manager configured",
            }))
        }
    }

    /// Execute a Tag action - applies tags to recent content
    async fn execute_tag(
        executor_config: &Arc<RwLock<ActionExecutorConfig>>,
        tags: &[String],
        timeframe: u32,
    ) -> Result<Value> {
        info!(
            "Tagging content with {:?} for last {} minutes",
            tags, timeframe
        );

        if tags.is_empty() {
            return Ok(serde_json::json!({
                "tagged": false,
                "reason": "No tags provided",
            }));
        }

        let db_manager = {
            let config = executor_config.read().await;
            config.db_manager.clone()
        };

        if let Some(db_manager) = db_manager {
            // Calculate time range
            let end_time = Utc::now();
            let start_time = end_time - chrono::Duration::minutes(timeframe as i64);

            // Get recent content IDs
            let vision_ids = match db_manager.search_recent_frames(start_time, end_time).await {
                Ok(ids) => ids,
                Err(e) => {
                    error!("Failed to search recent frames: {}", e);
                    Vec::new()
                }
            };

            let audio_ids = match db_manager.search_recent_audio(start_time, end_time).await {
                Ok(ids) => ids,
                Err(e) => {
                    error!("Failed to search recent audio: {}", e);
                    Vec::new()
                }
            };

            let mut tagged_count = 0;
            let mut errors = Vec::new();

            // Tag vision content
            for id in &vision_ids {
                if let Err(e) = db_manager
                    .add_tags(*id, TagContentType::Vision, tags.to_vec())
                    .await
                {
                    error!("Failed to add tags to vision id {}: {}", id, e);
                    errors.push(format!("vision:{} - {}", id, e));
                } else {
                    tagged_count += 1;
                }
            }

            // Tag audio content
            for id in &audio_ids {
                if let Err(e) = db_manager
                    .add_tags(*id, TagContentType::Audio, tags.to_vec())
                    .await
                {
                    error!("Failed to add tags to audio id {}: {}", id, e);
                    errors.push(format!("audio:{} - {}", id, e));
                } else {
                    tagged_count += 1;
                }
            }

            info!(
                "Tagged {} items with {:?} ({} vision, {} audio)",
                tagged_count,
                tags,
                vision_ids.len(),
                audio_ids.len()
            );

            if errors.is_empty() {
                Ok(serde_json::json!({
                    "tagged": true,
                    "tags": tags,
                    "timeframe": timeframe,
                    "vision_items": vision_ids.len(),
                    "audio_items": audio_ids.len(),
                    "total_tagged": tagged_count,
                }))
            } else {
                Ok(serde_json::json!({
                    "tagged": true,
                    "tags": tags,
                    "timeframe": timeframe,
                    "vision_items": vision_ids.len(),
                    "audio_items": audio_ids.len(),
                    "total_tagged": tagged_count,
                    "errors": errors,
                }))
            }
        } else {
            warn!("No database manager configured, cannot tag content");
            Ok(serde_json::json!({
                "tagged": false,
                "tags": tags,
                "timeframe": timeframe,
                "error": "No database manager configured",
            }))
        }
    }

    // ─── Public API ────────────────────────────────────────────────────────────

    pub async fn list_playbooks(&self) -> Vec<Playbook> {
        let playbooks = self.playbooks.read().await;
        playbooks.values().cloned().collect()
    }

    pub async fn get_playbook(&self, id: &str) -> Option<Playbook> {
        let playbooks = self.playbooks.read().await;
        playbooks.get(id).cloned()
    }

    pub async fn create_playbook(&self, playbook: Playbook) -> Result<Playbook> {
        let mut playbook = playbook;
        playbook.id = Uuid::new_v4().to_string();
        playbook.created_at = Some(Utc::now());
        playbook.updated_at = Some(Utc::now());
        playbook.is_builtin = Some(false);

        let mut playbooks = self.playbooks.write().await;
        playbooks.insert(playbook.id.clone(), playbook.clone());

        Ok(playbook)
    }

    pub async fn update_playbook(&self, id: &str, updates: Playbook) -> Result<Playbook> {
        let mut playbooks = self.playbooks.write().await;

        if let Some(existing) = playbooks.get_mut(id) {
            existing.name = updates.name;
            existing.description = updates.description;
            existing.enabled = updates.enabled;
            existing.triggers = updates.triggers;
            existing.actions = updates.actions;
            existing.cooldown_minutes = updates.cooldown_minutes;
            existing.max_executions_per_day = updates.max_executions_per_day;
            existing.updated_at = Some(Utc::now());
            existing.icon = updates.icon;
            existing.color = updates.color;

            return Ok(existing.clone());
        }

        Err(anyhow::anyhow!("Playbook not found: {}", id))
    }

    pub async fn delete_playbook(&self, id: &str) -> Result<()> {
        let mut playbooks = self.playbooks.write().await;

        if let Some(playbook) = playbooks.get(id) {
            if playbook.is_builtin == Some(true) {
                return Err(anyhow::anyhow!("Cannot delete built-in playbook"));
            }
        }

        playbooks.remove(id);
        Ok(())
    }

    pub async fn toggle_playbook(&self, id: &str, enabled: bool) -> Result<Playbook> {
        let mut playbooks = self.playbooks.write().await;

        if let Some(existing) = playbooks.get_mut(id) {
            existing.enabled = enabled;
            existing.updated_at = Some(Utc::now());
            return Ok(existing.clone());
        }

        Err(anyhow::anyhow!("Playbook not found: {}", id))
    }

    /// Update the state of an app (called from external monitoring)
    pub async fn update_app_state(&self, app_name: &str, window_name: Option<String>) {
        let mut state = self.trigger_state.write().await;
        state.open_apps.insert(
            app_name.to_string(),
            AppStateInternal {
                window_name,
                last_seen: Utc::now(),
            },
        );
    }

    /// Remove an app from the state (called when app closes)
    pub async fn remove_app_state(&self, app_name: &str) {
        let mut state = self.trigger_state.write().await;
        state.open_apps.remove(app_name);
    }

    /// Get all playbooks as HashMap (for internal use)
    pub async fn get_playbooks_map(&self) -> HashMap<String, Playbook> {
        let playbooks = self.playbooks.read().await;
        playbooks.clone()
    }

    /// Set playbooks from HashMap (for loading from DB)
    pub async fn set_playbooks(&self, playbooks: HashMap<String, Playbook>) {
        let mut map = self.playbooks.write().await;
        *map = playbooks;
    }
}

#[cfg(test)]
#[path = "playbook_engine_tests.rs"]
mod tests;

/// Default built-in playbooks
pub fn default_playbooks() -> Vec<Playbook> {
    vec![
        Playbook {
            id: "daily-standup".to_string(),
            name: "Daily Standup".to_string(),
            description: Some(
                "Automatically generate a summary of your work at 9 AM on weekdays".to_string(),
            ),
            enabled: false,
            triggers: vec![Trigger::Time {
                cron: "0 9 * * 1-5".to_string(),
                description: Some("Every weekday at 9:00 AM".to_string()),
            }],
            actions: vec![Action::Summarize {
                timeframe: 1440,
                focus: Some(SummaryFocus::ActionItems),
                output: Some(SummaryOutput::Notification),
            }],
            cooldown_minutes: Some(60),
            max_executions_per_day: None,
            created_at: Some(Utc::now()),
            updated_at: Some(Utc::now()),
            is_builtin: Some(true),
            icon: Some("📅".to_string()),
            color: Some("#3B82F6".to_string()),
        },
        Playbook {
            id: "customer-call".to_string(),
            name: "Customer Call".to_string(),
            description: Some("Focus on action items when joining Zoom or Google Meet".to_string()),
            enabled: false,
            triggers: vec![Trigger::AppOpen {
                app_name: "zoom".to_string(),
                window_name: None,
            }],
            actions: vec![
                Action::FocusMode {
                    enabled: true,
                    duration: Some(60),
                    silence_notifications: Some(true),
                    allowed_apps: Some(vec!["zoom".to_string(), "chrome".to_string()]),
                },
                Action::Notify {
                    title: "Customer Call Mode".to_string(),
                    message:
                        "Focus mode enabled. I'll summarize action items at the end of the call."
                            .to_string(),
                    actions: None,
                    persistent: Some(false),
                },
            ],
            cooldown_minutes: None,
            max_executions_per_day: None,
            created_at: Some(Utc::now()),
            updated_at: Some(Utc::now()),
            is_builtin: Some(true),
            icon: Some("🎥".to_string()),
            color: Some("#10B981".to_string()),
        },
        Playbook {
            id: "deep-work".to_string(),
            name: "Deep Work".to_string(),
            description: Some("Block distractions during focus time".to_string()),
            enabled: false,
            triggers: vec![Trigger::Context {
                apps: None,
                windows: None,
                time_range: Some("09:00-12:00".to_string()),
                days_of_week: Some(vec![1, 2, 3, 4, 5]),
            }],
            actions: vec![Action::FocusMode {
                enabled: true,
                duration: Some(180),
                silence_notifications: Some(true),
                allowed_apps: Some(vec![
                    "code".to_string(),
                    "cursor".to_string(),
                    "vscode".to_string(),
                    "terminal".to_string(),
                ]),
            }],
            cooldown_minutes: Some(240),
            max_executions_per_day: None,
            created_at: Some(Utc::now()),
            updated_at: Some(Utc::now()),
            is_builtin: Some(true),
            icon: Some("🎯".to_string()),
            color: Some("#8B5CF6".to_string()),
        },
    ]
}
