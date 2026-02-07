-- Migration: Create playbooks table for automation rules
-- Created: 2026-02-09

-- Playbooks table stores automation rules with triggers and actions
CREATE TABLE IF NOT EXISTS playbooks (
    id TEXT PRIMARY KEY,
    tenant_id TEXT,
    name TEXT NOT NULL,
    description TEXT,
    enabled BOOLEAN NOT NULL DEFAULT 0,
    triggers_json TEXT NOT NULL,
    actions_json TEXT NOT NULL,
    cooldown_minutes INTEGER,
    max_executions_per_day INTEGER,
    created_at DATETIME DEFAULT CURRENT_TIMESTAMP,
    updated_at DATETIME DEFAULT CURRENT_TIMESTAMP,
    is_builtin BOOLEAN DEFAULT 0,
    icon TEXT,
    color TEXT,
    FOREIGN KEY (tenant_id) REFERENCES tenants(id) ON DELETE CASCADE
);

-- Index for tenant-scoped queries
CREATE INDEX IF NOT EXISTS idx_playbooks_tenant ON playbooks(tenant_id);

-- Index for enabled playbooks (for engine queries)
CREATE INDEX IF NOT EXISTS idx_playbooks_enabled ON playbooks(enabled);

-- Index for built-in playbooks
CREATE INDEX IF NOT EXISTS idx_playbooks_builtin ON playbooks(is_builtin);

-- Playbook executions table for audit and history
CREATE TABLE IF NOT EXISTS playbook_executions (
    id TEXT PRIMARY KEY,
    playbook_id TEXT NOT NULL,
    tenant_id TEXT,
    started_at DATETIME NOT NULL,
    completed_at DATETIME,
    status TEXT NOT NULL, -- running, completed, failed, cancelled
    triggered_by TEXT NOT NULL, -- JSON of the trigger that activated
    action_results TEXT NOT NULL, -- JSON array of action results
    error TEXT,
    created_at DATETIME DEFAULT CURRENT_TIMESTAMP,
    FOREIGN KEY (playbook_id) REFERENCES playbooks(id) ON DELETE CASCADE,
    FOREIGN KEY (tenant_id) REFERENCES tenants(id) ON DELETE CASCADE
);

-- Index for playbook execution history
CREATE INDEX IF NOT EXISTS idx_playbook_executions_playbook ON playbook_executions(playbook_id);

-- Index for tenant-scoped execution queries
CREATE INDEX IF NOT EXISTS idx_playbook_executions_tenant ON playbook_executions(tenant_id);

-- Index for recent executions
CREATE INDEX IF NOT EXISTS idx_playbook_executions_started ON playbook_executions(started_at DESC);

-- Trigger to update updated_at timestamp on playbook updates
CREATE TRIGGER IF NOT EXISTS update_playbooks_timestamp
AFTER UPDATE ON playbooks
BEGIN
    UPDATE playbooks SET updated_at = CURRENT_TIMESTAMP WHERE id = NEW.id;
END;
