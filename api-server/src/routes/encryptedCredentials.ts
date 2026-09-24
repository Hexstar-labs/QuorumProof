import { Router, Request, Response } from 'express';
import { ThresholdEncryption, type EncryptedData } from '../crypto/thresholdEncryption.js';
import { EncryptedCredentialStore } from '../services/encryptedCredentialStore.js';

/**
 * Encrypted Credentials Routes
 * Issue #1572: Implement Threshold Encryption for Sensitive Data
 *
 * Endpoints:
 * POST /api/credentials/:id/encrypt - Encrypt sensitive fields
 * GET /api/credentials/:id/encrypted - Get encrypted credential data
 * POST /api/credentials/:id/decrypt/authorize - Request decryption authorization
 * POST /api/credentials/:id/decrypt - Decrypt with authorization
 * GET /api/credentials/:id/decrypt/status - Check decryption authorization status
 */

export function createEncryptedCredentialsRouter() {
  const router = Router();
  const store = new EncryptedCredentialStore();
  const encryption = new ThresholdEncryption(3, 5); // 3-of-5 threshold encryption

  // POST /api/credentials/:id/encrypt
  // Encrypt sensitive fields of a credential
  router.post('/:id/encrypt', async (req: Request, res: Response) => {
    const { id } = req.params;
    const { sensitive_fields, keyCustodians } = req.body as {
      sensitive_fields?: Record<string, unknown>;
      keyCustodians?: string[];
    };

    if (!sensitive_fields || typeof sensitive_fields !== 'object') {
      res.status(400).json({
        error: 'Invalid request',
        message: 'sensitive_fields is required and must be an object',
      });
      return;
    }

    if (!keyCustodians || !Array.isArray(keyCustodians) || keyCustodians.length < encryption.getConfig().threshold) {
      res.status(400).json({
        error: 'Invalid key custodians',
        message: `At least ${encryption.getConfig().threshold} key custodians are required`,
      });
      return;
    }

    try {
      const credentialId = BigInt(id);
      const { encrypted, keyShares } = encryption.encryptSensitiveData(
        JSON.stringify(sensitive_fields)
      );

      // Store encrypted data
      await store.storeEncryptedCredential(credentialId, encrypted, keyShares, keyCustodians);

      res.json({
        success: true,
        message: 'Sensitive fields encrypted successfully',
        credential_id: id,
        encryption_info: {
          threshold: encryption.getConfig().threshold,
          shares_count: encryption.getConfig().shares,
          key_custodians: keyCustodians,
          share_metadata: encrypted.share_metadata,
        },
      });
    } catch (err: unknown) {
      const message = err instanceof Error ? err.message : String(err);
      res.status(500).json({
        error: 'Encryption failed',
        message,
      });
    }
  });

  // GET /api/credentials/:id/encrypted
  // Get encrypted credential data (metadata only, no key shares)
  router.get('/:id/encrypted', async (req: Request, res: Response) => {
    const { id } = req.params;

    try {
      const credentialId = BigInt(id);
      const encrypted = await store.getEncryptedCredential(credentialId);

      if (!encrypted) {
        res.status(404).json({
          error: 'Not found',
          message: 'No encrypted data found for this credential',
        });
        return;
      }

      res.json({
        credential_id: id,
        encryption_info: {
          threshold: encrypted.encrypted_data.threshold,
          shares_count: encrypted.encrypted_data.shares_count,
          key_custodians: encrypted.key_custodians,
          share_metadata: encrypted.encrypted_data.share_metadata,
        },
        created_at: new Date(encrypted.created_at).toISOString(),
        updated_at: new Date(encrypted.updated_at).toISOString(),
      });
    } catch (err: unknown) {
      const message = err instanceof Error ? err.message : String(err);
      res.status(500).json({
        error: 'Failed to retrieve encrypted data',
        message,
      });
    }
  });

  // POST /api/credentials/:id/decrypt/authorize
  // Request decryption authorization
  router.post('/:id/decrypt/authorize', async (req: Request, res: Response) => {
    const { id } = req.params;
    const { requester_address, reason } = req.body as {
      requester_address?: string;
      reason?: string;
    };

    if (!requester_address) {
      res.status(400).json({
        error: 'Invalid request',
        message: 'requester_address is required',
      });
      return;
    }

    try {
      const credentialId = BigInt(id);

      // Authorize decryption for 1 hour by default
      const authorization = await store.authorizeDecryption(
        credentialId,
        requester_address,
        3600000,
        reason || 'Not specified',
        [] // In production, require approval from multiple parties
      );

      res.json({
        success: true,
        message: 'Decryption authorization granted',
        credential_id: id,
        authorized_address: requester_address,
        expires_at: new Date(authorization.expires_at).toISOString(),
      });
    } catch (err: unknown) {
      const message = err instanceof Error ? err.message : String(err);
      res.status(500).json({
        error: 'Authorization failed',
        message,
      });
    }
  });

  // POST /api/credentials/:id/decrypt
  // Decrypt with key shares (requires authorization)
  router.post('/:id/decrypt', async (req: Request, res: Response) => {
    const { id } = req.params;
    const { requester_address, key_shares } = req.body as {
      requester_address?: string;
      key_shares?: unknown[];
    };

    if (!requester_address || !key_shares || !Array.isArray(key_shares)) {
      res.status(400).json({
        error: 'Invalid request',
        message: 'requester_address and key_shares array are required',
      });
      return;
    }

    try {
      const credentialId = BigInt(id);

      // Check authorization
      const isAuthorized = await store.isDecryptionAuthorized(credentialId, requester_address);
      if (!isAuthorized) {
        res.status(403).json({
          error: 'Not authorized',
          message: 'Decryption authorization not found or expired',
        });
        return;
      }

      // Get encrypted data
      const encrypted = await store.getEncryptedCredential(credentialId);
      if (!encrypted) {
        res.status(404).json({
          error: 'Not found',
          message: 'No encrypted data found for this credential',
        });
        return;
      }

      // Decrypt
      try {
        const decrypted = encryption.decryptSensitiveData(
          encrypted.encrypted_data,
          key_shares as any[]
        );

        // Log successful decryption
        await store.logDecryptionAttempt(credentialId, requester_address, true, {
          shared_count: key_shares.length,
        });

        res.json({
          success: true,
          message: 'Decryption successful',
          credential_id: id,
          decrypted_data: JSON.parse(decrypted.toString()),
          decrypted_at: new Date().toISOString(),
        });
      } catch (decryptErr: unknown) {
        // Log failed decryption
        await store.logDecryptionAttempt(credentialId, requester_address, false, {
          error: decryptErr instanceof Error ? decryptErr.message : String(decryptErr),
          share_count: key_shares.length,
        });

        res.status(400).json({
          error: 'Decryption failed',
          message: decryptErr instanceof Error ? decryptErr.message : 'Invalid key shares or data corruption',
        });
      }
    } catch (err: unknown) {
      const message = err instanceof Error ? err.message : String(err);
      res.status(500).json({
        error: 'Decryption request failed',
        message,
      });
    }
  });

  // GET /api/credentials/:id/decrypt/status
  // Check decryption authorization status
  router.get('/:id/decrypt/status', async (req: Request, res: Response) => {
    const { id } = req.params;
    const { requester_address } = req.query as { requester_address?: string };

    if (!requester_address) {
      res.status(400).json({
        error: 'Invalid request',
        message: 'requester_address query parameter is required',
      });
      return;
    }

    try {
      const credentialId = BigInt(id);

      const isAuthorized = await store.isDecryptionAuthorized(credentialId, requester_address as string);
      const authorizations = await store.getActiveAuthorizations(credentialId);

      res.json({
        credential_id: id,
        requester_address,
        authorized: isAuthorized,
        active_authorizations: authorizations
          .filter((a) => a.requester_address === requester_address)
          .map((a) => ({
            authorized_at: new Date(a.authorized_at).toISOString(),
            expires_at: new Date(a.expires_at).toISOString(),
            reason: a.reason,
          }))[0] || null,
      });
    } catch (err: unknown) {
      const message = err instanceof Error ? err.message : String(err);
      res.status(500).json({
        error: 'Failed to get authorization status',
        message,
      });
    }
  });

  // GET /api/credentials/:id/decrypt/audit
  // Get decryption audit log (admin only)
  router.get('/:id/decrypt/audit', async (req: Request, res: Response) => {
    const { id } = req.params;

    // TODO: Add authorization check for admin-only access

    try {
      const credentialId = BigInt(id);
      const authorizations = await store.getActiveAuthorizations(credentialId);

      res.json({
        credential_id: id,
        active_authorizations: authorizations.map((a) => ({
          requester_address: a.requester_address,
          authorized_at: new Date(a.authorized_at).toISOString(),
          expires_at: new Date(a.expires_at).toISOString(),
          reason: a.reason,
          approved_by: a.approved_by,
        })),
      });
    } catch (err: unknown) {
      const message = err instanceof Error ? err.message : String(err);
      res.status(500).json({
        error: 'Failed to get audit log',
        message,
      });
    }
  });

  return router;
}

export default createEncryptedCredentialsRouter();
