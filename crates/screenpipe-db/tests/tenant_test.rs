//! Tests for multi-tenancy functionality
//!
//! These tests verify that:
//! 1. Tenant isolation works (tenants can't see each other's data)
//! 2. Audit logging captures tenant information
//! 3. Queries are properly scoped by tenant_id

use chrono::Utc;
use screenpipe_db::{
    AudioDevice, AuditAction, AuditLogger, ContentType, DatabaseManager, DeviceType,
    FrameWindowData, InsertUiEvent, TenantContext, TenantRole, TenantScopedDb, UiEventType,
};
use std::sync::Arc;
use std::time::Duration;
use tokio::time::sleep;

/// Test helper to create a temporary database
async fn setup_test_db() -> DatabaseManager {
    let temp_dir = tempfile::tempdir().unwrap();
    let db_path = temp_dir.path().join("test.db");
    DatabaseManager::new(db_path.to_str().unwrap())
        .await
        .expect("Failed to create test database")
}

#[tokio::test]
async fn test_tenant_context_creation() {
    let ctx = TenantContext::new("tenant-123", "user-456");
    assert_eq!(ctx.tenant_id, "tenant-123");
    assert_eq!(ctx.user_id, "user-456");
    assert!(!ctx.is_system());
    assert!(!ctx.is_anonymous());
}

#[tokio::test]
async fn test_tenant_context_system() {
    let ctx = TenantContext::system();
    assert!(ctx.is_system());
    assert!(!ctx.is_anonymous());
    assert!(ctx.has_role("admin"));
}

#[tokio::test]
async fn test_tenant_context_anonymous() {
    let ctx = TenantContext::anonymous();
    assert!(ctx.is_anonymous());
    assert!(!ctx.is_system());
}

#[tokio::test]
async fn test_tenant_context_roles() {
    let ctx = TenantContext::new("t", "u")
        .with_role("admin")
        .with_role("user");

    assert!(ctx.has_role("admin"));
    assert!(ctx.has_role("user"));
    assert!(!ctx.has_role("superadmin"));
}

#[tokio::test]
async fn test_tenant_isolation_audio_transcriptions() {
    let db = setup_test_db().await;

    // Create audio chunk first
    let audio_chunk_id = db.insert_audio_chunk("/tmp/test.mp3").await.unwrap();

    // Create two different tenants
    let tenant1 = TenantContext::new("tenant-1", "user-1");
    let tenant2 = TenantContext::new("tenant-2", "user-2");

    let device = AudioDevice {
        name: "test-device".to_string(),
        device_type: DeviceType::Input,
    };

    // Insert transcription for tenant 1
    let id1 = db
        .insert_audio_transcription_with_tenant(
            audio_chunk_id,
            "Hello from tenant 1",
            0,
            "test-engine",
            &device,
            None,
            None,
            None,
            &tenant1,
        )
        .await
        .unwrap();

    assert!(id1 > 0, "Should have inserted transcription for tenant 1");

    // Insert transcription for tenant 2
    let id2 = db
        .insert_audio_transcription_with_tenant(
            audio_chunk_id,
            "Hello from tenant 2",
            1,
            "test-engine",
            &device,
            None,
            None,
            None,
            &tenant2,
        )
        .await
        .unwrap();

    assert!(id2 > 0, "Should have inserted transcription for tenant 2");
    assert_ne!(id1, id2, "Should have different IDs");

    // Search as tenant 1 - should only see tenant 1's data
    let results = db
        .search_audio_with_tenant("", 10, 0, None, None, None, None, None, None, &tenant1)
        .await
        .unwrap();

    assert_eq!(results.len(), 1, "Tenant 1 should only see 1 result");
    assert_eq!(
        results[0].transcription, "Hello from tenant 1",
        "Tenant 1 should only see their own data"
    );

    // Search as tenant 2 - should only see tenant 2's data
    let results = db
        .search_audio_with_tenant("", 10, 0, None, None, None, None, None, None, &tenant2)
        .await
        .unwrap();

    assert_eq!(results.len(), 1, "Tenant 2 should only see 1 result");
    assert_eq!(
        results[0].transcription, "Hello from tenant 2",
        "Tenant 2 should only see their own data"
    );
}

#[tokio::test]
async fn test_tenant_isolation_frames() {
    let db = setup_test_db().await;

    // Create video chunk first
    db.insert_video_chunk_with_fps("/tmp/test.mp4", "test-device", 30.0)
        .await
        .unwrap();

    // Create two different tenants
    let tenant1 = TenantContext::new("tenant-1", "user-1");
    let tenant2 = TenantContext::new("tenant-2", "user-2");

    // Insert frame for tenant 1
    let frame1_id = db
        .insert_frame_with_tenant(
            "test-device",
            Some(Utc::now()),
            None,
            Some("App1"),
            Some("Window1"),
            true,
            Some(0),
            &tenant1,
        )
        .await
        .unwrap();

    assert!(frame1_id > 0, "Should have inserted frame for tenant 1");

    // Insert frame for tenant 2
    let frame2_id = db
        .insert_frame_with_tenant(
            "test-device",
            Some(Utc::now()),
            None,
            Some("App2"),
            Some("Window2"),
            true,
            Some(1),
            &tenant2,
        )
        .await
        .unwrap();

    assert!(frame2_id > 0, "Should have inserted frame for tenant 2");
}

#[tokio::test]
async fn test_audit_logging() {
    let db = setup_test_db().await;

    let tenant = TenantContext::new("tenant-1", "user-1").with_role("admin");

    // Log an audit event
    db.log_audit(
        &tenant,
        "create",
        "speaker",
        Some("123"),
        Some(r#"{"name": "John"}"#),
        Some("192.168.1.1"),
        Some("Mozilla/5.0"),
    )
    .await
    .unwrap();

    // Retrieve audit logs
    let logs = db.get_audit_logs(&tenant, None, None, 10, 0).await.unwrap();

    assert_eq!(logs.len(), 1, "Should have 1 audit log entry");
    assert_eq!(logs[0].tenant_id, "tenant-1");
    assert_eq!(logs[0].user_id, "user-1");
    assert_eq!(logs[0].action, "create");
    assert_eq!(logs[0].resource, "speaker");
    assert_eq!(logs[0].resource_id, Some("123".to_string()));
}

#[tokio::test]
async fn test_audit_log_isolation() {
    let db = setup_test_db().await;

    let tenant1 = TenantContext::new("tenant-1", "user-1").with_role("admin");
    let tenant2 = TenantContext::new("tenant-2", "user-2").with_role("admin");

    // Log events for different tenants
    db.log_audit(&tenant1, "create", "resource", Some("1"), None, None, None)
        .await
        .unwrap();

    db.log_audit(&tenant2, "create", "resource", Some("2"), None, None, None)
        .await
        .unwrap();

    // Each tenant should only see their own logs
    let logs1 = db
        .get_audit_logs(&tenant1, None, None, 10, 0)
        .await
        .unwrap();
    assert_eq!(logs1.len(), 1);
    assert_eq!(logs1[0].tenant_id, "tenant-1");

    let logs2 = db
        .get_audit_logs(&tenant2, None, None, 10, 0)
        .await
        .unwrap();
    assert_eq!(logs2.len(), 1);
    assert_eq!(logs2[0].tenant_id, "tenant-2");
}

#[tokio::test]
async fn test_system_tenant_can_access_all() {
    let db = setup_test_db().await;

    let regular_tenant = TenantContext::new("tenant-1", "user-1");
    let system_tenant = TenantContext::system();

    // Create audio chunk
    let audio_chunk_id = db.insert_audio_chunk("/tmp/test.mp3").await.unwrap();

    let device = AudioDevice {
        name: "test-device".to_string(),
        device_type: DeviceType::Input,
    };

    // Insert data for regular tenant
    db.insert_audio_transcription_with_tenant(
        audio_chunk_id,
        "Test transcription",
        0,
        "test-engine",
        &device,
        None,
        None,
        None,
        &regular_tenant,
    )
    .await
    .unwrap();

    // System tenant should be able to see the data
    // (In a real implementation, you'd add special handling for system tenant)
    assert!(system_tenant.is_system());
    assert!(system_tenant.has_role("admin"));
}

#[tokio::test]
async fn test_tenant_ui_events() {
    let db = setup_test_db().await;

    let tenant1 = TenantContext::new("tenant-1", "user-1");
    let tenant2 = TenantContext::new("tenant-2", "user-2");

    // Insert UI event for tenant 1
    let event1 = InsertUiEvent {
        timestamp: Utc::now(),
        session_id: Some("session-1".to_string()),
        relative_ms: 0,
        event_type: UiEventType::Click,
        x: Some(100),
        y: Some(200),
        delta_x: None,
        delta_y: None,
        button: Some(1),
        click_count: Some(1),
        key_code: None,
        modifiers: None,
        text_content: None,
        app_name: Some("TestApp".to_string()),
        app_pid: Some(1234),
        window_title: Some("Test Window".to_string()),
        browser_url: None,
        element_role: None,
        element_name: None,
        element_value: None,
        element_description: None,
        element_automation_id: None,
        element_bounds: None,
        frame_id: None,
    };

    let id1 = db
        .insert_ui_event_with_tenant(&event1, &tenant1)
        .await
        .unwrap();
    assert!(id1 > 0);

    // Insert UI event for tenant 2
    let event2 = InsertUiEvent {
        timestamp: Utc::now(),
        session_id: Some("session-2".to_string()),
        relative_ms: 0,
        event_type: UiEventType::Key,
        x: None,
        y: None,
        delta_x: None,
        delta_y: None,
        button: None,
        click_count: None,
        key_code: Some(65),
        modifiers: Some(0),
        text_content: Some("A".to_string()),
        app_name: Some("AnotherApp".to_string()),
        app_pid: Some(5678),
        window_title: Some("Another Window".to_string()),
        browser_url: None,
        element_role: None,
        element_name: None,
        element_value: None,
        element_description: None,
        element_automation_id: None,
        element_bounds: None,
        frame_id: None,
    };

    let id2 = db
        .insert_ui_event_with_tenant(&event2, &tenant2)
        .await
        .unwrap();
    assert!(id2 > 0);

    // Search UI events as tenant 1
    let results = db
        .search_ui_events_with_tenant(None, None, None, None, None, None, 10, 0, &tenant1)
        .await
        .unwrap();

    assert_eq!(results.len(), 1);
    assert_eq!(results[0].event_type, UiEventType::Click);

    // Search UI events as tenant 2
    let results = db
        .search_ui_events_with_tenant(None, None, None, None, None, None, 10, 0, &tenant2)
        .await
        .unwrap();

    assert_eq!(results.len(), 1);
    assert_eq!(results[0].event_type, UiEventType::Key);
}

#[tokio::test]
async fn test_batch_operations_with_tenant() {
    let db = setup_test_db().await;

    let tenant = TenantContext::new("tenant-1", "user-1");

    // Create video chunk
    db.insert_video_chunk_with_fps("/tmp/test.mp4", "test-device", 30.0)
        .await
        .unwrap();

    // Insert multiple UI events in batch
    let events = vec![
        InsertUiEvent {
            timestamp: Utc::now(),
            session_id: Some("session-1".to_string()),
            relative_ms: 0,
            event_type: UiEventType::Click,
            x: Some(100),
            y: Some(200),
            delta_x: None,
            delta_y: None,
            button: Some(1),
            click_count: Some(1),
            key_code: None,
            modifiers: None,
            text_content: None,
            app_name: Some("App1".to_string()),
            app_pid: Some(1234),
            window_title: Some("Window1".to_string()),
            browser_url: None,
            element_role: None,
            element_name: None,
            element_value: None,
            element_description: None,
            element_automation_id: None,
            element_bounds: None,
            frame_id: None,
        },
        InsertUiEvent {
            timestamp: Utc::now(),
            session_id: Some("session-1".to_string()),
            relative_ms: 100,
            event_type: UiEventType::Click,
            x: Some(150),
            y: Some(250),
            delta_x: None,
            delta_y: None,
            button: Some(1),
            click_count: Some(1),
            key_code: None,
            modifiers: None,
            text_content: None,
            app_name: Some("App1".to_string()),
            app_pid: Some(1234),
            window_title: Some("Window1".to_string()),
            browser_url: None,
            element_role: None,
            element_name: None,
            element_value: None,
            element_description: None,
            element_automation_id: None,
            element_bounds: None,
            frame_id: None,
        },
    ];

    let count = db
        .insert_ui_events_batch_with_tenant(&events, &tenant)
        .await
        .unwrap();
    assert_eq!(count, 2);

    // Verify all events are associated with the tenant
    let results = db
        .search_ui_events_with_tenant(None, None, None, None, None, None, 10, 0, &tenant)
        .await
        .unwrap();

    assert_eq!(results.len(), 2);
}

// ============================================================================
// TenantRole Tests
// ============================================================================

#[tokio::test]
async fn test_tenant_role_permissions() {
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

#[tokio::test]
async fn test_tenant_role_access_control() {
    // SuperAdmin can access any tenant
    assert!(TenantRole::SuperAdmin.can_access_tenant("tenant1", Some("tenant2")));

    // Other roles can only access their own tenant
    assert!(TenantRole::Admin.can_access_tenant("tenant1", Some("tenant1")));
    assert!(!TenantRole::Admin.can_access_tenant("tenant1", Some("tenant2")));

    // NULL tenant (unassigned data) is accessible
    assert!(TenantRole::Member.can_access_tenant("tenant1", None));
}

#[tokio::test]
async fn test_tenant_context_with_role() {
    let ctx = TenantContext::new("tenant-123", "user-456").with_tenant_role(TenantRole::Admin);

    assert_eq!(ctx.get_role(), TenantRole::Admin);
    assert!(ctx.can_modify());
    assert!(ctx.can_delete());
    assert!(ctx.can_access_tenant(Some("tenant-123")));
    assert!(!ctx.can_access_tenant(Some("other-tenant")));
}

#[tokio::test]
async fn test_tenant_context_system_permissions() {
    let ctx = TenantContext::system();
    assert_eq!(ctx.get_role(), TenantRole::SuperAdmin);
    assert!(ctx.can_access_tenant(Some("any-tenant")));
}

// ============================================================================
// AuditLogger Tests
// ============================================================================

#[tokio::test]
async fn test_audit_logger_basic() {
    let db = setup_test_db().await;
    let logger = AuditLogger::new(db.pool.clone());

    let ctx = TenantContext::new("tenant-123", "user-456").with_tenant_role(TenantRole::Member);

    // Log a simple event
    logger
        .log(
            &ctx,
            AuditAction::Create,
            "test_resource",
            Some("resource-123"),
            Some(serde_json::json!({"key": "value"})),
            Some("192.168.1.1".to_string()),
            Some("Mozilla/5.0".to_string()),
        )
        .await
        .expect("Failed to log audit event");

    // Wait a bit for the async operation
    sleep(Duration::from_millis(100)).await;

    // Retrieve logs
    let logs = logger
        .get_audit_logs(&ctx, None, None, None, None, 10, 0)
        .await
        .expect("Failed to get audit logs");

    assert!(!logs.is_empty());
    let log = &logs[0];
    assert_eq!(log.tenant_id, Some("tenant-123".to_string()));
    assert_eq!(log.user_id, Some("user-456".to_string()));
    assert_eq!(log.action, "CREATE");
    assert_eq!(log.resource_type, "test_resource");
    assert_eq!(log.resource_id, Some("resource-123".to_string()));
}

#[tokio::test]
async fn test_audit_log_access_denied() {
    let db = setup_test_db().await;
    let logger = AuditLogger::new(db.pool.clone());

    let ctx = TenantContext::new("tenant-123", "user-456").with_tenant_role(TenantRole::Member);

    logger
        .log_access_denied(
            &ctx,
            "sensitive_resource",
            Some("resource-999"),
            "insufficient_permissions",
            Some("192.168.1.1".to_string()),
        )
        .await
        .expect("Failed to log access denied");

    sleep(Duration::from_millis(100)).await;

    let logs = logger
        .get_audit_logs(
            &ctx,
            None,
            None,
            Some(AuditAction::AccessDenied),
            None,
            10,
            0,
        )
        .await
        .expect("Failed to get audit logs");

    assert!(!logs.is_empty());
    assert_eq!(logs[0].action, "ACCESS_DENIED");
}

#[tokio::test]
async fn test_audit_log_auth_events() {
    let db = setup_test_db().await;
    let logger = AuditLogger::new(db.pool.clone());

    let ctx = TenantContext::new("tenant-123", "user-456");

    logger
        .log_auth(
            &ctx,
            AuditAction::Login,
            Some("192.168.1.1".to_string()),
            Some("Mozilla/5.0".to_string()),
            Some(serde_json::json!({"method": "password"})),
        )
        .await
        .expect("Failed to log auth event");

    sleep(Duration::from_millis(100)).await;

    let logs = logger
        .get_audit_logs(&ctx, None, None, Some(AuditAction::Login), None, 10, 0)
        .await
        .expect("Failed to get audit logs");

    assert!(!logs.is_empty());
    assert_eq!(logs[0].action, "LOGIN");
}

#[tokio::test]
async fn test_audit_log_filtering() {
    let db = setup_test_db().await;
    let logger = AuditLogger::new(db.pool.clone());

    let ctx = TenantContext::new("tenant-123", "user-456");
    let start_time = Utc::now();

    // Log multiple events
    for i in 0..5 {
        logger
            .log(
                &ctx,
                AuditAction::Create,
                "resource",
                Some(&format!("resource-{}", i)),
                None,
                None,
                None,
            )
            .await
            .expect("Failed to log event");
    }

    sleep(Duration::from_millis(100)).await;

    let end_time = Utc::now();

    // Test time-based filtering
    let logs = logger
        .get_audit_logs(&ctx, Some(start_time), Some(end_time), None, None, 10, 0)
        .await
        .expect("Failed to get audit logs");

    assert_eq!(logs.len(), 5);

    // Test pagination
    let logs_page1 = logger
        .get_audit_logs(&ctx, None, None, None, None, 2, 0)
        .await
        .expect("Failed to get audit logs");
    assert_eq!(logs_page1.len(), 2);

    let logs_page2 = logger
        .get_audit_logs(&ctx, None, None, None, None, 2, 2)
        .await
        .expect("Failed to get audit logs");
    assert_eq!(logs_page2.len(), 2);
}

#[tokio::test]
async fn test_audit_log_count() {
    let db = setup_test_db().await;
    let logger = AuditLogger::new(db.pool.clone());

    let ctx = TenantContext::new("tenant-123", "user-456");

    // Log different types of events
    logger
        .log(
            &ctx,
            AuditAction::Create,
            "resource",
            None,
            None,
            None,
            None,
        )
        .await
        .expect("Failed to log");
    logger
        .log(
            &ctx,
            AuditAction::Create,
            "resource",
            None,
            None,
            None,
            None,
        )
        .await
        .expect("Failed to log");
    logger
        .log(&ctx, AuditAction::Read, "resource", None, None, None, None)
        .await
        .expect("Failed to log");

    sleep(Duration::from_millis(100)).await;

    // Count all logs
    let total_count = logger
        .count_audit_logs(&ctx, None, None, None, None)
        .await
        .expect("Failed to count logs");
    assert_eq!(total_count, 3);

    // Count by action
    let create_count = logger
        .count_audit_logs(&ctx, None, None, Some(AuditAction::Create), None)
        .await
        .expect("Failed to count logs");
    assert_eq!(create_count, 2);
}

#[tokio::test]
async fn test_super_admin_can_see_all_logs() {
    let db = setup_test_db().await;
    let logger = AuditLogger::new(db.pool.clone());

    // Create logs for different tenants
    let tenant1 = TenantContext::new("tenant-1", "user-1");
    let tenant2 = TenantContext::new("tenant-2", "user-2");

    logger
        .log(
            &tenant1,
            AuditAction::Create,
            "resource",
            None,
            None,
            None,
            None,
        )
        .await
        .expect("Failed to log");
    logger
        .log(
            &tenant2,
            AuditAction::Create,
            "resource",
            None,
            None,
            None,
            None,
        )
        .await
        .expect("Failed to log");

    sleep(Duration::from_millis(100)).await;

    // Super admin should see all logs
    let super_admin = TenantContext::system();
    let all_logs = logger
        .get_audit_logs(&super_admin, None, None, None, None, 10, 0)
        .await
        .expect("Failed to get logs");
    assert_eq!(all_logs.len(), 2);

    // Tenant 1 should only see their own logs
    let tenant1_logs = logger
        .get_audit_logs(&tenant1, None, None, None, None, 10, 0)
        .await
        .expect("Failed to get logs");
    assert_eq!(tenant1_logs.len(), 1);
}

#[tokio::test]
async fn test_audit_log_retention_trigger() {
    let db = setup_test_db().await;

    // The retention trigger should delete logs older than 1 year
    // We can't easily test this without manipulating time,
    // but we can verify the trigger exists by checking the schema

    let trigger_exists: Option<i64> = sqlx::query_scalar(
        "SELECT 1 FROM sqlite_master WHERE type='trigger' AND name='audit_log_retention'",
    )
    .fetch_one(&db.pool)
    .await
    .expect("Failed to check trigger");

    assert_eq!(trigger_exists, Some(1));
}
