-- Rollback audit trail tables
-- Issue #1573: Add Audit Trail for Credential Amendments

DROP TABLE IF EXISTS audit_trail_summary CASCADE;
DROP TABLE IF EXISTS credential_change_history CASCADE;

ALTER TABLE IF EXISTS credentials DROP COLUMN IF EXISTS change_history;
