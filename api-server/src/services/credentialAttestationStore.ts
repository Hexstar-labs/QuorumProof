import { getPool } from '../db.js';
import type { HolderAttestation } from '../crypto/attestation.js';

/**
 * Credential Attestation Store
 * Manages database persistence of credential holder attestations
 */

export interface CredentialAttestationRecord {
  id: number;
  credential_id: bigint;
  holder_address: string;
  challenge_id: string;
  challenge_message: string;
  signature: string | null;
  signed_at: number | null;
  verified: boolean;
  created_at: number;
  expires_at: number;
}

export class CredentialAttestationStore {
  /**
   * Store a new challenge for a credential holder
   */
  async storeChallenge(
    credentialId: bigint,
    holderAddress: string,
    challengeId: string,
    message: string,
    expiresAt: number
  ): Promise<CredentialAttestationRecord> {
    const pool = getPool();
    const now = Date.now();

    const query = `
      INSERT INTO credential_attestations (
        credential_id,
        holder_address,
        challenge_id,
        challenge_message,
        created_at,
        expires_at,
        verified
      ) VALUES ($1, $2, $3, $4, $5, $6, $7)
      ON CONFLICT (credential_id, holder_address)
      DO UPDATE SET
        challenge_id = $3,
        challenge_message = $4,
        created_at = $5,
        expires_at = $6,
        verified = FALSE,
        signature = NULL,
        signed_at = NULL
      RETURNING *;
    `;

    const result = await pool.query<CredentialAttestationRecord>(query, [
      credentialId,
      holderAddress,
      challengeId,
      message,
      now,
      expiresAt,
      false,
    ]);

    return result.rows[0];
  }

  /**
   * Store a signed attestation
   */
  async storeAttestation(
    credentialId: bigint,
    holderAddress: string,
    challengeId: string,
    signature: string
  ): Promise<CredentialAttestationRecord> {
    const pool = getPool();
    const now = Date.now();

    const query = `
      UPDATE credential_attestations
      SET
        signature = $3,
        signed_at = $4,
        verified = TRUE
      WHERE credential_id = $1 AND holder_address = $2 AND challenge_id = $5
      RETURNING *;
    `;

    const result = await pool.query<CredentialAttestationRecord>(query, [
      credentialId,
      holderAddress,
      signature,
      now,
      challengeId,
    ]);

    return result.rows[0];
  }

  /**
   * Get the latest attestation for a credential holder
   */
  async getLatestAttestation(
    credentialId: bigint,
    holderAddress: string
  ): Promise<CredentialAttestationRecord | null> {
    const pool = getPool();

    const query = `
      SELECT *
      FROM credential_attestations
      WHERE credential_id = $1 AND holder_address = $2
      ORDER BY created_at DESC
      LIMIT 1;
    `;

    const result = await pool.query<CredentialAttestationRecord>(query, [
      credentialId,
      holderAddress,
    ]);

    return result.rows[0] || null;
  }

  /**
   * Get a specific challenge
   */
  async getChallenge(challengeId: string): Promise<CredentialAttestationRecord | null> {
    const pool = getPool();

    const query = `
      SELECT *
      FROM credential_attestations
      WHERE challenge_id = $1;
    `;

    const result = await pool.query<CredentialAttestationRecord>(query, [challengeId]);

    return result.rows[0] || null;
  }

  /**
   * Check if a holder has a valid, verified attestation
   */
  async hasValidAttestation(
    credentialId: bigint,
    holderAddress: string
  ): Promise<boolean> {
    const pool = getPool();
    const now = Date.now();

    const query = `
      SELECT 1
      FROM credential_attestations
      WHERE credential_id = $1
        AND holder_address = $2
        AND verified = TRUE
        AND (expires_at IS NULL OR expires_at > $3)
      LIMIT 1;
    `;

    const result = await pool.query<{ one: number }>(query, [
      credentialId,
      holderAddress,
      now,
    ]);

    return result.rows.length > 0;
  }

  /**
   * Revoke an attestation
   */
  async revokeAttestation(
    credentialId: bigint,
    holderAddress: string
  ): Promise<CredentialAttestationRecord | null> {
    const pool = getPool();

    const query = `
      UPDATE credential_attestations
      SET verified = FALSE
      WHERE credential_id = $1 AND holder_address = $2
      RETURNING *;
    `;

    const result = await pool.query<CredentialAttestationRecord>(query, [
      credentialId,
      holderAddress,
    ]);

    return result.rows[0] || null;
  }

  /**
   * Cleanup expired attestations
   */
  async cleanupExpiredAttestations(): Promise<number> {
    const pool = getPool();
    const now = Date.now();

    const query = `
      DELETE FROM credential_attestations
      WHERE expires_at < $1;
    `;

    const result = await pool.query(query, [now]);

    return result.rowCount || 0;
  }

  /**
   * Get attestations for a credential
   */
  async getCredentialAttestations(
    credentialId: bigint
  ): Promise<CredentialAttestationRecord[]> {
    const pool = getPool();

    const query = `
      SELECT *
      FROM credential_attestations
      WHERE credential_id = $1
      ORDER BY created_at DESC;
    `;

    const result = await pool.query<CredentialAttestationRecord>(query, [credentialId]);

    return result.rows;
  }
}
