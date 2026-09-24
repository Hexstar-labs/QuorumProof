-- Threshold encryption for sensitive credential data
-- Issue #1572: Implement Threshold Encryption for Sensitive Data

-- Store encrypted credential data
CREATE TABLE IF NOT EXISTS encrypted_credentials (
  id SERIAL PRIMARY KEY,
  credential_id BIGINT UNIQUE NOT NULL,
  encrypted_data JSONB NOT NULL,
  key_shares JSONB NOT NULL,
  key_custodians JSONB NOT NULL DEFAULT '[]'::jsonb,
  threshold INTEGER NOT NULL,
  created_at BIGINT NOT NULL DEFAULT EXTRACT(EPOCH FROM NOW())::BIGINT * 1000,
  updated_at BIGINT NOT NULL DEFAULT EXTRACT(EPOCH FROM NOW())::BIGINT * 1000
);

CREATE INDEX idx_encrypted_credentials_credential_id ON encrypted_credentials(credential_id);
CREATE INDEX idx_encrypted_credentials_created_at ON encrypted_credentials(created_at);

-- Track decryption authorizations
CREATE TABLE IF NOT EXISTS decryption_authorizations (
  id SERIAL PRIMARY KEY,
  credential_id BIGINT NOT NULL,
  requester_address VARCHAR(255) NOT NULL,
  authorized_at BIGINT NOT NULL,
  expires_at BIGINT NOT NULL,
  reason TEXT,
  approved_by JSONB NOT NULL DEFAULT '[]'::jsonb,
  created_at BIGINT NOT NULL DEFAULT EXTRACT(EPOCH FROM NOW())::BIGINT * 1000,
  UNIQUE (credential_id, requester_address)
);

CREATE INDEX idx_decryption_authorizations_credential_id ON decryption_authorizations(credential_id);
CREATE INDEX idx_decryption_authorizations_requester ON decryption_authorizations(requester_address);
CREATE INDEX idx_decryption_authorizations_expires_at ON decryption_authorizations(expires_at);

-- Audit log for decryption attempts
CREATE TABLE IF NOT EXISTS decryption_audit_log (
  id SERIAL PRIMARY KEY,
  credential_id BIGINT NOT NULL,
  requester_address VARCHAR(255) NOT NULL,
  success BOOLEAN NOT NULL,
  details JSONB,
  timestamp BIGINT NOT NULL
);

CREATE INDEX idx_decryption_audit_log_credential_id ON decryption_audit_log(credential_id);
CREATE INDEX idx_decryption_audit_log_requester ON decryption_audit_log(requester_address);
CREATE INDEX idx_decryption_audit_log_timestamp ON decryption_audit_log(timestamp);
