-- Add tenant_id to audio_chunks table (was missing from original migration)
ALTER TABLE audio_chunks ADD COLUMN tenant_id TEXT;

-- Add tenant_id to video_chunks table
ALTER TABLE video_chunks ADD COLUMN tenant_id TEXT;

-- Create composite indexes for tenant-scoped queries
CREATE INDEX IF NOT EXISTS idx_audio_chunks_tenant ON audio_chunks(tenant_id, timestamp);
CREATE INDEX IF NOT EXISTS idx_video_chunks_tenant ON video_chunks(tenant_id);
