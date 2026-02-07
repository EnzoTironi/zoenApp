//! Audit logging module for compliance and security tracking.
//!
//! This module provides functionality to log and retrieve audit events
//! for multi-tenant deployments. All actions that modify data should
//! be logged for compliance purposes.

use chrono::{DateTime, Utc};
use screenpipe_db::audit::{AuditAction, AuditLogEntry, AuditLogger};
use screenpipe_db::tenant::TenantContext;
use serde::{Deserialize, Serialize};
use std::sync::Arc;

/// Types of actions that can be audited
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Action {
    // Data operations
    Create,
    Read,
    Update,
    Delete,
    Search,

    // Auth operations
    Login,
    Logout,
    TokenRefresh,
    ApiKeyCreate,
    ApiKeyDelete,

    // Speaker operations
    SpeakerCreate,
    SpeakerUpdate,
    SpeakerDelete,
    SpeakerMerge,
    SpeakerReassign,

    // Tag operations
    TagAdd,
    TagRemove,

    // Export operations
    Export,
    Import,

    // Admin operations
    SettingsUpdate,
    TenantCreate,
    TenantUpdate,
    TenantDelete,
}

impl Action {
    /// Convert action to string representation
    pub fn as_str(&self) -> &'static str {
        match self {
            Action::Create => "create",
            Action::Read => "read",
            Action::Update => "update",
            Action::Delete => "delete",
            Action::Search => "search",
            Action::Login => "login",
            Action::Logout => "logout",
            Action::TokenRefresh => "token_refresh",
            Action::ApiKeyCreate => "api_key_create",
            Action::ApiKeyDelete => "api_key_delete",
            Action::SpeakerCreate => "speaker_create",
            Action::SpeakerUpdate => "speaker_update",
            Action::SpeakerDelete => "speaker_delete",
            Action::SpeakerMerge => "speaker_merge",
            Action::SpeakerReassign => "speaker_reassign",
            Action::TagAdd => "tag_add",
            Action::TagRemove => "tag_remove",
            Action::Export => "export",
            Action::Import => "import",
            Action::SettingsUpdate => "settings_update",
            Action::TenantCreate => "tenant_create",
            Action::TenantUpdate => "tenant_update",
            Action::TenantDelete => "tenant_delete",
        }
    }
}

impl std::fmt::Display for Action {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

impl From<Action> for AuditAction {
    fn from(action: Action) -> Self {
        match action {
            Action::Create => AuditAction::Create,
            Action::Read => AuditAction::Read,
            Action::Update => AuditAction::Update,
            Action::Delete => AuditAction::Delete,
            Action::Search => AuditAction::Search,
            Action::Login => AuditAction::Login,
            Action::Logout => AuditAction::Logout,
            Action::Export => AuditAction::Export,
            _ => AuditAction::Create, // Default for actions that don't have a direct mapping
        }
    }
}

/// Resource types that can be audited
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Resource {
    Frame,
    AudioChunk,
    AudioTranscription,
    OcrText,
    Speaker,
    Tag,
    UiEvent,
    UiMonitoring,
    User,
    ApiKey,
    Settings,
    Tenant,
    Search,
}

impl Resource {
    /// Convert resource to string representation
    pub fn as_str(&self) -> &'static str {
        match self {
            Resource::Frame => "frame",
            Resource::AudioChunk => "audio_chunk",
            Resource::AudioTranscription => "audio_transcription",
            Resource::OcrText => "ocr_text",
            Resource::Speaker => "speaker",
            Resource::Tag => "tag",
            Resource::UiEvent => "ui_event",
            Resource::UiMonitoring => "ui_monitoring",
            Resource::User => "user",
            Resource::ApiKey => "api_key",
            Resource::Settings => "settings",
            Resource::Tenant => "tenant",
            Resource::Search => "search",
        }
    }
}

impl std::fmt::Display for Resource {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

/// Audit log service for recording and retrieving audit events
#[derive(Clone)]
pub struct AuditService {
    logger: AuditLogger,
}

impl AuditService {
    /// Create a new audit service from a database pool
    pub fn new(pool: sqlx::SqlitePool) -> Self {
        Self {
            logger: AuditLogger::new(pool),
        }
    }

    /// Create a new audit service from an existing AuditLogger
    pub fn from_logger(logger: AuditLogger) -> Self {
        Self { logger }
    }

    /// Log an audit event
    ///
    /// # Arguments
    ///
    /// * `tenant` - The tenant context (includes user_id)
    /// * `action` - The action being performed
    /// * `resource` - The type of resource being accessed
    /// * `resource_id` - Optional ID of the specific resource
    /// * `details` - Optional JSON string with additional details
    /// * `ip_address` - Optional client IP address
    /// * `user_agent` - Optional user agent string
    pub async fn log(
        &self,
        tenant: &TenantContext,
        action: Action,
        resource: Resource,
        resource_id: Option<&str>,
        details: Option<&str>,
        ip_address: Option<&str>,
        user_agent: Option<&str>,
    ) -> Result<(), AuditError> {
        let details_json = details.and_then(|d| serde_json::from_str(d).ok());

        self.logger
            .log(
                tenant,
                action.into(),
                resource.as_str(),
                resource_id,
                details_json,
                ip_address.map(|s| s.to_string()),
                user_agent.map(|s| s.to_string()),
            )
            .await
            .map_err(|e| AuditError::DatabaseError(e.to_string()))?;

        Ok(())
    }

    /// Log a search query (for compliance tracking)
    pub async fn log_search(
        &self,
        tenant: &TenantContext,
        query: &str,
        content_type: &str,
        ip_address: Option<&str>,
    ) -> Result<(), AuditError> {
        let details = serde_json::json!({
            "query": query,
            "content_type": content_type,
        });

        self.logger
            .log(
                tenant,
                AuditAction::Search,
                "search",
                None,
                Some(details),
                ip_address.map(|s| s.to_string()),
                None,
            )
            .await
            .map_err(|e| AuditError::DatabaseError(e.to_string()))?;

        Ok(())
    }

    /// Log a data modification operation
    pub async fn log_data_op(
        &self,
        tenant: &TenantContext,
        action: Action,
        resource: Resource,
        resource_id: &str,
        details: Option<serde_json::Value>,
        ip_address: Option<&str>,
    ) -> Result<(), AuditError> {
        self.logger
            .log(
                tenant,
                action.into(),
                resource.as_str(),
                Some(resource_id),
                details,
                ip_address.map(|s| s.to_string()),
                None,
            )
            .await
            .map_err(|e| AuditError::DatabaseError(e.to_string()))?;

        Ok(())
    }

    /// Retrieve audit logs for a tenant
    ///
    /// # Arguments
    ///
    /// * `tenant` - The tenant context (only returns logs for this tenant)
    /// * `start_time` - Optional start time filter
    /// * `end_time` - Optional end time filter
    /// * `limit` - Maximum number of results to return
    /// * `offset` - Offset for pagination
    pub async fn get_logs(
        &self,
        tenant: &TenantContext,
        start_time: Option<DateTime<Utc>>,
        end_time: Option<DateTime<Utc>>,
        limit: u32,
        offset: u32,
    ) -> Result<Vec<AuditLogEntry>, AuditError> {
        // Only admins can view logs, or users can view their own logs
        if !tenant.has_role("admin") && !tenant.has_role("auditor") {
            return Err(AuditError::Unauthorized(
                "Insufficient permissions to view audit logs".to_string(),
            ));
        }

        self.logger
            .get_audit_logs(tenant, start_time, end_time, None, None, limit, offset)
            .await
            .map_err(|e| AuditError::DatabaseError(e.to_string()))
    }

    /// Get recent audit logs for a specific user
    pub async fn get_user_logs(
        &self,
        tenant: &TenantContext,
        user_id: &str,
        limit: u32,
    ) -> Result<Vec<AuditLogEntry>, AuditError> {
        // Users can only view their own logs, admins can view any
        if tenant.user_id != user_id && !tenant.has_role("admin") {
            return Err(AuditError::Unauthorized(
                "Cannot view logs for other users".to_string(),
            ));
        }

        // For now, filter from all logs
        // In production, add a dedicated query for user-specific logs
        let logs = self.get_logs(tenant, None, None, limit * 10, 0).await?;

        Ok(logs
            .into_iter()
            .filter(|log| log.user_id.as_deref() == Some(user_id))
            .take(limit as usize)
            .collect())
    }

    /// Count audit logs for a tenant
    pub async fn count_logs(
        &self,
        tenant: &TenantContext,
        start_time: Option<DateTime<Utc>>,
        end_time: Option<DateTime<Utc>>,
    ) -> Result<i64, AuditError> {
        if !tenant.has_role("admin") && !tenant.has_role("auditor") {
            return Err(AuditError::Unauthorized(
                "Insufficient permissions to count audit logs".to_string(),
            ));
        }

        self.logger
            .count_audit_logs(tenant, start_time, end_time, None, None)
            .await
            .map_err(|e| AuditError::DatabaseError(e.to_string()))
    }

    /// Delete old audit logs (admin only)
    pub async fn delete_old_logs(
        &self,
        tenant: &TenantContext,
        older_than_days: i32,
    ) -> Result<u64, AuditError> {
        if !tenant.has_role("admin") {
            return Err(AuditError::Unauthorized(
                "Only admins can delete audit logs".to_string(),
            ));
        }

        self.logger
            .delete_old_logs(older_than_days)
            .await
            .map_err(|e| AuditError::DatabaseError(e.to_string()))
    }
}

/// Errors that can occur during audit operations
#[derive(Debug, Clone)]
pub enum AuditError {
    DatabaseError(String),
    Unauthorized(String),
    InvalidInput(String),
}

impl std::fmt::Display for AuditError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            AuditError::DatabaseError(msg) => write!(f, "Database error: {}", msg),
            AuditError::Unauthorized(msg) => write!(f, "Unauthorized: {}", msg),
            AuditError::InvalidInput(msg) => write!(f, "Invalid input: {}", msg),
        }
    }
}

impl std::error::Error for AuditError {}

/// Middleware helper to extract client info for audit logging
pub struct AuditContext {
    pub ip_address: Option<String>,
    pub user_agent: Option<String>,
}

impl AuditContext {
    /// Create audit context from HTTP headers
    pub fn from_headers(headers: &axum::http::HeaderMap) -> Self {
        let ip_address = headers
            .get("x-forwarded-for")
            .or_else(|| headers.get("x-real-ip"))
            .and_then(|v| v.to_str().ok())
            .map(|s| s.split(',').next().unwrap_or(s).trim().to_string());

        let user_agent = headers
            .get(axum::http::header::USER_AGENT)
            .and_then(|v| v.to_str().ok())
            .map(|s| s.to_string());

        Self {
            ip_address,
            user_agent,
        }
    }
}

/// Macro to simplify audit logging in handlers
#[macro_export]
macro_rules! audit_log {
    ($audit:expr, $tenant:expr, $action:expr, $resource:expr) => {
        $audit
            .log($tenant, $action, $resource, None, None, None, None)
            .await
            .ok();
    };
    ($audit:expr, $tenant:expr, $action:expr, $resource:expr, $resource_id:expr) => {
        $audit
            .log(
                $tenant,
                $action,
                $resource,
                Some($resource_id),
                None,
                None,
                None,
            )
            .await
            .ok();
    };
    ($audit:expr, $tenant:expr, $action:expr, $resource:expr, $resource_id:expr, $details:expr) => {
        $audit
            .log(
                $tenant,
                $action,
                $resource,
                Some($resource_id),
                Some($details),
                None,
                None,
            )
            .await
            .ok();
    };
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_action_as_str() {
        assert_eq!(Action::Create.as_str(), "create");
        assert_eq!(Action::Search.as_str(), "search");
        assert_eq!(Action::SpeakerReassign.as_str(), "speaker_reassign");
    }

    #[test]
    fn test_resource_as_str() {
        assert_eq!(Resource::Frame.as_str(), "frame");
        assert_eq!(Resource::Speaker.as_str(), "speaker");
    }

    #[test]
    fn test_action_display() {
        assert_eq!(format!("{}", Action::Create), "create");
    }

    #[test]
    fn test_action_into_audit_action() {
        use screenpipe_db::audit::AuditAction;
        assert_eq!(AuditAction::from(Action::Create), AuditAction::Create);
        assert_eq!(AuditAction::from(Action::Read), AuditAction::Read);
        assert_eq!(AuditAction::from(Action::Search), AuditAction::Search);
    }
}
