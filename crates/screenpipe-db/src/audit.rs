//! Audit logging module for compliance and security tracking.
//!
//! This module provides functionality to log and query audit events
//! for tenant data access and modifications.

use crate::tenant::TenantContext;
use chrono::{DateTime, Utc};
use sqlx::{Error as SqlxError, SqlitePool};
use tracing::{debug, error};
use uuid::Uuid;

/// An entry in the audit log
#[derive(Debug, Clone, sqlx::FromRow)]
pub struct AuditLogEntry {
    pub id: String,
    pub tenant_id: Option<String>,
    pub user_id: Option<String>,
    pub action: String,
    pub resource_type: String,
    pub resource_id: Option<String>,
    pub details: Option<String>,
    pub ip_address: Option<String>,
    pub user_agent: Option<String>,
    pub timestamp: DateTime<Utc>,
}

/// Types of audit actions
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AuditAction {
    Create,
    Read,
    Update,
    Delete,
    Export,
    Login,
    Logout,
    Search,
    AccessDenied,
}

impl AuditAction {
    pub fn as_str(&self) -> &'static str {
        match self {
            AuditAction::Create => "CREATE",
            AuditAction::Read => "READ",
            AuditAction::Update => "UPDATE",
            AuditAction::Delete => "DELETE",
            AuditAction::Export => "EXPORT",
            AuditAction::Login => "LOGIN",
            AuditAction::Logout => "LOGOUT",
            AuditAction::Search => "SEARCH",
            AuditAction::AccessDenied => "ACCESS_DENIED",
        }
    }
}

impl std::fmt::Display for AuditAction {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

impl std::str::FromStr for AuditAction {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_uppercase().as_str() {
            "CREATE" => Ok(AuditAction::Create),
            "READ" => Ok(AuditAction::Read),
            "UPDATE" => Ok(AuditAction::Update),
            "DELETE" => Ok(AuditAction::Delete),
            "EXPORT" => Ok(AuditAction::Export),
            "LOGIN" => Ok(AuditAction::Login),
            "LOGOUT" => Ok(AuditAction::Logout),
            "SEARCH" => Ok(AuditAction::Search),
            "ACCESS_DENIED" => Ok(AuditAction::AccessDenied),
            _ => Err(format!("Unknown audit action: {}", s)),
        }
    }
}

/// Logger for audit events
#[derive(Debug, Clone)]
pub struct AuditLogger {
    pool: SqlitePool,
}

impl AuditLogger {
    /// Create a new audit logger
    pub fn new(pool: SqlitePool) -> Self {
        Self { pool }
    }

    /// Log an audit event
    #[allow(clippy::too_many_arguments)]
    pub async fn log(
        &self,
        tenant_ctx: &TenantContext,
        action: AuditAction,
        resource_type: &str,
        resource_id: Option<&str>,
        details: Option<serde_json::Value>,
        ip_address: Option<String>,
        user_agent: Option<String>,
    ) -> Result<(), SqlxError> {
        let id = Uuid::new_v4().to_string();

        sqlx::query(
            "INSERT INTO audit_logs (id, tenant_id, user_id, action, resource_type, resource_id, details, ip_address, user_agent, timestamp)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10)"
        )
        .bind(&id)
        .bind(&tenant_ctx.tenant_id)
        .bind(&tenant_ctx.user_id)
        .bind(action.as_str())
        .bind(resource_type)
        .bind(resource_id)
        .bind(details.map(|d| d.to_string()))
        .bind(ip_address)
        .bind(user_agent)
        .bind(Utc::now())
        .execute(&self.pool)
        .await?;

        debug!(
            "Audit log created: id={}, action={}, resource_type={}, tenant_id={}, user_id={}",
            id, action, resource_type, tenant_ctx.tenant_id, tenant_ctx.user_id
        );

        Ok(())
    }

    /// Log a data access event (READ/SEARCH)
    pub async fn log_access(
        &self,
        tenant_ctx: &TenantContext,
        action: AuditAction,
        resource_type: &str,
        resource_id: Option<&str>,
        query_details: Option<serde_json::Value>,
    ) -> Result<(), SqlxError> {
        self.log(
            tenant_ctx,
            action,
            resource_type,
            resource_id,
            query_details,
            None,
            None,
        )
        .await
    }

    /// Log a data modification event (CREATE/UPDATE/DELETE)
    pub async fn log_modification(
        &self,
        tenant_ctx: &TenantContext,
        action: AuditAction,
        resource_type: &str,
        resource_id: &str,
        changes: Option<serde_json::Value>,
    ) -> Result<(), SqlxError> {
        self.log(
            tenant_ctx,
            action,
            resource_type,
            Some(resource_id),
            changes,
            None,
            None,
        )
        .await
    }

    /// Log an authentication event (LOGIN/LOGOUT)
    pub async fn log_auth(
        &self,
        tenant_ctx: &TenantContext,
        action: AuditAction,
        ip_address: Option<String>,
        user_agent: Option<String>,
        details: Option<serde_json::Value>,
    ) -> Result<(), SqlxError> {
        self.log(
            tenant_ctx, action, "auth", None, details, ip_address, user_agent,
        )
        .await
    }

    /// Log an access denied event
    pub async fn log_access_denied(
        &self,
        tenant_ctx: &TenantContext,
        resource_type: &str,
        resource_id: Option<&str>,
        reason: &str,
        ip_address: Option<String>,
    ) -> Result<(), SqlxError> {
        let details = serde_json::json!({
            "reason": reason,
            "attempted_access": true
        });

        self.log(
            tenant_ctx,
            AuditAction::AccessDenied,
            resource_type,
            resource_id,
            Some(details),
            ip_address,
            None,
        )
        .await
    }

    /// Get audit logs for a tenant with optional filters
    #[allow(clippy::too_many_arguments)]
    pub async fn get_audit_logs(
        &self,
        tenant_ctx: &TenantContext,
        start_time: Option<DateTime<Utc>>,
        end_time: Option<DateTime<Utc>>,
        action: Option<AuditAction>,
        resource_type: Option<&str>,
        limit: u32,
        offset: u32,
    ) -> Result<Vec<AuditLogEntry>, SqlxError> {
        let mut sql = String::from(
            "SELECT id, tenant_id, user_id, action, resource_type, resource_id, details, ip_address, user_agent, timestamp
             FROM audit_logs WHERE 1=1"
        );

        // Apply tenant filter (super admins can see all)
        if !tenant_ctx.is_system() {
            sql.push_str(" AND (tenant_id = ? OR tenant_id IS NULL)");
        }

        // Apply time filters
        if start_time.is_some() {
            sql.push_str(" AND timestamp >= ?");
        }
        if end_time.is_some() {
            sql.push_str(" AND timestamp <= ?");
        }

        // Apply action filter
        if action.is_some() {
            sql.push_str(" AND action = ?");
        }

        // Apply resource type filter
        if resource_type.is_some() {
            sql.push_str(" AND resource_type = ?");
        }

        // Order by timestamp descending (newest first)
        sql.push_str(" ORDER BY timestamp DESC");

        // Apply pagination
        sql.push_str(" LIMIT ? OFFSET ?");

        // Build and execute query
        let mut query = sqlx::query_as::<_, AuditLogEntry>(&sql);

        // Bind tenant filter
        if !tenant_ctx.is_system() {
            query = query.bind(&tenant_ctx.tenant_id);
        }

        // Bind time filters
        if let Some(start) = start_time {
            query = query.bind(start);
        }
        if let Some(end) = end_time {
            query = query.bind(end);
        }

        // Bind action filter
        if let Some(act) = action {
            query = query.bind(act.as_str());
        }

        // Bind resource type filter
        if let Some(rt) = resource_type {
            query = query.bind(rt);
        }

        // Bind pagination
        query = query.bind(limit).bind(offset);

        let logs = query.fetch_all(&self.pool).await?;
        Ok(logs)
    }

    /// Count audit logs for a tenant with optional filters
    pub async fn count_audit_logs(
        &self,
        tenant_ctx: &TenantContext,
        start_time: Option<DateTime<Utc>>,
        end_time: Option<DateTime<Utc>>,
        action: Option<AuditAction>,
        resource_type: Option<&str>,
    ) -> Result<i64, SqlxError> {
        let mut sql = String::from("SELECT COUNT(*) FROM audit_logs WHERE 1=1");

        // Apply tenant filter
        if !tenant_ctx.is_system() {
            sql.push_str(" AND (tenant_id = ? OR tenant_id IS NULL)");
        }

        // Apply time filters
        if start_time.is_some() {
            sql.push_str(" AND timestamp >= ?");
        }
        if end_time.is_some() {
            sql.push_str(" AND timestamp <= ?");
        }

        // Apply action filter
        if action.is_some() {
            sql.push_str(" AND action = ?");
        }

        // Apply resource type filter
        if resource_type.is_some() {
            sql.push_str(" AND resource_type = ?");
        }

        // Build and execute query
        let mut query = sqlx::query_scalar::<_, i64>(&sql);

        // Bind tenant filter
        if !tenant_ctx.is_system() {
            query = query.bind(&tenant_ctx.tenant_id);
        }

        // Bind time filters
        if let Some(start) = start_time {
            query = query.bind(start);
        }
        if let Some(end) = end_time {
            query = query.bind(end);
        }

        // Bind action filter
        if let Some(act) = action {
            query = query.bind(act.as_str());
        }

        // Bind resource type filter
        if let Some(rt) = resource_type {
            query = query.bind(rt);
        }

        let count = query.fetch_one(&self.pool).await?;
        Ok(count)
    }

    /// Delete old audit logs (for manual cleanup)
    pub async fn delete_old_logs(&self, older_than_days: i32) -> Result<u64, SqlxError> {
        let result = sqlx::query("DELETE FROM audit_logs WHERE timestamp < datetime('now', ?1)")
            .bind(format!("-{} days", older_than_days))
            .execute(&self.pool)
            .await?;

        let deleted = result.rows_affected();
        debug!(
            "Deleted {} audit logs older than {} days",
            deleted, older_than_days
        );
        Ok(deleted)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_audit_action_as_str() {
        assert_eq!(AuditAction::Create.as_str(), "CREATE");
        assert_eq!(AuditAction::Read.as_str(), "READ");
        assert_eq!(AuditAction::Update.as_str(), "UPDATE");
        assert_eq!(AuditAction::Delete.as_str(), "DELETE");
        assert_eq!(AuditAction::Export.as_str(), "EXPORT");
        assert_eq!(AuditAction::Login.as_str(), "LOGIN");
        assert_eq!(AuditAction::Logout.as_str(), "LOGOUT");
        assert_eq!(AuditAction::Search.as_str(), "SEARCH");
        assert_eq!(AuditAction::AccessDenied.as_str(), "ACCESS_DENIED");
    }

    #[test]
    fn test_audit_action_display() {
        assert_eq!(format!("{}", AuditAction::Create), "CREATE");
        assert_eq!(format!("{}", AuditAction::Read), "READ");
    }

    #[test]
    fn test_audit_action_from_str() {
        use std::str::FromStr;

        assert_eq!(
            AuditAction::from_str("CREATE").unwrap(),
            AuditAction::Create
        );
        assert_eq!(AuditAction::from_str("read").unwrap(), AuditAction::Read);
        assert_eq!(
            AuditAction::from_str("Update").unwrap(),
            AuditAction::Update
        );
        assert!(AuditAction::from_str("UNKNOWN").is_err());
    }
}
