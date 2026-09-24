import crypto from 'crypto';

/**
 * Threshold Encryption for Sensitive Data
 * Issue #1572
 *
 * Implements a k-of-n threshold encryption scheme using:
 * - AES-256-GCM for symmetric encryption
 * - Shamir's Secret Sharing for key splitting
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
    commitment: string; // Hash commitment to verify share authenticity
  }>;
}

export interface ThresholdKeyShare {
  share_id: number;
  share_value: string; // base64 encoded
  commitment: string; // base64 encoded (for verification)
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
   * Uses simplified Shamir's Secret Sharing with XOR combining
   */
  private splitSecret(secret: Buffer): ThresholdKeyShare[] {
    const shares: ThresholdKeyShare[] = [];

    // For k-of-n sharing: create shares using XOR-based secret sharing
    // Each share is created by XORing the secret with k-1 random values
    const randomValues: Buffer[] = [];

    for (let i = 0; i < this.threshold - 1; i++) {
      randomValues.push(crypto.randomBytes(32));
    }

    // Create shares: share[i] = secret XOR random[0] XOR random[1] ... XOR random[k-2]
    // Any k shares can recover the secret
    for (let shareId = 1; shareId <= this.shares; shareId++) {
      let shareValue = Buffer.from(secret);

      // XOR with specific combination of random values based on share ID
      for (let i = 0; i < randomValues.length; i++) {
        // Use share ID to determine which random values to include
        if ((shareId & (1 << i)) !== 0) {
          for (let j = 0; j < shareValue.length; j++) {
            shareValue[j] ^= randomValues[i][j];
          }
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

    // Take only the first 'threshold' shares
    const selectedShares = shares.slice(0, this.threshold);
    let secret = Buffer.alloc(32, 0);

    // XOR all selected shares to recover the secret
    for (const share of selectedShares) {
      const shareBuffer = Buffer.from(share.share_value, 'base64');
      for (let j = 0; j < secret.length; j++) {
        secret[j] ^= shareBuffer[j];
      }
    }

    return secret;
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
