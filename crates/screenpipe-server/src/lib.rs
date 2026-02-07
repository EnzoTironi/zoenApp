mod add;
pub mod analytics;
#[cfg(test)]
pub mod test_utils;
#[cfg(feature = "apple-intelligence")]
mod apple_intelligence_api;
pub mod audit;
pub mod auth;
mod auto_destruct;
pub mod chunking;
pub mod cli;
pub mod cloud_search;
pub mod core;
pub mod filtering;
pub mod pipe_manager;
pub mod playbook_manager;
mod resource_monitor;
mod server;
pub mod sleep_monitor;
mod sync_api;
pub mod sync_provider;
pub mod text_embeds;
pub mod ui_events_api;
pub mod ui_recorder;
mod video;
pub mod video_cache;
pub mod video_utils;
pub mod vision_manager;
pub use add::handle_index_command;
pub use audit::{Action, AuditContext, AuditError, AuditService, Resource};
pub use auth::{
    auth_middleware_with_state, generate_jwt, generate_jwt_with_tenant, get_tenant_from_request,
    is_auth_enabled, tenant_middleware, validate_api_key, validate_jwt, AuthenticatedUser, Role,
};
pub use auto_destruct::watch_pid;
pub use axum::Json as JsonResponse;
pub use cli::Cli;
pub use core::{record_video, start_continuous_recording};
pub use pipe_manager::PipeManager;
pub use playbook_manager::PlaybookManager;
pub use resource_monitor::{ResourceMonitor, RestartSignal};
pub use screenpipe_core::Language;
pub use server::health_check;
pub use server::AppState;
pub use server::ContentItem;
pub use server::HealthCheckResponse;
pub use server::PaginatedResponse;
pub use server::SCServer;
pub use server::{api_list_monitors, MonitorInfo};
pub use sleep_monitor::start_sleep_monitor;
pub use video::{
    video_quality_to_crf, video_quality_to_jpeg_q, video_quality_to_preset, FrameWriteInfo,
    FrameWriteTracker, VideoCapture,
};
pub mod embedding;
pub use cloud_search::{CloudSearchClient, CloudSearchMetadata, CloudStatus};
pub use ui_recorder::{start_ui_recording, UiRecorderConfig, UiRecorderHandle};
