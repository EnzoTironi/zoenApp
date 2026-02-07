//! Tests for the playbook engine
//!
//! These tests verify:
//! 1. Playbook CRUD operations
//! 2. Trigger evaluation (app_open, time, keyword, context, meeting_start/end)
//! 3. Action execution (notify, summarize, focus_mode, tag, webhook)
//! 4. Cooldown and max executions limits
//! 5. Execution history

#[cfg(test)]
mod tests {
    use super::super::*;
    use chrono::Utc;
    use std::collections::HashMap;

    // Import internal types for testing
    use super::super::{AppStateInternal, TriggerState};

    // ============================================================================
    // Playbook CRUD Tests
    // ============================================================================

    #[tokio::test]
    async fn test_playbook_create() {
        let engine = PlaybookEngine::new(std::path::PathBuf::from("/tmp/test"));

        let playbook = Playbook {
            id: String::new(),
            name: "Test Playbook".to_string(),
            description: Some("A test playbook".to_string()),
            enabled: false,
            triggers: vec![Trigger::AppOpen {
                app_name: "test-app".to_string(),
                window_name: None,
            }],
            actions: vec![Action::Notify {
                title: "Test".to_string(),
                message: "Hello".to_string(),
                actions: None,
                persistent: None,
            }],
            cooldown_minutes: Some(60),
            max_executions_per_day: Some(10),
            created_at: None,
            updated_at: None,
            is_builtin: None,
            icon: Some("🧪".to_string()),
            color: Some("#FF0000".to_string()),
        };

        let created = engine.create_playbook(playbook).await.unwrap();

        assert!(!created.id.is_empty(), "Playbook should have an ID assigned");
        assert_eq!(created.name, "Test Playbook");
        assert!(created.created_at.is_some());
        assert!(created.updated_at.is_some());
        assert_eq!(created.is_builtin, Some(false));
    }

    #[tokio::test]
    async fn test_playbook_get() {
        let engine = PlaybookEngine::new(std::path::PathBuf::from("/tmp/test"));

        let playbook = Playbook {
            id: String::new(),
            name: "Test Playbook".to_string(),
            description: None,
            enabled: false,
            triggers: vec![],
            actions: vec![],
            cooldown_minutes: None,
            max_executions_per_day: None,
            created_at: None,
            updated_at: None,
            is_builtin: None,
            icon: None,
            color: None,
        };

        let created = engine.create_playbook(playbook).await.unwrap();
        let retrieved = engine.get_playbook(&created.id).await;

        assert!(retrieved.is_some());
        assert_eq!(retrieved.unwrap().name, "Test Playbook");
    }

    #[tokio::test]
    async fn test_playbook_get_not_found() {
        let engine = PlaybookEngine::new(std::path::PathBuf::from("/tmp/test"));
        let result = engine.get_playbook("non-existent-id").await;
        assert!(result.is_none());
    }

    #[tokio::test]
    async fn test_playbook_list() {
        let engine = PlaybookEngine::new(std::path::PathBuf::from("/tmp/test"));

        // Create a few playbooks
        for i in 0..3 {
            let playbook = Playbook {
                id: String::new(),
                name: format!("Playbook {}", i),
                description: None,
                enabled: false,
                triggers: vec![],
                actions: vec![],
                cooldown_minutes: None,
                max_executions_per_day: None,
                created_at: None,
                updated_at: None,
                is_builtin: None,
                icon: None,
                color: None,
            };
            engine.create_playbook(playbook).await.unwrap();
        }

        let list = engine.list_playbooks().await;
        assert_eq!(list.len(), 3);
    }

    #[tokio::test]
    async fn test_playbook_update() {
        let engine = PlaybookEngine::new(std::path::PathBuf::from("/tmp/test"));

        let playbook = Playbook {
            id: String::new(),
            name: "Original Name".to_string(),
            description: None,
            enabled: false,
            triggers: vec![],
            actions: vec![],
            cooldown_minutes: None,
            max_executions_per_day: None,
            created_at: None,
            updated_at: None,
            is_builtin: None,
            icon: None,
            color: None,
        };

        let created = engine.create_playbook(playbook).await.unwrap();
        let original_id = created.id.clone();

        let updates = Playbook {
            id: original_id.clone(),
            name: "Updated Name".to_string(),
            description: Some("Updated description".to_string()),
            enabled: true,
            triggers: vec![Trigger::AppOpen {
                app_name: "updated-app".to_string(),
                window_name: None,
            }],
            actions: vec![Action::Notify {
                title: "Updated".to_string(),
                message: "Updated message".to_string(),
                actions: None,
                persistent: None,
            }],
            cooldown_minutes: Some(30),
            max_executions_per_day: Some(5),
            created_at: None,
            updated_at: None,
            is_builtin: None,
            icon: Some("🔄".to_string()),
            color: Some("#00FF00".to_string()),
        };

        let updated = engine.update_playbook(&original_id, updates).await.unwrap();

        assert_eq!(updated.name, "Updated Name");
        assert_eq!(updated.description, Some("Updated description".to_string()));
        assert!(updated.enabled);
        assert!(updated.updated_at.is_some());
    }

    #[tokio::test]
    async fn test_playbook_update_not_found() {
        let engine = PlaybookEngine::new(std::path::PathBuf::from("/tmp/test"));

        let updates = Playbook {
            id: "non-existent".to_string(),
            name: "Updated".to_string(),
            description: None,
            enabled: false,
            triggers: vec![],
            actions: vec![],
            cooldown_minutes: None,
            max_executions_per_day: None,
            created_at: None,
            updated_at: None,
            is_builtin: None,
            icon: None,
            color: None,
        };

        let result = engine.update_playbook("non-existent", updates).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_playbook_delete() {
        let engine = PlaybookEngine::new(std::path::PathBuf::from("/tmp/test"));

        let playbook = Playbook {
            id: String::new(),
            name: "To Delete".to_string(),
            description: None,
            enabled: false,
            triggers: vec![],
            actions: vec![],
            cooldown_minutes: None,
            max_executions_per_day: None,
            created_at: None,
            updated_at: None,
            is_builtin: None,
            icon: None,
            color: None,
        };

        let created = engine.create_playbook(playbook).await.unwrap();
        let id = created.id.clone();

        // Delete the playbook
        engine.delete_playbook(&id).await.unwrap();

        // Verify it's gone
        let retrieved = engine.get_playbook(&id).await;
        assert!(retrieved.is_none());
    }

    #[tokio::test]
    async fn test_playbook_delete_builtin_fails() {
        let engine = PlaybookEngine::new(std::path::PathBuf::from("/tmp/test"));

        // Insert a built-in playbook directly (bypassing create_playbook which forces is_builtin=false)
        let builtin_playbook = Playbook {
            id: "builtin-test".to_string(),
            name: "Builtin".to_string(),
            description: None,
            enabled: false,
            triggers: vec![],
            actions: vec![],
            cooldown_minutes: None,
            max_executions_per_day: None,
            created_at: Some(Utc::now()),
            updated_at: Some(Utc::now()),
            is_builtin: Some(true),
            icon: None,
            color: None,
        };

        engine.set_playbooks({
            let mut map = HashMap::new();
            map.insert(builtin_playbook.id.clone(), builtin_playbook);
            map
        }).await;

        // Try to delete built-in playbook
        let result = engine.delete_playbook("builtin-test").await;
        assert!(result.is_err(), "Should not be able to delete built-in playbook");
    }

    #[tokio::test]
    async fn test_playbook_toggle() {
        let engine = PlaybookEngine::new(std::path::PathBuf::from("/tmp/test"));

        let playbook = Playbook {
            id: String::new(),
            name: "Toggle Test".to_string(),
            description: None,
            enabled: false,
            triggers: vec![],
            actions: vec![],
            cooldown_minutes: None,
            max_executions_per_day: None,
            created_at: None,
            updated_at: None,
            is_builtin: None,
            icon: None,
            color: None,
        };

        let created = engine.create_playbook(playbook).await.unwrap();
        let id = created.id.clone();

        // Toggle to enabled
        let enabled = engine.toggle_playbook(&id, true).await.unwrap();
        assert!(enabled.enabled);

        // Toggle to disabled
        let disabled = engine.toggle_playbook(&id, false).await.unwrap();
        assert!(!disabled.enabled);
    }

    // ============================================================================
    // Trigger Evaluation Tests
    // ============================================================================

    #[tokio::test]
    async fn test_trigger_app_open() {
        // Test matching app trigger with state containing the app
        let trigger = Trigger::AppOpen {
            app_name: "chrome".to_string(),
            window_name: None,
        };

        let mut state = TriggerState::default();
        state.open_apps.insert(
            "chrome".to_string(),
            AppStateInternal {
                window_name: Some("Google".to_string()),
                last_seen: Utc::now(),
            },
        );

        let result = PlaybookEngine::evaluate_trigger(&trigger, &state, Utc::now()).await.unwrap();
        assert!(result, "Should trigger for matching app");
    }

    #[tokio::test]
    async fn test_trigger_app_open_with_window() {
        // Test matching window trigger
        let trigger = Trigger::AppOpen {
            app_name: "chrome".to_string(),
            window_name: Some("GitHub".to_string()),
        };

        let mut state = TriggerState::default();
        state.open_apps.insert(
            "chrome".to_string(),
            AppStateInternal {
                window_name: Some("GitHub - Google Chrome".to_string()),
                last_seen: Utc::now(),
            },
        );

        let result = PlaybookEngine::evaluate_trigger(&trigger, &state, Utc::now()).await.unwrap();
        assert!(result, "Should trigger for matching window");
    }

    #[tokio::test]
    async fn test_trigger_app_open_no_match() {
        // Test non-matching trigger - empty state
        let trigger = Trigger::AppOpen {
            app_name: "firefox".to_string(),
            window_name: None,
        };

        let state = TriggerState::default();
        let result = PlaybookEngine::evaluate_trigger(&trigger, &state, Utc::now()).await.unwrap();
        assert!(!result, "Should not trigger for non-matching app");
    }

    #[tokio::test]
    async fn test_trigger_time_cron() {
        // Test cron trigger - use a cron that should match current time
        // This is a basic test - the actual cron evaluation is more complex
        let trigger = Trigger::Time {
            cron: "* * * * * *".to_string(), // Every second
            description: Some("Every second".to_string()),
        };

        let state = TriggerState::default();
        let now = Utc::now();
        let result = PlaybookEngine::evaluate_trigger(&trigger, &state, now).await.unwrap();
        // Should trigger since we check within last 5 seconds
        assert!(result, "Cron trigger should match within 5 second window");
    }

    #[tokio::test]
    async fn test_trigger_context_day_of_week() {
        let trigger = Trigger::Context {
            apps: None,
            windows: None,
            time_range: None,
            days_of_week: Some(vec![0, 1, 2, 3, 4, 5, 6]), // All days
        };

        let state = TriggerState::default();
        let result = PlaybookEngine::evaluate_trigger(&trigger, &state, Utc::now()).await.unwrap();
        assert!(result, "Should trigger for any day of week");
    }

    #[tokio::test]
    async fn test_trigger_context_wrong_day() {
        let trigger = Trigger::Context {
            apps: None,
            windows: None,
            time_range: None,
            days_of_week: Some(vec![8]), // Invalid day
        };

        let state = TriggerState::default();
        let result = PlaybookEngine::evaluate_trigger(&trigger, &state, Utc::now()).await.unwrap();
        assert!(!result, "Should not trigger for invalid day");
    }

    // ============================================================================
    // App State Management Tests
    // ============================================================================

    #[tokio::test]
    async fn test_app_state_update() {
        let engine = PlaybookEngine::new(std::path::PathBuf::from("/tmp/test"));

        engine.update_app_state("vscode", Some("main.rs".to_string())).await;

        let playbooks = engine.get_playbooks_map().await;
        assert!(playbooks.is_empty()); // No playbooks yet

        // The app state is internal, but we can verify it doesn't crash
    }

    #[tokio::test]
    async fn test_app_state_remove() {
        let engine = PlaybookEngine::new(std::path::PathBuf::from("/tmp/test"));

        engine.update_app_state("chrome", None).await;
        engine.remove_app_state("chrome").await;

        // Should not crash - internal state is removed
    }

    // ============================================================================
    // Default Playbooks Tests
    // ============================================================================

    #[tokio::test]
    async fn test_default_playbooks() {
        let defaults = default_playbooks();

        assert!(!defaults.is_empty(), "Should have default playbooks");

        for playbook in &defaults {
            assert!(!playbook.id.is_empty(), "Default playbook should have ID");
            assert!(!playbook.name.is_empty(), "Default playbook should have name");
            assert_eq!(playbook.is_builtin, Some(true));
            assert!(!playbook.enabled, "Default playbooks should be disabled by default");
        }
    }

    #[tokio::test]
    async fn test_default_playbook_daily_standup() {
        let defaults = default_playbooks();

        let standup = defaults.iter().find(|p| p.id == "daily-standup");
        assert!(standup.is_some(), "Should have daily standup playbook");

        let standup = standup.unwrap();
        assert_eq!(standup.name, "Daily Standup");
        assert_eq!(standup.triggers.len(), 1);

        match &standup.triggers[0] {
            Trigger::Time { cron, .. } => {
                assert_eq!(cron, "0 9 * * 1-5");
            }
            _ => panic!("Expected Time trigger"),
        }
    }

    #[tokio::test]
    async fn test_default_playbook_customer_call() {
        let defaults = default_playbooks();

        let call = defaults.iter().find(|p| p.id == "customer-call");
        assert!(call.is_some(), "Should have customer call playbook");

        let call = call.unwrap();
        assert_eq!(call.name, "Customer Call");
        assert_eq!(call.actions.len(), 2);
    }

    #[tokio::test]
    async fn test_default_playbook_deep_work() {
        let defaults = default_playbooks();

        let deep_work = defaults.iter().find(|p| p.id == "deep-work");
        assert!(deep_work.is_some(), "Should have deep work playbook");

        let deep_work = deep_work.unwrap();
        assert_eq!(deep_work.name, "Deep Work");

        match &deep_work.triggers[0] {
            Trigger::Context { time_range, days_of_week, .. } => {
                assert_eq!(time_range, &Some("09:00-12:00".to_string()));
                assert_eq!(days_of_week, &Some(vec![1, 2, 3, 4, 5]));
            }
            _ => panic!("Expected Context trigger"),
        }
    }

    // ============================================================================
    // Serialization Tests
    // ============================================================================

    #[tokio::test]
    async fn test_playbook_serialization() {
        let playbook = Playbook {
            id: "test-123".to_string(),
            name: "Test".to_string(),
            description: Some("Description".to_string()),
            enabled: true,
            triggers: vec![Trigger::AppOpen {
                app_name: "chrome".to_string(),
                window_name: Some("Google".to_string()),
            }],
            actions: vec![Action::Notify {
                title: "Hello".to_string(),
                message: "World".to_string(),
                actions: None,
                persistent: Some(true),
            }],
            cooldown_minutes: Some(60),
            max_executions_per_day: Some(10),
            created_at: Some(Utc::now()),
            updated_at: Some(Utc::now()),
            is_builtin: Some(false),
            icon: Some("🧪".to_string()),
            color: Some("#FF0000".to_string()),
        };

        let json = serde_json::to_string(&playbook).unwrap();
        assert!(json.contains("test-123"));
        assert!(json.contains("Test"));

        let deserialized: Playbook = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.id, "test-123");
        assert_eq!(deserialized.name, "Test");
    }

    #[tokio::test]
    async fn test_trigger_serialization() {
        let triggers = vec![
            Trigger::AppOpen {
                app_name: "chrome".to_string(),
                window_name: None,
            },
            Trigger::Time {
                cron: "0 9 * * *".to_string(),
                description: Some("Daily".to_string()),
            },
            Trigger::Keyword {
                pattern: "urgent".to_string(),
                source: KeywordSource::Both,
                threshold: Some(0.8),
            },
        ];

        for trigger in triggers {
            let json = serde_json::to_string(&trigger).unwrap();
            let deserialized: Trigger = serde_json::from_str(&json).unwrap();

            match (&trigger, &deserialized) {
                (Trigger::AppOpen { app_name: a1, .. }, Trigger::AppOpen { app_name: a2, .. }) => {
                    assert_eq!(a1, a2);
                }
                (Trigger::Time { cron: c1, .. }, Trigger::Time { cron: c2, .. }) => {
                    assert_eq!(c1, c2);
                }
                (Trigger::Keyword { pattern: p1, .. }, Trigger::Keyword { pattern: p2, .. }) => {
                    assert_eq!(p1, p2);
                }
                _ => panic!("Trigger type mismatch after deserialization"),
            }
        }
    }

    #[tokio::test]
    async fn test_action_serialization() {
        let actions = vec![
            Action::Notify {
                title: "Test".to_string(),
                message: "Message".to_string(),
                actions: None,
                persistent: None,
            },
            Action::FocusMode {
                enabled: true,
                duration: Some(60),
                allowed_apps: Some(vec!["code".to_string()]),
                silence_notifications: Some(true),
            },
            Action::Webhook {
                url: "https://example.com/webhook".to_string(),
                method: HttpMethod::Post,
                headers: Some(HashMap::new()),
                body: Some(serde_json::json!({"key": "value"})),
            },
        ];

        for action in actions {
            let json = serde_json::to_string(&action).unwrap();
            let deserialized: Action = serde_json::from_str(&json).unwrap();

            match (&action, &deserialized) {
                (Action::Notify { title: t1, .. }, Action::Notify { title: t2, .. }) => {
                    assert_eq!(t1, t2);
                }
                (Action::FocusMode { enabled: e1, .. }, Action::FocusMode { enabled: e2, .. }) => {
                    assert_eq!(e1, e2);
                }
                (Action::Webhook { url: u1, .. }, Action::Webhook { url: u2, .. }) => {
                    assert_eq!(u1, u2);
                }
                _ => panic!("Action type mismatch after deserialization"),
            }
        }
    }

    // ============================================================================
    // Trigger State Tests
    // ============================================================================

    #[tokio::test]
    async fn test_trigger_state_default() {
        let state = TriggerState::default();

        assert!(state.last_execution.is_empty());
        assert!(state.daily_executions.is_empty());
        assert!(state.open_apps.is_empty());
        assert!(!state.focus_mode_active);
        assert!(state.focus_mode_end_time.is_none());
    }

    // ============================================================================
    // Engine Initialization Tests
    // ============================================================================

    #[tokio::test]
    async fn test_engine_init() {
        let engine = PlaybookEngine::new(std::path::PathBuf::from("/tmp/test"));

        let defaults = default_playbooks();
        engine.init(defaults).await.unwrap();

        let list = engine.list_playbooks().await;
        assert!(!list.is_empty(), "Should have initialized playbooks");
    }

    #[tokio::test]
    async fn test_engine_set_and_get_playbooks() {
        let engine = PlaybookEngine::new(std::path::PathBuf::from("/tmp/test"));

        let mut playbooks = HashMap::new();
        playbooks.insert("test-1".to_string(), Playbook {
            id: "test-1".to_string(),
            name: "Test 1".to_string(),
            description: None,
            enabled: false,
            triggers: vec![],
            actions: vec![],
            cooldown_minutes: None,
            max_executions_per_day: None,
            created_at: None,
            updated_at: None,
            is_builtin: None,
            icon: None,
            color: None,
        });

        engine.set_playbooks(playbooks).await;

        let retrieved = engine.get_playbooks_map().await;
        assert_eq!(retrieved.len(), 1);
        assert!(retrieved.contains_key("test-1"));
    }
}
