-- Create comprehensive audit logs table
CREATE TABLE IF NOT EXISTS audit_logs (
    id TEXT PRIMARY KEY,
    tenant_id TEXT,
    user_id TEXT,
    action TEXT NOT NULL,  -- CREATE, READ, UPDATE, DELETE, EXPORT, LOGIN, LOGOUT
    resource_type TEXT NOT NULL,  -- frame, transcription, user, etc
    resource_id TEXT,
    details TEXT,  -- JSON with additional details
    ip_address TEXT,
    user_agent TEXT,
    timestamp DATETIME DEFAULT CURRENT_TIMESTAMP
);

-- Create indexes for common audit log queries
CREATE INDEX IF NOT EXISTS idx_audit_logs_tenant ON audit_logs(tenant_id, timestamp);
CREATE INDEX IF NOT EXISTS idx_audit_logs_user ON audit_logs(user_id, timestamp);
CREATE INDEX IF NOT EXISTS idx_audit_logs_action ON audit_logs(action, timestamp);
CREATE INDEX IF NOT EXISTS idx_audit_logs_resource ON audit_logs(resource_type, resource_id);
CREATE INDEX IF NOT EXISTS idx_audit_logs_timestamp ON audit_logs(timestamp);

-- Retention trigger: delete logs older than 1 year
CREATE TRIGGER IF NOT EXISTS audit_log_retention
AFTER INSERT ON audit_logs
BEGIN
    DELETE FROM audit_logs
    WHERE timestamp < datetime('now', '-1 year');
END;
