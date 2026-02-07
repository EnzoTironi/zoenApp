//! Action Items Extraction Module
//!
//! This module provides functionality to automatically extract action items
//! from meeting transcripts using LLM models.

use anyhow::{Context, Result};
use chrono::{DateTime, NaiveDateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use tracing::{debug, error, info, warn};
use uuid::Uuid;

/// Source of the action item
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ActionItemSource {
    Meeting,
    Email,
    Chat,
    Document,
    Other(String),
}

impl Default for ActionItemSource {
    fn default() -> Self {
        ActionItemSource::Meeting
    }
}

impl std::fmt::Display for ActionItemSource {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ActionItemSource::Meeting => write!(f, "meeting"),
            ActionItemSource::Email => write!(f, "email"),
            ActionItemSource::Chat => write!(f, "chat"),
            ActionItemSource::Document => write!(f, "document"),
            ActionItemSource::Other(s) => write!(f, "{}", s),
        }
    }
}

/// Status of an action item
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ActionItemStatus {
    Pending,
    InProgress,
    Done,
    Cancelled,
}

impl Default for ActionItemStatus {
    fn default() -> Self {
        ActionItemStatus::Pending
    }
}

/// Priority level for action items
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord)]
#[serde(rename_all = "snake_case")]
pub enum ActionItemPriority {
    Low,
    Medium,
    High,
    Critical,
}

impl Default for ActionItemPriority {
    fn default() -> Self {
        ActionItemPriority::Medium
    }
}

/// Represents an extracted action item
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ActionItem {
    pub id: Uuid,
    pub text: String,
    pub assignee: Option<String>,
    pub deadline: Option<DateTime<Utc>>,
    pub source: ActionItemSource,
    pub source_id: Option<String>, // e.g., meeting ID, email ID
    pub confidence: f32,
    pub status: ActionItemStatus,
    pub priority: ActionItemPriority,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub completed_at: Option<DateTime<Utc>>,
    pub metadata: HashMap<String, serde_json::Value>,
}

impl ActionItem {
    /// Create a new action item with generated ID
    pub fn new(text: impl Into<String>) -> Self {
        let now = Utc::now();
        Self {
            id: Uuid::new_v4(),
            text: text.into(),
            assignee: None,
            deadline: None,
            source: ActionItemSource::default(),
            source_id: None,
            confidence: 0.0,
            status: ActionItemStatus::Pending,
            priority: ActionItemPriority::Medium,
            created_at: now,
            updated_at: now,
            completed_at: None,
            metadata: HashMap::new(),
        }
    }

    /// Mark the action item as done
    pub fn mark_done(&mut self) {
        self.status = ActionItemStatus::Done;
        self.completed_at = Some(Utc::now());
        self.updated_at = Utc::now();
    }

    /// Mark the action item as in progress
    pub fn mark_in_progress(&mut self) {
        self.status = ActionItemStatus::InProgress;
        self.updated_at = Utc::now();
    }

    /// Update the priority
    pub fn with_priority(mut self, priority: ActionItemPriority) -> Self {
        self.priority = priority;
        self
    }

    /// Update the assignee
    pub fn with_assignee(mut self, assignee: impl Into<String>) -> Self {
        self.assignee = Some(assignee.into());
        self
    }

    /// Update the deadline
    pub fn with_deadline(mut self, deadline: DateTime<Utc>) -> Self {
        self.deadline = Some(deadline);
        self
    }

    /// Update the source
    pub fn with_source(mut self, source: ActionItemSource) -> Self {
        self.source = source;
        self
    }

    /// Update the source ID
    pub fn with_source_id(mut self, source_id: impl Into<String>) -> Self {
        self.source_id = Some(source_id.into());
        self
    }

    /// Update the confidence score
    pub fn with_confidence(mut self, confidence: f32) -> Self {
        self.confidence = confidence.clamp(0.0, 1.0);
        self
    }
}

/// Raw action item returned by the LLM
#[derive(Debug, Clone, Deserialize)]
struct RawActionItem {
    text: String,
    #[serde(default)]
    assignee: Option<String>,
    #[serde(default)]
    deadline: Option<String>,
    #[serde(default)]
    priority: Option<String>,
    #[serde(default)]
    confidence: Option<f32>,
}

/// LLM trait for abstraction over different LLM providers
#[async_trait::async_trait]
pub trait LLM: Send + Sync {
    async fn complete(&self, prompt: &str) -> Result<String>;
}

/// Boxed LLM trait object for dynamic dispatch
pub type BoxedLLM = Box<dyn LLM + Send + Sync>;

/// Helper function to extract action items using a boxed LLM
pub async fn extract_action_items_boxed(
    transcript: &str,
    llm: BoxedLLM,
    source: ActionItemSource,
    source_id: Option<String>,
) -> Result<Vec<ActionItem>> {
    extract_action_items_with_llm(transcript, llm, source, source_id).await
}

/// Internal helper that works with boxed LLM
async fn extract_action_items_with_llm(
    transcript: &str,
    llm: BoxedLLM,
    source: ActionItemSource,
    source_id: Option<String>,
) -> Result<Vec<ActionItem>> {
    if transcript.trim().is_empty() {
        return Ok(Vec::new());
    }

    let prompt = DEFAULT_PROMPT_TEMPLATE.replace("{transcript}", transcript);

    debug!("Sending action item extraction prompt to LLM");
    let response = llm.complete(&prompt).await?;

    parse_action_items(&response, source, source_id)
}

/// Default prompt template for action item extraction
const DEFAULT_PROMPT_TEMPLATE: &str = r#"Analyze the following transcript and extract action items.

For each action item, identify:
- The task to be done (clear, actionable description)
- Who is responsible (if mentioned - person name or role)
- Any deadline mentioned (date/time)
- Priority level (low, medium, high, critical) based on context

Guidelines:
- Only extract actual tasks, not general discussion points
- Be specific and actionable
- If a deadline is relative (e.g., "next week"), include it as stated
- Confidence score should reflect how clear the action item is (0.0-1.0)

Transcript:
"""
{transcript}
"""

Return ONLY a JSON array in this exact format:
[
  {
    "text": "Complete project proposal",
    "assignee": "John Smith",
    "deadline": "2024-02-15",
    "priority": "high",
    "confidence": 0.95
  }
]

If no action items are found, return an empty array: []"#;

/// Extract action items from a transcript using an LLM
pub async fn extract_action_items(
    transcript: &str,
    llm: &dyn LLM,
    source: ActionItemSource,
    source_id: Option<String>,
) -> Result<Vec<ActionItem>> {
    if transcript.trim().is_empty() {
        return Ok(Vec::new());
    }

    let prompt = DEFAULT_PROMPT_TEMPLATE.replace("{transcript}", transcript);

    debug!("Sending action item extraction prompt to LLM");
    let response = llm.complete(&prompt).await?;

    parse_action_items(&response, source, source_id)
}

/// Parse action items from LLM response
fn parse_action_items(
    response: &str,
    source: ActionItemSource,
    source_id: Option<String>,
) -> Result<Vec<ActionItem>> {
    // Try to extract JSON from the response (handle markdown code blocks)
    let json_str = extract_json_from_response(response);

    let raw_items: Vec<RawActionItem> =
        serde_json::from_str(&json_str).context("Failed to parse LLM response as JSON")?;

    let mut action_items = Vec::new();

    for raw in raw_items {
        // Parse deadline if present
        let deadline = raw.deadline.and_then(|d| parse_deadline(&d));

        // Parse priority
        let priority = raw
            .priority
            .as_ref()
            .map(|p| parse_priority(p))
            .unwrap_or_default();

        // Extract assignee before consuming raw
        let assignee = raw.assignee.clone();

        // Build action item
        let mut item = ActionItem::new(raw.text)
            .with_source(source.clone())
            .with_source_id(source_id.clone().unwrap_or_default())
            .with_priority(priority)
            .with_confidence(raw.confidence.unwrap_or(0.5));

        item.source_id = source_id.clone();
        if deadline.is_some() {
            item.deadline = deadline;
        }
        if let Some(assignee) = assignee {
            item.assignee = Some(assignee);
        }

        action_items.push(item);
    }

    info!(
        "Extracted {} action items from transcript",
        action_items.len()
    );
    Ok(action_items)
}

/// Extract JSON from response, handling markdown code blocks
fn extract_json_from_response(response: &str) -> String {
    // Try to find JSON in markdown code blocks
    if let Some(start) = response.find("```json") {
        if let Some(end) = response[start + 7..].find("```") {
            return response[start + 7..start + 7 + end].trim().to_string();
        }
    }

    // Try to find JSON in generic code blocks
    if let Some(start) = response.find("```") {
        if let Some(end) = response[start + 3..].find("```") {
            let content = &response[start + 3..start + 3 + end];
            // Check if it looks like JSON
            if content.trim().starts_with('[') || content.trim().starts_with('{') {
                return content.trim().to_string();
            }
        }
    }

    // Try to find array brackets directly
    if let Some(start) = response.find('[') {
        if let Some(end) = response.rfind(']') {
            if end > start {
                return response[start..=end].to_string();
            }
        }
    }

    // Return trimmed response as fallback
    response.trim().to_string()
}

/// Parse deadline string into DateTime
fn parse_deadline(deadline_str: &str) -> Option<DateTime<Utc>> {
    // Try various date formats
    let formats = [
        "%Y-%m-%d",
        "%Y-%m-%d %H:%M",
        "%Y-%m-%d %H:%M:%S",
        "%d/%m/%Y",
        "%m/%d/%Y",
        "%B %d, %Y",
        "%b %d, %Y",
        "%d %B %Y",
        "%d %b %Y",
    ];

    for format in &formats {
        if let Ok(naive) = NaiveDateTime::parse_from_str(
            &format!("{} 00:00:00", deadline_str),
            &format!("{} %H:%M:%S", format),
        ) {
            return Some(DateTime::from_naive_utc_and_offset(naive, Utc));
        }

        if let Ok(naive) = NaiveDateTime::parse_from_str(deadline_str, format) {
            return Some(DateTime::from_naive_utc_and_offset(naive, Utc));
        }
    }

    // Try chrono's flexible parsing
    if let Ok(datetime) = DateTime::parse_from_rfc3339(deadline_str) {
        return Some(datetime.with_timezone(&Utc));
    }

    if let Ok(datetime) = DateTime::parse_from_rfc2822(deadline_str) {
        return Some(datetime.with_timezone(&Utc));
    }

    warn!("Could not parse deadline: {}", deadline_str);
    None
}

/// Parse priority string
fn parse_priority(priority_str: &str) -> ActionItemPriority {
    match priority_str.to_lowercase().as_str() {
        "critical" | "urgent" | "highest" => ActionItemPriority::Critical,
        "high" => ActionItemPriority::High,
        "low" => ActionItemPriority::Low,
        _ => ActionItemPriority::Medium,
    }
}

/// Cache for action items to avoid reprocessing
pub struct ActionItemCache {
    cache: HashMap<String, Vec<ActionItem>>,
    max_entries: usize,
}

impl ActionItemCache {
    pub fn new(max_entries: usize) -> Self {
        Self {
            cache: HashMap::with_capacity(max_entries),
            max_entries,
        }
    }

    /// Get cached action items for a transcript hash
    pub fn get(&self, transcript_hash: &str) -> Option<&Vec<ActionItem>> {
        self.cache.get(transcript_hash)
    }

    /// Cache action items for a transcript hash
    pub fn set(&mut self, transcript_hash: &str, items: Vec<ActionItem>) {
        if self.cache.len() >= self.max_entries {
            // Simple LRU: remove oldest entry
            if let Some(first_key) = self.cache.keys().next().cloned() {
                self.cache.remove(&first_key);
            }
        }
        self.cache.insert(transcript_hash.to_string(), items);
    }

    /// Check if transcript has been processed
    pub fn contains(&self, transcript_hash: &str) -> bool {
        self.cache.contains_key(transcript_hash)
    }

    /// Clear the cache
    pub fn clear(&mut self) {
        self.cache.clear();
    }
}

impl Default for ActionItemCache {
    fn default() -> Self {
        Self::new(1000)
    }
}

/// Extract action items with caching
pub async fn extract_action_items_cached(
    transcript: &str,
    llm: &dyn LLM,
    source: ActionItemSource,
    source_id: Option<String>,
    cache: &mut ActionItemCache,
) -> Result<Vec<ActionItem>> {
    // Generate hash of transcript
    let hash = format!("{:x}", md5::compute(transcript.as_bytes()));

    // Check cache
    if let Some(cached) = cache.get(&hash) {
        debug!("Returning cached action items for transcript");
        return Ok(cached.clone());
    }

    // Extract action items
    let items = extract_action_items(transcript, llm, source, source_id).await?;

    // Cache results
    cache.set(&hash, items.clone());

    Ok(items)
}

/// Export format for integrations (Todoist, Notion, etc.)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ActionItemExport {
    pub items: Vec<ActionItem>,
    pub export_format: ExportFormat,
    pub webhook_url: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ExportFormat {
    Json,
    Todoist,
    Notion,
    Webhook,
}

/// Convert action items to Todoist format
pub fn to_todoist_format(items: &[ActionItem]) -> Vec<serde_json::Value> {
    items
        .iter()
        .map(|item| {
            serde_json::json!({
                "content": item.text,
                "description": format!(
                    "Source: {} | Assignee: {} | Confidence: {:.0}%",
                    item.source,
                    item.assignee.as_deref().unwrap_or("Unassigned"),
                    item.confidence * 100.0
                ),
                "priority": match item.priority {
                    ActionItemPriority::Critical => 4,
                    ActionItemPriority::High => 3,
                    ActionItemPriority::Medium => 2,
                    ActionItemPriority::Low => 1,
                },
                "due_string": item.deadline.map(|d| d.format("%Y-%m-%d").to_string()),
                "labels": ["screenpipe", "action-item"],
            })
        })
        .collect()
}

/// Convert action items to Notion format
pub fn to_notion_format(items: &[ActionItem]) -> Vec<serde_json::Value> {
    items
        .iter()
        .map(|item| {
            serde_json::json!({
                "parent": { "database_id": "your-database-id" },
                "properties": {
                    "Name": {
                        "title": [{ "text": { "content": item.text } }]
                    },
                    "Assignee": {
                        "rich_text": [{ "text": { "content": item.assignee.as_deref().unwrap_or("Unassigned") } }]
                    },
                    "Status": {
                        "select": { "name": format!("{:?}", item.status) }
                    },
                    "Priority": {
                        "select": { "name": format!("{:?}", item.priority) }
                    },
                    "Source": {
                        "rich_text": [{ "text": { "content": item.source.to_string() } }]
                    },
                    "Confidence": {
                        "number": item.confidence
                    },
                    "Deadline": item.deadline.map(|d| {
                        serde_json::json!({ "date": { "start": d.to_rfc3339() } })
                    }).unwrap_or(serde_json::json!({})),
                }
            })
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    struct MockLLM {
        response: String,
    }

    #[async_trait::async_trait]
    impl LLM for MockLLM {
        async fn complete(&self, _prompt: &str) -> Result<String> {
            Ok(self.response.clone())
        }
    }

    #[test]
    fn test_action_item_creation() {
        let item = ActionItem::new("Test task")
            .with_assignee("John Doe")
            .with_priority(ActionItemPriority::High)
            .with_confidence(0.9);

        assert_eq!(item.text, "Test task");
        assert_eq!(item.assignee, Some("John Doe".to_string()));
        assert_eq!(item.priority, ActionItemPriority::High);
        assert!((item.confidence - 0.9).abs() < f32::EPSILON);
        assert_eq!(item.status, ActionItemStatus::Pending);
    }

    #[test]
    fn test_action_item_mark_done() {
        let mut item = ActionItem::new("Test task");
        item.mark_done();

        assert_eq!(item.status, ActionItemStatus::Done);
        assert!(item.completed_at.is_some());
    }

    #[test]
    fn test_extract_json_from_response() {
        let response = r#"Here are the action items:
```json
[
  {"text": "Task 1", "assignee": "John"}
]
```"#;

        let json = extract_json_from_response(response);
        assert!(json.contains("Task 1"));
    }

    #[test]
    fn test_extract_json_without_markdown() {
        let response = r#"[
  {"text": "Task 1", "assignee": "John"}
]"#;

        let json = extract_json_from_response(response);
        assert!(json.contains("Task 1"));
    }

    #[test]
    fn test_parse_priority() {
        assert_eq!(parse_priority("high"), ActionItemPriority::High);
        assert_eq!(parse_priority("HIGH"), ActionItemPriority::High);
        assert_eq!(parse_priority("critical"), ActionItemPriority::Critical);
        assert_eq!(parse_priority("urgent"), ActionItemPriority::Critical);
        assert_eq!(parse_priority("unknown"), ActionItemPriority::Medium);
    }

    #[test]
    fn test_cache_operations() {
        let mut cache = ActionItemCache::new(2);
        let items = vec![ActionItem::new("Test")];

        cache.set("hash1", items.clone());
        assert!(cache.contains("hash1"));
        assert_eq!(cache.get("hash1").unwrap().len(), 1);

        // Test LRU eviction - when cache is full, oldest entry is removed
        cache.set("hash2", items.clone());
        cache.set("hash3", items.clone());

        // Cache should have exactly 2 entries (max_entries)
        assert_eq!(cache.cache.len(), 2);
        // hash3 should be in cache (just added)
        assert!(cache.contains("hash3"));
    }

    #[test]
    fn test_to_todoist_format() {
        let items = vec![ActionItem::new("Task 1")
            .with_assignee("John")
            .with_priority(ActionItemPriority::High)
            .with_confidence(0.9)];

        let todoist = to_todoist_format(&items);
        assert_eq!(todoist.len(), 1);
        assert_eq!(todoist[0]["content"], "Task 1");
        assert_eq!(todoist[0]["priority"], 3);
    }

    #[test]
    fn test_to_notion_format() {
        let items = vec![ActionItem::new("Task 1").with_assignee("John")];

        let notion = to_notion_format(&items);
        assert_eq!(notion.len(), 1);
    }

    #[tokio::test]
    async fn test_extract_action_items() {
        let mock_llm = MockLLM {
            response: r#"[
  {
    "text": "Complete the project proposal",
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
            .to_string(),
        };

        let transcript = "Alice will complete the project proposal by Feb 15. Bob needs to review the code changes.";
        let items = extract_action_items(
            transcript,
            &mock_llm as &dyn LLM,
            ActionItemSource::Meeting,
            Some("meeting-123".to_string()),
        )
        .await
        .unwrap();

        assert_eq!(items.len(), 2);
        assert_eq!(items[0].text, "Complete the project proposal");
        assert_eq!(items[0].assignee, Some("Alice".to_string()));
        assert_eq!(items[0].priority, ActionItemPriority::High);
        assert_eq!(items[1].assignee, Some("Bob".to_string()));
    }

    #[tokio::test]
    async fn test_extract_action_items_empty_transcript() {
        let mock_llm = MockLLM {
            response: "[]".to_string(),
        };

        let items =
            extract_action_items("", &mock_llm as &dyn LLM, ActionItemSource::Meeting, None)
                .await
                .unwrap();

        assert!(items.is_empty());
    }

    #[tokio::test]
    async fn test_extract_action_items_cached() {
        let mock_llm = MockLLM {
            response: r#"[{"text": "Task 1", "confidence": 0.9}]"#.to_string(),
        };

        let mut cache = ActionItemCache::new(10);
        let transcript = "Test transcript";

        // First call should hit LLM
        let items1 = extract_action_items_cached(
            transcript,
            &mock_llm as &dyn LLM,
            ActionItemSource::Meeting,
            None,
            &mut cache,
        )
        .await
        .unwrap();

        assert_eq!(items1.len(), 1);

        // Second call should use cache (even with different mock)
        let mock_llm2 = MockLLM {
            response: "[]".to_string(),
        };

        let items2 = extract_action_items_cached(
            transcript,
            &mock_llm2 as &dyn LLM,
            ActionItemSource::Meeting,
            None,
            &mut cache,
        )
        .await
        .unwrap();

        // Should return cached result, not empty
        assert_eq!(items2.len(), 1);
    }
}
