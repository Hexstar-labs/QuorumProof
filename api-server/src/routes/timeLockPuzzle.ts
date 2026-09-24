/**
 * Time-Lock Puzzle Routes (Issue #1580)
 *
 * Endpoints for generating and solving time-lock puzzles during proof verification
 */

import { Router, Request, Response } from 'express';
import { TimeLockPuzzleService, type PuzzleDifficulty } from '../services/timeLockPuzzle.js';
import { problemJson } from '../middleware/problemDetails.js';

const router = Router();
const puzzleService = new TimeLockPuzzleService();

/**
 * POST /api/time-lock-puzzle/generate
 * Generates a new time-lock puzzle for verification
 */
router.post('/generate', (req: Request, res: Response) => {
  try {
    const { difficulty } = req.body as Record<string, unknown>;

    let puzzleDifficulty: PuzzleDifficulty = 'medium';
    if (difficulty && ['low', 'medium', 'high'].includes(difficulty as string)) {
      puzzleDifficulty = difficulty as PuzzleDifficulty;
    }

    const puzzle = puzzleService.generatePuzzle(puzzleDifficulty);
    const estimatedTime = puzzleService.estimateComputationTime(puzzleDifficulty);

    res.status(201).json({
      ok: true,
      data: {
        ...puzzle,
        estimated_computation_time_ms: estimatedTime,
      },
    });
  } catch (error: unknown) {
    const msg = error instanceof Error ? error.message : String(error);
    res.status(400).json(problemJson(400, 'error', msg));
  }
});

/**
 * POST /api/time-lock-puzzle/:puzzle_id/solve
 * Solves a time-lock puzzle
 */
router.post('/:puzzle_id/solve', (req: Request, res: Response) => {
  try {
    const { puzzle_id } = req.params;

    const solution = puzzleService.solvePuzzle(puzzle_id);

    res.json({
      ok: true,
      data: solution,
    });
  } catch (error: unknown) {
    const msg = error instanceof Error ? error.message : String(error);
    res.status(400).json(problemJson(400, 'error', msg));
  }
});

/**
 * POST /api/time-lock-puzzle/:puzzle_id/verify
 * Verifies a puzzle solution
 */
router.post('/:puzzle_id/verify', (req: Request, res: Response) => {
  try {
    const { puzzle_id } = req.params;
    const { solution } = req.body as Record<string, unknown>;

    if (!solution || typeof solution !== 'string') {
      res.status(400).json(problemJson(400, 'invalid-parameter', 'solution is required'));
      return;
    }

    const isValid = puzzleService.verifySolution(puzzle_id, solution);

    res.json({
      ok: true,
      data: {
        puzzle_id,
        valid: isValid,
      },
    });
  } catch (error: unknown) {
    const msg = error instanceof Error ? error.message : String(error);
    res.status(400).json(problemJson(400, 'error', msg));
  }
});

/**
 * GET /api/time-lock-puzzle/:puzzle_id
 * Gets puzzle information
 */
router.get('/:puzzle_id', (req: Request, res: Response) => {
  try {
    const { puzzle_id } = req.params;

    const puzzle = puzzleService.getPuzzle(puzzle_id);

    if (!puzzle) {
      res.status(404).json(problemJson(404, 'not-found', `Puzzle ${puzzle_id} not found`));
      return;
    }

    res.json({ ok: true, data: puzzle });
  } catch (error: unknown) {
    const msg = error instanceof Error ? error.message : String(error);
    res.status(400).json(problemJson(400, 'error', msg));
  }
});

/**
 * GET /api/time-lock-puzzle/:puzzle_id/solution
 * Gets puzzle solution information
 */
router.get('/:puzzle_id/solution', (req: Request, res: Response) => {
  try {
    const { puzzle_id } = req.params;

    const solution = puzzleService.getSolution(puzzle_id);

    if (!solution) {
      res.status(404).json(problemJson(404, 'not-found', `Solution for puzzle ${puzzle_id} not found`));
      return;
    }

    res.json({ ok: true, data: solution });
  } catch (error: unknown) {
    const msg = error instanceof Error ? error.message : String(error);
    res.status(400).json(problemJson(400, 'error', msg));
  }
});

/**
 * GET /api/time-lock-puzzle/estimate/:difficulty
 * Estimates computation time for a puzzle difficulty
 */
router.get('/estimate/:difficulty', (req: Request, res: Response) => {
  try {
    const difficulty = (req.params.difficulty || '') as string;

    if (!['low', 'medium', 'high'].includes(difficulty)) {
      res.status(400).json(problemJson(400, 'invalid-parameter', 'difficulty must be low, medium, or high'));
      return;
    }

    const estimatedTime = puzzleService.estimateComputationTime(difficulty as PuzzleDifficulty);

    res.json({
      ok: true,
      data: {
        difficulty,
        estimated_time_ms: estimatedTime,
        estimated_time_seconds: (estimatedTime / 1000).toFixed(2),
      },
    });
  } catch (error: unknown) {
    const msg = error instanceof Error ? error.message : String(error);
    res.status(400).json(problemJson(400, 'error', msg));
  }
});

/**
 * GET /api/time-lock-puzzle/stats
 * Gets puzzle statistics
 */
router.get('/stats', (req: Request, res: Response) => {
  try {
    const stats = puzzleService.getPuzzleStats();

    res.json({
      ok: true,
      data: {
        ...stats,
        solve_rate: stats.total_generated > 0 ? (stats.total_solved / stats.total_generated * 100).toFixed(1) : 0,
      },
    });
  } catch (error: unknown) {
    const msg = error instanceof Error ? error.message : String(error);
    res.status(400).json(problemJson(400, 'error', msg));
  }
});

export default router;
