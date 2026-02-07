-- Add tenant_id column to speakers table for multi-tenancy support
ALTER TABLE speakers ADD COLUMN tenant_id TEXT;

-- Create index for efficient tenant-scoped speaker queries
CREATE INDEX idx_speakers_tenant ON speakers(tenant_id, id);
