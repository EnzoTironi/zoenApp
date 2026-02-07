//! Test utilities for screenpipe-server tests
//!
//! This module provides shared test helpers for authentication,
//! tenant management, and HTTP testing.

use axum::body::Body;
use axum::http::{Request, StatusCode};
use axum::Router;
use screenpipe_db::DatabaseManager;
use std::net::SocketAddr;
use std::path::PathBuf;
use std::sync::Arc;
use tower::ServiceExt;

use crate::auth::{generate_jwt, generate_jwt_with_tenant, validate_api_key, AuthenticatedUser, Role};
use crate::{AppState, PipeManager, SCServer};

/// Test user credentials for authentication tests
pub struct TestUser {
    pub user_id: String,
    pub role: Role,
    pub tenant_id: Option<String>,
}

impl TestUser {
    /// Create an admin test user
    pub fn admin() -> Self {
        Self {
            user_id: "admin".to_string(),
            role: Role::Admin,
            tenant_id: None,
        }
    }

    /// Create a regular user
    pub fn user() -> Self {
        Self {
            user_id: "user".to_string(),
            role: Role::User,
            tenant_id: None,
        }
    }

    /// Create a readonly user
    pub fn readonly() -> Self {
        Self {
            user_id: "readonly".to_string(),
            role: Role::Readonly,
            tenant_id: None,
        }
    }

    /// Create a user with a specific tenant
    pub fn with_tenant(self, tenant_id: impl Into<String>) -> Self {
        Self {
            tenant_id: Some(tenant_id.into()),
            ..self
        }
    }

    /// Generate a JWT token for this user
    pub fn generate_token(&self) -> String {
        std::env::set_var("JWT_SECRET", "test_secret_for_testing");

        if let Some(tenant_id) = &self.tenant_id {
            generate_jwt_with_tenant(&self.user_id, self.role.clone(), tenant_id)
                .expect("Failed to generate JWT")
        } else {
            generate_jwt(&self.user_id, self.role.clone())
                .expect("Failed to generate JWT")
        }
    }
}

/// API key types for testing
pub enum TestApiKey {
    Admin,
    User,
    Readonly,
    Invalid,
}

impl TestApiKey {
    /// Get the API key value
    pub fn value(&self) -> String {
        match self {
            TestApiKey::Admin => std::env::var("API_KEY_ADMIN")
                .unwrap_or_else(|_| "sk-admin-test-key".to_string()),
            TestApiKey::User => std::env::var("API_KEY_USER")
                .unwrap_or_else(|_| "sk-user-test-key".to_string()),
            TestApiKey::Readonly => std::env::var("API_KEY_READONLY")
                .unwrap_or_else(|_| "sk-readonly-test-key".to_string()),
            TestApiKey::Invalid => "sk-invalid-key".to_string(),
        }
    }

    /// Get as header value
    pub fn header_value(&self) -> String {
        self.value()
    }
}

/// Setup a test app with the given configuration
pub async fn setup_test_app() -> Router {
    let db = Arc::new(DatabaseManager::new("sqlite::memory:").await.unwrap());

    let audio_manager = Arc::new(
        screenpipe_audio::audio_manager::AudioManagerBuilder::new()
            .output_path("/tmp/screenpipe".into())
            .build(db.clone())
            .await
            .unwrap(),
    );

    let app = SCServer::new(
        db.clone(),
        SocketAddr::from(([127, 0, 0, 1], 23949)),
        PathBuf::from(""),
        Arc::new(PipeManager::new(PathBuf::from(""))),
        false,
        false,
        audio_manager,
        true,
        false,
        "medium".to_string(),
    );

    app.create_router(true).await
}

/// Setup a test app with auth enabled
pub async fn setup_test_app_with_auth() -> Router {
    std::env::set_var("JWT_SECRET", "test_secret_for_testing");
    std::env::set_var("API_KEY_ADMIN", "sk-admin-test-key");
    std::env::set_var("API_KEY_USER", "sk-user-test-key");
    std::env::set_var("API_KEY_READONLY", "sk-readonly-test-key");

    setup_test_app().await
}

/// Make an authenticated request to the test app
pub async fn make_authenticated_request(
    app: &Router,
    method: &str,
    uri: &str,
    token: &str,
    body: Option<Body>,
) -> StatusCode {
    let request = Request::builder()
        .method(method)
        .uri(uri)
        .header("Authorization", format!("Bearer {}", token))
        .body(body.unwrap_or(Body::empty()))
        .unwrap();

    let response = app.clone().oneshot(request).await.unwrap();
    response.status()
}

/// Make a request with API key
pub async fn make_api_key_request(
    app: &Router,
    method: &str,
    uri: &str,
    api_key: &str,
    body: Option<Body>,
) -> StatusCode {
    let request = Request::builder()
        .method(method)
        .uri(uri)
        .header("X-API-Key", api_key)
        .body(body.unwrap_or(Body::empty()))
        .unwrap();

    let response = app.clone().oneshot(request).await.unwrap();
    response.status()
}

/// Assert that a request returns the expected status code
pub fn assert_status(
    actual: StatusCode,
    expected: StatusCode,
    context: &str,
) {
    assert_eq!(
        actual, expected,
        "Expected status {} but got {} for: {}",
        expected, actual, context
    );
}

/// Test tenant setup helper
pub struct TestTenant {
    pub tenant_id: String,
    pub user_id: String,
    pub role: Role,
}

impl TestTenant {
    /// Create a new test tenant
    pub fn new(tenant_id: impl Into<String>, user_id: impl Into<String>) -> Self {
        Self {
            tenant_id: tenant_id.into(),
            user_id: user_id.into(),
            role: Role::User,
        }
    }

    /// Set the role for this tenant user
    pub fn with_role(mut self, role: Role) -> Self {
        self.role = role;
        self
    }

    /// Generate a JWT token for this tenant
    pub fn token(&self) -> String {
        std::env::set_var("JWT_SECRET", "test_secret_for_testing");
        generate_jwt_with_tenant(&self.user_id, self.role.clone(), &self.tenant_id)
            .expect("Failed to generate JWT")
    }
}

/// Common SQL injection payloads for testing
pub struct SqlInjectionPayloads;

impl SqlInjectionPayloads {
    /// Get all dangerous SQL injection payloads that should be blocked
    pub fn dangerous() -> Vec<(&'static str, &'static str)> {
        vec![
            ("DROP TABLE frames", "DROP statement"),
            ("DELETE FROM users", "DELETE statement"),
            ("INSERT INTO users VALUES ('x')", "INSERT statement"),
            ("UPDATE users SET role='admin'", "UPDATE statement"),
            ("SELECT * FROM frames; DROP TABLE users", "Stacked queries"),
            ("SELECT * FROM frames UNION SELECT * FROM users", "UNION injection"),
            ("ALTER TABLE frames ADD COLUMN x TEXT", "ALTER statement"),
            ("CREATE TABLE x (id INTEGER)", "CREATE statement"),
            ("TRUNCATE TABLE frames", "TRUNCATE statement"),
            ("PRAGMA table_info(frames)", "PRAGMA statement"),
            ("ATTACH DATABASE '/etc/passwd' AS x", "ATTACH statement"),
            ("VACUUM", "VACUUM statement"),
        ]
    }

    /// Get valid SELECT queries that should be allowed
    pub fn valid() -> Vec<&'static str> {
        vec![
            "SELECT * FROM frames LIMIT 10",
            "SELECT id, timestamp FROM frames WHERE id = 1",
            "SELECT f.id, o.text FROM frames f JOIN ocr_text o ON f.id = o.frame_id",
            "WITH recent AS (SELECT * FROM frames LIMIT 5) SELECT * FROM recent",
            "SELECT * FROM frames WHERE timestamp > '2024-01-01'",
        ]
    }
}

/// Path traversal payloads for testing
pub struct PathTraversalPayloads;

impl PathTraversalPayloads {
    /// Get all path traversal payloads that should be blocked
    pub fn all() -> Vec<&'static str> {
        vec![
            "../../../etc/passwd",
            "..\\..\\windows\\system32\\config\\sam",
            "/etc/passwd",
            "frames/../../../secret",
            "..//..//etc//passwd",
            "..%2F..%2Fetc%2Fpasswd",
            "....//....//etc/passwd",
            "%2e%2e%2f%2e%2e%2fetc%2fpasswd",
        ]
    }
}

/// Security header assertions
pub fn assert_security_headers_present(headers: &axum::http::HeaderMap) {
    assert!(
        headers.contains_key("x-content-type-options"),
        "X-Content-Type-Options header should be present"
    );
    assert!(
        headers.contains_key("x-frame-options"),
        "X-Frame-Options header should be present"
    );
    assert!(
        headers.contains_key("referrer-policy"),
        "Referrer-Policy header should be present"
    );
    assert!(
        headers.contains_key("strict-transport-security"),
        "Strict-Transport-Security header should be present"
    );
}

/// Setup test environment variables
pub fn setup_test_env() {
    std::env::set_var("JWT_SECRET", "test_secret_for_testing");
    std::env::set_var("API_KEY_ADMIN", "sk-admin-test-key");
    std::env::set_var("API_KEY_USER", "sk-user-test-key");
    std::env::set_var("API_KEY_READONLY", "sk-readonly-test-key");
    std::env::set_var("ADMIN_USERNAME", "admin");
    std::env::set_var("ADMIN_PASSWORD", "admin");
}
