import { Request, Response, NextFunction } from 'express';
import { PoWManager, type PoWSolution } from '../crypto/pow.js';

declare global {
  namespace Express {
    interface Request {
      powVerified?: boolean;
    }
  }
}

/**
 * PoW Rate Limiter Middleware
 *
 * Provides endpoints to:
 * 1. Request a PoW challenge
 * 2. Submit a PoW solution to bypass traditional rate limiting
 *
 * Once a valid solution is provided, the client IP is marked as
 * having completed PoW and gets a higher rate limit for some duration.
 */

export interface PoWRateLimiterConfig {
  enablePoW: boolean;
  powExemptDuration: number; // How long PoW exemption lasts (ms)
  powEnabled: boolean;
}

export function createPoWRateLimiter(config: PoWRateLimiterConfig = {
  enablePoW: true,
  powExemptDuration: 3600000, // 1 hour
  powEnabled: true,
}) {
  const powManager = new PoWManager();
  const exemptedIPs = new Map<string, number>();

  function getClientIp(req: Request): string {
    return (req.ip ?? req.socket.remoteAddress ?? 'unknown');
  }

  function isPoWExempt(req: Request): boolean {
    if (!config.enablePoW) return false;

    const ip = getClientIp(req);
    const exemptTime = exemptedIPs.get(ip);

    if (exemptTime && Date.now() < exemptTime) {
      return true;
    }

    if (exemptTime) {
      exemptedIPs.delete(ip);
    }

    return false;
  }

  // Middleware that skips rate limiting for PoW-verified clients
  const middleware = (req: Request, res: Response, next: NextFunction): void => {
    if (isPoWExempt(req)) {
      req.powVerified = true;
      powManager.trackRequest();
    }
    next();
  };

  // Route handler for requesting a PoW challenge
  const requestChallenge = (req: Request, res: Response): void => {
    const challenge = powManager.generateChallenge();

    res.json({
      challenge: {
        nonce: challenge.nonce,
        target: challenge.target,
        timestamp: challenge.timestamp,
        expires_at: challenge.expires_at,
      },
      difficulty_info: {
        current_difficulty: powManager.getDifficulty(),
        estimated_work: powManager.getEstimatedWork(),
        request_rate: powManager.getRequestRate(),
      },
    });
  };

  // Route handler for submitting a PoW solution
  const submitSolution = (req: Request, res: Response): void => {
    const body = req.body as Partial<PoWSolution>;

    if (!body.challenge_nonce || !body.nonce_solution) {
      res.status(400).json({
        error: 'Invalid request',
        message: 'Missing challenge_nonce or nonce_solution',
      });
      return;
    }

    const isValid = powManager.verifySolution({
      challenge_nonce: body.challenge_nonce,
      nonce_solution: body.nonce_solution,
    });

    if (!isValid) {
      res.status(400).json({
        error: 'Invalid solution',
        message: 'PoW solution verification failed',
      });
      return;
    }

    // Mark this IP as exempt from rate limiting
    const ip = getClientIp(req);
    exemptedIPs.set(ip, Date.now() + config.powExemptDuration);
    powManager.trackRequest();

    res.json({
      success: true,
      message: 'PoW solution accepted',
      exempt_until: Date.now() + config.powExemptDuration,
    });
  };

  // Periodically adjust difficulty based on load
  const difficultyAdjustmentInterval = setInterval(() => {
    const requestRate = powManager.getRequestRate();
    powManager.adjustDifficulty(requestRate);
  }, 60000); // Adjust every minute

  return {
    middleware,
    requestChallenge,
    submitSolution,
    powManager,
    cleanup: () => clearInterval(difficultyAdjustmentInterval),
  };
}
