-- Create action_items table for automatic extraction from transcripts
CREATE TABLE IF NOT EXISTS action_items (
    id TEXT PRIMARY KEY,
    text TEXT NOT NULL,
    assignee TEXT,
    deadline TIMESTAMP,
    source TEXT NOT NULL DEFAULT 'meeting',
    source_id TEXT,
    confidence REAL NOT NULL DEFAULT 0.0,
    status TEXT NOT NULL DEFAULT 'pending',
    priority TEXT NOT NULL DEFAULT 'medium',
    created_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
    completed_at TIMESTAMP,
    metadata TEXT, -- JSON object for additional metadata
    tenant_id TEXT DEFAULT 'default'
);

-- Create indexes for common queries
CREATE INDEX IF NOT EXISTS idx_action_items_status ON action_items(status);
CREATE INDEX IF NOT EXISTS idx_action_items_source ON action_items(source);
CREATE INDEX IF NOT EXISTS idx_action_items_source_id ON action_items(source_id);
CREATE INDEX IF NOT EXISTS idx_action_items_assignee ON action_items(assignee);
CREATE INDEX IF NOT EXISTS idx_action_items_deadline ON action_items(deadline);
CREATE INDEX IF NOT EXISTS idx_action_items_created_at ON action_items(created_at);
CREATE INDEX IF NOT EXISTS idx_action_items_tenant_id ON action_items(tenant_id);

-- Create trigger to update updated_at timestamp
CREATE TRIGGER IF NOT EXISTS action_items_updated_at
AFTER UPDATE ON action_items
BEGIN
    UPDATE action_items SET updated_at = CURRENT_TIMESTAMP WHERE id = NEW.id;
END;
