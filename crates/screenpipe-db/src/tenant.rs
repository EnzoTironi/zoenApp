use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::{Sqlite, SqlitePool};
use std::fmt;
use uuid::Uuid;

/// Tenant roles for access control
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum TenantRole {
    /// Super admin - access to all tenants
    SuperAdmin,
    /// Admin of a specific tenant
    Admin,
    /// Regular member of a tenant
    Member,
    /// Read-only access
    ReadOnly,
}

impl TenantRole {
    /// Check if this role can access data from another tenant
    pub fn can_access_tenant(&self, user_tenant: &str, resource_tenant: Option<&str>) -> bool {
        match self {
            TenantRole::SuperAdmin => true,
            TenantRole::Admin | TenantRole::Member | TenantRole::ReadOnly => {
                resource_tenant.map_or(true, |rt| rt == user_tenant)
            }
        }
    }

    /// Check if this role can modify data
    pub fn can_modify(&self) -> bool {
        matches!(
            self,
            TenantRole::SuperAdmin | TenantRole::Admin | TenantRole::Member
        )
    }

    /// Check if this role can delete data
    pub fn can_delete(&self) -> bool {
        matches!(self, TenantRole::SuperAdmin | TenantRole::Admin)
    }

    /// Check if this role can manage users
    pub fn can_manage_users(&self) -> bool {
        matches!(self, TenantRole::SuperAdmin | TenantRole::Admin)
    }

    /// Convert to string representation
    pub fn as_str(&self) -> &'static str {
        match self {
            TenantRole::SuperAdmin => "super_admin",
            TenantRole::Admin => "admin",
            TenantRole::Member => "member",
            TenantRole::ReadOnly => "readonly",
        }
    }
}

impl fmt::Display for TenantRole {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

impl std::str::FromStr for TenantRole {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "super_admin" | "superadmin" => Ok(TenantRole::SuperAdmin),
            "admin" => Ok(TenantRole::Admin),
            "member" => Ok(TenantRole::Member),
            "readonly" | "read_only" | "read-only" => Ok(TenantRole::ReadOnly),
            _ => Err(format!("Unknown tenant role: {}", s)),
        }
    }
}

/// Context identifying the current tenant and user for a request.
/// This is typically extracted from JWT tokens in middleware and passed
/// through the request lifecycle.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub struct TenantContext {
    /// The unique identifier for the tenant (organization/account)
    pub tenant_id: String,
    /// The unique identifier for the user within the tenant
    pub user_id: String,
    /// Optional session ID for tracking
    #[serde(skip_serializing_if = "Option::is_none")]
    pub session_id: Option<String>,
    /// Optional roles/permissions for the user
    #[serde(skip_serializing_if = "Vec::is_empty", default)]
    pub roles: Vec<String>,
    /// The primary role of the user for access control
    #[serde(skip_serializing_if = "Option::is_none")]
    pub role: Option<TenantRole>,
}

impl TenantContext {
    /// Create a new tenant context
    pub fn new(tenant_id: impl Into<String>, user_id: impl Into<String>) -> Self {
        Self {
            tenant_id: tenant_id.into(),
            user_id: user_id.into(),
            session_id: None,
            roles: Vec::new(),
            role: None,
        }
    }

    /// Create a new tenant context with UUIDs
    pub fn new_uuid(tenant_id: Uuid, user_id: Uuid) -> Self {
        Self {
            tenant_id: tenant_id.to_string(),
            user_id: user_id.to_string(),
            session_id: None,
            roles: Vec::new(),
            role: None,
        }
    }

    /// Create a system/admin context for background tasks
    pub fn system() -> Self {
        Self {
            tenant_id: "system".to_string(),
            user_id: "system".to_string(),
            session_id: None,
            roles: vec!["admin".to_string()],
            role: Some(TenantRole::SuperAdmin),
        }
    }

    /// Create an anonymous context for unauthenticated requests
    /// Use with caution - this bypasses tenant isolation
    pub fn anonymous() -> Self {
        Self {
            tenant_id: "anonymous".to_string(),
            user_id: "anonymous".to_string(),
            session_id: None,
            roles: Vec::new(),
            role: None,
        }
    }

    /// Check if this is a system context
    pub fn is_system(&self) -> bool {
        self.tenant_id == "system"
            || self.roles.contains(&"admin".to_string())
            || self.role == Some(TenantRole::SuperAdmin)
    }

    /// Check if this is an anonymous context
    pub fn is_anonymous(&self) -> bool {
        self.tenant_id == "anonymous"
    }

    /// Add a role to the context
    pub fn with_role(mut self, role: impl Into<String>) -> Self {
        self.roles.push(role.into());
        self
    }

    /// Set the primary role for access control
    pub fn with_tenant_role(mut self, role: TenantRole) -> Self {
        self.role = Some(role);
        self
    }

    /// Add session ID to the context
    pub fn with_session(mut self, session_id: impl Into<String>) -> Self {
        self.session_id = Some(session_id.into());
        self
    }

    /// Check if the user has a specific role
    pub fn has_role(&self, role: &str) -> bool {
        self.roles.contains(&role.to_string())
    }

    /// Get the effective tenant role
    pub fn get_role(&self) -> TenantRole {
        self.role.unwrap_or_else(|| {
            if self.is_system() {
                TenantRole::SuperAdmin
            } else {
                TenantRole::Member
            }
        })
    }

    /// Check if the user can access data from a specific tenant
    pub fn can_access_tenant(&self, resource_tenant_id: Option<&str>) -> bool {
        self.get_role()
            .can_access_tenant(&self.tenant_id, resource_tenant_id)
    }

    /// Check if the user can modify data
    pub fn can_modify(&self) -> bool {
        self.get_role().can_modify()
    }

    /// Check if the user can delete data
    pub fn can_delete(&self) -> bool {
        self.get_role().can_delete()
    }
}

impl fmt::Display for TenantContext {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "TenantContext(tenant={}, user={})",
            self.tenant_id, self.user_id
        )
    }
}

/// Trait for types that can be scoped to a tenant.
/// This allows query builders and database operations to be
/// automatically filtered by tenant.
pub trait TenantScoped {
    /// Apply tenant scoping to this query/operation
    fn for_tenant(self, tenant: &TenantContext) -> Self;

    /// Check if this operation is properly scoped to a tenant
    fn is_tenant_scoped(&self) -> bool;
}

/// Extension trait for SQLx queries to add tenant filtering
#[async_trait::async_trait]
pub trait TenantQueryExt {
    /// Bind tenant_id to a query
    fn bind_tenant(self, tenant: &TenantContext) -> Self;
}

#[async_trait::async_trait]
impl<'q> TenantQueryExt for sqlx::query::Query<'q, Sqlite, sqlx::sqlite::SqliteArguments<'q>> {
    fn bind_tenant(self, tenant: &TenantContext) -> Self {
        self.bind(tenant.tenant_id.clone())
    }
}

/// Error type for tenant-related failures
#[derive(Debug, Clone)]
pub enum TenantError {
    MissingTenantId,
    InvalidTenantId(String),
    TenantNotFound(String),
    Unauthorized(String),
}

impl fmt::Display for TenantError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            TenantError::MissingTenantId => write!(f, "Tenant ID is required but was not provided"),
            TenantError::InvalidTenantId(id) => write!(f, "Invalid tenant ID: {}", id),
            TenantError::TenantNotFound(id) => write!(f, "Tenant not found: {}", id),
            TenantError::Unauthorized(msg) => write!(f, "Unauthorized: {}", msg),
        }
    }
}

impl std::error::Error for TenantError {}

/// Metadata about a tenant stored in the database
#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct TenantMetadata {
    pub tenant_id: String,
    pub name: Option<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub settings: Option<String>, // JSON blob
}

/// Manager for tenant-related operations
pub struct TenantManager {
    pool: SqlitePool,
}

impl TenantManager {
    pub fn new(pool: SqlitePool) -> Self {
        Self { pool }
    }

    /// Get or create tenant metadata
    pub async fn get_or_create_tenant(
        &self,
        tenant_id: &str,
        name: Option<&str>,
    ) -> Result<TenantMetadata, sqlx::Error> {
        // Try to get existing tenant
        if let Some(tenant) = sqlx::query_as::<_, TenantMetadata>(
            "SELECT tenant_id, name, created_at, updated_at, settings FROM tenant_metadata WHERE tenant_id = ?1"
        )
        .bind(tenant_id)
        .fetch_optional(&self.pool)
        .await? {
            return Ok(tenant);
        }

        // Create new tenant
        let tenant = sqlx::query_as::<_, TenantMetadata>(
            "INSERT INTO tenant_metadata (tenant_id, name) VALUES (?1, ?2) RETURNING tenant_id, name, created_at, updated_at, settings"
        )
        .bind(tenant_id)
        .bind(name)
        .fetch_one(&self.pool)
        .await?;

        Ok(tenant)
    }

    /// Update tenant settings
    pub async fn update_settings(
        &self,
        tenant_id: &str,
        settings: &str,
    ) -> Result<(), sqlx::Error> {
        sqlx::query(
            "UPDATE tenant_metadata SET settings = ?1, updated_at = ?2 WHERE tenant_id = ?3",
        )
        .bind(settings)
        .bind(Utc::now())
        .bind(tenant_id)
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    /// List all tenants (admin only)
    pub async fn list_tenants(&self) -> Result<Vec<TenantMetadata>, sqlx::Error> {
        let tenants = sqlx::query_as::<_, TenantMetadata>(
            "SELECT tenant_id, name, created_at, updated_at, settings FROM tenant_metadata ORDER BY created_at DESC"
        )
        .fetch_all(&self.pool)
        .await?;

        Ok(tenants)
    }
}

/// Helper function to validate a tenant ID format (UUID)
pub fn validate_tenant_id(tenant_id: &str) -> Result<Uuid, TenantError> {
    if tenant_id == "system" || tenant_id == "anonymous" {
        // Special tenant IDs that don't need to be UUIDs
        return Ok(Uuid::nil());
    }

    Uuid::parse_str(tenant_id).map_err(|_| TenantError::InvalidTenantId(tenant_id.to_string()))
}

/// SQL fragments for tenant filtering
pub mod sql {
    /// WHERE clause for tenant filtering
    pub const TENANT_FILTER: &str = "tenant_id = ? OR tenant_id IS NULL";

    /// AND clause for tenant filtering (when other WHERE conditions exist)
    pub const TENANT_FILTER_AND: &str = "AND (tenant_id = ? OR tenant_id IS NULL)";

    /// WHERE clause for strict tenant filtering (no NULL allowed)
    pub const TENANT_FILTER_STRICT: &str = "tenant_id = ?";

    /// AND clause for strict tenant filtering
    pub const TENANT_FILTER_STRICT_AND: &str = "AND tenant_id = ?";
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_tenant_context_creation() {
        let ctx = TenantContext::new("tenant-123", "user-456");
        assert_eq!(ctx.tenant_id, "tenant-123");
        assert_eq!(ctx.user_id, "user-456");
        assert!(!ctx.is_system());
        assert!(!ctx.is_anonymous());
    }

    #[test]
    fn test_tenant_context_system() {
        let ctx = TenantContext::system();
        assert!(ctx.is_system());
        assert!(!ctx.is_anonymous());
        assert!(ctx.has_role("admin"));
    }

    #[test]
    fn test_tenant_context_anonymous() {
        let ctx = TenantContext::anonymous();
        assert!(ctx.is_anonymous());
        assert!(!ctx.is_system());
    }

    #[test]
    fn test_tenant_context_roles() {
        let ctx = TenantContext::new("t", "u")
            .with_role("admin")
            .with_role("user");

        assert!(ctx.has_role("admin"));
        assert!(ctx.has_role("user"));
        assert!(!ctx.has_role("superadmin"));
    }

    #[test]
    fn test_validate_tenant_id() {
        let valid_uuid = "550e8400-e29b-41d4-a716-446655440000";
        assert!(validate_tenant_id(valid_uuid).is_ok());

        assert!(validate_tenant_id("invalid").is_err());

        // Special IDs
        assert!(validate_tenant_id("system").is_ok());
        assert!(validate_tenant_id("anonymous").is_ok());
    }

    #[test]
    fn test_tenant_role_as_str() {
        assert_eq!(TenantRole::SuperAdmin.as_str(), "super_admin");
        assert_eq!(TenantRole::Admin.as_str(), "admin");
        assert_eq!(TenantRole::Member.as_str(), "member");
        assert_eq!(TenantRole::ReadOnly.as_str(), "readonly");
    }

    #[test]
    fn test_tenant_role_from_str() {
        use std::str::FromStr;

        assert_eq!(
            TenantRole::from_str("super_admin").unwrap(),
            TenantRole::SuperAdmin
        );
        assert_eq!(
            TenantRole::from_str("superadmin").unwrap(),
            TenantRole::SuperAdmin
        );
        assert_eq!(TenantRole::from_str("admin").unwrap(), TenantRole::Admin);
        assert_eq!(TenantRole::from_str("member").unwrap(), TenantRole::Member);
        assert_eq!(
            TenantRole::from_str("readonly").unwrap(),
            TenantRole::ReadOnly
        );
        assert_eq!(
            TenantRole::from_str("read_only").unwrap(),
            TenantRole::ReadOnly
        );
        assert_eq!(
            TenantRole::from_str("read-only").unwrap(),
            TenantRole::ReadOnly
        );
        assert!(TenantRole::from_str("unknown").is_err());
    }

    #[test]
    fn test_tenant_role_permissions() {
        // SuperAdmin can do everything
        assert!(TenantRole::SuperAdmin.can_access_tenant("any", Some("other")));
        assert!(TenantRole::SuperAdmin.can_modify());
        assert!(TenantRole::SuperAdmin.can_delete());
        assert!(TenantRole::SuperAdmin.can_manage_users());

        // Admin can modify and delete
        assert!(TenantRole::Admin.can_modify());
        assert!(TenantRole::Admin.can_delete());
        assert!(TenantRole::Admin.can_manage_users());

        // Member can only modify
        assert!(TenantRole::Member.can_modify());
        assert!(!TenantRole::Member.can_delete());
        assert!(!TenantRole::Member.can_manage_users());

        // ReadOnly cannot modify
        assert!(!TenantRole::ReadOnly.can_modify());
        assert!(!TenantRole::ReadOnly.can_delete());
        assert!(!TenantRole::ReadOnly.can_manage_users());
    }

    #[test]
    fn test_tenant_role_access_control() {
        // SuperAdmin can access any tenant
        assert!(TenantRole::SuperAdmin.can_access_tenant("tenant1", Some("tenant2")));

        // Other roles can only access their own tenant
        assert!(TenantRole::Admin.can_access_tenant("tenant1", Some("tenant1")));
        assert!(!TenantRole::Admin.can_access_tenant("tenant1", Some("tenant2")));

        // NULL tenant (unassigned data) is accessible
        assert!(TenantRole::Member.can_access_tenant("tenant1", None));
    }

    #[test]
    fn test_tenant_context_with_role() {
        let ctx = TenantContext::new("tenant-123", "user-456").with_tenant_role(TenantRole::Admin);

        assert_eq!(ctx.get_role(), TenantRole::Admin);
        assert!(ctx.can_modify());
        assert!(ctx.can_delete());
        assert!(ctx.can_access_tenant(Some("tenant-123")));
        assert!(!ctx.can_access_tenant(Some("other-tenant")));
    }

    #[test]
    fn test_tenant_context_system_permissions() {
        let ctx = TenantContext::system();
        assert_eq!(ctx.get_role(), TenantRole::SuperAdmin);
        assert!(ctx.can_access_tenant(Some("any-tenant")));
    }
}
