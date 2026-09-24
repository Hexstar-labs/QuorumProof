import { Router, Request, Response } from 'express';
import { AttestationManager } from '../crypto/attestation.js';
import { CredentialAttestationStore } from '../services/credentialAttestationStore.js';

/**
 * Credential Holder Attestation Routes
 * Issue #1571: Add Credential Holder Attestation
 *
 * Endpoints:
 * POST /api/credentials/:id/attestation/challenge - Request a signing challenge
 * POST /api/credentials/:id/attestation/verify - Verify a signature
 * GET /api/credentials/:id/attestation/status - Check attestation status
 */

export function createHolderAttestationRouter() {
  const router = Router();
  const attestationManager = new AttestationManager();
  const attestationStore = new CredentialAttestationStore();

  // POST /api/credentials/:id/attestation/challenge
  // Request a signing challenge for the credential holder
  router.post('/:id/attestation/challenge', async (req: Request, res: Response) => {
    const { id } = req.params;
    const { holder_address } = req.body as { holder_address?: string };

    if (!holder_address || typeof holder_address !== 'string') {
      res.status(400).json({
        error: 'Invalid request',
        message: 'holder_address is required and must be a string',
      });
      return;
    }

    // Validate Stellar address format
    if (!holder_address.startsWith('G') || holder_address.length !== 56) {
      res.status(400).json({
        error: 'Invalid address',
        message: 'holder_address must be a valid Stellar address',
      });
      return;
    }

    try {
      const credentialId = BigInt(id);
      const challenge = attestationManager.generateChallenge(id, holder_address);

      // Store challenge in database
      await attestationStore.storeChallenge(
        credentialId,
        holder_address,
        challenge.challenge_id,
        challenge.message,
        challenge.expires_at
      );

      res.json({
        challenge_id: challenge.challenge_id,
        message: challenge.message,
        timestamp: challenge.timestamp,
        expires_at: challenge.expires_at,
        instructions: 'Sign this message with your Stellar keypair and submit the signature to /verify',
      });
    } catch (err: unknown) {
      const message = err instanceof Error ? err.message : String(err);
      res.status(500).json({
        error: 'Failed to generate challenge',
        message,
      });
    }
  });

  // POST /api/credentials/:id/attestation/verify
  // Verify a holder's signature
  router.post('/:id/attestation/verify', async (req: Request, res: Response) => {
    const { id } = req.params;
    const { challenge_id, holder_address, signature } = req.body as {
      challenge_id?: string;
      holder_address?: string;
      signature?: string;
    };

    if (!challenge_id || !holder_address || !signature) {
      res.status(400).json({
        error: 'Invalid request',
        message: 'challenge_id, holder_address, and signature are required',
      });
      return;
    }

    try {
      const credentialId = BigInt(id);

      // Verify the signature using the attestation manager
      const isValid = attestationManager.verifySignature(
        challenge_id,
        holder_address,
        signature,
        id
      );

      if (!isValid) {
        res.status(400).json({
          error: 'Signature verification failed',
          message: 'The signature does not match the challenge or challenge has expired',
        });
        return;
      }

      // Store the successful attestation in database
      await attestationStore.storeAttestation(credentialId, holder_address, challenge_id, signature);

      res.json({
        success: true,
        message: 'Holder attestation verified successfully',
        credential_id: id,
        holder_address,
        verified_at: new Date().toISOString(),
      });
    } catch (err: unknown) {
      const message = err instanceof Error ? err.message : String(err);
      res.status(500).json({
        error: 'Verification failed',
        message,
      });
    }
  });

  // GET /api/credentials/:id/attestation/status
  // Check the attestation status for a holder
  router.get('/:id/attestation/status', async (req: Request, res: Response) => {
    const { id } = req.params;
    const { holder_address } = req.query as { holder_address?: string };

    if (!holder_address) {
      res.status(400).json({
        error: 'Invalid request',
        message: 'holder_address query parameter is required',
      });
      return;
    }

    try {
      const credentialId = BigInt(id);

      const attestation = await attestationStore.getLatestAttestation(credentialId, holder_address as string);

      if (!attestation) {
        res.json({
          status: 'no_attestation',
          message: 'No attestation exists for this credential and holder',
        });
        return;
      }

      const now = Date.now();
      const isExpired = attestation.expires_at < now;

      res.json({
        status: attestation.verified && !isExpired ? 'verified' : 'invalid',
        credential_id: id,
        holder_address,
        verified: attestation.verified && !isExpired,
        signed_at: attestation.signed_at ? new Date(attestation.signed_at).toISOString() : null,
        created_at: new Date(attestation.created_at).toISOString(),
        expires_at: new Date(attestation.expires_at).toISOString(),
      });
    } catch (err: unknown) {
      const message = err instanceof Error ? err.message : String(err);
      res.status(500).json({
        error: 'Failed to get attestation status',
        message,
      });
    }
  });

  return router;
}

export default createHolderAttestationRouter();
