import crypto from 'crypto';

/**
 * Proof-of-Work Rate Limiting
 *
 * Implements a Hashcash-style PoW mechanism to prevent distributed attacks by
 * requiring clients to perform computational work before accepting requests.
 *
 * Algorithm:
 * - Client receives a challenge: { nonce, target, timestamp }
 * - Client must find a nonce_solution such that:
 *   SHA256(challenge_data + nonce_solution) has leading_zeros >= target
 * - Server verifies the solution and adjusts difficulty based on load
 *
 * Difficulty Adjustment:
 * - difficulty (target zeros) ranges from 1 to 32
 * - Increases when request rate exceeds threshold
 * - Decreases gradually to keep system responsive during normal load
 * - Estimated work: 2^difficulty hash operations
 */

export interface PoWChallenge {
  nonce: string;
  target: number;
  timestamp: number;
  expires_at: number;
}

export interface PoWSolution {
  challenge_nonce: string;
  nonce_solution: string;
}

export class PoWManager {
  private difficulty: number;
  private challengeCache: Map<string, { timestamp: number; target: number }>;
  private maxDifficulty: number;
  private minDifficulty: number;
  private adjustmentInterval: number;
  private lastAdjustmentTime: number;
  private requestCountWindow: number;
  private requestTimestamps: number[];

  constructor(
    initialDifficulty: number = 2,
    maxDifficulty: number = 20,
    minDifficulty: number = 1,
    adjustmentInterval: number = 60000
  ) {
    this.difficulty = initialDifficulty;
    this.maxDifficulty = maxDifficulty;
    this.minDifficulty = minDifficulty;
    this.adjustmentInterval = adjustmentInterval;
    this.lastAdjustmentTime = Date.now();
    this.challengeCache = new Map();
    this.requestCountWindow = 60000; // 60 second window
    this.requestTimestamps = [];
    this.cleanupChallenges();
  }

  /**
   * Generate a new PoW challenge for a client
   */
  generateChallenge(): PoWChallenge {
    const nonce = crypto.randomBytes(16).toString('hex');
    const timestamp = Date.now();
    const expiresAt = timestamp + 300000; // 5 minute expiry

    this.challengeCache.set(nonce, { timestamp, target: this.difficulty });

    return {
      nonce,
      target: this.difficulty,
      timestamp,
      expires_at: expiresAt,
    };
  }

  /**
   * Verify a PoW solution from a client
   */
  verifySolution(solution: PoWSolution): boolean {
    const challenge = this.challengeCache.get(solution.challenge_nonce);

    if (!challenge) {
      return false; // Challenge not found or expired
    }

    const now = Date.now();
    if (now - challenge.timestamp > 300000) {
      this.challengeCache.delete(solution.challenge_nonce);
      return false; // Challenge expired
    }

    // Verify the PoW: check if hash has required leading zeros
    const data = solution.challenge_nonce + solution.nonce_solution;
    const hash = crypto.createHash('sha256').update(data).digest();

    if (!this.hasLeadingZeros(hash, challenge.target)) {
      return false;
    }

    // Solution is valid, remove challenge to prevent replay
    this.challengeCache.delete(solution.challenge_nonce);
    return true;
  }

  /**
   * Check if a hash has the required number of leading zero bits
   */
  private hasLeadingZeros(hash: Buffer, targetZeros: number): boolean {
    let zeros = 0;
    for (let i = 0; i < hash.length; i++) {
      for (let j = 7; j >= 0; j--) {
        if ((hash[i] & (1 << j)) === 0) {
          zeros++;
        } else {
          return zeros >= targetZeros;
        }
      }
    }
    return zeros >= targetZeros;
  }

  /**
   * Adjust difficulty based on request load
   * Called periodically to adapt to system load
   */
  adjustDifficulty(requestsPerSecond: number, targetRps: number = 10): void {
    const now = Date.now();
    if (now - this.lastAdjustmentTime < this.adjustmentInterval) {
      return;
    }

    this.lastAdjustmentTime = now;

    if (requestsPerSecond > targetRps * 1.5) {
      // Increase difficulty if under heavy load
      this.difficulty = Math.min(this.difficulty + 1, this.maxDifficulty);
    } else if (requestsPerSecond < targetRps * 0.5) {
      // Decrease difficulty gradually if light load
      this.difficulty = Math.max(this.difficulty - 1, this.minDifficulty);
    }
  }

  /**
   * Track request timestamp for rate calculation
   */
  trackRequest(): void {
    const now = Date.now();
    this.requestTimestamps.push(now);

    // Remove timestamps outside the window
    const windowStart = now - this.requestCountWindow;
    this.requestTimestamps = this.requestTimestamps.filter((t) => t >= windowStart);
  }

  /**
   * Get current request rate (requests per second)
   */
  getRequestRate(): number {
    const now = Date.now();
    const windowStart = now - this.requestCountWindow;
    const recentRequests = this.requestTimestamps.filter((t) => t >= windowStart).length;
    return recentRequests / (this.requestCountWindow / 1000);
  }

  /**
   * Get current difficulty level
   */
  getDifficulty(): number {
    return this.difficulty;
  }

  /**
   * Get estimated work (expected hash attempts)
   */
  getEstimatedWork(): number {
    return Math.pow(2, this.difficulty);
  }

  /**
   * Cleanup expired challenges periodically
   */
  private cleanupChallenges(): void {
    setInterval(() => {
      const now = Date.now();
      const expired: string[] = [];

      for (const [nonce, challenge] of this.challengeCache.entries()) {
        if (now - challenge.timestamp > 300000) {
          expired.push(nonce);
        }
      }

      for (const nonce of expired) {
        this.challengeCache.delete(nonce);
      }
    }, 60000); // Cleanup every minute
  }
}

/**
 * Utility function to solve a PoW challenge (for testing or client simulation)
 * WARNING: This is CPU-intensive. Used only for testing/demonstration.
 */
export async function solvePow(
  challengeNonce: string,
  targetZeros: number,
  maxAttempts: number = 1000000
): Promise<string | null> {
  for (let i = 0; i < maxAttempts; i++) {
    const solution = crypto.randomBytes(16).toString('hex');
    const data = challengeNonce + solution;
    const hash = crypto.createHash('sha256').update(data).digest();

    let zeros = 0;
    for (let j = 0; j < hash.length; j++) {
      for (let k = 7; k >= 0; k--) {
        if ((hash[j] & (1 << k)) === 0) {
          zeros++;
        } else {
          break;
        }
      }
      if (zeros >= targetZeros) break;
    }

    if (zeros >= targetZeros) {
      return solution;
    }
  }

  return null;
}
