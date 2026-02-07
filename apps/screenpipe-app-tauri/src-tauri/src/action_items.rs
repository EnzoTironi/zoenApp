//! Action Items Commands for Tauri
//!
//! Provides Tauri commands for managing action items extracted from meetings.

use serde::{Deserialize, Serialize};
use tauri::{Emitter, Manager};
use tracing::{debug, error, info, warn};

/// Action item status
#[derive(Debug, Clone, Serialize, Deserialize, specta::Type)]
#[serde(rename_all = "snake_case")]
pub enum ActionItemStatus {
    Pending,
    InProgress,
    Done,
    Cancelled,
}

impl From<ActionItemStatus> for screenpipe_db::ActionItemStatus {
    fn from(status: ActionItemStatus) -> Self {
        match status {
            ActionItemStatus::Pending => screenpipe_db::ActionItemStatus::Pending,
            ActionItemStatus::InProgress => screenpipe_db::ActionItemStatus::InProgress,
            ActionItemStatus::Done => screenpipe_db::ActionItemStatus::Done,
            ActionItemStatus::Cancelled => screenpipe_db::ActionItemStatus::Cancelled,
        }
    }
}

impl From<screenpipe_db::ActionItemStatus> for ActionItemStatus {
    fn from(status: screenpipe_db::ActionItemStatus) -> Self {
        match status {
            screenpipe_db::ActionItemStatus::Pending => ActionItemStatus::Pending,
            screenpipe_db::ActionItemStatus::InProgress => ActionItemStatus::InProgress,
            screenpipe_db::ActionItemStatus::Done => ActionItemStatus::Done,
            screenpipe_db::ActionItemStatus::Cancelled => ActionItemStatus::Cancelled,
        }
    }
}

/// Action item priority
#[derive(Debug, Clone, Serialize, Deserialize, specta::Type)]
#[serde(rename_all = "snake_case")]
pub enum ActionItemPriority {
    Low,
    Medium,
    High,
    Critical,
}

impl From<ActionItemPriority> for screenpipe_db::ActionItemPriority {
    fn from(priority: ActionItemPriority) -> Self {
        match priority {
            ActionItemPriority::Low => screenpipe_db::ActionItemPriority::Low,
            ActionItemPriority::Medium => screenpipe_db::ActionItemPriority::Medium,
            ActionItemPriority::High => screenpipe_db::ActionItemPriority::High,
            ActionItemPriority::Critical => screenpipe_db::ActionItemPriority::Critical,
        }
    }
}

impl From<screenpipe_db::ActionItemPriority> for ActionItemPriority {
    fn from(priority: screenpipe_db::ActionItemPriority) -> Self {
        match priority {
            screenpipe_db::ActionItemPriority::Low => ActionItemPriority::Low,
            screenpipe_db::ActionItemPriority::Medium => ActionItemPriority::Medium,
            screenpipe_db::ActionItemPriority::High => ActionItemPriority::High,
            screenpipe_db::ActionItemPriority::Critical => ActionItemPriority::Critical,
        }
    }
}

/// Action item data transfer object
#[derive(Debug, Clone, Serialize, Deserialize, specta::Type)]
pub struct ActionItem {
    pub id: String,
    pub text: String,
    pub assignee: Option<String>,
    pub deadline: Option<String>,
    pub source: String,
    pub source_id: Option<String>,
    pub confidence: f64,
    pub status: ActionItemStatus,
    pub priority: ActionItemPriority,
    pub created_at: String,
    pub updated_at: String,
    pub completed_at: Option<String>,
    pub metadata: Option<serde_json::Value>,
}

impl From<screenpipe_db::ActionItem> for ActionItem {
    fn from(item: screenpipe_db::ActionItem) -> Self {
        Self {
            id: item.id,
            text: item.text,
            assignee: item.assignee,
            deadline: item.deadline.map(|d| d.to_rfc3339()),
            source: item.source,
            source_id: item.source_id,
            confidence: item.confidence,
            status: item.status.into(),
            priority: item.priority.into(),
            created_at: item.created_at.to_rfc3339(),
            updated_at: item.updated_at.to_rfc3339(),
            completed_at: item.completed_at.map(|d| d.to_rfc3339()),
            metadata: item.metadata.and_then(|m| serde_json::from_str(&m).ok()),
        }
    }
}

/// Query parameters for fetching action items
#[derive(Debug, Clone, Serialize, Deserialize, specta::Type)]
pub struct ActionItemsQuery {
    pub status: Option<ActionItemStatus>,
    pub source: Option<String>,
    pub assignee: Option<String>,
    pub from_date: Option<String>,
    pub to_date: Option<String>,
    pub limit: Option<i64>,
    pub offset: Option<i64>,
}

impl From<ActionItemsQuery> for screenpipe_db::ActionItemQuery {
    fn from(query: ActionItemsQuery) -> Self {
        use chrono::DateTime;
        Self {
            status: query.status.map(|s| s.into()),
            source: query.source,
            assignee: query.assignee,
            from_date: query.from_date.and_then(|d| DateTime::parse_from_rfc3339(&d).ok().map(|dt| dt.with_timezone(&chrono::Utc))),
            to_date: query.to_date.and_then(|d| DateTime::parse_from_rfc3339(&d).ok().map(|dt| dt.with_timezone(&chrono::Utc))),
            limit: query.limit,
            offset: query.offset,
        }
    }
}

/// Statistics for action items
#[derive(Debug, Clone, Serialize, Deserialize, specta::Type)]
pub struct ActionItemsStats {
    pub total: i64,
    pub pending: i64,
    pub in_progress: i64,
    pub done: i64,
    pub cancelled: i64,
}

/// Export configuration
#[derive(Debug, Clone, Serialize, Deserialize, specta::Type)]
pub struct ExportConfig {
    pub format: ExportFormat,
    pub webhook_url: Option<String>,
    pub api_token: Option<String>,
    pub enabled: bool,
}

/// Export format
#[derive(Debug, Clone, Serialize, Deserialize, specta::Type)]
#[serde(rename_all = "snake_case")]
pub enum ExportFormat {
    Json,
    Todoist,
    Notion,
    Webhook,
}

/// Get action items with optional filtering
#[tauri::command]
#[specta::specta]
pub async fn get_action_items(
    _app_handle: tauri::AppHandle,
    query: Option<ActionItemsQuery>,
) -> Result<Vec<ActionItem>, String> {
    debug!("get_action_items called with query: {:?}", query);

    // Get the database path from the app handle
    let db_path = crate::get_base_dir(&_app_handle, None)
        .map_err(|e| format!("Failed to get base dir: {}", e))?
        .join("db.sqlite");

    let db = screenpipe_db::DatabaseManager::new(db_path.to_str().unwrap())
        .await
        .map_err(|e| format!("Failed to connect to database: {}", e))?;

    let db_query = query.map(|q| q.into()).unwrap_or_default();

    let items = db.query_action_items(&db_query)
        .await
        .map_err(|e| format!("Failed to query action items: {}", e))?
        .into_iter()
        .map(ActionItem::from)
        .collect();

    Ok(items)
}

/// Get statistics for action items
#[tauri::command]
#[specta::specta]
pub async fn get_action_items_stats(
    _app_handle: tauri::AppHandle,
) -> Result<ActionItemsStats, String> {
    debug!("get_action_items_stats called");

    // Get the database path from the app handle
    let db_path = crate::get_base_dir(&_app_handle, None)
        .map_err(|e| format!("Failed to get base dir: {}", e))?
        .join("db.sqlite");

    let db = screenpipe_db::DatabaseManager::new(db_path.to_str().unwrap())
        .await
        .map_err(|e| format!("Failed to connect to database: {}", e))?;

    let counts = db.get_action_items_count_by_status()
        .await
        .map_err(|e| format!("Failed to get action items stats: {}", e))?;

    let mut stats = ActionItemsStats {
        total: 0,
        pending: 0,
        in_progress: 0,
        done: 0,
        cancelled: 0,
    };

    for (status, count) in counts {
        match status.as_str() {
            "pending" => stats.pending = count,
            "in_progress" => stats.in_progress = count,
            "done" => stats.done = count,
            "cancelled" => stats.cancelled = count,
            _ => {}
        }
        stats.total += count;
    }

    Ok(stats)
}

/// Update the status of an action item
#[tauri::command]
#[specta::specta]
pub async fn update_action_item_status(
    _app_handle: tauri::AppHandle,
    id: String,
    status: ActionItemStatus,
) -> Result<ActionItem, String> {
    debug!("update_action_item_status called for id: {}, status: {:?}", id, status);

    // Get the database path from the app handle
    let db_path = crate::get_base_dir(&_app_handle, None)
        .map_err(|e| format!("Failed to get base dir: {}", e))?
        .join("db.sqlite");

    let db = screenpipe_db::DatabaseManager::new(db_path.to_str().unwrap())
        .await
        .map_err(|e| format!("Failed to connect to database: {}", e))?;

    let db_status: screenpipe_db::ActionItemStatus = status.into();

    let item = db.update_action_item_status(&id, db_status)
        .await
        .map_err(|e| format!("Failed to update action item status: {}", e))?
        .ok_or_else(|| "Action item not found".to_string())?;

    Ok(ActionItem::from(item))
}

/// Delete an action item
#[tauri::command]
#[specta::specta]
pub async fn delete_action_item(
    _app_handle: tauri::AppHandle,
    id: String,
) -> Result<(), String> {
    debug!("delete_action_item called for id: {}", id);

    // Get the database path from the app handle
    let db_path = crate::get_base_dir(&_app_handle, None)
        .map_err(|e| format!("Failed to get base dir: {}", e))?
        .join("db.sqlite");

    let db = screenpipe_db::DatabaseManager::new(db_path.to_str().unwrap())
        .await
        .map_err(|e| format!("Failed to connect to database: {}", e))?;

    db.delete_action_item(&id)
        .await
        .map_err(|e| format!("Failed to delete action item: {}", e))?;

    Ok(())
}

/// Export action items to external service
#[tauri::command]
#[specta::specta]
pub async fn export_action_items(
    _app_handle: tauri::AppHandle,
    ids: Vec<String>,
    config: ExportConfig,
) -> Result<(), String> {
    debug!(
        "export_action_items called for {} items to format {:?}",
        ids.len(),
        config.format
    );

    match config.format {
        ExportFormat::Todoist => {
            if let Some(webhook_url) = config.webhook_url {
                export_to_todoist(&ids, &webhook_url).await?;
            } else {
                return Err("Webhook URL required for Todoist export".to_string());
            }
        }
        ExportFormat::Notion => {
            if let Some(webhook_url) = config.webhook_url {
                export_to_notion(&ids, &webhook_url).await?;
            } else {
                return Err("Webhook URL required for Notion export".to_string());
            }
        }
        ExportFormat::Webhook => {
            if let Some(webhook_url) = config.webhook_url {
                export_to_webhook(&ids, &webhook_url).await?;
            } else {
                return Err("Webhook URL required".to_string());
            }
        }
        ExportFormat::Json => {
            // JSON export is handled by the frontend
        }
    }

    Ok(())
}

/// Extract action items from a transcript
#[tauri::command]
#[specta::specta]
pub async fn extract_action_items_from_transcript(
    app_handle: tauri::AppHandle,
    transcript: String,
    source_id: Option<String>,
) -> Result<Vec<ActionItem>, String> {
    use screenpipe_core::action_items::{extract_action_items, ActionItemSource};

    info!(
        "extract_action_items_from_transcript called with transcript length: {}",
        transcript.len()
    );

    // Get the database path from the app handle
    let db_path = crate::get_base_dir(&app_handle, None)
        .map_err(|e| format!("Failed to get base dir: {}", e))?
        .join("db.sqlite");

    let db = screenpipe_db::DatabaseManager::new(db_path.to_str().unwrap())
        .await
        .map_err(|e| format!("Failed to connect to database: {}", e))?;

    // TODO: Implement LLM client for action item extraction
    // For now, use a simple mock implementation that looks for action item patterns
    let items = extract_action_items_mock(&transcript,
        source_id.as_deref().unwrap_or("manual"),
    ).await?;

    // Save extracted items to database
    for item in &items {
        let insert_item = screenpipe_db::InsertActionItem {
            id: item.id.clone(),
            text: item.text.clone(),
            assignee: item.assignee.clone(),
            deadline: item.deadline.as_ref().and_then(|d| chrono::DateTime::parse_from_rfc3339(d).ok().map(|dt| dt.with_timezone(&chrono::Utc))),
            source: item.source.clone(),
            source_id: item.source_id.clone(),
            confidence: item.confidence,
            status: screenpipe_db::ActionItemStatus::Pending,
            priority: screenpipe_db::ActionItemPriority::Medium,
            metadata: None,
        };

        if let Err(e) = db.insert_action_item(&insert_item).await {
            warn!("Failed to insert action item: {}", e);
        }
    }

    // Emit event for real-time updates
    let _ = app_handle.emit("action-items-extracted", &items);

    info!("Extracted {} action items from transcript", items.len());
    Ok(items)
}

/// Mock implementation for action item extraction
/// In production, this would use an LLM client
async fn extract_action_items_mock(
    transcript: &str,
    source_id: &str,
) -> Result<Vec<ActionItem>, String> {
    use regex::Regex;
    use uuid::Uuid;

    let mut items = Vec::new();

    // Simple pattern matching for action items
    // Look for phrases like "I will", "You should", "We need to", etc.
    let patterns = [
        (r"(?i)(\w+)\s+will\s+(.+?)(?:\.|$)", "will"),
        (r"(?i)(\w+)\s+should\s+(.+?)(?:\.|$)", "should"),
        (r"(?i)(\w+)\s+needs?\s+to\s+(.+?)(?:\.|$)", "needs to"),
        (r"(?i)let['']?s\s+(.+?)(?:\.|$)", "let's"),
        (r"(?i)action\s+item:\s*(.+?)(?:\.|$)", "action item"),
        (r"(?i)todo:\s*(.+?)(?:\.|$)", "todo"),
        (r"(?i)task:\s*(.+?)(?:\.|$)", "task"),
    ];

    for (pattern, _) in &patterns {
        let regex = Regex::new(pattern).map_err(|e| e.to_string())?;
        for cap in regex.captures_iter(transcript) {
            if cap.len() >= 2 {
                let text = cap[cap.len() - 1].trim().to_string();
                let assignee = if cap.len() > 2 {
                    Some(cap[1].to_string())
                } else {
                    None
                };

                if !text.is_empty() && text.len() > 10 {
                    items.push(ActionItem {
                        id: Uuid::new_v4().to_string(),
                        text: text.clone(),
                        assignee: assignee.clone(),
                        deadline: None,
                        source: "meeting".to_string(),
                        source_id: Some(source_id.to_string()),
                        confidence: 0.7,
                        status: ActionItemStatus::Pending,
                        priority: ActionItemPriority::Medium,
                        created_at: chrono::Utc::now().to_rfc3339(),
                        updated_at: chrono::Utc::now().to_rfc3339(),
                        completed_at: None,
                        metadata: None,
                    });
                }
            }
        }
    }

    // Remove duplicates based on text similarity
    items.sort_by(|a, b| a.text.cmp(&b.text));
    items.dedup_by(|a, b| a.text.to_lowercase() == b.text.to_lowercase());

    Ok(items)
}

/// Export action items to Todoist
async fn export_to_todoist(ids: &[String], webhook_url: &str) -> Result<(), String> {
    debug!("Exporting {} action items to Todoist", ids.len());

    let client = reqwest::Client::new();
    let payload = serde_json::json!({
        "action_items": ids,
        "source": "screenpipe"
    });

    match client.post(webhook_url).json(&payload).send().await {
        Ok(response) => {
            if response.status().is_success() {
                info!("Successfully exported to Todoist");
                Ok(())
            } else {
                Err(format!("Todoist webhook returned: {}", response.status()))
            }
        }
        Err(e) => Err(format!("Failed to send to Todoist: {}", e)),
    }
}

/// Export action items to Notion
async fn export_to_notion(ids: &[String], webhook_url: &str) -> Result<(), String> {
    debug!("Exporting {} action items to Notion", ids.len());

    let client = reqwest::Client::new();
    let payload = serde_json::json!({
        "action_items": ids,
        "source": "screenpipe"
    });

    match client.post(webhook_url).json(&payload).send().await {
        Ok(response) => {
            if response.status().is_success() {
                info!("Successfully exported to Notion");
                Ok(())
            } else {
                Err(format!("Notion webhook returned: {}", response.status()))
            }
        }
        Err(e) => Err(format!("Failed to send to Notion: {}", e)),
    }
}

/// Export action items to generic webhook
async fn export_to_webhook(ids: &[String], webhook_url: &str) -> Result<(), String> {
    debug!("Exporting {} action items to webhook", ids.len());

    let client = reqwest::Client::new();
    let payload = serde_json::json!({
        "action_items": ids,
        "source": "screenpipe",
        "exported_at": chrono::Utc::now().to_rfc3339()
    });

    match client.post(webhook_url).json(&payload).send().await {
        Ok(response) => {
            if response.status().is_success() {
                info!("Successfully exported to webhook");
                Ok(())
            } else {
                Err(format!("Webhook returned: {}", response.status()))
            }
        }
        Err(e) => Err(format!("Failed to send webhook: {}", e)),
    }
}

/// Notify the user that new action items have been extracted
pub fn notify_action_items_extracted(
    app_handle: &tauri::AppHandle,
    source_id: &str,
    count: usize,
) -> Result<(), String> {
    info!(
        "Notifying user of {} action items extracted from {}",
        count, source_id
    );

    let notification = serde_json::json!({
        "source_id": source_id,
        "count": count,
        "timestamp": chrono::Utc::now().to_rfc3339()
    });

    app_handle
        .emit("action-items-notification", notification)
        .map_err(|e| format!("Failed to emit notification: {}", e))
}
