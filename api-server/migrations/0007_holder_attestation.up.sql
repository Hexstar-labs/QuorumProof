-- Add holder attestation fields to credentials table
-- Issue #1571: Credential Holder Attestation

ALTER TABLE IF EXISTS credentials ADD COLUMN IF NOT EXISTS holder_attestation JSONB DEFAULT NULL;

-- Create attestations table to store signing challenges and verification records
CREATE TABLE IF NOT EXISTS credential_attestations (
  id SERIAL PRIMARY KEY,
  credential_id BIGINT NOT NULL,
  holder_address VARCHAR(255) NOT NULL,
  challenge_id VARCHAR(255) UNIQUE NOT NULL,
  challenge_message TEXT NOT NULL,
  signature VARCHAR(1024),
  signed_at BIGINT,
  verified BOOLEAN DEFAULT FALSE,
  created_at BIGINT NOT NULL DEFAULT EXTRACT(EPOCH FROM NOW())::BIGINT * 1000,
  expires_at BIGINT NOT NULL,
  CONSTRAINT fk_credential_attestations_holder_address UNIQUE (credential_id, holder_address)
);

CREATE INDEX idx_credential_attestations_credential_id ON credential_attestations(credential_id);
CREATE INDEX idx_credential_attestations_holder ON credential_attestations(holder_address);
CREATE INDEX idx_credential_attestations_challenge_id ON credential_attestations(challenge_id);
CREATE INDEX idx_credential_attestations_expires_at ON credential_attestations(expires_at);
