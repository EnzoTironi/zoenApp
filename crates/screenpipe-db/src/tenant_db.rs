use crate::tenant::TenantContext;
use crate::{
    AudioChunksResponse, AudioDevice, AudioEntry, AudioResult, AudioResultRaw, ContentType,
    FrameData, FrameWindowData, ImmediateTx, InsertUiEvent, OCRResult, OCRResultRaw, OcrEngine,
    Order, SearchResult, Speaker, TagContentType, UiContent, UiEventRecord, UiEventRow,
};
use chrono::{DateTime, Utc};
use futures::future::try_join_all;
use sqlx::Error as SqlxError;
use std::sync::Arc;
use tracing::{debug, error};

/// Extension trait for DatabaseManager that adds tenant-scoped operations.
/// These methods automatically filter queries by tenant_id to ensure
/// data isolation between tenants.
#[async_trait::async_trait]
pub trait TenantScopedDb {
    // ============================================================================
    // Insert Operations with Tenant Context
    // ============================================================================

    /// Insert an audio transcription with tenant context
    #[allow(clippy::too_many_arguments)]
    async fn insert_audio_transcription_with_tenant(
        &self,
        audio_chunk_id: i64,
        transcription: &str,
        offset_index: i64,
        transcription_engine: &str,
        device: &AudioDevice,
        speaker_id: Option<i64>,
        start_time: Option<f64>,
        end_time: Option<f64>,
        tenant: &TenantContext,
    ) -> Result<i64, SqlxError>;

    /// Insert a frame with tenant context
    async fn insert_frame_with_tenant(
        &self,
        device_name: &str,
        timestamp: Option<DateTime<Utc>>,
        browser_url: Option<&str>,
        app_name: Option<&str>,
        window_name: Option<&str>,
        focused: bool,
        offset_index: Option<i64>,
        tenant: &TenantContext,
    ) -> Result<i64, SqlxError>;

    /// Insert OCR text with tenant context
    async fn insert_ocr_text_with_tenant(
        &self,
        frame_id: i64,
        text: &str,
        text_json: &str,
        ocr_engine: Arc<OcrEngine>,
        tenant: &TenantContext,
    ) -> Result<(), SqlxError>;

    /// Batch insert frames with OCR and tenant context
    async fn insert_frames_with_ocr_batch_with_tenant(
        &self,
        device_name: &str,
        timestamp: Option<DateTime<Utc>>,
        offset_index: i64,
        windows: &[FrameWindowData],
        ocr_engine: Arc<OcrEngine>,
        tenant: &TenantContext,
    ) -> Result<Vec<(i64, usize)>, SqlxError>;

    /// Insert a UI event with tenant context
    async fn insert_ui_event_with_tenant(
        &self,
        event: &InsertUiEvent,
        tenant: &TenantContext,
    ) -> Result<i64, SqlxError>;

    /// Insert multiple UI events with tenant context
    async fn insert_ui_events_batch_with_tenant(
        &self,
        events: &[InsertUiEvent],
        tenant: &TenantContext,
    ) -> Result<usize, SqlxError>;

    // ============================================================================
    // Search Operations with Tenant Context
    // ============================================================================

    /// Search with tenant filtering
    #[allow(clippy::too_many_arguments)]
    async fn search_with_tenant(
        &self,
        query: &str,
        content_type: ContentType,
        limit: u32,
        offset: u32,
        start_time: Option<DateTime<Utc>>,
        end_time: Option<DateTime<Utc>>,
        app_name: Option<&str>,
        window_name: Option<&str>,
        min_length: Option<usize>,
        max_length: Option<usize>,
        speaker_ids: Option<Vec<i64>>,
        frame_name: Option<&str>,
        browser_url: Option<&str>,
        focused: Option<bool>,
        speaker_name: Option<&str>,
        tenant: &TenantContext,
    ) -> Result<Vec<SearchResult>, SqlxError>;

    /// Count search results with tenant filtering
    #[allow(clippy::too_many_arguments)]
    async fn count_search_results_with_tenant(
        &self,
        query: &str,
        content_type: ContentType,
        start_time: Option<DateTime<Utc>>,
        end_time: Option<DateTime<Utc>>,
        app_name: Option<&str>,
        window_name: Option<&str>,
        min_length: Option<usize>,
        max_length: Option<usize>,
        speaker_ids: Option<Vec<i64>>,
        frame_name: Option<&str>,
        browser_url: Option<&str>,
        focused: Option<bool>,
        speaker_name: Option<&str>,
        tenant: &TenantContext,
    ) -> Result<usize, SqlxError>;

    /// Search audio with tenant filtering
    #[allow(clippy::too_many_arguments)]
    async fn search_audio_with_tenant(
        &self,
        query: &str,
        limit: u32,
        offset: u32,
        start_time: Option<DateTime<Utc>>,
        end_time: Option<DateTime<Utc>>,
        min_length: Option<usize>,
        max_length: Option<usize>,
        speaker_ids: Option<Vec<i64>>,
        speaker_name: Option<&str>,
        tenant: &TenantContext,
    ) -> Result<Vec<AudioResult>, SqlxError>;

    /// Search UI monitoring with tenant filtering
    async fn search_ui_monitoring_with_tenant(
        &self,
        query: &str,
        app_name: Option<&str>,
        window_name: Option<&str>,
        start_time: Option<DateTime<Utc>>,
        end_time: Option<DateTime<Utc>>,
        limit: u32,
        offset: u32,
        tenant: &TenantContext,
    ) -> Result<Vec<UiContent>, SqlxError>;

    /// Search UI events with tenant filtering
    #[allow(clippy::too_many_arguments)]
    async fn search_ui_events_with_tenant(
        &self,
        query: Option<&str>,
        event_type: Option<&str>,
        app_name: Option<&str>,
        window_name: Option<&str>,
        start_time: Option<DateTime<Utc>>,
        end_time: Option<DateTime<Utc>>,
        limit: u32,
        offset: u32,
        tenant: &TenantContext,
    ) -> Result<Vec<UiEventRecord>, SqlxError>;

    // ============================================================================
    // Audit Logging
    // ============================================================================

    /// Log an audit event
    async fn log_audit(
        &self,
        tenant: &TenantContext,
        action: &str,
        resource: &str,
        resource_id: Option<&str>,
        details: Option<&str>,
        ip_address: Option<&str>,
        user_agent: Option<&str>,
    ) -> Result<(), SqlxError>;

    /// Get audit logs for a tenant
    async fn get_audit_logs(
        &self,
        tenant: &TenantContext,
        start_time: Option<DateTime<Utc>>,
        end_time: Option<DateTime<Utc>>,
        limit: u32,
        offset: u32,
    ) -> Result<Vec<AuditLogEntry>, SqlxError>;

    /// Count audit logs for a tenant with optional filters
    async fn count_audit_logs(
        &self,
        tenant: &TenantContext,
        start_time: Option<DateTime<Utc>>,
        end_time: Option<DateTime<Utc>>,
    ) -> Result<i64, SqlxError>;

    // ============================================================================
    // Helper Methods
    // ============================================================================

    /// Helper method for OCR search with tenant
    #[allow(clippy::too_many_arguments)]
    async fn search_ocr_with_tenant(
        &self,
        query: &str,
        limit: u32,
        offset: u32,
        start_time: Option<DateTime<Utc>>,
        end_time: Option<DateTime<Utc>>,
        app_name: Option<&str>,
        window_name: Option<&str>,
        min_length: Option<usize>,
        max_length: Option<usize>,
        frame_name: Option<&str>,
        browser_url: Option<&str>,
        focused: Option<bool>,
        tenant: &TenantContext,
    ) -> Result<Vec<OCRResult>, SqlxError>;

    // ============================================================================
    // Tag Operations with Tenant Context
    // ============================================================================

    /// Add tags to a resource with tenant context
    async fn add_tags_with_tenant(
        &self,
        resource_id: i64,
        content_type: TagContentType,
        tags: Vec<String>,
        tenant: &TenantContext,
    ) -> Result<(), SqlxError>;

    /// Remove tags from a resource with tenant context
    async fn remove_tags_with_tenant(
        &self,
        resource_id: i64,
        content_type: TagContentType,
        tags: Vec<String>,
        tenant: &TenantContext,
    ) -> Result<(), SqlxError>;

    // ============================================================================
    // Speaker Operations with Tenant Context
    // ============================================================================

    /// Update speaker with tenant context
    async fn update_speaker_with_tenant(
        &self,
        speaker_id: i64,
        name: Option<&str>,
        metadata: Option<serde_json::Value>,
        tenant: &TenantContext,
    ) -> Result<(), SqlxError>;

    /// Delete speaker with tenant context
    async fn delete_speaker_with_tenant(
        &self,
        speaker_id: i64,
        tenant: &TenantContext,
    ) -> Result<(), SqlxError>;

    /// Merge speakers with tenant context
    async fn merge_speakers_with_tenant(
        &self,
        source_speaker_id: i64,
        target_speaker_id: i64,
        tenant: &TenantContext,
    ) -> Result<(), SqlxError>;

    /// Reassign speaker with tenant context
    async fn reassign_speaker_with_tenant(
        &self,
        transcription_id: i64,
        new_speaker_id: i64,
        tenant: &TenantContext,
    ) -> Result<(), SqlxError>;
}

/// An entry in the audit log
#[derive(Debug, Clone, sqlx::FromRow)]
pub struct AuditLogEntry {
    pub id: i64,
    pub tenant_id: String,
    pub user_id: String,
    pub action: String,
    pub resource: String,
    pub resource_id: Option<String>,
    pub timestamp: DateTime<Utc>,
    pub details: Option<String>,
    pub ip_address: Option<String>,
    pub user_agent: Option<String>,
}

use crate::db::DatabaseManager;

#[async_trait::async_trait]
impl TenantScopedDb for DatabaseManager {
    #[allow(clippy::too_many_arguments)]
    async fn insert_audio_transcription_with_tenant(
        &self,
        audio_chunk_id: i64,
        transcription: &str,
        offset_index: i64,
        transcription_engine: &str,
        device: &AudioDevice,
        speaker_id: Option<i64>,
        start_time: Option<f64>,
        end_time: Option<f64>,
        tenant: &TenantContext,
    ) -> Result<i64, SqlxError> {
        let trimmed = transcription.trim();
        if trimmed.is_empty() {
            return Ok(0);
        }

        let text_length = transcription.len() as i64;
        let mut tx = self.begin_immediate_with_retry().await?;

        let result = sqlx::query(
            "INSERT OR IGNORE INTO audio_transcriptions (audio_chunk_id, transcription, offset_index, timestamp, transcription_engine, device, is_input_device, speaker_id, start_time, end_time, text_length, tenant_id) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12)",
        )
        .bind(audio_chunk_id)
        .bind(transcription)
        .bind(offset_index)
        .bind(Utc::now())
        .bind(transcription_engine)
        .bind(&device.name)
        .bind(device.device_type == crate::DeviceType::Input)
        .bind(speaker_id)
        .bind(start_time)
        .bind(end_time)
        .bind(text_length)
        .bind(&tenant.tenant_id)
        .execute(&mut **tx.conn())
        .await?;

        tx.commit().await?;

        if result.rows_affected() == 0 {
            Ok(0)
        } else {
            Ok(result.last_insert_rowid())
        }
    }

    async fn insert_frame_with_tenant(
        &self,
        device_name: &str,
        timestamp: Option<DateTime<Utc>>,
        browser_url: Option<&str>,
        app_name: Option<&str>,
        window_name: Option<&str>,
        focused: bool,
        offset_index: Option<i64>,
        tenant: &TenantContext,
    ) -> Result<i64, SqlxError> {
        let mut tx = self.begin_immediate_with_retry().await?;

        let video_chunk: Option<(i64, String)> = sqlx::query_as(
            "SELECT id, file_path FROM video_chunks WHERE device_name = ?1 ORDER BY id DESC LIMIT 1",
        )
        .bind(device_name)
        .fetch_optional(&mut **tx.conn())
        .await?;

        let (video_chunk_id, file_path) = match video_chunk {
            Some((id, path)) => (id, path),
            None => return Ok(0),
        };

        let offset_index: i64 = match offset_index {
            Some(idx) => idx,
            None => sqlx::query_scalar(
                "SELECT COALESCE(MAX(offset_index), -1) + 1 FROM frames WHERE video_chunk_id = ?1",
            )
            .bind(video_chunk_id)
            .fetch_one(&mut **tx.conn())
            .await?,
        };

        let timestamp = timestamp.unwrap_or_else(Utc::now);

        let id = sqlx::query(
            "INSERT INTO frames (video_chunk_id, offset_index, timestamp, name, browser_url, app_name, window_name, focused, device_name, tenant_id) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10)",
        )
        .bind(video_chunk_id)
        .bind(offset_index)
        .bind(timestamp)
        .bind(file_path)
        .bind(browser_url)
        .bind(app_name)
        .bind(window_name)
        .bind(focused)
        .bind(device_name)
        .bind(&tenant.tenant_id)
        .execute(&mut **tx.conn())
        .await?
        .last_insert_rowid();

        tx.commit().await?;
        Ok(id)
    }

    async fn insert_ocr_text_with_tenant(
        &self,
        frame_id: i64,
        text: &str,
        text_json: &str,
        ocr_engine: Arc<OcrEngine>,
        tenant: &TenantContext,
    ) -> Result<(), SqlxError> {
        let text_length = text.len() as i64;
        let mut tx = self.begin_immediate_with_retry().await?;

        sqlx::query(
            "INSERT INTO ocr_text (frame_id, text, text_json, ocr_engine, text_length, tenant_id) VALUES (?1, ?2, ?3, ?4, ?5, ?6)"
        )
        .bind(frame_id)
        .bind(text)
        .bind(text_json)
        .bind(format!("{:?}", *ocr_engine))
        .bind(text_length)
        .bind(&tenant.tenant_id)
        .execute(&mut **tx.conn())
        .await?;

        tx.commit().await?;
        Ok(())
    }

    async fn insert_frames_with_ocr_batch_with_tenant(
        &self,
        device_name: &str,
        timestamp: Option<DateTime<Utc>>,
        offset_index: i64,
        windows: &[FrameWindowData],
        ocr_engine: Arc<OcrEngine>,
        tenant: &TenantContext,
    ) -> Result<Vec<(i64, usize)>, SqlxError> {
        let mut tx = self.begin_immediate_with_retry().await?;

        let video_chunk: Option<(i64, String)> = sqlx::query_as(
            "SELECT id, file_path FROM video_chunks WHERE device_name = ?1 ORDER BY id DESC LIMIT 1",
        )
        .bind(device_name)
        .fetch_optional(&mut **tx.conn())
        .await?;

        let (video_chunk_id, file_path) = match video_chunk {
            Some((id, path)) => (id, path),
            None => return Ok(vec![]),
        };

        let timestamp = timestamp.unwrap_or_else(Utc::now);
        let ocr_engine_str = format!("{:?}", *ocr_engine);
        let mut results = Vec::with_capacity(windows.len());

        for (idx, window) in windows.iter().enumerate() {
            let frame_id = sqlx::query(
                "INSERT INTO frames (video_chunk_id, offset_index, timestamp, name, browser_url, app_name, window_name, focused, device_name, tenant_id) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10)",
            )
            .bind(video_chunk_id)
            .bind(offset_index)
            .bind(timestamp)
            .bind(&file_path)
            .bind(window.browser_url.as_deref())
            .bind(window.app_name.as_deref())
            .bind(window.window_name.as_deref())
            .bind(window.focused)
            .bind(device_name)
            .bind(&tenant.tenant_id)
            .execute(&mut **tx.conn())
            .await?
            .last_insert_rowid();

            let text_length = window.text.len() as i64;
            sqlx::query(
                "INSERT INTO ocr_text (frame_id, text, text_json, ocr_engine, text_length, tenant_id) VALUES (?1, ?2, ?3, ?4, ?5, ?6)"
            )
            .bind(frame_id)
            .bind(&window.text)
            .bind(&window.text_json)
            .bind(&ocr_engine_str)
            .bind(text_length)
            .bind(&tenant.tenant_id)
            .execute(&mut **tx.conn())
            .await?;

            results.push((frame_id, idx));
        }

        tx.commit().await?;
        Ok(results)
    }

    async fn insert_ui_event_with_tenant(
        &self,
        event: &InsertUiEvent,
        tenant: &TenantContext,
    ) -> Result<i64, SqlxError> {
        let text_length = event.text_content.as_ref().map(|s| s.len() as i32);

        let result = sqlx::query(
            r#"
            INSERT INTO ui_events (
                timestamp, session_id, relative_ms, event_type,
                x, y, delta_x, delta_y,
                button, click_count, key_code, modifiers,
                text_content, text_length,
                app_name, app_pid, window_title, browser_url,
                element_role, element_name, element_value, element_description,
                element_automation_id, element_bounds, frame_id, tenant_id
            ) VALUES (
                ?1, ?2, ?3, ?4,
                ?5, ?6, ?7, ?8,
                ?9, ?10, ?11, ?12,
                ?13, ?14,
                ?15, ?16, ?17, ?18,
                ?19, ?20, ?21, ?22,
                ?23, ?24, ?25, ?26
            )
            "#,
        )
        .bind(event.timestamp)
        .bind(&event.session_id)
        .bind(event.relative_ms)
        .bind(event.event_type.to_string())
        .bind(event.x)
        .bind(event.y)
        .bind(event.delta_x.map(|v| v as i32))
        .bind(event.delta_y.map(|v| v as i32))
        .bind(event.button.map(|v| v as i32))
        .bind(event.click_count.map(|v| v as i32))
        .bind(event.key_code.map(|v| v as i32))
        .bind(event.modifiers.map(|v| v as i32))
        .bind(&event.text_content)
        .bind(text_length)
        .bind(&event.app_name)
        .bind(event.app_pid)
        .bind(&event.window_title)
        .bind(&event.browser_url)
        .bind(&event.element_role)
        .bind(&event.element_name)
        .bind(&event.element_value)
        .bind(&event.element_description)
        .bind(&event.element_automation_id)
        .bind(&event.element_bounds)
        .bind(event.frame_id)
        .bind(&tenant.tenant_id)
        .execute(&self.pool)
        .await?;

        Ok(result.last_insert_rowid())
    }

    async fn insert_ui_events_batch_with_tenant(
        &self,
        events: &[InsertUiEvent],
        tenant: &TenantContext,
    ) -> Result<usize, SqlxError> {
        if events.is_empty() {
            return Ok(0);
        }

        let mut tx = self.begin_immediate_with_retry().await?;
        let mut count = 0;

        for event in events {
            let text_length = event.text_content.as_ref().map(|s| s.len() as i32);

            sqlx::query(
                r#"
                INSERT INTO ui_events (
                    timestamp, session_id, relative_ms, event_type,
                    x, y, delta_x, delta_y,
                    button, click_count, key_code, modifiers,
                    text_content, text_length,
                    app_name, app_pid, window_title, browser_url,
                    element_role, element_name, element_value, element_description,
                    element_automation_id, element_bounds, frame_id, tenant_id
                ) VALUES (
                    ?1, ?2, ?3, ?4,
                    ?5, ?6, ?7, ?8,
                    ?9, ?10, ?11, ?12,
                    ?13, ?14,
                    ?15, ?16, ?17, ?18,
                    ?19, ?20, ?21, ?22,
                    ?23, ?24, ?25, ?26
                )
                "#,
            )
            .bind(event.timestamp)
            .bind(&event.session_id)
            .bind(event.relative_ms)
            .bind(event.event_type.to_string())
            .bind(event.x)
            .bind(event.y)
            .bind(event.delta_x.map(|v| v as i32))
            .bind(event.delta_y.map(|v| v as i32))
            .bind(event.button.map(|v| v as i32))
            .bind(event.click_count.map(|v| v as i32))
            .bind(event.key_code.map(|v| v as i32))
            .bind(event.modifiers.map(|v| v as i32))
            .bind(&event.text_content)
            .bind(text_length)
            .bind(&event.app_name)
            .bind(event.app_pid)
            .bind(&event.window_title)
            .bind(&event.browser_url)
            .bind(&event.element_role)
            .bind(&event.element_name)
            .bind(&event.element_value)
            .bind(&event.element_description)
            .bind(&event.element_automation_id)
            .bind(&event.element_bounds)
            .bind(event.frame_id)
            .bind(&tenant.tenant_id)
            .execute(&mut **tx.conn())
            .await?;

            count += 1;
        }

        tx.commit().await?;
        Ok(count)
    }

    #[allow(clippy::too_many_arguments)]
    async fn search_with_tenant(
        &self,
        query: &str,
        mut content_type: ContentType,
        limit: u32,
        offset: u32,
        start_time: Option<DateTime<Utc>>,
        end_time: Option<DateTime<Utc>>,
        app_name: Option<&str>,
        window_name: Option<&str>,
        min_length: Option<usize>,
        max_length: Option<usize>,
        speaker_ids: Option<Vec<i64>>,
        frame_name: Option<&str>,
        browser_url: Option<&str>,
        focused: Option<bool>,
        speaker_name: Option<&str>,
        tenant: &TenantContext,
    ) -> Result<Vec<SearchResult>, SqlxError> {
        let mut results = Vec::new();

        if focused.is_some() || browser_url.is_some() {
            content_type = ContentType::OCR;
        }

        match content_type {
            ContentType::All => {
                let (ocr_results, audio_results, ui_results) =
                    if app_name.is_none() && window_name.is_none() && frame_name.is_none() {
                        let (ocr, audio, ui) = tokio::try_join!(
                            self.search_ocr_with_tenant(
                                query,
                                limit,
                                offset,
                                start_time,
                                end_time,
                                app_name,
                                window_name,
                                min_length,
                                max_length,
                                frame_name,
                                browser_url,
                                focused,
                                tenant,
                            ),
                            self.search_audio_with_tenant(
                                query,
                                limit,
                                offset,
                                start_time,
                                end_time,
                                min_length,
                                max_length,
                                speaker_ids.clone(),
                                speaker_name,
                                tenant,
                            ),
                            self.search_ui_monitoring_with_tenant(
                                query,
                                app_name,
                                window_name,
                                start_time,
                                end_time,
                                limit,
                                offset,
                                tenant,
                            )
                        )?;
                        (ocr, Some(audio), ui)
                    } else {
                        let (ocr, ui) = tokio::try_join!(
                            self.search_ocr_with_tenant(
                                query,
                                limit,
                                offset,
                                start_time,
                                end_time,
                                app_name,
                                window_name,
                                min_length,
                                max_length,
                                frame_name,
                                browser_url,
                                focused,
                                tenant,
                            ),
                            self.search_ui_monitoring_with_tenant(
                                query,
                                app_name,
                                window_name,
                                start_time,
                                end_time,
                                limit,
                                offset,
                                tenant,
                            )
                        )?;
                        (ocr, None, ui)
                    };

                results.extend(ocr_results.into_iter().map(SearchResult::OCR));
                if let Some(audio) = audio_results {
                    results.extend(audio.into_iter().map(SearchResult::Audio));
                }
                results.extend(ui_results.into_iter().map(SearchResult::UI));
            }
            ContentType::OCR => {
                let ocr_results = self
                    .search_ocr_with_tenant(
                        query,
                        limit,
                        offset,
                        start_time,
                        end_time,
                        app_name,
                        window_name,
                        min_length,
                        max_length,
                        frame_name,
                        browser_url,
                        focused,
                        tenant,
                    )
                    .await?;
                results.extend(ocr_results.into_iter().map(SearchResult::OCR));
            }
            ContentType::Audio => {
                if app_name.is_none() && window_name.is_none() {
                    let audio_results = self
                        .search_audio_with_tenant(
                            query,
                            limit,
                            offset,
                            start_time,
                            end_time,
                            min_length,
                            max_length,
                            speaker_ids,
                            speaker_name,
                            tenant,
                        )
                        .await?;
                    results.extend(audio_results.into_iter().map(SearchResult::Audio));
                }
            }
            ContentType::UI => {
                let ui_results = self
                    .search_ui_monitoring_with_tenant(
                        query,
                        app_name,
                        window_name,
                        start_time,
                        end_time,
                        limit,
                        offset,
                        tenant,
                    )
                    .await?;
                results.extend(ui_results.into_iter().map(SearchResult::UI));
            }
            _ => {}
        }

        results.sort_by(|a, b| {
            let timestamp_a = match a {
                SearchResult::OCR(ocr) => ocr.timestamp,
                SearchResult::Audio(audio) => audio.timestamp,
                SearchResult::UI(ui) => ui.timestamp,
                SearchResult::Input(input) => input.timestamp,
            };
            let timestamp_b = match b {
                SearchResult::OCR(ocr) => ocr.timestamp,
                SearchResult::Audio(audio) => audio.timestamp,
                SearchResult::UI(ui) => ui.timestamp,
                SearchResult::Input(input) => input.timestamp,
            };
            timestamp_b.cmp(&timestamp_a)
        });

        results = results
            .into_iter()
            .skip(offset as usize)
            .take(limit as usize)
            .collect();

        Ok(results)
    }

    #[allow(clippy::too_many_arguments)]
    async fn count_search_results_with_tenant(
        &self,
        query: &str,
        content_type: ContentType,
        start_time: Option<DateTime<Utc>>,
        end_time: Option<DateTime<Utc>>,
        app_name: Option<&str>,
        window_name: Option<&str>,
        min_length: Option<usize>,
        max_length: Option<usize>,
        speaker_ids: Option<Vec<i64>>,
        frame_name: Option<&str>,
        browser_url: Option<&str>,
        focused: Option<bool>,
        speaker_name: Option<&str>,
        tenant: &TenantContext,
    ) -> Result<usize, SqlxError> {
        // For now, delegate to search and count results
        // In production, implement a proper COUNT query with tenant filtering
        let results = self
            .search_with_tenant(
                query,
                content_type,
                10000,
                0,
                start_time,
                end_time,
                app_name,
                window_name,
                min_length,
                max_length,
                speaker_ids,
                frame_name,
                browser_url,
                focused,
                speaker_name,
                tenant,
            )
            .await?;
        Ok(results.len())
    }

    #[allow(clippy::too_many_arguments)]
    async fn search_audio_with_tenant(
        &self,
        query: &str,
        limit: u32,
        offset: u32,
        start_time: Option<DateTime<Utc>>,
        end_time: Option<DateTime<Utc>>,
        min_length: Option<usize>,
        max_length: Option<usize>,
        speaker_ids: Option<Vec<i64>>,
        speaker_name: Option<&str>,
        tenant: &TenantContext,
    ) -> Result<Vec<AudioResult>, SqlxError> {
        let mut base_sql = String::from(
            "SELECT
                audio_transcriptions.audio_chunk_id,
                audio_transcriptions.transcription,
                audio_transcriptions.timestamp,
                audio_chunks.file_path,
                audio_transcriptions.offset_index,
                audio_transcriptions.transcription_engine,
                GROUP_CONCAT(tags.name, ',') as tags,
                audio_transcriptions.device as device_name,
                audio_transcriptions.is_input_device,
                audio_transcriptions.speaker_id,
                audio_transcriptions.start_time,
                audio_transcriptions.end_time
             FROM audio_transcriptions
             JOIN audio_chunks ON audio_transcriptions.audio_chunk_id = audio_chunks.id
             LEFT JOIN speakers ON audio_transcriptions.speaker_id = speakers.id
             LEFT JOIN audio_tags ON audio_chunks.id = audio_tags.audio_chunk_id
             LEFT JOIN tags ON audio_tags.tag_id = tags.id",
        );

        if !query.is_empty() {
            base_sql.push_str(" JOIN audio_transcriptions_fts ON audio_transcriptions_fts.audio_chunk_id = audio_transcriptions.audio_chunk_id");
        }

        let mut conditions = Vec::new();
        conditions.push(
            "(audio_transcriptions.tenant_id = ? OR audio_transcriptions.tenant_id IS NULL)"
                .to_string(),
        );

        if !query.is_empty() {
            conditions.push("audio_transcriptions_fts MATCH ?".to_string());
        }
        if start_time.is_some() {
            conditions.push("audio_transcriptions.timestamp >= ?".to_string());
        }
        if end_time.is_some() {
            conditions.push("audio_transcriptions.timestamp <= ?".to_string());
        }
        if min_length.is_some() {
            conditions.push("COALESCE(audio_transcriptions.text_length, LENGTH(audio_transcriptions.transcription)) >= ?".to_string());
        }
        if max_length.is_some() {
            conditions.push("COALESCE(audio_transcriptions.text_length, LENGTH(audio_transcriptions.transcription)) <= ?".to_string());
        }
        conditions.push("(speakers.id IS NULL OR speakers.hallucination = 0)".to_string());
        if speaker_ids.is_some() {
            conditions.push("(json_array_length(?) = 0 OR audio_transcriptions.speaker_id IN (SELECT value FROM json_each(?)))".to_string());
        }
        if speaker_name.is_some() {
            conditions.push("speakers.name LIKE '%' || ? || '%' COLLATE NOCASE".to_string());
        }

        let where_clause = format!("WHERE {}", conditions.join(" AND "));

        let sql = format!(
            "{} {} GROUP BY audio_transcriptions.audio_chunk_id, audio_transcriptions.offset_index ORDER BY audio_transcriptions.timestamp DESC LIMIT ? OFFSET ?",
            base_sql, where_clause
        );

        let speaker_ids_json = speaker_ids.as_ref().map_or_else(
            || "[]".to_string(),
            |ids| serde_json::to_string(&ids).unwrap_or_else(|_| "[]".to_string()),
        );

        let mut query_builder = sqlx::query_as::<_, AudioResultRaw>(&sql);
        query_builder = query_builder.bind(&tenant.tenant_id);

        if !query.is_empty() {
            query_builder = query_builder.bind(query);
        }
        if let Some(start) = start_time {
            query_builder = query_builder.bind(start);
        }
        if let Some(end) = end_time {
            query_builder = query_builder.bind(end);
        }
        if let Some(min) = min_length {
            query_builder = query_builder.bind(min as i64);
        }
        if let Some(max) = max_length {
            query_builder = query_builder.bind(max as i64);
        }
        if speaker_ids.is_some() {
            query_builder = query_builder
                .bind(&speaker_ids_json)
                .bind(&speaker_ids_json);
        }
        if let Some(name) = speaker_name {
            query_builder = query_builder.bind(name);
        }
        query_builder = query_builder.bind(limit as i64).bind(offset as i64);

        let results_raw: Vec<AudioResultRaw> = query_builder.fetch_all(&self.pool).await?;

        let futures: Vec<_> = results_raw
            .into_iter()
            .map(|raw| async move {
                let speaker = match raw.speaker_id {
                    Some(id) => (self.get_speaker_by_id(id).await).ok(),
                    None => None,
                };

                Ok::<AudioResult, SqlxError>(AudioResult {
                    audio_chunk_id: raw.audio_chunk_id,
                    transcription: raw.transcription,
                    timestamp: raw.timestamp,
                    file_path: raw.file_path,
                    offset_index: raw.offset_index,
                    transcription_engine: raw.transcription_engine,
                    tags: raw
                        .tags
                        .map(|s| s.split(',').map(|s| s.to_owned()).collect())
                        .unwrap_or_default(),
                    device_name: raw.device_name,
                    device_type: if raw.is_input_device {
                        crate::DeviceType::Input
                    } else {
                        crate::DeviceType::Output
                    },
                    speaker,
                    start_time: raw.start_time,
                    end_time: raw.end_time,
                })
            })
            .collect();

        Ok(try_join_all(futures).await?.into_iter().collect())
    }

    async fn search_ui_monitoring_with_tenant(
        &self,
        query: &str,
        app_name: Option<&str>,
        window_name: Option<&str>,
        start_time: Option<DateTime<Utc>>,
        end_time: Option<DateTime<Utc>>,
        limit: u32,
        offset: u32,
        tenant: &TenantContext,
    ) -> Result<Vec<UiContent>, SqlxError> {
        let mut fts_parts = Vec::new();
        if !query.is_empty() {
            fts_parts.push(query.to_owned());
        }
        if let Some(app) = app_name {
            fts_parts.push(format!("app:{}", app));
        }
        if let Some(window) = window_name {
            fts_parts.push(format!("window:{}", window));
        }
        let combined_query = fts_parts.join(" ");

        let base_sql = if combined_query.is_empty() {
            "ui_monitoring"
        } else {
            "ui_monitoring_fts JOIN ui_monitoring ON ui_monitoring_fts.ui_id = ui_monitoring.id"
        };

        let where_clause = if combined_query.is_empty() {
            "WHERE (ui_monitoring.tenant_id = ? OR ui_monitoring.tenant_id IS NULL)"
        } else {
            "WHERE ui_monitoring_fts MATCH ?1 AND (ui_monitoring.tenant_id = ?2 OR ui_monitoring.tenant_id IS NULL)"
        };

        let sql = format!(
            r#"
            SELECT
                ui_monitoring.id,
                ui_monitoring.text_output,
                ui_monitoring.timestamp,
                ui_monitoring.app as app_name,
                ui_monitoring.window as window_name,
                ui_monitoring.initial_traversal_at,
                video_chunks.file_path,
                frames.offset_index,
                frames.name as frame_name,
                frames.browser_url
            FROM {}
            LEFT JOIN frames ON
                frames.timestamp BETWEEN
                    datetime(ui_monitoring.timestamp, '-1 seconds')
                    AND datetime(ui_monitoring.timestamp, '+1 seconds')
            LEFT JOIN video_chunks ON frames.video_chunk_id = video_chunks.id
            {}
                AND (?3 IS NULL OR ui_monitoring.timestamp >= ?3)
                AND (?4 IS NULL OR ui_monitoring.timestamp <= ?4)
            GROUP BY ui_monitoring.id
            ORDER BY ui_monitoring.timestamp DESC
            LIMIT ?5 OFFSET ?6
            "#,
            base_sql, where_clause
        );

        let mut query_builder = sqlx::query_as(&sql);

        if !combined_query.is_empty() {
            query_builder = query_builder.bind(&combined_query);
        }
        query_builder = query_builder.bind(&tenant.tenant_id);
        query_builder = query_builder.bind(start_time);
        query_builder = query_builder.bind(end_time);
        query_builder = query_builder.bind(limit);
        query_builder = query_builder.bind(offset);

        query_builder.fetch_all(&self.pool).await
    }

    #[allow(clippy::too_many_arguments)]
    async fn search_ui_events_with_tenant(
        &self,
        query: Option<&str>,
        event_type: Option<&str>,
        app_name: Option<&str>,
        window_name: Option<&str>,
        start_time: Option<DateTime<Utc>>,
        end_time: Option<DateTime<Utc>>,
        limit: u32,
        offset: u32,
        tenant: &TenantContext,
    ) -> Result<Vec<UiEventRecord>, SqlxError> {
        let mut conditions =
            vec!["(ui_events.tenant_id = ? OR ui_events.tenant_id IS NULL)".to_string()];
        let mut search_params: Vec<String> = vec![];

        if let Some(q) = query {
            if !q.is_empty() {
                conditions.push(
                    "(text_content LIKE '%' || ? || '%' OR app_name LIKE '%' || ? || '%' OR window_title LIKE '%' || ? || '%')".to_string()
                );
                search_params.push(q.to_string());
                search_params.push(q.to_string());
                search_params.push(q.to_string());
            }
        }
        if let Some(et) = event_type {
            if !et.is_empty() {
                conditions.push("event_type = ?".to_string());
                search_params.push(et.to_string());
            }
        }
        if let Some(app) = app_name {
            if !app.is_empty() {
                conditions.push("app_name LIKE '%' || ? || '%'".to_string());
                search_params.push(app.to_string());
            }
        }
        if let Some(window) = window_name {
            if !window.is_empty() {
                conditions.push("window_title LIKE '%' || ? || '%'".to_string());
                search_params.push(window.to_string());
            }
        }

        let where_clause = conditions.join(" AND ");

        let sql = format!(
            r#"
            SELECT
                id, timestamp, session_id, relative_ms, event_type,
                x, y, delta_x, delta_y, button, click_count,
                key_code, modifiers, text_content, text_length,
                app_name, app_pid, window_title, browser_url,
                element_role, element_name, element_value,
                element_description, element_automation_id, element_bounds,
                frame_id
            FROM ui_events
            WHERE {}
                AND (? IS NULL OR timestamp >= ?)
                AND (? IS NULL OR timestamp <= ?)
            ORDER BY timestamp DESC
            LIMIT ? OFFSET ?
            "#,
            where_clause
        );

        let mut query_builder = sqlx::query_as(&sql).bind(&tenant.tenant_id);

        for param in search_params {
            query_builder = query_builder.bind(param);
        }

        let rows: Vec<UiEventRow> = query_builder
            .bind(start_time)
            .bind(start_time)
            .bind(end_time)
            .bind(end_time)
            .bind(limit)
            .bind(offset)
            .fetch_all(&self.pool)
            .await?;

        Ok(rows.into_iter().map(|r| r.into()).collect())
    }

    async fn log_audit(
        &self,
        tenant: &TenantContext,
        action: &str,
        resource: &str,
        resource_id: Option<&str>,
        details: Option<&str>,
        ip_address: Option<&str>,
        user_agent: Option<&str>,
    ) -> Result<(), SqlxError> {
        sqlx::query(
            "INSERT INTO audit_logs (tenant_id, user_id, action, resource, resource_id, details, ip_address, user_agent) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)"
        )
        .bind(&tenant.tenant_id)
        .bind(&tenant.user_id)
        .bind(action)
        .bind(resource)
        .bind(resource_id)
        .bind(details)
        .bind(ip_address)
        .bind(user_agent)
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    async fn get_audit_logs(
        &self,
        tenant: &TenantContext,
        start_time: Option<DateTime<Utc>>,
        end_time: Option<DateTime<Utc>>,
        limit: u32,
        offset: u32,
    ) -> Result<Vec<AuditLogEntry>, SqlxError> {
        let mut sql = String::from(
            "SELECT id, tenant_id, user_id, action, resource, resource_id, timestamp, details, ip_address, user_agent FROM audit_logs WHERE tenant_id = ?"
        );

        if start_time.is_some() {
            sql.push_str(" AND timestamp >= ?");
        }
        if end_time.is_some() {
            sql.push_str(" AND timestamp <= ?");
        }

        sql.push_str(" ORDER BY timestamp DESC LIMIT ? OFFSET ?");

        let mut query = sqlx::query_as(&sql).bind(&tenant.tenant_id);

        if let Some(start) = start_time {
            query = query.bind(start);
        }
        if let Some(end) = end_time {
            query = query.bind(end);
        }

        query
            .bind(limit as i64)
            .bind(offset as i64)
            .fetch_all(&self.pool)
            .await
    }

    // ============================================================================
    // Helper method for OCR search with tenant
    // ============================================================================

    #[allow(clippy::too_many_arguments)]
    async fn search_ocr_with_tenant(
        &self,
        query: &str,
        limit: u32,
        offset: u32,
        start_time: Option<DateTime<Utc>>,
        end_time: Option<DateTime<Utc>>,
        app_name: Option<&str>,
        window_name: Option<&str>,
        min_length: Option<usize>,
        max_length: Option<usize>,
        frame_name: Option<&str>,
        browser_url: Option<&str>,
        focused: Option<bool>,
        tenant: &TenantContext,
    ) -> Result<Vec<OCRResult>, SqlxError> {
        let mut frame_fts_parts = Vec::new();

        if let Some(app) = app_name {
            if !app.is_empty() {
                frame_fts_parts.push(format!("app_name:{}", app));
            }
        }
        if let Some(window) = window_name {
            if !window.is_empty() {
                frame_fts_parts.push(format!("window_name:{}", window));
            }
        }
        if let Some(browser) = browser_url {
            if !browser.is_empty() {
                frame_fts_parts.push(format!("browser_url:{}", browser));
            }
        }
        if let Some(is_focused) = focused {
            frame_fts_parts.push(format!("focused:{}", if is_focused { "1" } else { "0" }));
        }
        if let Some(frame_name) = frame_name {
            if !frame_name.is_empty() {
                frame_fts_parts.push(format!("name:{}", frame_name));
            }
        }

        let frame_query = frame_fts_parts.join(" ");

        let sql = format!(
            r#"
        SELECT
            ocr_text.frame_id,
            ocr_text.text as ocr_text,
            ocr_text.text_json,
            frames.timestamp,
            frames.name as frame_name,
            video_chunks.file_path,
            frames.offset_index,
            frames.app_name,
            ocr_text.ocr_engine,
            frames.window_name,
            video_chunks.device_name,
            GROUP_CONCAT(tags.name, ',') as tags,
            frames.browser_url,
            frames.focused
        FROM frames
        JOIN video_chunks ON frames.video_chunk_id = video_chunks.id
        JOIN ocr_text ON frames.id = ocr_text.frame_id
        LEFT JOIN vision_tags ON frames.id = vision_tags.vision_id
        LEFT JOIN tags ON vision_tags.tag_id = tags.id
        {frame_fts_join}
        {ocr_fts_join}
        WHERE (frames.tenant_id = ?1 OR frames.tenant_id IS NULL)
            AND (ocr_text.tenant_id = ?2 OR ocr_text.tenant_id IS NULL)
            {frame_fts_condition}
            {ocr_fts_condition}
            AND (?3 IS NULL OR frames.timestamp >= ?3)
            AND (?4 IS NULL OR frames.timestamp <= ?4)
            AND (?5 IS NULL OR COALESCE(ocr_text.text_length, LENGTH(ocr_text.text)) >= ?5)
            AND (?6 IS NULL OR COALESCE(ocr_text.text_length, LENGTH(ocr_text.text)) <= ?6)
        GROUP BY frames.id
        ORDER BY {order_clause}
        LIMIT ?9 OFFSET ?10
        "#,
            frame_fts_join = if frame_query.trim().is_empty() {
                ""
            } else {
                "JOIN frames_fts ON frames.id = frames_fts.id"
            },
            ocr_fts_join = if query.trim().is_empty() {
                ""
            } else {
                "JOIN ocr_text_fts ON ocr_text.frame_id = ocr_text_fts.frame_id"
            },
            frame_fts_condition = if frame_query.trim().is_empty() {
                ""
            } else {
                "AND frames_fts MATCH ?7"
            },
            ocr_fts_condition = if query.trim().is_empty() {
                ""
            } else {
                "AND ocr_text_fts MATCH ?8"
            },
            order_clause = if query.trim().is_empty() {
                "frames.timestamp DESC"
            } else {
                "ocr_text_fts.rank, frames.timestamp DESC"
            }
        );

        let query_builder = sqlx::query_as(&sql);

        let raw_results: Vec<OCRResultRaw> = query_builder
            .bind(&tenant.tenant_id) // ?1: frames.tenant_id
            .bind(&tenant.tenant_id) // ?2: ocr_text.tenant_id
            .bind(start_time) // ?3: start_time
            .bind(end_time) // ?4: end_time
            .bind(min_length.map(|l| l as i64)) // ?5: min_length
            .bind(max_length.map(|l| l as i64)) // ?6: max_length
            .bind(if frame_query.trim().is_empty() {
                // ?7: frame_fts_match
                None
            } else {
                Some(&frame_query)
            })
            .bind(if query.trim().is_empty() {
                // ?8: ocr_fts_match
                None
            } else {
                Some(query)
            })
            .bind(limit) // ?9: limit
            .bind(offset) // ?10: offset
            .fetch_all(&self.pool)
            .await?;

        Ok(raw_results
            .into_iter()
            .map(|raw| OCRResult {
                frame_id: raw.frame_id,
                ocr_text: raw.ocr_text,
                text_json: raw.text_json,
                timestamp: raw.timestamp,
                frame_name: raw.frame_name,
                file_path: raw.file_path,
                offset_index: raw.offset_index,
                app_name: raw.app_name,
                ocr_engine: raw.ocr_engine,
                window_name: raw.window_name,
                device_name: raw.device_name,
                tags: raw
                    .tags
                    .map(|t| t.split(',').map(String::from).collect())
                    .unwrap_or_default(),
                browser_url: raw.browser_url,
                focused: raw.focused,
            })
            .collect())
    }

    // ============================================================================
    // Tag Operations with Tenant Context
    // ============================================================================

    async fn add_tags_with_tenant(
        &self,
        resource_id: i64,
        content_type: TagContentType,
        tags: Vec<String>,
        tenant: &TenantContext,
    ) -> Result<(), SqlxError> {
        // Check if tenant can modify this resource
        if !tenant.can_modify() {
            return Err(SqlxError::RowNotFound);
        }

        let mut tx = self.begin_immediate_with_retry().await?;

        for tag in tags {
            // Insert tag if not exists
            sqlx::query("INSERT OR IGNORE INTO tags (name) VALUES (?1)")
                .bind(&tag)
                .execute(&mut **tx.conn())
                .await?;

            // Get tag id
            let tag_id: i64 = sqlx::query_scalar("SELECT id FROM tags WHERE name = ?1")
                .bind(&tag)
                .fetch_one(&mut **tx.conn())
                .await?;

            // Link tag to resource
            let (table, resource_column) = match content_type {
                TagContentType::Vision => ("vision_tags", "vision_id"),
                TagContentType::Audio => ("audio_tags", "audio_chunk_id"),
            };

            sqlx::query(&format!(
                "INSERT OR IGNORE INTO {} (tag_id, {}) VALUES (?1, ?2)",
                table, resource_column
            ))
            .bind(tag_id)
            .bind(resource_id)
            .execute(&mut **tx.conn())
            .await?;
        }

        tx.commit().await?;
        Ok(())
    }

    async fn remove_tags_with_tenant(
        &self,
        resource_id: i64,
        content_type: TagContentType,
        tags: Vec<String>,
        tenant: &TenantContext,
    ) -> Result<(), SqlxError> {
        // Check if tenant can modify this resource
        if !tenant.can_modify() {
            return Err(SqlxError::RowNotFound);
        }

        let mut tx = self.begin_immediate_with_retry().await?;

        let (table, resource_column) = match content_type {
            TagContentType::Vision => ("vision_tags", "vision_id"),
            TagContentType::Audio => ("audio_tags", "audio_chunk_id"),
        };

        for tag in tags {
            // Get tag id
            let tag_id: Option<i64> = sqlx::query_scalar("SELECT id FROM tags WHERE name = ?1")
                .bind(&tag)
                .fetch_optional(&mut **tx.conn())
                .await?;

            if let Some(tag_id) = tag_id {
                sqlx::query(&format!(
                    "DELETE FROM {} WHERE tag_id = ?1 AND {} = ?2",
                    table, resource_column
                ))
                .bind(tag_id)
                .bind(resource_id)
                .execute(&mut **tx.conn())
                .await?;
            }
        }

        tx.commit().await?;
        Ok(())
    }

    // ============================================================================
    // Speaker Operations with Tenant Context
    // ============================================================================

    async fn update_speaker_with_tenant(
        &self,
        speaker_id: i64,
        name: Option<&str>,
        metadata: Option<serde_json::Value>,
        tenant: &TenantContext,
    ) -> Result<(), SqlxError> {
        // Check if tenant can modify
        if !tenant.can_modify() {
            return Err(SqlxError::RowNotFound);
        }

        let mut tx = self.begin_immediate_with_retry().await?;

        // Verify speaker belongs to tenant
        let speaker_tenant: Option<String> =
            sqlx::query_scalar("SELECT tenant_id FROM speakers WHERE id = ?1")
                .bind(speaker_id)
                .fetch_optional(&mut **tx.conn())
                .await?;

        if let Some(st) = speaker_tenant {
            if st != tenant.tenant_id && !tenant.is_system() {
                return Err(SqlxError::RowNotFound);
            }
        }

        if let Some(name) = name {
            sqlx::query("UPDATE speakers SET name = ?1, updated_at = ?2 WHERE id = ?3")
                .bind(name)
                .bind(Utc::now())
                .bind(speaker_id)
                .execute(&mut **tx.conn())
                .await?;
        }

        if let Some(metadata) = metadata {
            sqlx::query("UPDATE speakers SET metadata = ?1, updated_at = ?2 WHERE id = ?3")
                .bind(metadata.to_string())
                .bind(Utc::now())
                .bind(speaker_id)
                .execute(&mut **tx.conn())
                .await?;
        }

        tx.commit().await?;
        Ok(())
    }

    // ============================================================================
    // Speaker Operations with Tenant Context
    // ============================================================================

    async fn delete_speaker_with_tenant(
        &self,
        speaker_id: i64,
        tenant: &TenantContext,
    ) -> Result<(), SqlxError> {
        // Check if tenant can delete
        if !tenant.can_delete() {
            return Err(SqlxError::RowNotFound);
        }

        let mut tx = self.begin_immediate_with_retry().await?;

        // Verify speaker belongs to tenant
        let speaker_tenant: Option<String> =
            sqlx::query_scalar("SELECT tenant_id FROM speakers WHERE id = ?1")
                .bind(speaker_id)
                .fetch_optional(&mut **tx.conn())
                .await?;

        if let Some(st) = speaker_tenant {
            if st != tenant.tenant_id && !tenant.is_system() {
                return Err(SqlxError::RowNotFound);
            }
        }

        // Reassign transcriptions to unknown speaker (id: 0)
        sqlx::query("UPDATE audio_transcriptions SET speaker_id = 0 WHERE speaker_id = ?1")
            .bind(speaker_id)
            .execute(&mut **tx.conn())
            .await?;

        // Delete speaker
        sqlx::query("DELETE FROM speakers WHERE id = ?1")
            .bind(speaker_id)
            .execute(&mut **tx.conn())
            .await?;

        tx.commit().await?;
        Ok(())
    }

    async fn merge_speakers_with_tenant(
        &self,
        source_speaker_id: i64,
        target_speaker_id: i64,
        tenant: &TenantContext,
    ) -> Result<(), SqlxError> {
        // Check if tenant can modify
        if !tenant.can_modify() {
            return Err(SqlxError::RowNotFound);
        }

        let mut tx = self.begin_immediate_with_retry().await?;

        // Verify both speakers belong to tenant
        let source_tenant: Option<String> =
            sqlx::query_scalar("SELECT tenant_id FROM speakers WHERE id = ?1")
                .bind(source_speaker_id)
                .fetch_optional(&mut **tx.conn())
                .await?;

        let target_tenant: Option<String> =
            sqlx::query_scalar("SELECT tenant_id FROM speakers WHERE id = ?1")
                .bind(target_speaker_id)
                .fetch_optional(&mut **tx.conn())
                .await?;

        if let Some(st) = source_tenant {
            if st != tenant.tenant_id && !tenant.is_system() {
                return Err(SqlxError::RowNotFound);
            }
        }

        if let Some(tt) = target_tenant {
            if tt != tenant.tenant_id && !tenant.is_system() {
                return Err(SqlxError::RowNotFound);
            }
        }

        // Reassign transcriptions
        sqlx::query("UPDATE audio_transcriptions SET speaker_id = ?1 WHERE speaker_id = ?2")
            .bind(target_speaker_id)
            .bind(source_speaker_id)
            .execute(&mut **tx.conn())
            .await?;

        // Delete source speaker
        sqlx::query("DELETE FROM speakers WHERE id = ?1")
            .bind(source_speaker_id)
            .execute(&mut **tx.conn())
            .await?;

        tx.commit().await?;
        Ok(())
    }

    async fn reassign_speaker_with_tenant(
        &self,
        transcription_id: i64,
        new_speaker_id: i64,
        tenant: &TenantContext,
    ) -> Result<(), SqlxError> {
        // Check if tenant can modify
        if !tenant.can_modify() {
            return Err(SqlxError::RowNotFound);
        }

        let mut tx = self.begin_immediate_with_retry().await?;

        // Verify transcription belongs to tenant
        let trans_tenant: Option<String> = sqlx::query_scalar(
            "SELECT at.tenant_id FROM audio_transcriptions at
             JOIN audio_chunks ac ON at.audio_chunk_id = ac.id
             WHERE at.id = ?1",
        )
        .bind(transcription_id)
        .fetch_optional(&mut **tx.conn())
        .await?;

        if let Some(tt) = trans_tenant {
            if tt != tenant.tenant_id && !tenant.is_system() {
                return Err(SqlxError::RowNotFound);
            }
        }

        // Verify speaker belongs to tenant
        let speaker_tenant: Option<String> =
            sqlx::query_scalar("SELECT tenant_id FROM speakers WHERE id = ?1")
                .bind(new_speaker_id)
                .fetch_optional(&mut **tx.conn())
                .await?;

        if let Some(st) = speaker_tenant {
            if st != tenant.tenant_id && !tenant.is_system() {
                return Err(SqlxError::RowNotFound);
            }
        }

        sqlx::query("UPDATE audio_transcriptions SET speaker_id = ?1 WHERE id = ?2")
            .bind(new_speaker_id)
            .bind(transcription_id)
            .execute(&mut **tx.conn())
            .await?;

        tx.commit().await?;
        Ok(())
    }

    async fn count_audit_logs(
        &self,
        tenant: &TenantContext,
        start_time: Option<DateTime<Utc>>,
        end_time: Option<DateTime<Utc>>,
    ) -> Result<i64, SqlxError> {
        let mut sql = String::from("SELECT COUNT(*) FROM audit_logs WHERE 1=1");

        if !tenant.is_system() {
            sql.push_str(" AND (tenant_id = ? OR tenant_id IS NULL)");
        }

        if start_time.is_some() {
            sql.push_str(" AND timestamp >= ?");
        }
        if end_time.is_some() {
            sql.push_str(" AND timestamp <= ?");
        }

        let mut query = sqlx::query_scalar::<_, i64>(&sql);

        if !tenant.is_system() {
            query = query.bind(&tenant.tenant_id);
        }

        if let Some(start) = start_time {
            query = query.bind(start);
        }
        if let Some(end) = end_time {
            query = query.bind(end);
        }

        let count = query.fetch_one(&self.pool).await?;
        Ok(count)
    }
}
