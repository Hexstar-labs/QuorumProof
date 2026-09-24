-- Rollback threshold encryption tables
-- Issue #1572: Implement Threshold Encryption for Sensitive Data

DROP TABLE IF EXISTS decryption_audit_log CASCADE;
DROP TABLE IF EXISTS decryption_authorizations CASCADE;
DROP TABLE IF EXISTS encrypted_credentials CASCADE;
