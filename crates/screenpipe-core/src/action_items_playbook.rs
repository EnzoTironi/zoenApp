//! Action Items Playbook Integration
//!
//! This module provides playbook triggers and actions for automatic
//! action item extraction from meeting transcripts.

use crate::action_items::{
    extract_action_items_cached, to_notion_format, to_todoist_format, ActionItem, ActionItemCache,
    ActionItemSource, ExportFormat, LLM,
};
use anyhow::{Context, Result};
use chrono::{DateTime, Duration, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use tracing::{debug, error, info};

/// Configuration for the action items playbook
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ActionItemsPlaybookConfig {
    /// Minimum duration of meeting to trigger extraction (in minutes)
    pub min_meeting_duration_minutes: i64,
    /// Cooldown period between extractions for same source (in minutes)
    pub extraction_cooldown_minutes: i64,
    /// Minimum confidence threshold for action items
    pub min_confidence_threshold: f32,
    /// Auto-export to external services
    pub auto_export: Vec<ExportConfig>,
    /// Notify user when action items are extracted
    pub notify_user: bool,
}

impl Default for ActionItemsPlaybookConfig {
    fn default() -> Self {
        Self {
            min_meeting_duration_minutes: 5,
            extraction_cooldown_minutes: 30,
            min_confidence_threshold: 0.7,
            auto_export: Vec::new(),
            notify_user: true,
        }
    }
}

/// Export configuration for external services
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExportConfig {
    pub format: ExportFormat,
    pub webhook_url: Option<String>,
    pub api_token: Option<String>,
    pub enabled: bool,
}

/// Meeting detection state
#[derive(Debug, Clone)]
pub struct MeetingState {
    pub source_id: String,
    pub started_at: DateTime<Utc>,
    pub last_activity: DateTime<Utc>,
    pub transcript_buffer: Vec<String>,
    pub is_active: bool,
    pub participant_count: Option<usize>,
    pub metadata: HashMap<String, serde_json::Value>,
}

impl MeetingState {
    pub fn new(source_id: impl Into<String>) -> Self {
        let now = Utc::now();
        Self {
            source_id: source_id.into(),
            started_at: now,
            last_activity: now,
            transcript_buffer: Vec::new(),
            is_active: true,
            participant_count: None,
            metadata: HashMap::new(),
        }
    }

    pub fn duration(&self) -> Duration {
        Utc::now() - self.started_at
    }

    pub fn idle_duration(&self) -> Duration {
        Utc::now() - self.last_activity
    }

    pub fn add_transcript(&mut self, text: impl Into<String>) {
        self.transcript_buffer.push(text.into());
        self.last_activity = Utc::now();
    }

    pub fn full_transcript(&self) -> String {
        self.transcript_buffer.join("\n")
    }
}

/// Playbook engine for action items extraction
pub struct ActionItemsPlaybook {
    config: ActionItemsPlaybookConfig,
    active_meetings: HashMap<String, MeetingState>,
    extraction_cache: ActionItemCache,
    last_extraction: HashMap<String, DateTime<Utc>>,
}

impl ActionItemsPlaybook {
    pub fn new(config: ActionItemsPlaybookConfig) -> Self {
        Self {
            config,
            active_meetings: HashMap::new(),
            extraction_cache: ActionItemCache::default(),
            last_extraction: HashMap::new(),
        }
    }

    /// Process a transcription event
    /// Returns true if action items were extracted
    pub async fn process_transcription(
        &mut self,
        source_id: &str,
        transcript: &str,
        llm: &dyn LLM,
    ) -> Result<Option<Vec<ActionItem>>> {
        // Check if this is a new meeting or continuation
        let is_new_meeting = self.detect_meeting_start(source_id, transcript);

        if is_new_meeting {
            info!("Detected new meeting: {}", source_id);
            self.start_meeting(source_id);
        }

        // Add transcript to buffer
        if let Some(meeting) = self.active_meetings.get_mut(source_id) {
            meeting.add_transcript(transcript);
        }

        // Check if meeting has ended
        if self.detect_meeting_end(source_id) {
            info!("Detected end of meeting: {}", source_id);
            return self.extract_action_items(source_id, llm).await.map(Some);
        }

        Ok(None)
    }

    /// Start tracking a new meeting
    pub fn start_meeting(&mut self, source_id: &str) {
        let state = MeetingState::new(source_id);
        self.active_meetings.insert(source_id.to_string(), state);
    }

    /// End a meeting and extract action items
    pub async fn end_meeting(
        &mut self,
        source_id: &str,
        llm: &dyn LLM,
    ) -> Result<Option<Vec<ActionItem>>> {
        if let Some(meeting) = self.active_meetings.remove(source_id) {
            if !meeting.is_active {
                return Ok(None);
            }

            let duration_minutes = meeting.duration().num_minutes();
            if duration_minutes < self.config.min_meeting_duration_minutes {
                debug!(
                    "Meeting {} too short ({} min), skipping extraction",
                    source_id, duration_minutes
                );
                return Ok(None);
            }

            self.extract_from_meeting(&meeting, llm).await.map(Some)
        } else {
            Ok(None)
        }
    }

    /// Detect if a new meeting has started
    fn detect_meeting_start(&self, source_id: &str, transcript: &str) -> bool {
        // Check if we already have an active meeting for this source
        if let Some(meeting) = self.active_meetings.get(source_id) {
            // If meeting has been idle for too long, consider it a new meeting
            let idle_threshold = Duration::minutes(10);
            if meeting.idle_duration() > idle_threshold {
                return true;
            }
            return false;
        }

        // Check for meeting start indicators in transcript
        let start_indicators = [
            "let's start",
            "meeting started",
            "call started",
            "begin the",
            "welcome everyone",
            "let's get started",
            "meeting is now",
        ];

        let lower = transcript.to_lowercase();
        start_indicators
            .iter()
            .any(|indicator| lower.contains(indicator))
    }

    /// Detect if a meeting has ended
    fn detect_meeting_end(&self, source_id: &str) -> bool {
        if let Some(meeting) = self.active_meetings.get(source_id) {
            // Check for idle timeout (no activity for 5 minutes)
            let idle_timeout = Duration::minutes(5);
            if meeting.idle_duration() > idle_timeout {
                return true;
            }

            // Check for end indicators in recent transcript
            let end_indicators = [
                "let's wrap up",
                "meeting ended",
                "call ended",
                "thank you everyone",
                "that's all for today",
                "see you next time",
                "goodbye",
                "end of meeting",
            ];

            // Check last few transcript entries
            let recent_transcript: String = meeting
                .transcript_buffer
                .iter()
                .rev()
                .take(5)
                .cloned()
                .collect::<Vec<_>>()
                .join(" ");

            let lower = recent_transcript.to_lowercase();
            if end_indicators
                .iter()
                .any(|indicator| lower.contains(indicator))
            {
                return true;
            }
        }

        false
    }

    /// Extract action items from a meeting
    async fn extract_from_meeting(
        &mut self,
        meeting: &MeetingState,
        llm: &dyn LLM,
    ) -> Result<Vec<ActionItem>> {
        let transcript = meeting.full_transcript();

        // Check cooldown
        if let Some(last_time) = self.last_extraction.get(&meeting.source_id) {
            let cooldown = Duration::minutes(self.config.extraction_cooldown_minutes);
            if Utc::now() - *last_time < cooldown {
                debug!("Extraction cooldown active for {}", meeting.source_id);
                return Ok(Vec::new());
            }
        }

        // Extract action items with caching
        let items = extract_action_items_cached(
            &transcript,
            llm as &dyn LLM,
            ActionItemSource::Meeting,
            Some(meeting.source_id.clone()),
            &mut self.extraction_cache,
        )
        .await?;

        // Filter by confidence threshold
        let filtered_items: Vec<ActionItem> = items
            .into_iter()
            .filter(|item| item.confidence >= self.config.min_confidence_threshold)
            .collect();

        // Update last extraction time
        self.last_extraction
            .insert(meeting.source_id.clone(), Utc::now());

        info!(
            "Extracted {} action items from meeting {}",
            filtered_items.len(),
            meeting.source_id
        );

        // Auto-export if configured
        if !filtered_items.is_empty() {
            self.auto_export_items(&filtered_items).await?;
        }

        Ok(filtered_items)
    }

    /// Extract action items for a specific source
    async fn extract_action_items(
        &mut self,
        source_id: &str,
        llm: &dyn LLM,
    ) -> Result<Vec<ActionItem>> {
        // Clone the meeting to avoid borrow issues
        let meeting = self.active_meetings.get(source_id).cloned();
        if let Some(meeting) = meeting {
            self.extract_from_meeting(&meeting, llm).await
        } else {
            Ok(Vec::new())
        }
    }

    /// Auto-export action items to configured services
    async fn auto_export_items(&self, items: &[ActionItem]) -> Result<()> {
        for config in &self.config.auto_export {
            if !config.enabled {
                continue;
            }

            match self.export_items(items, config).await {
                Ok(_) => debug!("Exported action items to {:?}", config.format),
                Err(e) => error!("Failed to export action items: {}", e),
            }
        }

        Ok(())
    }

    /// Export action items to a specific service
    async fn export_items(&self, items: &[ActionItem], config: &ExportConfig) -> Result<()> {
        match config.format {
            ExportFormat::Todoist => {
                let todoist_items = to_todoist_format(items);
                if let Some(webhook_url) = &config.webhook_url {
                    self.send_webhook(webhook_url, &todoist_items).await?;
                }
            }
            ExportFormat::Notion => {
                let notion_items = to_notion_format(items);
                if let Some(webhook_url) = &config.webhook_url {
                    self.send_webhook(webhook_url, &notion_items).await?;
                }
            }
            ExportFormat::Webhook | ExportFormat::Json => {
                if let Some(webhook_url) = &config.webhook_url {
                    self.send_webhook(webhook_url, items).await?;
                }
            }
        }

        Ok(())
    }

    /// Send webhook payload
    async fn send_webhook(&self, url: &str, payload: &(impl Serialize + ?Sized)) -> Result<()> {
        let client = reqwest::Client::new();
        let response = client
            .post(url)
            .json(payload)
            .send()
            .await
            .context("Failed to send webhook")?;

        if !response.status().is_success() {
            anyhow::bail!("Webhook returned error: {}", response.status());
        }

        Ok(())
    }

    /// Get active meetings
    pub fn get_active_meetings(&self) -> &HashMap<String, MeetingState> {
        &self.active_meetings
    }

    /// Force end all active meetings
    pub async fn end_all_meetings(
        &mut self,
        llm: &dyn LLM,
    ) -> Result<HashMap<String, Vec<ActionItem>>> {
        let mut results = HashMap::new();
        let source_ids: Vec<String> = self.active_meetings.keys().cloned().collect();

        for source_id in source_ids {
            if let Some(items) = self.end_meeting(&source_id, llm).await? {
                if !items.is_empty() {
                    results.insert(source_id, items);
                }
            }
        }

        Ok(results)
    }

    /// Update configuration
    pub fn update_config(&mut self, config: ActionItemsPlaybookConfig) {
        self.config = config;
    }

    /// Get current configuration
    pub fn config(&self) -> &ActionItemsPlaybookConfig {
        &self.config
    }
}

/// Notification payload for action items extraction
#[derive(Debug, Clone, Serialize)]
pub struct ActionItemsNotification {
    pub source_id: String,
    pub source_type: String,
    pub item_count: usize,
    pub items: Vec<ActionItemSummary>,
    pub extracted_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize)]
pub struct ActionItemSummary {
    pub text: String,
    pub assignee: Option<String>,
    pub deadline: Option<DateTime<Utc>>,
    pub priority: String,
}

impl From<&ActionItem> for ActionItemSummary {
    fn from(item: &ActionItem) -> Self {
        Self {
            text: item.text.clone(),
            assignee: item.assignee.clone(),
            deadline: item.deadline,
            priority: format!("{:?}", item.priority),
        }
    }
}

impl ActionItemsNotification {
    pub fn new(source_id: String, items: &[ActionItem]) -> Self {
        Self {
            source_id,
            source_type: "meeting".to_string(),
            item_count: items.len(),
            items: items.iter().map(ActionItemSummary::from).collect(),
            extracted_at: Utc::now(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::action_items::{ActionItemPriority, ActionItemStatus};

    struct MockLLM;

    #[async_trait::async_trait]
    impl LLM for MockLLM {
        async fn complete(&self, _prompt: &str) -> Result<String> {
            Ok(r#"[
                {
                    "text": "Complete project proposal",
                    "assignee": "Alice",
                    "deadline": "2024-02-15",
                    "priority": "high",
                    "confidence": 0.95
                },
                {
                    "text": "Review code changes",
                    "assignee": "Bob",
                    "priority": "medium",
                    "confidence": 0.85
                }
            ]"#
            .to_string())
        }
    }

    #[test]
    fn test_meeting_state() {
        let mut state = MeetingState::new("meeting-123");
        assert!(state.is_active);
        assert_eq!(state.transcript_buffer.len(), 0);

        state.add_transcript("Hello everyone");
        assert_eq!(state.transcript_buffer.len(), 1);

        let full = state.full_transcript();
        assert!(full.contains("Hello everyone"));
    }

    #[test]
    fn test_detect_meeting_start() {
        let playbook = ActionItemsPlaybook::new(ActionItemsPlaybookConfig::default());

        assert!(playbook.detect_meeting_start("new-meeting", "Let's get started"));
        assert!(playbook.detect_meeting_start("new-meeting", "Welcome everyone to the call"));
        assert!(!playbook.detect_meeting_start("new-meeting", "Just a random message"));
    }

    #[tokio::test]
    async fn test_process_transcription() {
        let mut playbook = ActionItemsPlaybook::new(ActionItemsPlaybookConfig::default());
        let llm = MockLLM;

        // First transcription - starts meeting
        let result = playbook
            .process_transcription("meeting-1", "Let's get started", &llm)
            .await
            .unwrap();
        assert!(result.is_none());
        assert_eq!(playbook.get_active_meetings().len(), 1);

        // Add more transcriptions
        for i in 0..5 {
            let result = playbook
                .process_transcription("meeting-1", &format!("Message {}", i), &llm)
                .await
                .unwrap();
            assert!(result.is_none());
        }
    }

    #[test]
    fn test_action_items_notification() {
        let items = vec![ActionItem {
            id: uuid::Uuid::new_v4(),
            text: "Test task".to_string(),
            assignee: Some("John".to_string()),
            deadline: None,
            source: crate::action_items::ActionItemSource::Meeting,
            source_id: Some("meeting-1".to_string()),
            confidence: 0.9,
            status: ActionItemStatus::Pending,
            priority: ActionItemPriority::High,
            created_at: Utc::now(),
            updated_at: Utc::now(),
            completed_at: None,
            metadata: std::collections::HashMap::new(),
        }];

        let notification = ActionItemsNotification::new("meeting-1".to_string(), &items);
        assert_eq!(notification.item_count, 1);
        assert_eq!(notification.source_type, "meeting");
    }
}
