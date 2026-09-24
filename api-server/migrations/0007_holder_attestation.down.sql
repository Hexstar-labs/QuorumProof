-- Rollback holder attestation fields
-- Issue #1571: Credential Holder Attestation

DROP TABLE IF EXISTS credential_attestations CASCADE;

ALTER TABLE IF EXISTS credentials DROP COLUMN IF EXISTS holder_attestation;
