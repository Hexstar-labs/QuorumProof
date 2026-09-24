import crypto from 'crypto';
import { Keypair, sign } from '@stellar/stellar-sdk';

/**
 * Credential Holder Attestation
 *
 * Implements a signing challenge protocol to prove that a holder
 * legitimately possesses a credential. The holder must:
 * 1. Request a signing challenge
 * 2. Sign the challenge with their Stellar keypair
 * 3. Submit the signature as proof of ownership
 *
 * This prevents unauthorized transfers or claims of credentials.
 */

export interface SigningChallenge {
  challenge_id: string;
  message: string;
  timestamp: number;
  expires_at: number;
}

export interface HolderAttestation {
  credential_id: string;
  holder_address: string;
  challenge_id: string;
  signature: string;
  signed_at: number;
  verified: boolean;
}

export class AttestationManager {
  private challengeStore: Map<string, { message: string; timestamp: number; holder: string }>;
  private attestationStore: Map<string, HolderAttestation>;
  private maxChallengeAge: number;

  constructor(maxChallengeAge: number = 600000) { // 10 minutes
    this.challengeStore = new Map();
    this.attestationStore = new Map();
    this.maxChallengeAge = maxChallengeAge;
    this.cleanupExpiredChallenges();
  }

  /**
   * Generate a signing challenge for a credential holder
   */
  generateChallenge(credentialId: string, holderAddress: string): SigningChallenge {
    const challengeId = crypto.randomBytes(32).toString('hex');
    const timestamp = Date.now();
    const message = this.buildChallengeMessage(credentialId, holderAddress, timestamp);

    this.challengeStore.set(challengeId, {
      message,
      timestamp,
      holder: holderAddress,
    });

    return {
      challenge_id: challengeId,
      message,
      timestamp,
      expires_at: timestamp + this.maxChallengeAge,
    };
  }

  /**
   * Build the message that holders must sign
   * Format: "Prove ownership of credential {credential_id} as {holder_address} at {timestamp}"
   */
  private buildChallengeMessage(
    credentialId: string,
    holderAddress: string,
    timestamp: number
  ): string {
    return `Prove ownership of credential ${credentialId} as ${holderAddress} at ${timestamp}`;
  }

  /**
   * Verify a holder's signature against a challenge
   * The signature must be a valid Stellar XDR-encoded signature
   */
  verifySignature(
    challengeId: string,
    holderAddress: string,
    signature: string,
    credentialId: string
  ): boolean {
    const challenge = this.challengeStore.get(challengeId);

    if (!challenge) {
      return false; // Challenge not found or expired
    }

    // Verify challenge hasn't expired
    if (Date.now() - challenge.timestamp > this.maxChallengeAge) {
      this.challengeStore.delete(challengeId);
      return false;
    }

    // Verify holder matches the challenge
    if (challenge.holder !== holderAddress) {
      return false;
    }

    // Verify the signature using Stellar SDK
    try {
      const keypair = Keypair.fromPublicKey(holderAddress);
      const messageBuffer = Buffer.from(challenge.message);
      const signatureBuffer = Buffer.from(signature, 'base64');

      // Use the Stellar SDK's verify function
      const isValid = keypair.verify(messageBuffer, signatureBuffer);

      if (isValid) {
        // Create attestation record
        const attestation: HolderAttestation = {
          credential_id: credentialId,
          holder_address: holderAddress,
          challenge_id: challengeId,
          signature,
          signed_at: Date.now(),
          verified: true,
        };

        const attestationId = `${credentialId}:${holderAddress}`;
        this.attestationStore.set(attestationId, attestation);

        // Remove challenge after successful verification
        this.challengeStore.delete(challengeId);
      }

      return isValid;
    } catch {
      return false;
    }
  }

  /**
   * Get a previously verified attestation
   */
  getAttestation(credentialId: string, holderAddress: string): HolderAttestation | null {
    const attestationId = `${credentialId}:${holderAddress}`;
    return this.attestationStore.get(attestationId) ?? null;
  }

  /**
   * Check if a holder has a valid, verified attestation for a credential
   */
  hasValidAttestation(credentialId: string, holderAddress: string): boolean {
    const attestation = this.getAttestation(credentialId, holderAddress);
    return attestation !== null && attestation.verified;
  }

  /**
   * Revoke an attestation (mark it as invalid)
   */
  revokeAttestation(credentialId: string, holderAddress: string): void {
    const attestationId = `${credentialId}:${holderAddress}`;
    const attestation = this.attestationStore.get(attestationId);
    if (attestation) {
      attestation.verified = false;
    }
  }

  /**
   * Cleanup expired challenges periodically
   */
  private cleanupExpiredChallenges(): void {
    setInterval(() => {
      const now = Date.now();
      const expired: string[] = [];

      for (const [id, challenge] of this.challengeStore.entries()) {
        if (now - challenge.timestamp > this.maxChallengeAge) {
          expired.push(id);
        }
      }

      for (const id of expired) {
        this.challengeStore.delete(id);
      }
    }, 60000); // Cleanup every minute
  }
}

/**
 * Utility to create a signing challenge message that the holder should sign
 */
export function createSigningMessage(
  credentialId: string,
  holderAddress: string,
  timestamp: number
): string {
  return `Prove ownership of credential ${credentialId} as ${holderAddress} at ${timestamp}`;
}
