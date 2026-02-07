//! Tests for audit logging functionality in screenpipe-server

use screenpipe_db::{DatabaseManager, TenantContext};
use screenpipe_server::{Action, AuditService, Resource};
use std::sync::Arc;

async fn setup_test_db() -> Arc<DatabaseManager> {
    let temp_dir = tempfile::tempdir().unwrap();
    let db_path = temp_dir.path().join("test.db");
    Arc::new(
        DatabaseManager::new(db_path.to_str().unwrap())
            .await
            .expect("Failed to create test database"),
    )
}

#[tokio::test]
async fn test_audit_service_log() {
    let db = setup_test_db().await;
    let audit = AuditService::new(db.pool.clone());

    let tenant = TenantContext::new("tenant-1", "user-1").with_role("admin");

    // Log an action
    audit
        .log(
            &tenant,
            Action::Create,
            Resource::Speaker,
            Some("123"),
            Some(r#"{"name": "John"}"#),
            Some("192.168.1.1"),
            Some("Mozilla/5.0"),
        )
        .await
        .unwrap();

    // Wait a bit for async logging
    tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;

    // Verify logs can be retrieved
    let logs = audit.get_logs(&tenant, None, None, 10, 0).await.unwrap();
    assert_eq!(logs.len(), 1);
    assert_eq!(logs[0].action, "CREATE");
    assert_eq!(logs[0].resource_type, "speaker");
    assert_eq!(logs[0].tenant_id, Some("tenant-1".to_string()));
    assert_eq!(logs[0].user_id, Some("user-1".to_string()));
}

#[tokio::test]
async fn test_audit_service_search_logging() {
    let db = setup_test_db().await;
    let audit = AuditService::new(db.pool.clone());

    let tenant = TenantContext::new("tenant-1", "user-1").with_role("admin");

    // Log a search query
    audit
        .log_search(&tenant, "test query", "audio", Some("192.168.1.1"))
        .await
        .unwrap();

    // Wait a bit for async logging
    tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;

    let logs = audit.get_logs(&tenant, None, None, 10, 0).await.unwrap();
    assert_eq!(logs.len(), 1);
    assert_eq!(logs[0].action, "SEARCH");
    assert_eq!(logs[0].resource_type, "search");
}

#[tokio::test]
async fn test_audit_service_data_operations() {
    let db = setup_test_db().await;
    let audit = AuditService::new(db.pool.clone());

    let tenant = TenantContext::new("tenant-1", "user-1").with_role("admin");

    // Log data operations
    audit
        .log_data_op(
            &tenant,
            Action::Create,
            Resource::Speaker,
            "speaker-123",
            Some(serde_json::json!({"name": "John"})),
            Some("192.168.1.1"),
        )
        .await
        .unwrap();

    audit
        .log_data_op(
            &tenant,
            Action::Update,
            Resource::Speaker,
            "speaker-123",
            Some(serde_json::json!({"name": "John Doe"})),
            Some("192.168.1.1"),
        )
        .await
        .unwrap();

    audit
        .log_data_op(
            &tenant,
            Action::Delete,
            Resource::Speaker,
            "speaker-123",
            None,
            Some("192.168.1.1"),
        )
        .await
        .unwrap();

    // Wait a bit for async logging
    tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;

    let logs = audit.get_logs(&tenant, None, None, 10, 0).await.unwrap();
    assert_eq!(logs.len(), 3);

    // Verify order (most recent first)
    assert_eq!(logs[0].action, "DELETE");
    assert_eq!(logs[1].action, "UPDATE");
    assert_eq!(logs[2].action, "CREATE");
}

#[tokio::test]
async fn test_audit_service_unauthorized_access() {
    let db = setup_test_db().await;
    let audit = AuditService::new(db.pool.clone());

    // Regular user without admin role
    let regular_user = TenantContext::new("tenant-1", "user-1");

    // Try to get logs without admin role - should fail
    let result = audit.get_logs(&regular_user, None, None, 10, 0).await;
    assert!(result.is_err());

    // Admin user with role
    let admin_user = TenantContext::new("tenant-1", "admin-1").with_role("admin");

    // Should succeed for admin
    let logs = audit.get_logs(&admin_user, None, None, 10, 0).await;
    assert!(logs.is_ok());
}

#[tokio::test]
async fn test_audit_service_user_logs() {
    let db = setup_test_db().await;
    let audit = AuditService::new(db.pool.clone());

    let user1 = TenantContext::new("tenant-1", "user-1").with_role("admin");
    let user2 = TenantContext::new("tenant-1", "user-2").with_role("admin");

    // Log actions for different users
    audit
        .log(
            &user1,
            Action::Login,
            Resource::User,
            None,
            None,
            None,
            None,
        )
        .await
        .unwrap();

    audit
        .log(
            &user2,
            Action::Login,
            Resource::User,
            None,
            None,
            None,
            None,
        )
        .await
        .unwrap();

    // Wait a bit for async logging
    tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;

    // Get logs for specific user
    let user1_logs = audit.get_user_logs(&user1, "user-1", 10).await.unwrap();
    assert_eq!(user1_logs.len(), 1);
    assert_eq!(user1_logs[0].user_id, Some("user-1".to_string()));

    let user2_logs = audit.get_user_logs(&user2, "user-2", 10).await.unwrap();
    assert_eq!(user2_logs.len(), 1);
    assert_eq!(user2_logs[0].user_id, Some("user-2".to_string()));
}

#[tokio::test]
async fn test_action_display() {
    assert_eq!(format!("{}", Action::Create), "create");
    assert_eq!(format!("{}", Action::Search), "search");
    assert_eq!(format!("{}", Action::SpeakerReassign), "speaker_reassign");
}

#[tokio::test]
async fn test_resource_display() {
    assert_eq!(format!("{}", Resource::Frame), "frame");
    assert_eq!(format!("{}", Resource::Speaker), "speaker");
    assert_eq!(
        format!("{}", Resource::AudioTranscription),
        "audio_transcription"
    );
}
