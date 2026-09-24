/**
 * Time-Lock Puzzle Service (Issue #1580)
 *
 * Implements Rivest-Shamir-Wagner time-lock puzzles to prevent fast replay attacks
 * by introducing computational delays into proof verification.
 *
 * Algorithm: RSA-based time-lock puzzle where:
 * - A puzzle is generated with difficulty level T
 * - Solving requires ~T sequential squarings (cannot be parallelized)
 * - Solution must be provided before accepting verification
 * - Computational cost scales with difficulty level
 */

import { randomBytes, createHash } from 'crypto';

export type PuzzleDifficulty = 'low' | 'medium' | 'high';

export interface TimeLockPuzzle {
  puzzle_id: string;
  difficulty: PuzzleDifficulty;
  challenge: string;
  created_at: string;
  expires_at: string;
  solved: boolean;
  solution?: string | null;
}

export interface PuzzleSolution {
  puzzle_id: string;
  solution: string;
  verification_time_ms: number;
}

export class TimeLockPuzzleService {
  private puzzles: Map<string, TimeLockPuzzle> = new Map();
  private solutions: Map<string, PuzzleSolution> = new Map();

  private readonly PUZZLE_TIMEOUT_MS = 10 * 60 * 1000; // 10 minutes
  private readonly DIFFICULTY_ITERATIONS: Record<PuzzleDifficulty, number> = {
    low: 1000,
    medium: 10000,
    high: 100000,
  };

  private readonly DIFFICULTY_MIN_TIME_MS: Record<PuzzleDifficulty, number> = {
    low: 100,
    medium: 1000,
    high: 10000,
  };

  /**
   * Generates a new time-lock puzzle
   */
  generatePuzzle(difficulty: PuzzleDifficulty = 'medium'): TimeLockPuzzle {
    const puzzleId = this.generatePuzzleId();
    const challenge = this.generateChallenge(difficulty);
    const now = new Date();
    const expiresAt = new Date(now.getTime() + this.PUZZLE_TIMEOUT_MS);

    const puzzle: TimeLockPuzzle = {
      puzzle_id: puzzleId,
      difficulty,
      challenge,
      created_at: now.toISOString(),
      expires_at: expiresAt.toISOString(),
      solved: false,
      solution: null,
    };

    this.puzzles.set(puzzleId, puzzle);
    return puzzle;
  }

  /**
   * Solves a time-lock puzzle by performing sequential squarings
   */
  solvePuzzle(puzzleId: string): PuzzleSolution {
    const puzzle = this.puzzles.get(puzzleId);
    if (!puzzle) {
      throw new Error(`Puzzle ${puzzleId} not found`);
    }

    if (new Date(puzzle.expires_at) < new Date()) {
      throw new Error(`Puzzle ${puzzleId} has expired`);
    }

    if (puzzle.solved) {
      throw new Error(`Puzzle ${puzzleId} already solved`);
    }

    const startTime = Date.now();
    const solution = this.performSequentialSquarings(
      puzzle.challenge,
      this.DIFFICULTY_ITERATIONS[puzzle.difficulty]
    );
    const verificationTime = Date.now() - startTime;

    const minTime = this.DIFFICULTY_MIN_TIME_MS[puzzle.difficulty];
    if (verificationTime < minTime) {
      throw new Error(
        `Solution verification too fast (${verificationTime}ms < ${minTime}ms). ` +
        `Puzzle requires proper sequential computation.`
      );
    }

    puzzle.solved = true;
    puzzle.solution = solution;

    const solutionRecord: PuzzleSolution = {
      puzzle_id: puzzleId,
      solution,
      verification_time_ms: verificationTime,
    };

    this.solutions.set(puzzleId, solutionRecord);
    return solutionRecord;
  }

  /**
   * Verifies that a solution is correct for a puzzle
   */
  verifySolution(puzzleId: string, solution: string): boolean {
    const puzzle = this.puzzles.get(puzzleId);
    if (!puzzle) {
      return false;
    }

    if (!puzzle.solved) {
      return false;
    }

    if (puzzle.solution !== solution) {
      return false;
    }

    if (new Date(puzzle.expires_at) < new Date()) {
      return false;
    }

    return true;
  }

  /**
   * Gets puzzle information
   */
  getPuzzle(puzzleId: string): TimeLockPuzzle | null {
    const puzzle = this.puzzles.get(puzzleId);
    if (!puzzle) {
      return null;
    }

    if (new Date(puzzle.expires_at) < new Date()) {
      puzzle.solved = false;
      return puzzle;
    }

    return puzzle;
  }

  /**
   * Gets puzzle solution information
   */
  getSolution(puzzleId: string): PuzzleSolution | null {
    return this.solutions.get(puzzleId) || null;
  }

  /**
   * Estimates the computational cost in milliseconds for a puzzle
   */
  estimateComputationTime(difficulty: PuzzleDifficulty): number {
    return this.DIFFICULTY_MIN_TIME_MS[difficulty];
  }

  /**
   * Gets puzzle statistics
   */
  getPuzzleStats(): {
    total_generated: number;
    total_solved: number;
    by_difficulty: Record<PuzzleDifficulty, { generated: number; solved: number }>;
  } {
    const stats: Record<PuzzleDifficulty, { generated: number; solved: number }> = {
      low: { generated: 0, solved: 0 },
      medium: { generated: 0, solved: 0 },
      high: { generated: 0, solved: 0 },
    };

    for (const puzzle of this.puzzles.values()) {
      stats[puzzle.difficulty].generated++;
      if (puzzle.solved) {
        stats[puzzle.difficulty].solved++;
      }
    }

    const totalGenerated = Array.from(this.puzzles.values()).length;
    const totalSolved = Array.from(this.puzzles.values()).filter(p => p.solved).length;

    return {
      total_generated: totalGenerated,
      total_solved: totalSolved,
      by_difficulty: stats,
    };
  }

  /**
   * Performs sequential squarings (cannot be efficiently parallelized)
   * This is a simplified implementation that demonstrates the concept.
   * In production, use actual RSA modular exponentiation for cryptographic security.
   */
  private performSequentialSquarings(challenge: string, iterations: number): string {
    let result = challenge;

    for (let i = 0; i < iterations; i++) {
      result = createHash('sha256').update(result).digest('hex');
    }

    return result;
  }

  /**
   * Generates a puzzle challenge based on difficulty
   */
  private generateChallenge(difficulty: PuzzleDifficulty): string {
    const random = randomBytes(32).toString('hex');
    const timestamp = Date.now().toString();
    return createHash('sha256').update(random + timestamp + difficulty).digest('hex');
  }

  /**
   * Generates a unique puzzle ID
   */
  private generatePuzzleId(): string {
    return 'puzzle_' + Date.now() + '_' + Math.random().toString(36).substring(2, 11);
  }

  /**
   * Cleans up expired puzzles (call periodically)
   */
  cleanupExpiredPuzzles(): number {
    let cleaned = 0;
    const now = new Date();

    for (const [puzzleId, puzzle] of this.puzzles.entries()) {
      if (new Date(puzzle.expires_at) < now) {
        this.puzzles.delete(puzzleId);
        this.solutions.delete(puzzleId);
        cleaned++;
      }
    }

    return cleaned;
  }
}
