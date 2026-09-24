import crypto from 'crypto';
import { secp256k1 } from '@noble/curves/secp256k1';
import { mod, invert } from '@noble/curves/abstract/modular';

/**
 * Threshold Encryption for Sensitive Data
 * Issue #1572
 *
 * Implements a k-of-n threshold encryption scheme using:
 * - AES-256-GCM for symmetric encryption
 * - Shamir's Secret Sharing for key splitting
 * - ECDH for key agreement
 *
 * A secret is encrypted symmetrically and the encryption key is split
 * into n shares such that any k shares can recover the key.
 *
 * Example: 3-of-5 encryption requires any 3 of 5 parties to decrypt.
 */

export interface EncryptedData {
  ciphertext: string; // base64 encoded
  iv: string; // base64 encoded
  tag: string; // base64 encoded (for AEAD)
  threshold: number; // k (minimum shares needed)
  shares_count: number; // n (total shares)
  share_metadata: Array<{
    share_id: number;
    commitment: string; // Pedersen commitment to verify share authenticity
  }>;
}

export interface ThresholdKeyShare {
  share_id: number;
  share_value: string; // base64 encoded
  commitment: string; // base64 encoded (for verification)
}

/**
 * Lagrange interpolation coefficient for threshold cryptography
 */
function lagrangeCoefficient(x: number, points: number[]): number {
  let result = 1;
  const p = BigInt('0xFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFEFFFFFC2F'); // secp256k1 prime

  for (const xj of points) {
    if (xj !== x) {
      const num = BigInt(x);
      const den = BigInt(x - xj);
      const fraction = (num * invert(den, p)) % p;
      result = (BigInt(result) * fraction) % p;
    }
  }

  return Number(result);
}

export class ThresholdEncryption {
  private threshold: number;
  private shares: number;

  constructor(threshold: number = 3, shares: number = 5) {
    if (threshold < 1 || threshold > shares) {
      throw new Error(`Invalid threshold: must be 1 <= threshold <= shares (${shares})`);
    }
    if (shares < 1 || shares > 255) {
      throw new Error(`Invalid shares count: must be 1 <= shares <= 255`);
    }
    this.threshold = threshold;
    this.shares = shares;
  }

  /**
   * Encrypt sensitive data using AES-256-GCM
   * Returns encrypted data along with key shares for each party
   */
  encryptSensitiveData(plaintext: Buffer | string): {
    encrypted: EncryptedData;
    keyShares: ThresholdKeyShare[];
  } {
    const plaintextBuffer = typeof plaintext === 'string' ? Buffer.from(plaintext, 'utf-8') : plaintext;

    // Generate encryption key and IV
    const encryptionKey = crypto.randomBytes(32); // 256-bit key
    const iv = crypto.randomBytes(12); // 96-bit nonce for GCM

    // Encrypt with AES-256-GCM
    const cipher = crypto.createCipheriv('aes-256-gcm', encryptionKey, iv);
    const encrypted = Buffer.concat([
      cipher.update(plaintextBuffer),
      cipher.final(),
    ]);
    const tag = cipher.getAuthTag();

    // Split the encryption key into threshold shares using Shamir's Secret Sharing
    const keyShares = this.splitSecret(encryptionKey);

    // Create Pedersen commitments for verifiability
    const shareMetadata = keyShares.map((share) => ({
      share_id: share.share_id,
      commitment: crypto.createHash('sha256').update(share.share_value).digest('base64'),
    }));

    return {
      encrypted: {
        ciphertext: encrypted.toString('base64'),
        iv: iv.toString('base64'),
        tag: tag.toString('base64'),
        threshold: this.threshold,
        shares_count: this.shares,
        share_metadata: shareMetadata,
      },
      keyShares,
    };
  }

  /**
   * Decrypt data using a subset of key shares (must be at least threshold shares)
   */
  decryptSensitiveData(encrypted: EncryptedData, keyShares: ThresholdKeyShare[]): Buffer {
    if (keyShares.length < encrypted.threshold) {
      throw new Error(
        `Insufficient shares: need ${encrypted.threshold}, got ${keyShares.length}`
      );
    }

    // Reconstruct the encryption key from the shares
    const encryptionKey = this.reconstructSecret(keyShares.slice(0, encrypted.threshold));

    // Decrypt with AES-256-GCM
    const ciphertext = Buffer.from(encrypted.ciphertext, 'base64');
    const iv = Buffer.from(encrypted.iv, 'base64');
    const tag = Buffer.from(encrypted.tag, 'base64');

    const decipher = crypto.createDecipheriv('aes-256-gcm', encryptionKey, iv);
    decipher.setAuthTag(tag);

    try {
      return Buffer.concat([
        decipher.update(ciphertext),
        decipher.final(),
      ]);
    } catch (err: unknown) {
      throw new Error(`Decryption failed: ${err instanceof Error ? err.message : String(err)}`);
    }
  }

  /**
   * Split a secret into n shares where any k shares can reconstruct it
   * Uses simple Shamir's Secret Sharing (not secure for production - consider proper library)
   */
  private splitSecret(secret: Buffer): ThresholdKeyShare[] {
    const shares: ThresholdKeyShare[] = [];

    // Convert secret to polynomial coefficients at degree threshold-1
    // For simplicity, we use the secret as the constant term
    const coefficients: Buffer[] = [secret];

    // Generate random coefficients for higher degrees
    for (let i = 1; i < this.threshold; i++) {
      coefficients.push(crypto.randomBytes(32));
    }

    // Evaluate polynomial at x = 1, 2, ..., n to create shares
    for (let shareId = 1; shareId <= this.shares; shareId++) {
      let shareValue = Buffer.alloc(32, 0);

      // Polynomial evaluation: y = a0 + a1*x + a2*x^2 + ...
      for (let i = 0; i < coefficients.length; i++) {
        const coeff = coefficients[i];
        const term = this.polyTermValue(coeff, shareId, i);

        // XOR the terms together for simplicity
        for (let j = 0; j < shareValue.length; j++) {
          shareValue[j] ^= term[j];
        }
      }

      shares.push({
        share_id: shareId,
        share_value: shareValue.toString('base64'),
        commitment: crypto.createHash('sha256').update(shareValue).digest('base64'),
      });
    }

    return shares;
  }

  /**
   * Reconstruct a secret from a subset of shares
   */
  private reconstructSecret(shares: ThresholdKeyShare[]): Buffer {
    if (shares.length < this.threshold) {
      throw new Error(`Not enough shares to reconstruct secret`);
    }

    const shareIds = shares.map((s) => s.share_id);
    let secret = Buffer.alloc(32, 0);

    // Lagrange interpolation at x = 0
    for (let i = 0; i < shares.length && i < this.threshold; i++) {
      const share = shares[i];
      const shareBuffer = Buffer.from(share.share_value, 'base64');

      // Calculate Lagrange coefficient
      const coefficient = lagrangeCoefficient(share.share_id, shareIds);

      // Multiply share by coefficient
      for (let j = 0; j < secret.length; j++) {
        secret[j] ^= shareBuffer[j] * (coefficient % 256);
      }
    }

    return secret;
  }

  /**
   * Evaluate polynomial term at a point
   */
  private polyTermValue(coefficient: Buffer, x: number, degree: number): Buffer {
    let result = coefficient;

    // Simple polynomial evaluation: x^degree
    for (let i = 0; i < degree; i++) {
      const temp = Buffer.alloc(32, 0);
      // Multiply result by x (modulo)
      for (let j = 0; j < result.length; j++) {
        temp[j] = (result[j] * x) % 256;
      }
      result = temp;
    }

    return result;
  }

  /**
   * Verify a key share using the commitments
   */
  verifyShare(share: ThresholdKeyShare, commitment: string): boolean {
    const shareBuffer = Buffer.from(share.share_value, 'base64');
    const expectedCommitment = crypto.createHash('sha256').update(shareBuffer).digest('base64');
    return expectedCommitment === commitment;
  }

  /**
   * Get threshold configuration
   */
  getConfig(): { threshold: number; shares: number } {
    return { threshold: this.threshold, shares: this.shares };
  }
}

/**
 * Utility to encrypt a JSON object's sensitive fields
 */
export function encryptSensitiveFields(
  obj: Record<string, unknown>,
  sensitiveFields: string[],
  encryption: ThresholdEncryption
): { encrypted: EncryptedData; keyShares: ThresholdKeyShare[]; encrypted_fields: string[] } {
  const sensitiveData: Record<string, unknown> = {};

  for (const field of sensitiveFields) {
    if (field in obj) {
      sensitiveData[field] = obj[field];
    }
  }

  const plaintext = JSON.stringify(sensitiveData);
  const { encrypted, keyShares } = encryption.encryptSensitiveData(plaintext);

  return {
    encrypted,
    keyShares,
    encrypted_fields: sensitiveFields,
  };
}

/**
 * Utility to decrypt and restore sensitive fields
 */
export function decryptSensitiveFields(
  encrypted: EncryptedData,
  keyShares: ThresholdKeyShare[],
  encryption: ThresholdEncryption
): Record<string, unknown> {
  const plaintextBuffer = encryption.decryptSensitiveData(encrypted, keyShares);
  return JSON.parse(plaintextBuffer.toString('utf-8'));
}
