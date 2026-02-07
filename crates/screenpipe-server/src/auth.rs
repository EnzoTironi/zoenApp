use axum::{
    async_trait,
    extract::{FromRequestParts, Request, State},
    http::{header, request::Parts, Method, StatusCode},
    middleware::Next,
    response::{IntoResponse, Json, Response},
};
use bcrypt::{hash, verify, DEFAULT_COST};
use governor::middleware::NoOpMiddleware;
use jsonwebtoken::{decode, encode, Algorithm, DecodingKey, EncodingKey, Header, Validation};
use oasgen::OaSchema;
use screenpipe_db::tenant::TenantContext;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tower_governor::{
    governor::GovernorConfigBuilder, key_extractor::PeerIpKeyExtractor, GovernorLayer,
};
use tracing::{debug, error, warn};

use crate::server::AppState;

/// Bcrypt cost factor for password hashing
/// Higher values are more secure but slower. 12 is a good balance.
const BCRYPT_COST: u32 = 12;

/// Hash a password using bcrypt
///
/// # Arguments
/// * `password` - The plaintext password to hash
///
/// # Returns
/// * `Ok(String)` - The bcrypt hash of the password
/// * `Err(AuthError)` - If hashing fails
pub fn hash_password(password: &str) -> Result<String, AuthError> {
    hash(password, BCRYPT_COST).map_err(|e| {
        error!("Failed to hash password: {}", e);
        AuthError::InternalError
    })
}

/// Verify a password against a bcrypt hash
///
/// # Arguments
/// * `password` - The plaintext password to verify
/// * `hash` - The bcrypt hash to verify against
///
/// # Returns
/// * `Ok(bool)` - true if the password matches, false otherwise
/// * `Err(AuthError)` - If verification fails
pub fn verify_password(password: &str, hash: &str) -> Result<bool, AuthError> {
    verify(password, hash).map_err(|e| {
        error!("Failed to verify password: {}", e);
        AuthError::InternalError
    })
}

/// User roles for authorization
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, OaSchema)]
#[serde(rename_all = "snake_case")]
pub enum Role {
    /// Full access to all endpoints
    Admin,
    /// Standard user access
    User,
    /// Read-only access
    Readonly,
}

impl Role {
    /// Check if this role has permission to access an endpoint requiring a specific role
    pub fn has_permission(&self, required: Role) -> bool {
        match (self, required) {
            (Role::Admin, _) => true,
            (Role::User, Role::User) | (Role::User, Role::Readonly) => true,
            (Role::Readonly, Role::Readonly) => true,
            _ => false,
        }
    }
}

impl Default for Role {
    fn default() -> Self {
        Role::Readonly
    }
}

/// Authenticated user information extracted from JWT or API key
#[derive(Debug, Clone, Serialize, Deserialize, OaSchema)]
pub struct AuthenticatedUser {
    pub user_id: String,
    pub role: Role,
    /// Optional API key ID if authenticated via API key
    pub api_key_id: Option<String>,
    /// Tenant ID for multi-tenant deployments
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tenant_id: Option<String>,
    /// Additional roles for fine-grained permissions
    #[serde(skip_serializing_if = "Vec::is_empty", default)]
    pub roles: Vec<String>,
}

impl AuthenticatedUser {
    /// Convert to TenantContext for database operations
    pub fn to_tenant_context(&self) -> TenantContext {
        let tenant_id = self
            .tenant_id
            .clone()
            .unwrap_or_else(|| "default".to_string());

        let mut ctx = TenantContext::new(&tenant_id, &self.user_id);

        // Add roles
        ctx = ctx.with_role(format!("{:?}", self.role).to_lowercase());
        for role in &self.roles {
            ctx = ctx.with_role(role.clone());
        }

        ctx
    }

    /// Check if user belongs to a specific tenant
    pub fn belongs_to_tenant(&self, tenant_id: &str) -> bool {
        self.tenant_id
            .as_ref()
            .map(|t| t == tenant_id)
            .unwrap_or(true) // Default tenant matches all
    }

    /// Check if user has a specific role
    pub fn has_role(&self, role: &str) -> bool {
        let base_role = format!("{:?}", self.role).to_lowercase();
        if base_role == role {
            return true;
        }
        self.roles
            .iter()
            .any(|r| r.to_lowercase() == role.to_lowercase())
    }
}

/// Token type to distinguish between access and refresh tokens
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TokenType {
    Access,
    Refresh,
}

impl Default for TokenType {
    fn default() -> Self {
        TokenType::Access
    }
}

/// JWT claims structure with tenant support
#[derive(Debug, Serialize, Deserialize)]
struct Claims {
    sub: String, // user_id
    role: Role,
    /// Token type (access or refresh)
    #[serde(default)]
    token_type: TokenType,
    /// Tenant ID for multi-tenant deployments
    #[serde(skip_serializing_if = "Option::is_none")]
    tenant_id: Option<String>,
    /// Additional roles/permissions for the user
    #[serde(skip_serializing_if = "Vec::is_empty", default)]
    roles: Vec<String>,
    exp: usize, // expiration timestamp
    iat: usize, // issued at timestamp
    /// JWT ID for token revocation support
    #[serde(skip_serializing_if = "Option::is_none")]
    jti: Option<String>,
}

/// Login request payload
#[derive(Debug, Deserialize)]
pub struct LoginRequest {
    pub username: String,
    pub password: String,
}

/// Login response payload with refresh token support
#[derive(Debug, Serialize)]
pub struct LoginResponse {
    pub access_token: String,
    pub refresh_token: String,
    pub token_type: String,
    pub expires_in: u64,
    pub user: UserInfo,
}

/// User information returned after login
#[derive(Debug, Serialize)]
pub struct UserInfo {
    pub user_id: String,
    pub role: Role,
}

/// Refresh token request
#[derive(Debug, Deserialize)]
pub struct RefreshTokenRequest {
    pub refresh_token: String,
}

/// Refresh token response
#[derive(Debug, Serialize)]
pub struct RefreshTokenResponse {
    pub access_token: String,
    pub refresh_token: String,
    pub token_type: String,
    pub expires_in: u64,
}

/// Logout request
#[derive(Debug, Deserialize)]
pub struct LogoutRequest {
    pub refresh_token: Option<String>,
}

/// API key validation request (for internal use)
#[derive(Debug, Deserialize)]
pub struct ApiKeyRequest {
    pub api_key: String,
}

/// Custom error type for authentication failures
#[derive(Debug)]
pub enum AuthError {
    MissingCredentials,
    InvalidToken,
    InvalidApiKey,
    ExpiredToken,
    InsufficientPermissions,
    InternalError,
}

impl IntoResponse for AuthError {
    fn into_response(self) -> Response {
        let (status, message) = match self {
            AuthError::MissingCredentials => (
                StatusCode::UNAUTHORIZED,
                "Missing authentication credentials",
            ),
            AuthError::InvalidToken => (StatusCode::UNAUTHORIZED, "Invalid token"),
            AuthError::InvalidApiKey => (StatusCode::UNAUTHORIZED, "Invalid API key"),
            AuthError::ExpiredToken => (StatusCode::UNAUTHORIZED, "Token expired"),
            AuthError::InsufficientPermissions => {
                (StatusCode::FORBIDDEN, "Insufficient permissions")
            }
            AuthError::InternalError => (
                StatusCode::INTERNAL_SERVER_ERROR,
                "Internal authentication error",
            ),
        };

        let body = Json(serde_json::json!({
            "error": message,
            "success": false
        }));

        (status, body).into_response()
    }
}

/// Extract the JWT secret from environment
fn get_jwt_secret() -> Vec<u8> {
    std::env::var("JWT_SECRET")
        .map(|s| s.into_bytes())
        .expect("JWT_SECRET environment variable must be set")
}

/// Validate a JWT token and return the authenticated user
pub fn validate_jwt(token: &str) -> Result<AuthenticatedUser, AuthError> {
    let secret = get_jwt_secret();
    let validation = Validation::new(Algorithm::HS256);

    let token_data = decode::<Claims>(token, &DecodingKey::from_secret(&secret), &validation)
        .map_err(|e| {
            debug!("JWT validation failed: {}", e);
            match e.kind() {
                jsonwebtoken::errors::ErrorKind::ExpiredSignature => AuthError::ExpiredToken,
                _ => AuthError::InvalidToken,
            }
        })?;

    Ok(AuthenticatedUser {
        user_id: token_data.claims.sub,
        role: token_data.claims.role,
        api_key_id: None,
        tenant_id: token_data.claims.tenant_id,
        roles: token_data.claims.roles,
    })
}

/// Generate a new JWT access token for a user (15 minutes expiration)
pub fn generate_jwt(user_id: &str, role: Role) -> Result<String, AuthError> {
    generate_jwt_with_tenant(user_id, role, None, vec![])
}

/// Generate a new JWT access token for a user with tenant information (15 minutes expiration)
pub fn generate_jwt_with_tenant(
    user_id: &str,
    role: Role,
    tenant_id: Option<&str>,
    roles: Vec<String>,
) -> Result<String, AuthError> {
    let secret = get_jwt_secret();
    let now = jsonwebtoken::get_current_timestamp() as usize;
    let expiration = now + 15 * 60; // 15 minutes

    let claims = Claims {
        sub: user_id.to_string(),
        role,
        token_type: TokenType::Access,
        tenant_id: tenant_id.map(|s| s.to_string()),
        roles,
        exp: expiration,
        iat: now,
        jti: None,
    };

    encode(
        &Header::new(Algorithm::HS256),
        &claims,
        &EncodingKey::from_secret(&secret),
    )
    .map_err(|e| {
        error!("Failed to generate JWT: {}", e);
        AuthError::InternalError
    })
}

/// Generate a new JWT refresh token for a user (30 days expiration)
pub fn generate_refresh_token(
    user_id: &str,
    role: Role,
    tenant_id: Option<&str>,
    roles: Vec<String>,
) -> Result<String, AuthError> {
    let secret = get_jwt_secret();
    let now = jsonwebtoken::get_current_timestamp() as usize;
    let expiration = now + 30 * 24 * 60 * 60; // 30 days

    let claims = Claims {
        sub: user_id.to_string(),
        role,
        token_type: TokenType::Refresh,
        tenant_id: tenant_id.map(|s| s.to_string()),
        roles,
        exp: expiration,
        iat: now,
        jti: None,
    };

    encode(
        &Header::new(Algorithm::HS256),
        &claims,
        &EncodingKey::from_secret(&secret),
    )
    .map_err(|e| {
        error!("Failed to generate refresh token: {}", e);
        AuthError::InternalError
    })
}

/// Validate an API key and return the authenticated user
/// In a production environment, this would query a database
pub async fn validate_api_key(key: &str) -> Result<AuthenticatedUser, AuthError> {
    // Check for admin API key
    let admin_key = std::env::var("API_KEY_ADMIN").ok();
    if let Some(admin_key) = admin_key {
        // Use constant-time comparison to prevent timing attacks
        if constant_time_eq::constant_time_eq(key.as_bytes(), admin_key.as_bytes()) {
            return Ok(AuthenticatedUser {
                user_id: "admin".to_string(),
                role: Role::Admin,
                api_key_id: Some("admin_key".to_string()),
                tenant_id: std::env::var("API_KEY_ADMIN_TENANT").ok(),
                roles: vec!["admin".to_string()],
            });
        }
    }

    // Check for readonly API key
    let readonly_key = std::env::var("API_KEY_READONLY").ok();
    if let Some(readonly_key) = readonly_key {
        if constant_time_eq::constant_time_eq(key.as_bytes(), readonly_key.as_bytes()) {
            return Ok(AuthenticatedUser {
                user_id: "readonly".to_string(),
                role: Role::Readonly,
                api_key_id: Some("readonly_key".to_string()),
                tenant_id: std::env::var("API_KEY_READONLY_TENANT").ok(),
                roles: vec!["readonly".to_string()],
            });
        }
    }

    // Check for user API key
    let user_key = std::env::var("API_KEY_USER").ok();
    if let Some(user_key) = user_key {
        if constant_time_eq::constant_time_eq(key.as_bytes(), user_key.as_bytes()) {
            return Ok(AuthenticatedUser {
                user_id: "user".to_string(),
                role: Role::User,
                api_key_id: Some("user_key".to_string()),
                tenant_id: std::env::var("API_KEY_USER_TENANT").ok(),
                roles: vec!["user".to_string()],
            });
        }
    }

    // Check for tenant-specific API keys (format: API_KEY_TENANT_{TENANT_ID})
    for (key_name, value) in std::env::vars() {
        if key_name.starts_with("API_KEY_TENANT_") {
            if constant_time_eq::constant_time_eq(key.as_bytes(), value.as_bytes()) {
                let tenant_id = key_name
                    .strip_prefix("API_KEY_TENANT_")
                    .map(|s| s.to_string())
                    .unwrap_or_default();
                return Ok(AuthenticatedUser {
                    user_id: format!("tenant_{}", tenant_id),
                    role: Role::User,
                    api_key_id: Some(key_name),
                    tenant_id: Some(tenant_id),
                    roles: vec!["user".to_string()],
                });
            }
        }
    }

    Err(AuthError::InvalidApiKey)
}

/// Extract authentication from request parts (used as Axum extractor)
#[async_trait]
impl<S> FromRequestParts<S> for AuthenticatedUser
where
    S: Send + Sync,
{
    type Rejection = AuthError;

    async fn from_request_parts(parts: &mut Parts, _state: &S) -> Result<Self, Self::Rejection> {
        // Extract Authorization header
        let auth_header = parts
            .headers
            .get(header::AUTHORIZATION)
            .and_then(|value| value.to_str().ok())
            .ok_or(AuthError::MissingCredentials)?;

        // Check for Bearer token (JWT)
        if let Some(token) = auth_header.strip_prefix("Bearer ") {
            return validate_jwt(token);
        }

        // Check for API key (X-API-Key header format)
        if let Some(api_key) = auth_header.strip_prefix("ApiKey ") {
            return validate_api_key(api_key).await;
        }

        Err(AuthError::MissingCredentials)
    }
}

/// Middleware to require authentication
pub async fn auth_middleware(
    State(_state): State<Arc<AppState>>,
    req: Request,
    next: Next,
) -> Result<Response, AuthError> {
    // Extract Authorization header
    let auth_header = req
        .headers()
        .get(header::AUTHORIZATION)
        .and_then(|value| value.to_str().ok());

    let user = match auth_header {
        Some(header) => {
            if let Some(token) = header.strip_prefix("Bearer ") {
                validate_jwt(token)?
            } else if let Some(api_key) = header.strip_prefix("ApiKey ") {
                validate_api_key(api_key).await?
            } else {
                return Err(AuthError::MissingCredentials);
            }
        }
        None => {
            // Check for X-API-Key header as alternative
            let api_key_header = req
                .headers()
                .get("X-API-Key")
                .and_then(|value| value.to_str().ok());

            match api_key_header {
                Some(api_key) => validate_api_key(api_key).await?,
                None => return Err(AuthError::MissingCredentials),
            }
        }
    };

    // Store user in request extensions for later use
    let mut req = req;
    req.extensions_mut().insert(user);

    Ok(next.run(req).await)
}

/// Middleware version without State extractor for use with `layer()`
/// This wraps the auth_middleware logic without requiring State in the signature
pub async fn auth_middleware_with_state(req: Request, next: Next) -> Result<Response, AuthError> {
    // Extract Authorization header
    let auth_header = req
        .headers()
        .get(header::AUTHORIZATION)
        .and_then(|value| value.to_str().ok());

    let user = match auth_header {
        Some(header) => {
            if let Some(token) = header.strip_prefix("Bearer ") {
                validate_jwt(token)?
            } else if let Some(api_key) = header.strip_prefix("ApiKey ") {
                validate_api_key(api_key).await?
            } else {
                return Err(AuthError::MissingCredentials);
            }
        }
        None => {
            // Check for X-API-Key header as alternative
            let api_key_header = req
                .headers()
                .get("X-API-Key")
                .and_then(|value| value.to_str().ok());

            match api_key_header {
                Some(api_key) => validate_api_key(api_key).await?,
                None => return Err(AuthError::MissingCredentials),
            }
        }
    };

    // Store user in request extensions for later use
    let mut req = req;
    req.extensions_mut().insert(user);

    Ok(next.run(req).await)
}

/// Middleware to require specific role - use require_role in handler instead
/// This function is deprecated, use `require_role` function in handlers
pub async fn require_role_middleware(req: Request, next: Next) -> Result<Response, AuthError> {
    // This middleware cannot determine the required role at compile time
    // Use require_role function in the handler instead
    Ok(next.run(req).await)
}

/// Check role requirement for a user (for use in handlers)
pub fn require_role(user: &AuthenticatedUser, required_role: Role) -> Result<(), AuthError> {
    if !user.role.has_permission(required_role) {
        return Err(AuthError::InsufficientPermissions);
    }
    Ok(())
}

/// Create rate limiting layer with default settings
pub fn create_rate_limit_layer() -> GovernorLayer<PeerIpKeyExtractor, NoOpMiddleware> {
    let config = GovernorConfigBuilder::default()
        .per_second(10)
        .burst_size(100)
        .finish()
        .expect("Failed to create rate limit config");

    GovernorLayer {
        config: Arc::new(config),
    }
}

/// Create rate limiting layer with custom settings
pub fn create_rate_limit_layer_custom(
    per_second: u64,
    burst_size: u32,
) -> GovernorLayer<PeerIpKeyExtractor, NoOpMiddleware> {
    let config = GovernorConfigBuilder::default()
        .per_second(per_second)
        .burst_size(burst_size)
        .finish()
        .expect("Failed to create rate limit config");

    GovernorLayer {
        config: Arc::new(config),
    }
}

/// Create a stricter rate limit for sensitive endpoints (login, etc.)
pub fn create_strict_rate_limit_layer() -> GovernorLayer<PeerIpKeyExtractor, NoOpMiddleware> {
    let config = GovernorConfigBuilder::default()
        .per_second(1)
        .burst_size(5)
        .finish()
        .expect("Failed to create rate limit config");

    GovernorLayer {
        config: Arc::new(config),
    }
}

/// Login handler - authenticates user and returns JWT
pub async fn login_handler(
    State(_state): State<Arc<AppState>>,
    Json(req): Json<LoginRequest>,
) -> Result<Json<LoginResponse>, AuthError> {
    // In production, this would validate against a database
    // For now, we use environment variables for simple authentication
    let admin_username = std::env::var("ADMIN_USERNAME").unwrap_or_else(|_| "admin".to_string());
    let admin_password_hash = std::env::var("ADMIN_PASSWORD_HASH").ok();

    // Validate admin credentials using bcrypt
    let (user_id, role) = if req.username == admin_username {
        let password_valid = match &admin_password_hash {
            Some(hash) => {
                // Verify against bcrypt hash
                verify_password(&req.password, hash)?
            }
            None => {
                // Fallback to plaintext for development (not recommended for production)
                let admin_password =
                    std::env::var("ADMIN_PASSWORD").unwrap_or_else(|_| "admin".to_string());
                req.password == admin_password
            }
        };

        if password_valid {
            ("admin".to_string(), Role::Admin)
        } else {
            return Err(AuthError::InvalidToken);
        }
    } else {
        // Check for readonly user
        let readonly_user = std::env::var("READONLY_USERNAME").ok();
        let readonly_pass_hash = std::env::var("READONLY_PASSWORD_HASH").ok();

        if let Some(user) = readonly_user {
            if req.username == user {
                let password_valid = match &readonly_pass_hash {
                    Some(hash) => {
                        // Verify against bcrypt hash
                        verify_password(&req.password, hash)?
                    }
                    None => {
                        // Fallback to plaintext for development
                        let readonly_pass = std::env::var("READONLY_PASSWORD").ok();
                        match readonly_pass {
                            Some(pass) => req.password == pass,
                            None => false,
                        }
                    }
                };

                if password_valid {
                    ("readonly".to_string(), Role::Readonly)
                } else {
                    return Err(AuthError::InvalidToken);
                }
            } else {
                return Err(AuthError::InvalidToken);
            }
        } else {
            return Err(AuthError::InvalidToken);
        }
    };

    let access_token = generate_jwt(&user_id, role)?;
    let refresh_token = generate_refresh_token(&user_id, role, None, vec![])?;

    Ok(Json(LoginResponse {
        access_token,
        refresh_token,
        token_type: "Bearer".to_string(),
        expires_in: 15 * 60, // 15 minutes for access token
        user: UserInfo { user_id, role },
    }))
}

/// Create CORS layer with configurable origins
///
/// Origins can be configured via CORS_ALLOWED_ORIGINS environment variable.
/// Format: comma-separated list of origins (e.g., "https://app.example.com,https://admin.example.com")
///
/// Defaults:
/// - Production (no CORS_ALLOWED_ORIGINS set): only allows tauri://localhost and https://tauri.localhost
/// - Development (CORS_ALLOWED_ORIGINS=*): allows localhost:3000, localhost:3001, and tauri origins
pub fn create_cors_layer() -> tower_http::cors::CorsLayer {
    use tower_http::cors::{AllowOrigin, CorsLayer};

    let origins_env = std::env::var("CORS_ALLOWED_ORIGINS").unwrap_or_default();

    let allow_origin = if origins_env.is_empty() {
        // Production default: only allow Tauri origins
        AllowOrigin::list([
            "tauri://localhost".parse().expect("valid origin"),
            "https://tauri.localhost".parse().expect("valid origin"),
        ])
    } else if origins_env == "*" {
        // Development mode: allow common localhost ports and Tauri origins
        AllowOrigin::list([
            "http://localhost:3000".parse().expect("valid origin"),
            "http://localhost:3001".parse().expect("valid origin"),
            "tauri://localhost".parse().expect("valid origin"),
            "https://tauri.localhost".parse().expect("valid origin"),
        ])
    } else {
        // Custom origins from environment variable
        let origins: Vec<_> = origins_env
            .split(',')
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty())
            .map(|origin| {
                origin
                    .parse()
                    .unwrap_or_else(|_| panic!("invalid CORS origin: {}", origin))
            })
            .collect();

        if origins.is_empty() {
            // Fallback to secure default if parsing resulted in empty list
            AllowOrigin::list([
                "tauri://localhost".parse().expect("valid origin"),
                "https://tauri.localhost".parse().expect("valid origin"),
            ])
        } else {
            AllowOrigin::list(origins)
        }
    };

    CorsLayer::new()
        .allow_origin(allow_origin)
        .allow_methods([Method::GET, Method::POST, Method::PUT, Method::DELETE])
        .allow_headers([
            header::CONTENT_TYPE,
            header::AUTHORIZATION,
            header::ACCEPT,
            header::ORIGIN,
        ])
        .expose_headers([header::CONTENT_TYPE, header::CACHE_CONTROL])
}

/// Check if authentication is enabled
/// Authentication is enabled if JWT_SECRET is set or any API keys are configured
pub fn is_auth_enabled() -> bool {
    std::env::var("JWT_SECRET").is_ok()
        || std::env::var("API_KEY_ADMIN").is_ok()
        || std::env::var("API_KEY_READONLY").is_ok()
}

/// Development-only: create a test user bypass for local development
#[cfg(debug_assertions)]
pub fn dev_auth_bypass() -> AuthenticatedUser {
    AuthenticatedUser {
        user_id: "dev_user".to_string(),
        role: Role::Admin,
        api_key_id: None,
        tenant_id: Some("dev_tenant".to_string()),
        roles: vec!["admin".to_string()],
    }
}

/// Middleware to extract and inject TenantContext into request extensions
pub async fn tenant_middleware(req: Request, next: Next) -> Response {
    // Try to extract tenant from authenticated user in extensions
    let tenant = if let Some(user) = req.extensions().get::<AuthenticatedUser>() {
        user.to_tenant_context()
    } else {
        // Create anonymous tenant context
        TenantContext::anonymous()
    };

    let mut req = req;
    req.extensions_mut().insert(tenant);

    next.run(req).await
}

/// Get tenant context from request extensions
pub fn get_tenant_from_request(req: &Request) -> TenantContext {
    req.extensions()
        .get::<TenantContext>()
        .cloned()
        .unwrap_or_else(TenantContext::anonymous)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_role_permissions() {
        assert!(Role::Admin.has_permission(Role::Admin));
        assert!(Role::Admin.has_permission(Role::User));
        assert!(Role::Admin.has_permission(Role::Readonly));

        assert!(!Role::User.has_permission(Role::Admin));
        assert!(Role::User.has_permission(Role::User));
        assert!(Role::User.has_permission(Role::Readonly));

        assert!(!Role::Readonly.has_permission(Role::Admin));
        assert!(!Role::Readonly.has_permission(Role::User));
        assert!(Role::Readonly.has_permission(Role::Readonly));
    }

    #[test]
    fn test_jwt_generation_and_validation() {
        std::env::set_var("JWT_SECRET", "test_secret_for_unit_tests");

        let user_id = "test_user";
        let role = Role::User;

        let token = generate_jwt(user_id, role).expect("Failed to generate JWT");
        let user = validate_jwt(&token).expect("Failed to validate JWT");

        assert_eq!(user.user_id, user_id);
        assert!(matches!(user.role, Role::User));
    }

    #[test]
    fn test_jwt_with_tenant() {
        std::env::set_var("JWT_SECRET", "test_secret_for_unit_tests");

        let user_id = "test_user";
        let role = Role::Admin;
        let tenant_id = "tenant_123";
        let roles = vec!["custom_role".to_string()];

        let token = generate_jwt_with_tenant(user_id, role, Some(tenant_id), roles.clone())
            .expect("Failed to generate JWT with tenant");
        let user = validate_jwt(&token).expect("Failed to validate JWT");

        assert_eq!(user.user_id, user_id);
        assert!(matches!(user.role, Role::Admin));
        assert_eq!(user.tenant_id, Some(tenant_id.to_string()));
        assert_eq!(user.roles, roles);
    }

    #[test]
    fn test_jwt_invalid_token() {
        std::env::set_var("JWT_SECRET", "test_secret_for_unit_tests");

        let result = validate_jwt("invalid_token");
        assert!(matches!(result, Err(AuthError::InvalidToken)));
    }

    #[test]
    fn test_jwt_wrong_secret() {
        std::env::set_var("JWT_SECRET", "test_secret_for_unit_tests");

        let token = generate_jwt("test_user", Role::User).expect("Failed to generate JWT");

        // Change the secret
        std::env::set_var("JWT_SECRET", "different_secret");

        let result = validate_jwt(&token);
        assert!(matches!(result, Err(AuthError::InvalidToken)));
    }

    #[test]
    fn test_authenticated_user_tenant_context() {
        let user = AuthenticatedUser {
            user_id: "user_123".to_string(),
            role: Role::User,
            api_key_id: None,
            tenant_id: Some("tenant_456".to_string()),
            roles: vec!["editor".to_string()],
        };

        let ctx = user.to_tenant_context();
        assert!(ctx.has_role("user"));
        assert!(ctx.has_role("editor"));
        assert!(!ctx.has_role("admin"));
    }

    #[test]
    fn test_authenticated_user_belongs_to_tenant() {
        let user = AuthenticatedUser {
            user_id: "user_123".to_string(),
            role: Role::User,
            api_key_id: None,
            tenant_id: Some("tenant_456".to_string()),
            roles: vec![],
        };

        assert!(user.belongs_to_tenant("tenant_456"));
        assert!(!user.belongs_to_tenant("tenant_999"));

        // Default tenant matches all
        let default_user = AuthenticatedUser {
            user_id: "user_789".to_string(),
            role: Role::User,
            api_key_id: None,
            tenant_id: None,
            roles: vec![],
        };
        assert!(default_user.belongs_to_tenant("any_tenant"));
    }

    #[test]
    fn test_authenticated_user_has_role() {
        let user = AuthenticatedUser {
            user_id: "user_123".to_string(),
            role: Role::Admin,
            api_key_id: None,
            tenant_id: None,
            roles: vec!["custom".to_string()],
        };

        assert!(user.has_role("admin"));
        assert!(user.has_role("custom"));
        assert!(!user.has_role("user"));
    }

    #[test]
    fn test_auth_error_into_response() {
        use axum::http::StatusCode;

        let error = AuthError::MissingCredentials;
        let response = error.into_response();
        assert_eq!(response.status(), StatusCode::UNAUTHORIZED);

        let error = AuthError::InvalidToken;
        let response = error.into_response();
        assert_eq!(response.status(), StatusCode::UNAUTHORIZED);

        let error = AuthError::InsufficientPermissions;
        let response = error.into_response();
        assert_eq!(response.status(), StatusCode::FORBIDDEN);

        let error = AuthError::InternalError;
        let response = error.into_response();
        assert_eq!(response.status(), StatusCode::INTERNAL_SERVER_ERROR);
    }

    #[tokio::test]
    async fn test_validate_api_key_admin() {
        std::env::set_var("JWT_SECRET", "test_secret_for_unit_tests");
        std::env::set_var("API_KEY_ADMIN", "admin_secret_key_123");
        std::env::set_var("API_KEY_ADMIN_TENANT", "admin_tenant");

        let result = validate_api_key("admin_secret_key_123").await;
        assert!(result.is_ok());

        let user = result.unwrap();
        assert_eq!(user.user_id, "admin");
        assert!(matches!(user.role, Role::Admin));
        assert_eq!(user.tenant_id, Some("admin_tenant".to_string()));
    }

    #[tokio::test]
    async fn test_validate_api_key_readonly() {
        std::env::set_var("JWT_SECRET", "test_secret_for_unit_tests");
        std::env::set_var("API_KEY_READONLY", "readonly_secret_key_456");

        let result = validate_api_key("readonly_secret_key_456").await;
        assert!(result.is_ok());

        let user = result.unwrap();
        assert_eq!(user.user_id, "readonly");
        assert!(matches!(user.role, Role::Readonly));
    }

    #[tokio::test]
    async fn test_validate_api_key_invalid() {
        std::env::set_var("JWT_SECRET", "test_secret_for_unit_tests");

        // Ensure no API keys are set
        std::env::remove_var("API_KEY_ADMIN");
        std::env::remove_var("API_KEY_READONLY");
        std::env::remove_var("API_KEY_USER");

        let result = validate_api_key("invalid_key").await;
        assert!(matches!(result, Err(AuthError::InvalidApiKey)));
    }

    #[tokio::test]
    async fn test_validate_api_key_tenant_specific() {
        std::env::set_var("JWT_SECRET", "test_secret_for_unit_tests");
        std::env::set_var("API_KEY_TENANT_ACME", "acme_tenant_key");

        let result = validate_api_key("acme_tenant_key").await;
        assert!(result.is_ok());

        let user = result.unwrap();
        assert_eq!(user.user_id, "tenant_ACME");
        assert!(matches!(user.role, Role::User));
        assert_eq!(user.tenant_id, Some("ACME".to_string()));
    }

    #[test]
    fn test_is_auth_enabled() {
        // Clear all auth env vars
        std::env::remove_var("JWT_SECRET");
        std::env::remove_var("API_KEY_ADMIN");
        std::env::remove_var("API_KEY_READONLY");

        assert!(!is_auth_enabled());

        // Test with JWT_SECRET
        std::env::set_var("JWT_SECRET", "some_secret");
        assert!(is_auth_enabled());

        // Reset
        std::env::remove_var("JWT_SECRET");
        assert!(!is_auth_enabled());

        // Test with API key
        std::env::set_var("API_KEY_ADMIN", "admin_key");
        assert!(is_auth_enabled());

        // Cleanup
        std::env::remove_var("API_KEY_ADMIN");
    }

    #[test]
    fn test_role_default() {
        let role: Role = Default::default();
        assert!(matches!(role, Role::Readonly));
    }

    #[test]
    fn test_require_role_success() {
        let user = AuthenticatedUser {
            user_id: "admin".to_string(),
            role: Role::Admin,
            api_key_id: None,
            tenant_id: None,
            roles: vec![],
        };

        assert!(require_role(&user, Role::Readonly).is_ok());
        assert!(require_role(&user, Role::User).is_ok());
        assert!(require_role(&user, Role::Admin).is_ok());
    }

    #[test]
    fn test_require_role_failure() {
        let user = AuthenticatedUser {
            user_id: "readonly".to_string(),
            role: Role::Readonly,
            api_key_id: None,
            tenant_id: None,
            roles: vec![],
        };

        assert!(require_role(&user, Role::Readonly).is_ok());
        assert!(matches!(
            require_role(&user, Role::User),
            Err(AuthError::InsufficientPermissions)
        ));
        assert!(matches!(
            require_role(&user, Role::Admin),
            Err(AuthError::InsufficientPermissions)
        ));
    }

    #[test]
    fn test_cors_layer_creation() {
        let cors = create_cors_layer();
        // Just verify it doesn't panic
        drop(cors);
    }

    #[test]
    fn test_hash_password_success() {
        let password = "my_secure_password_123";
        let hash_result = hash_password(password);
        assert!(hash_result.is_ok());

        let hash = hash_result.unwrap();
        // Verify the hash starts with bcrypt identifier
        assert!(hash.starts_with("$2b$"));
    }

    #[test]
    fn test_verify_password_success() {
        let password = "my_secure_password_123";
        let hash = hash_password(password).expect("Failed to hash password");

        // Verify correct password
        let verify_result = verify_password(password, &hash);
        assert!(verify_result.is_ok());
        assert!(verify_result.unwrap());
    }

    #[test]
    fn test_verify_password_failure() {
        let password = "my_secure_password_123";
        let wrong_password = "wrong_password";
        let hash = hash_password(password).expect("Failed to hash password");

        // Verify wrong password fails
        let verify_result = verify_password(wrong_password, &hash);
        assert!(verify_result.is_ok());
        assert!(!verify_result.unwrap());
    }

    #[test]
    fn test_verify_password_invalid_hash() {
        let password = "my_secure_password_123";
        let invalid_hash = "invalid_hash_format";

        // Verify against invalid hash should return error
        let verify_result = verify_password(password, invalid_hash);
        assert!(verify_result.is_err());
        assert!(matches!(
            verify_result.unwrap_err(),
            AuthError::InternalError
        ));
    }

    #[test]
    fn test_hash_password_different_salts() {
        let password = "my_secure_password_123";

        // Hash the same password twice
        let hash1 = hash_password(password).expect("Failed to hash password");
        let hash2 = hash_password(password).expect("Failed to hash password");

        // Hashes should be different due to random salt
        assert_ne!(hash1, hash2);

        // But both should verify correctly
        assert!(verify_password(password, &hash1).unwrap());
        assert!(verify_password(password, &hash2).unwrap());
    }
}
