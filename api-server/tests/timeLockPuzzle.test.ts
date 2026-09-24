import { describe, it, expect, beforeEach } from 'vitest';
import { TimeLockPuzzleService } from '../src/services/timeLockPuzzle.js';

describe('TimeLockPuzzleService', () => {
  let puzzleService: TimeLockPuzzleService;

  beforeEach(() => {
    puzzleService = new TimeLockPuzzleService();
  });

  it('should generate a puzzle with default difficulty', () => {
    const puzzle = puzzleService.generatePuzzle();

    expect(puzzle.puzzle_id).toBeDefined();
    expect(puzzle.difficulty).toBe('medium');
    expect(puzzle.challenge).toBeDefined();
    expect(puzzle.created_at).toBeDefined();
    expect(puzzle.expires_at).toBeDefined();
    expect(puzzle.solved).toBe(false);
  });

  it('should generate puzzles with different difficulties', () => {
    const lowPuzzle = puzzleService.generatePuzzle('low');
    const mediumPuzzle = puzzleService.generatePuzzle('medium');
    const highPuzzle = puzzleService.generatePuzzle('high');

    expect(lowPuzzle.difficulty).toBe('low');
    expect(mediumPuzzle.difficulty).toBe('medium');
    expect(highPuzzle.difficulty).toBe('high');
  });

  it('should solve a puzzle', () => {
    const puzzle = puzzleService.generatePuzzle('low');

    const solution = puzzleService.solvePuzzle(puzzle.puzzle_id);

    expect(solution.puzzle_id).toBe(puzzle.puzzle_id);
    expect(solution.solution).toBeDefined();
    expect(solution.verification_time_ms).toBeGreaterThanOrEqual(0);
  });

  it('should mark puzzle as solved after solving', () => {
    const puzzle = puzzleService.generatePuzzle('low');
    puzzleService.solvePuzzle(puzzle.puzzle_id);

    const updatedPuzzle = puzzleService.getPuzzle(puzzle.puzzle_id);
    expect(updatedPuzzle?.solved).toBe(true);
    expect(updatedPuzzle?.solution).toBeDefined();
  });

  it('should verify a correct solution', () => {
    const puzzle = puzzleService.generatePuzzle('low');
    const solution = puzzleService.solvePuzzle(puzzle.puzzle_id);

    const isValid = puzzleService.verifySolution(puzzle.puzzle_id, solution.solution);
    expect(isValid).toBe(true);
  });

  it('should reject an incorrect solution', () => {
    const puzzle = puzzleService.generatePuzzle('low');
    puzzleService.solvePuzzle(puzzle.puzzle_id);

    const isValid = puzzleService.verifySolution(puzzle.puzzle_id, 'incorrect_solution');
    expect(isValid).toBe(false);
  });

  it('should reject solving an already solved puzzle', () => {
    const puzzle = puzzleService.generatePuzzle('low');
    puzzleService.solvePuzzle(puzzle.puzzle_id);

    expect(() => {
      puzzleService.solvePuzzle(puzzle.puzzle_id);
    }).toThrow('already solved');
  });

  it('should reject solving a non-existent puzzle', () => {
    expect(() => {
      puzzleService.solvePuzzle('non_existent');
    }).toThrow('not found');
  });

  it('should get puzzle information', () => {
    const puzzle = puzzleService.generatePuzzle();
    const retrieved = puzzleService.getPuzzle(puzzle.puzzle_id);

    expect(retrieved?.puzzle_id).toBe(puzzle.puzzle_id);
    expect(retrieved?.difficulty).toBe(puzzle.difficulty);
  });

  it('should return null for non-existent puzzle', () => {
    const puzzle = puzzleService.getPuzzle('non_existent');
    expect(puzzle).toBeNull();
  });

  it('should get solution information', () => {
    const puzzle = puzzleService.generatePuzzle('low');
    const solution = puzzleService.solvePuzzle(puzzle.puzzle_id);

    const retrieved = puzzleService.getSolution(puzzle.puzzle_id);
    expect(retrieved?.puzzle_id).toBe(solution.puzzle_id);
    expect(retrieved?.solution).toBe(solution.solution);
  });

  it('should return null for non-existent solution', () => {
    const solution = puzzleService.getSolution('non_existent');
    expect(solution).toBeNull();
  });

  it('should estimate computation time for difficulties', () => {
    const lowTime = puzzleService.estimateComputationTime('low');
    const mediumTime = puzzleService.estimateComputationTime('medium');
    const highTime = puzzleService.estimateComputationTime('high');

    expect(lowTime).toBeLessThan(mediumTime);
    expect(mediumTime).toBeLessThan(highTime);
  });

  it('should track puzzle statistics', () => {
    puzzleService.generatePuzzle('low');
    puzzleService.generatePuzzle('medium');
    puzzleService.generatePuzzle('high');

    const puzzle = puzzleService.generatePuzzle('low');
    puzzleService.solvePuzzle(puzzle.puzzle_id);

    const stats = puzzleService.getPuzzleStats();
    expect(stats.total_generated).toBe(4);
    expect(stats.total_solved).toBe(1);
    expect(stats.by_difficulty.low.generated).toBe(2);
    expect(stats.by_difficulty.medium.generated).toBe(1);
    expect(stats.by_difficulty.high.generated).toBe(1);
  });

  it('should enforce minimum solution time', () => {
    const puzzle = puzzleService.generatePuzzle('high');

    // This test verifies that the solution process takes at least the minimum time
    const solution = puzzleService.solvePuzzle(puzzle.puzzle_id);

    expect(solution.verification_time_ms).toBeGreaterThanOrEqual(
      puzzleService.estimateComputationTime('high')
    );
  });

  it('should generate unique puzzle IDs', () => {
    const puzzle1 = puzzleService.generatePuzzle();
    const puzzle2 = puzzleService.generatePuzzle();
    const puzzle3 = puzzleService.generatePuzzle();

    expect(puzzle1.puzzle_id).not.toBe(puzzle2.puzzle_id);
    expect(puzzle2.puzzle_id).not.toBe(puzzle3.puzzle_id);
  });

  it('should generate unique challenges', () => {
    const puzzle1 = puzzleService.generatePuzzle();
    const puzzle2 = puzzleService.generatePuzzle();

    expect(puzzle1.challenge).not.toBe(puzzle2.challenge);
  });

  it('should handle cleanup of expired puzzles', (done) => {
    const puzzle = puzzleService.generatePuzzle('low');

    setTimeout(() => {
      const cleaned = puzzleService.cleanupExpiredPuzzles();
      const retrieved = puzzleService.getPuzzle(puzzle.puzzle_id);

      expect(cleaned).toBeGreaterThan(0);
      expect(retrieved).toBeNull();
      done();
    }, 100);
  });
});
