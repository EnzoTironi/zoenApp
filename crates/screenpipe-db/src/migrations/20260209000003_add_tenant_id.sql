-- Multi-tenancy migration: Add tenant_id columns to main tables
-- This enables data isolation between different tenants

-- Add tenant_id to frames table
ALTER TABLE frames ADD COLUMN tenant_id TEXT;

-- Add tenant_id to audio_transcriptions table
ALTER TABLE audio_transcriptions ADD COLUMN tenant_id TEXT;

-- Add tenant_id to ocr_text table
ALTER TABLE ocr_text ADD COLUMN tenant_id TEXT;

-- Add tenant_id to ui_events table (for input modality)
ALTER TABLE ui_events ADD COLUMN tenant_id TEXT;

-- Add tenant_id to ui_monitoring table (for accessibility text)
ALTER TABLE ui_monitoring ADD COLUMN tenant_id TEXT;

-- Create indexes for efficient tenant-scoped queries
CREATE INDEX IF NOT EXISTS idx_frames_tenant ON frames(tenant_id, timestamp);
CREATE INDEX IF NOT EXISTS idx_audio_tenant ON audio_transcriptions(tenant_id, timestamp);
CREATE INDEX IF NOT EXISTS idx_ocr_text_tenant ON ocr_text(tenant_id);
CREATE INDEX IF NOT EXISTS idx_ui_events_tenant ON ui_events(tenant_id, timestamp);
CREATE INDEX IF NOT EXISTS idx_ui_monitoring_tenant ON ui_monitoring(tenant_id, timestamp);

-- Tenant metadata table (optional, for future use)
CREATE TABLE tenant_metadata (
    tenant_id TEXT PRIMARY KEY,
    name TEXT,
    created_at DATETIME DEFAULT CURRENT_TIMESTAMP,
    updated_at DATETIME DEFAULT CURRENT_TIMESTAMP,
    settings TEXT -- JSON blob for tenant-specific settings
);

CREATE INDEX idx_tenant_metadata_created ON tenant_metadata(created_at);
