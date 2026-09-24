-- Audit trail for credential amendments
-- Issue #1573: Add Audit Trail for Credential Amendments

-- Add change_history field to credentials table if not present
ALTER TABLE IF EXISTS credentials ADD COLUMN IF NOT EXISTS change_history JSONB DEFAULT NULL;

-- Create change history table to track all amendments
CREATE TABLE IF NOT EXISTS credential_change_history (
  id SERIAL PRIMARY KEY,
  change_id VARCHAR(255) UNIQUE NOT NULL,
  credential_id BIGINT NOT NULL,
  field_name VARCHAR(255) NOT NULL,
  old_value TEXT,
  new_value TEXT,
  changed_at BIGINT NOT NULL,
  changed_by VARCHAR(255) NOT NULL,
  change_reason TEXT,
  hash VARCHAR(64) NOT NULL, -- SHA-256 hash for tamper detection
  created_at BIGINT NOT NULL DEFAULT EXTRACT(EPOCH FROM NOW())::BIGINT * 1000
);

CREATE INDEX idx_change_history_credential_id ON credential_change_history(credential_id);
CREATE INDEX idx_change_history_field_name ON credential_change_history(field_name);
CREATE INDEX idx_change_history_changed_by ON credential_change_history(changed_by);
CREATE INDEX idx_change_history_changed_at ON credential_change_history(changed_at);
CREATE INDEX idx_change_history_hash ON credential_change_history(hash);

-- Audit summary table for quick integrity checks
CREATE TABLE IF NOT EXISTS audit_trail_summary (
  id SERIAL PRIMARY KEY,
  credential_id BIGINT UNIQUE NOT NULL,
  total_changes INTEGER DEFAULT 0,
  unique_changers INTEGER DEFAULT 0,
  last_change_at BIGINT,
  integrity_status VARCHAR(20) DEFAULT 'verified', -- 'verified' or 'tampered'
  last_verified_at BIGINT,
  anomalies_detected JSONB DEFAULT '[]'::jsonb,
  updated_at BIGINT NOT NULL DEFAULT EXTRACT(EPOCH FROM NOW())::BIGINT * 1000
);

CREATE INDEX idx_audit_summary_credential_id ON audit_trail_summary(credential_id);
CREATE INDEX idx_audit_summary_integrity_status ON audit_trail_summary(integrity_status);
