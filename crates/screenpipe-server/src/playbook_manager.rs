use anyhow::Result;
use axum::extract::ws::WebSocket;
use screenpipe_core::playbook_engine::{
    default_playbooks, Action, Playbook, PlaybookEngine, Trigger,
};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use sqlx::Row;
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;
use tokio::sync::{mpsc, RwLock};
use tracing::{debug, error, info, warn};

/// Manager for playbooks - wraps the core PlaybookEngine and adds server-specific functionality
pub struct PlaybookManager {
    engine: Arc<PlaybookEngine>,
    /// WebSocket subscribers for real-time playbook events
    ws_subscribers: Arc<RwLock<HashMap<String, Vec<mpsc::Sender<PlaybookEvent>>>>>,
    screenpipe_dir: PathBuf,
    db_pool: Option<Arc<sqlx::SqlitePool>>,
}

/// Events that can be sent to WebSocket subscribers
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", content = "data")]
pub enum PlaybookEvent {
    PlaybookTriggered {
        playbook_id: String,
        playbook_name: String,
        trigger: Trigger,
    },
    PlaybookCompleted {
        playbook_id: String,
        playbook_name: String,
        success: bool,
    },
    PlaybookUpdated {
        playbook: Playbook,
    },
    PlaybookDeleted {
        playbook_id: String,
    },
    FocusModeChanged {
        enabled: bool,
        duration: Option<u32>,
    },
}

/// Request to create a new playbook
#[derive(Debug, Deserialize)]
pub struct CreatePlaybookRequest {
    pub name: String,
    #[serde(default)]
    pub description: Option<String>,
    pub triggers: Vec<Trigger>,
    pub actions: Vec<Action>,
    #[serde(default)]
    pub cooldown_minutes: Option<u32>,
    #[serde(default)]
    pub max_executions_per_day: Option<u32>,
    #[serde(default)]
    pub icon: Option<String>,
    #[serde(default)]
    pub color: Option<String>,
}

/// Request to update an existing playbook
#[derive(Debug, Deserialize)]
pub struct UpdatePlaybookRequest {
    #[serde(default)]
    pub name: Option<String>,
    #[serde(default)]
    pub description: Option<String>,
    #[serde(default)]
    pub enabled: Option<bool>,
    #[serde(default)]
    pub triggers: Option<Vec<Trigger>>,
    #[serde(default)]
    pub actions: Option<Vec<Action>>,
    #[serde(default)]
    pub cooldown_minutes: Option<u32>,
    #[serde(default)]
    pub max_executions_per_day: Option<u32>,
    #[serde(default)]
    pub icon: Option<String>,
    #[serde(default)]
    pub color: Option<String>,
}

/// Response for listing playbooks
#[derive(Debug, Serialize)]
pub struct ListPlaybooksResponse {
    pub playbooks: Vec<Playbook>,
    pub total: usize,
}

/// Response for playbook execution history
#[derive(Debug, Serialize)]
pub struct ExecutionHistoryResponse {
    pub executions: Vec<ExecutionRecord>,
    pub total: usize,
}

/// Execution record for history
#[derive(Debug, Serialize)]
pub struct ExecutionRecord {
    pub id: String,
    pub playbook_id: String,
    pub playbook_name: String,
    pub started_at: String,
    pub completed_at: Option<String>,
    pub status: String,
    pub triggered_by: String,
}

impl PlaybookManager {
    /// Create a new PlaybookManager
    pub async fn new(
        screenpipe_dir: PathBuf,
        db_pool: Option<Arc<sqlx::SqlitePool>>,
    ) -> Result<Self> {
        let engine = Arc::new(PlaybookEngine::new(screenpipe_dir.clone()));

        let manager = Self {
            engine,
            ws_subscribers: Arc::new(RwLock::new(HashMap::new())),
            screenpipe_dir,
            db_pool,
        };

        // Initialize database tables and load playbooks
        manager.init().await?;

        Ok(manager)
    }

    /// Initialize the database and load playbooks
    async fn init(&self) -> Result<()> {
        // Create tables if DB is available
        if let Some(pool) = &self.db_pool {
            self.create_tables(pool).await?;
            self.load_playbooks_from_db(pool).await?;
        } else {
            // No DB, just load default playbooks into memory
            let defaults = default_playbooks();
            self.engine.init(defaults).await?;
        }

        Ok(())
    }

    /// Create database tables for playbooks
    async fn create_tables(&self, pool: &sqlx::SqlitePool) -> Result<()> {
        sqlx::query(
            r#"
            CREATE TABLE IF NOT EXISTS playbooks (
                id TEXT PRIMARY KEY,
                name TEXT NOT NULL,
                description TEXT,
                enabled BOOLEAN NOT NULL DEFAULT 0,
                triggers_json TEXT NOT NULL,
                actions_json TEXT NOT NULL,
                cooldown_minutes INTEGER,
                max_executions_per_day INTEGER,
                created_at TEXT NOT NULL,
                updated_at TEXT NOT NULL,
                is_builtin BOOLEAN DEFAULT 0,
                icon TEXT,
                color TEXT
            )
            "#,
        )
        .execute(pool)
        .await?;

        sqlx::query(
            r#"
            CREATE TABLE IF NOT EXISTS playbook_executions (
                id TEXT PRIMARY KEY,
                playbook_id TEXT NOT NULL,
                started_at TEXT NOT NULL,
                completed_at TEXT,
                status TEXT NOT NULL,
                triggered_by TEXT NOT NULL,
                action_results TEXT NOT NULL,
                error TEXT,
                FOREIGN KEY (playbook_id) REFERENCES playbooks(id)
            )
            "#,
        )
        .execute(pool)
        .await?;

        Ok(())
    }

    /// Load playbooks from database
    async fn load_playbooks_from_db(&self, pool: &sqlx::SqlitePool) -> Result<()> {
        let rows = sqlx::query("SELECT * FROM playbooks")
            .fetch_all(pool)
            .await?;

        let mut playbooks = HashMap::new();

        for row in rows {
            let playbook = self.row_to_playbook(&row)?;
            playbooks.insert(playbook.id.clone(), playbook);
        }

        // Add default playbooks if they don't exist
        for default in default_playbooks() {
            if !playbooks.contains_key(&default.id) {
                playbooks.insert(default.id.clone(), default.clone());
                // Save to DB
                self.save_playbook_to_db(&default, pool).await?;
            }
        }

        // Set playbooks in engine
        self.engine.set_playbooks(playbooks).await;

        info!(
            "Loaded {} playbooks from database",
            self.engine.list_playbooks().await.len()
        );
        Ok(())
    }

    /// Convert a database row to a Playbook
    fn row_to_playbook(&self, row: &sqlx::sqlite::SqliteRow) -> Result<Playbook> {
        use sqlx::Column;
        use sqlx::Row;

        let id: String = row.try_get("id")?;
        let name: String = row.try_get("name")?;
        let description: Option<String> = row.try_get("description")?;
        let enabled: bool = row.try_get("enabled")?;
        let triggers_json: String = row.try_get("triggers_json")?;
        let actions_json: String = row.try_get("actions_json")?;
        let cooldown_minutes: Option<i64> = row.try_get("cooldown_minutes")?;
        let max_executions_per_day: Option<i64> = row.try_get("max_executions_per_day")?;
        let created_at: Option<String> = row.try_get("created_at")?;
        let updated_at: Option<String> = row.try_get("updated_at")?;
        let is_builtin: Option<bool> = row.try_get("is_builtin")?;
        let icon: Option<String> = row.try_get("icon")?;
        let color: Option<String> = row.try_get("color")?;

        let triggers: Vec<Trigger> = serde_json::from_str(&triggers_json)?;
        let actions: Vec<Action> = serde_json::from_str(&actions_json)?;

        Ok(Playbook {
            id,
            name,
            description,
            enabled,
            triggers,
            actions,
            cooldown_minutes: cooldown_minutes.map(|v| v as u32),
            max_executions_per_day: max_executions_per_day.map(|v| v as u32),
            created_at: created_at.and_then(|s| {
                chrono::DateTime::parse_from_rfc3339(&s)
                    .ok()
                    .map(|dt| dt.with_timezone(&chrono::Utc))
            }),
            updated_at: updated_at.and_then(|s| {
                chrono::DateTime::parse_from_rfc3339(&s)
                    .ok()
                    .map(|dt| dt.with_timezone(&chrono::Utc))
            }),
            is_builtin,
            icon,
            color,
        })
    }

    /// Save a playbook to the database
    async fn save_playbook_to_db(
        &self,
        playbook: &Playbook,
        pool: &sqlx::SqlitePool,
    ) -> Result<()> {
        let triggers_json = serde_json::to_string(&playbook.triggers)?;
        let actions_json = serde_json::to_string(&playbook.actions)?;

        sqlx::query(
            r#"
            INSERT OR REPLACE INTO playbooks (
                id, name, description, enabled, triggers_json, actions_json,
                cooldown_minutes, max_executions_per_day, created_at, updated_at,
                is_builtin, icon, color
            ) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
            "#,
        )
        .bind(&playbook.id)
        .bind(&playbook.name)
        .bind(&playbook.description)
        .bind(playbook.enabled)
        .bind(&triggers_json)
        .bind(&actions_json)
        .bind(playbook.cooldown_minutes.map(|v| v as i64))
        .bind(playbook.max_executions_per_day.map(|v| v as i64))
        .bind(playbook.created_at.map(|dt| dt.to_rfc3339()))
        .bind(playbook.updated_at.map(|dt| dt.to_rfc3339()))
        .bind(playbook.is_builtin)
        .bind(&playbook.icon)
        .bind(&playbook.color)
        .execute(pool)
        .await?;

        Ok(())
    }

    /// Start the playbook engine
    pub async fn start(&self) -> Result<()> {
        self.engine.start().await?;
        info!("Playbook manager started");
        Ok(())
    }

    // ─── CRUD Operations ────────────────────────────────────────────────────────

    /// List all playbooks
    pub async fn list_playbooks(&self) -> ListPlaybooksResponse {
        let playbooks = self.engine.list_playbooks().await;
        let total = playbooks.len();
        ListPlaybooksResponse { playbooks, total }
    }

    /// Get a single playbook by ID
    pub async fn get_playbook(&self, id: &str) -> Option<Playbook> {
        self.engine.get_playbook(id).await
    }

    /// Create a new playbook
    pub async fn create_playbook(&self, request: CreatePlaybookRequest) -> Result<Playbook> {
        let playbook = Playbook {
            id: String::new(), // Will be generated by engine
            name: request.name,
            description: request.description,
            enabled: false, // Start disabled by default
            triggers: request.triggers,
            actions: request.actions,
            cooldown_minutes: request.cooldown_minutes,
            max_executions_per_day: request.max_executions_per_day,
            created_at: None,
            updated_at: None,
            is_builtin: None,
            icon: request.icon,
            color: request.color,
        };

        let created = self.engine.create_playbook(playbook).await?;

        // Save to database if available
        if let Some(pool) = &self.db_pool {
            self.save_playbook_to_db(&created, pool).await?;
        }

        // Notify subscribers
        self.broadcast_event(PlaybookEvent::PlaybookUpdated {
            playbook: created.clone(),
        })
        .await;

        info!("Created playbook: {} ({})", created.name, created.id);
        Ok(created)
    }

    /// Update an existing playbook
    pub async fn update_playbook(
        &self,
        id: &str,
        request: UpdatePlaybookRequest,
    ) -> Result<Playbook> {
        // Get existing playbook
        let existing = self
            .engine
            .get_playbook(id)
            .await
            .ok_or_else(|| anyhow::anyhow!("Playbook not found: {}", id))?;

        // Build updated playbook
        let updated = Playbook {
            id: id.to_string(),
            name: request.name.unwrap_or(existing.name),
            description: request.description.or(existing.description),
            enabled: request.enabled.unwrap_or(existing.enabled),
            triggers: request.triggers.unwrap_or(existing.triggers),
            actions: request.actions.unwrap_or(existing.actions),
            cooldown_minutes: request.cooldown_minutes.or(existing.cooldown_minutes),
            max_executions_per_day: request
                .max_executions_per_day
                .or(existing.max_executions_per_day),
            created_at: existing.created_at,
            updated_at: None,
            is_builtin: existing.is_builtin,
            icon: request.icon.or(existing.icon),
            color: request.color.or(existing.color),
        };

        let result = self.engine.update_playbook(id, updated).await?;

        // Save to database if available
        if let Some(pool) = &self.db_pool {
            self.save_playbook_to_db(&result, pool).await?;
        }

        // Notify subscribers
        self.broadcast_event(PlaybookEvent::PlaybookUpdated {
            playbook: result.clone(),
        })
        .await;

        info!("Updated playbook: {} ({})", result.name, result.id);
        Ok(result)
    }

    /// Delete a playbook
    pub async fn delete_playbook(&self, id: &str) -> Result<()> {
        self.engine.delete_playbook(id).await?;

        // Delete from database if available
        if let Some(pool) = &self.db_pool {
            sqlx::query("DELETE FROM playbooks WHERE id = ?")
                .bind(id)
                .execute(pool.as_ref())
                .await?;
        }

        // Notify subscribers
        self.broadcast_event(PlaybookEvent::PlaybookDeleted {
            playbook_id: id.to_string(),
        })
        .await;

        info!("Deleted playbook: {}", id);
        Ok(())
    }

    /// Toggle playbook enabled state
    pub async fn toggle_playbook(&self, id: &str, enabled: bool) -> Result<Playbook> {
        let result = self.engine.toggle_playbook(id, enabled).await?;

        // Save to database if available
        if let Some(pool) = &self.db_pool {
            self.save_playbook_to_db(&result, pool).await?;
        }

        // Notify subscribers
        self.broadcast_event(PlaybookEvent::PlaybookUpdated {
            playbook: result.clone(),
        })
        .await;

        info!("Toggled playbook {} to enabled={}", id, enabled);
        Ok(result)
    }

    // ─── WebSocket Subscriptions ────────────────────────────────────────────────

    /// Subscribe to playbook events
    pub async fn subscribe(&self, client_id: String) -> mpsc::Receiver<PlaybookEvent> {
        let (tx, rx) = mpsc::channel(100);

        let mut subscribers = self.ws_subscribers.write().await;
        subscribers
            .entry(client_id)
            .or_insert_with(Vec::new)
            .push(tx);

        rx
    }

    /// Unsubscribe from playbook events
    pub async fn unsubscribe(&self, client_id: &str) {
        let mut subscribers = self.ws_subscribers.write().await;
        subscribers.remove(client_id);
    }

    /// Broadcast an event to all subscribers
    async fn broadcast_event(&self, event: PlaybookEvent) {
        let subscribers = self.ws_subscribers.read().await;

        for (client_id, senders) in subscribers.iter() {
            for sender in senders {
                if let Err(e) = sender.send(event.clone()).await {
                    debug!("Failed to send event to client {}: {}", client_id, e);
                }
            }
        }
    }

    // ─── App State Management ───────────────────────────────────────────────────

    /// Update the state of an application (called from app monitoring)
    pub async fn update_app_state(&self, app_name: &str, window_name: Option<String>) {
        self.engine.update_app_state(app_name, window_name).await;
    }

    /// Remove an application from state (called when app closes)
    pub async fn remove_app_state(&self, app_name: &str) {
        self.engine.remove_app_state(app_name).await;
    }

    // ─── Execution History ──────────────────────────────────────────────────────

    /// Get execution history for a playbook
    pub async fn get_execution_history(
        &self,
        playbook_id: Option<&str>,
        limit: usize,
    ) -> ExecutionHistoryResponse {
        // This would query the database for execution history
        // For now, return empty
        ExecutionHistoryResponse {
            executions: vec![],
            total: 0,
        }
    }

    // ─── Built-in Playbooks ─────────────────────────────────────────────────────

    /// Get built-in playbook templates
    pub async fn get_builtin_templates(&self) -> Vec<Playbook> {
        default_playbooks()
    }
}

/// Handle WebSocket connections for real-time playbook events
pub async fn handle_playbook_ws(
    socket: WebSocket,
    manager: Arc<PlaybookManager>,
    client_id: String,
) {
    use axum::extract::ws::Message;
    use futures::{sink::SinkExt, stream::StreamExt};

    let mut rx = manager.subscribe(client_id.clone()).await;
    let (mut sender, mut receiver) = socket.split();

    // Spawn task to send events to WebSocket
    let send_task = tokio::spawn(async move {
        while let Some(event) = rx.recv().await {
            let msg = match serde_json::to_string(&event) {
                Ok(json) => Message::Text(json),
                Err(e) => {
                    error!("Failed to serialize playbook event: {}", e);
                    continue;
                }
            };

            if let Err(e) = sender.send(msg).await {
                error!("Failed to send WebSocket message: {}", e);
                break;
            }
        }
    });

    // Handle incoming messages (mainly ping/pong)
    while let Some(msg) = receiver.next().await {
        match msg {
            Ok(Message::Ping(_data)) => {
                // Pong is handled automatically by axum
                debug!("Received ping from client {}", client_id);
            }
            Ok(Message::Close(_)) => {
                debug!("Client {} closed connection", client_id);
                break;
            }
            Err(e) => {
                error!("WebSocket error for client {}: {}", client_id, e);
                break;
            }
            _ => {}
        }
    }

    // Clean up subscription
    manager.unsubscribe(&client_id).await;
    send_task.abort();
}
