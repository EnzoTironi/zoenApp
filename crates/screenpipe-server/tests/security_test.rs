//! Security integration tests for screenpipe-server
//!
//! These tests verify security protections including:
//! 1. SQL injection prevention in /raw_sql endpoint
//! 2. Path traversal protection in file endpoints
//! 3. CORS origin enforcement
//! 4. Security headers presence
//! 5. Authentication and authorization

use axum::body::to_bytes;
use axum::body::Body;
use axum::http::{HeaderValue, Request, StatusCode};
use axum::Router;
use screenpipe_audio::audio_manager::AudioManagerBuilder;
use screenpipe_db::DatabaseManager;
use screenpipe_server::auth::{generate_jwt, Role};
use screenpipe_server::PipeManager;
use screenpipe_server::SCServer;
use serde_json::json;
use std::net::SocketAddr;
use std::path::PathBuf;
use std::sync::Arc;
use tokio::sync::Mutex;
use tower::ServiceExt;

/// Test helper to create a test app with security settings
async fn setup_test_app() -> Router {
    // Use a unique file-based database for each test
    let temp_dir = tempfile::tempdir().unwrap();
    let db_path = temp_dir.path().join(format!("test_{}.db", uuid::Uuid::new_v4()));

    let db = Arc::new(DatabaseManager::new(db_path.to_str().unwrap()).await.unwrap());

    let audio_manager = Arc::new(
        AudioManagerBuilder::new()
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

/// Test helper to create a valid JWT token for testing
fn create_test_token(user_id: &str, role: Role) -> String {
    std::env::set_var("JWT_SECRET", "test_secret_for_security_tests");
    generate_jwt(user_id, role).expect("Failed to generate test JWT")
}

#[tokio::test]
async fn test_raw_sql_injection_blocked() {
    let app = setup_test_app().await;

    // Create admin token
    let admin_token = create_test_token("admin", Role::Admin);

    let malicious_queries = vec![
        ("DROP TABLE frames", "DROP"),
        ("DELETE FROM users", "DELETE"),
        ("INSERT INTO users VALUES ('hacker')", "INSERT"),
        ("UPDATE users SET role='admin'", "UPDATE"),
        (
            "SELECT * FROM users; DROP TABLE frames",
            "Multiple statements",
        ),
        ("SELECT * FROM frames UNION SELECT * FROM users", "UNION"),
        ("ALTER TABLE frames ADD COLUMN hacked TEXT", "ALTER"),
        ("CREATE TABLE hacked (id INTEGER)", "CREATE"),
        ("TRUNCATE TABLE frames", "TRUNCATE"),
        ("PRAGMA table_info(frames)", "PRAGMA"),
        ("ATTACH DATABASE '/etc/passwd' AS hack", "ATTACH"),
        ("VACUUM", "VACUUM"),
    ];

    for (query, description) in malicious_queries {
        let response = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/raw_sql")
                    .header("Authorization", format!("Bearer {}", admin_token))
                    .header("Content-Type", "application/json")
                    .body(Body::from(json!({ "query": query }).to_string()))
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(
            response.status(),
            StatusCode::BAD_REQUEST,
            "Query '{}' ({}) should have been blocked",
            query,
            description
        );
    }
}

#[tokio::test]
async fn test_raw_sql_non_admin_blocked() {
    let app = setup_test_app().await;

    // Create user token (not admin)
    let user_token = create_test_token("user", Role::User);

    let response = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/raw_sql")
                .header("Authorization", format!("Bearer {}", user_token))
                .header("Content-Type", "application/json")
                .body(Body::from(
                    json!({ "query": "SELECT * FROM frames" }).to_string(),
                ))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(
        response.status(),
        StatusCode::FORBIDDEN,
        "Non-admin user should not be able to execute raw SQL"
    );
}

#[tokio::test]
async fn test_raw_sql_valid_select_allowed() {
    let app = setup_test_app().await;

    // Create admin token
    let admin_token = create_test_token("admin", Role::Admin);

    let valid_queries = vec![
        "SELECT * FROM frames LIMIT 10",
        "SELECT id, timestamp FROM frames WHERE id = 1",
        "SELECT f.id, o.text FROM frames f JOIN ocr_text o ON f.id = o.frame_id",
        "WITH recent AS (SELECT * FROM frames LIMIT 5) SELECT * FROM recent",
    ];

    for query in valid_queries {
        let response = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/raw_sql")
                    .header("Authorization", format!("Bearer {}", admin_token))
                    .header("Content-Type", "application/json")
                    .body(Body::from(json!({ "query": query }).to_string()))
                    .unwrap(),
            )
            .await
            .unwrap();

        // Should not be BAD_REQUEST (validation failure)
        // It might be OK or INTERNAL_SERVER_ERROR (if table doesn't exist in test)
        assert_ne!(
            response.status(),
            StatusCode::BAD_REQUEST,
            "Valid query '{}' should not be rejected by validator",
            query
        );
    }
}

#[tokio::test]
async fn test_raw_sql_no_auth_blocked() {
    let app = setup_test_app().await;

    let response = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/raw_sql")
                .header("Content-Type", "application/json")
                .body(Body::from(
                    json!({ "query": "SELECT * FROM frames" }).to_string(),
                ))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(
        response.status(),
        StatusCode::UNAUTHORIZED,
        "Raw SQL endpoint should require authentication"
    );
}

#[tokio::test]
async fn test_path_traversal_blocked_in_media_validation() {
    let app = setup_test_app().await;

    let malicious_paths = vec![
        "../../../etc/passwd",
        "..\\..\\windows\\system32\\config\\sam",
        "/etc/passwd",
        "frames/../../../secret",
        "..//..//etc//passwd",
        "..%2F..%2Fetc%2Fpasswd",
    ];

    for path in malicious_paths {
        let response = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("GET")
                    .uri(&format!(
                        "/experimental/validate/media?file_path={}",
                        urlencoding::encode(path)
                    ))
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(
            response.status(),
            StatusCode::FORBIDDEN,
            "Path '{}' should be blocked for path traversal",
            path
        );
    }
}

#[tokio::test]
async fn test_security_headers_present() {
    let app = setup_test_app().await;

    let response = app
        .oneshot(
            Request::builder()
                .method("GET")
                .uri("/health")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);

    // Check security headers
    let headers = response.headers();

    assert!(
        headers.contains_key("x-content-type-options"),
        "X-Content-Type-Options header should be present"
    );
    assert_eq!(
        headers.get("x-content-type-options"),
        Some(&HeaderValue::from_static("nosniff"))
    );

    assert!(
        headers.contains_key("x-frame-options"),
        "X-Frame-Options header should be present"
    );
    assert_eq!(
        headers.get("x-frame-options"),
        Some(&HeaderValue::from_static("DENY"))
    );

    assert!(
        headers.contains_key("referrer-policy"),
        "Referrer-Policy header should be present"
    );

    // Strict-Transport-Security should be present
    assert!(
        headers.contains_key("strict-transport-security"),
        "Strict-Transport-Security header should be present"
    );
}

#[tokio::test]
async fn test_cors_origin_enforced() {
    let app = setup_test_app().await;

    // Test preflight request with disallowed origin
    let response = app
        .clone()
        .oneshot(
            Request::builder()
                .method("OPTIONS")
                .uri("/search")
                .header("Origin", "https://evil.com")
                .header("Access-Control-Request-Method", "GET")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    // Should not have Access-Control-Allow-Origin for evil.com
    let allow_origin = response.headers().get("access-control-allow-origin");
    if let Some(origin) = allow_origin {
        let origin_str = origin.to_str().unwrap_or("");
        assert!(
            !origin_str.contains("evil.com"),
            "CORS should not allow evil.com origin"
        );
    }

    // Test preflight request with allowed origin
    let response = app
        .oneshot(
            Request::builder()
                .method("OPTIONS")
                .uri("/search")
                .header("Origin", "http://localhost:3000")
                .header("Access-Control-Request-Method", "GET")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    // Should allow localhost:3000
    let allow_origin = response.headers().get("access-control-allow-origin");
    assert!(
        allow_origin.is_some(),
        "CORS should allow localhost:3000 origin"
    );
}

#[tokio::test]
async fn test_invalid_jwt_rejected() {
    let app = setup_test_app().await;

    let response = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/raw_sql")
                .header("Authorization", "Bearer invalid_token_here")
                .header("Content-Type", "application/json")
                .body(Body::from(
                    json!({ "query": "SELECT * FROM frames" }).to_string(),
                ))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(
        response.status(),
        StatusCode::UNAUTHORIZED,
        "Invalid JWT should be rejected"
    );
}

#[tokio::test]
async fn test_malformed_auth_header_rejected() {
    let app = setup_test_app().await;

    let response = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/raw_sql")
                .header("Authorization", "NotBearer token_here")
                .header("Content-Type", "application/json")
                .body(Body::from(
                    json!({ "query": "SELECT * FROM frames" }).to_string(),
                ))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(
        response.status(),
        StatusCode::UNAUTHORIZED,
        "Malformed auth header should be rejected"
    );
}

#[tokio::test]
async fn test_sql_query_length_limit() {
    let app = setup_test_app().await;

    // Create admin token
    let admin_token = create_test_token("admin", Role::Admin);

    // Create a query that's too long (> 10000 chars)
    let long_query = format!("SELECT {} FROM frames", "a".repeat(10010));

    let response = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/raw_sql")
                .header("Authorization", format!("Bearer {}", admin_token))
                .header("Content-Type", "application/json")
                .body(Body::from(json!({ "query": long_query }).to_string()))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(
        response.status(),
        StatusCode::BAD_REQUEST,
        "Query exceeding max length should be blocked"
    );

    let body = to_bytes(response.into_body(), usize::MAX).await.unwrap();
    let body_str = String::from_utf8(body.to_vec()).unwrap();
    assert!(
        body_str.contains("Query validation failed"),
        "Error should indicate query validation failure"
    );
}

#[tokio::test]
async fn test_sql_injection_union_blocked() {
    let app = setup_test_app().await;

    let admin_token = create_test_token("admin", Role::Admin);

    let union_queries = vec![
        "SELECT * FROM frames UNION SELECT * FROM users",
        "SELECT * FROM frames UNION ALL SELECT * FROM users",
        "SELECT * FROM frames INTERSECT SELECT * FROM users",
        "SELECT * FROM frames EXCEPT SELECT * FROM users",
    ];

    for query in union_queries {
        let response = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/raw_sql")
                    .header("Authorization", format!("Bearer {}", admin_token))
                    .header("Content-Type", "application/json")
                    .body(Body::from(json!({ "query": query }).to_string()))
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(
            response.status(),
            StatusCode::BAD_REQUEST,
            "UNION/INTERSECT/EXCEPT query '{}' should be blocked",
            query
        );
    }
}

#[tokio::test]
async fn test_sql_stacked_queries_blocked() {
    let app = setup_test_app().await;

    let admin_token = create_test_token("admin", Role::Admin);

    let stacked_queries = vec![
        "SELECT * FROM frames; SELECT * FROM users",
        "SELECT * FROM frames; DELETE FROM users",
        "SELECT * FROM frames; INSERT INTO users VALUES (1)",
    ];

    for query in stacked_queries {
        let response = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/raw_sql")
                    .header("Authorization", format!("Bearer {}", admin_token))
                    .header("Content-Type", "application/json")
                    .body(Body::from(json!({ "query": query }).to_string()))
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(
            response.status(),
            StatusCode::BAD_REQUEST,
            "Stacked query '{}' should be blocked",
            query
        );
    }
}
