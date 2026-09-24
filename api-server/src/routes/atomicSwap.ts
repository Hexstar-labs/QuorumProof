/**
 * Atomic Swap Routes (Issue #1578)
 *
 * Endpoints for trustless credential exchange via atomic swap protocol
 */

import { Router, Request, Response } from 'express';
import { AtomicSwapService } from '../services/atomicSwap.js';
import { problemJson } from '../middleware/problemDetails.js';

const router = Router();
const swapService = new AtomicSwapService();

/**
 * POST /api/atomic-swap/initiate
 * Initiates a new atomic swap between two parties
 */
router.post('/initiate', (req: Request, res: Response) => {
  try {
    const {
      initiator_address,
      responder_address,
      initiator_credential_id,
      responder_credential_id,
    } = req.body as Record<string, unknown>;

    if (!initiator_address || typeof initiator_address !== 'string') {
      res.status(400).json(problemJson(400, 'invalid-parameter', 'initiator_address is required'));
      return;
    }

    if (!responder_address || typeof responder_address !== 'string') {
      res.status(400).json(problemJson(400, 'invalid-parameter', 'responder_address is required'));
      return;
    }

    if (typeof initiator_credential_id !== 'number' || !Number.isInteger(initiator_credential_id)) {
      res.status(400).json(problemJson(400, 'invalid-parameter', 'initiator_credential_id must be a valid integer'));
      return;
    }

    if (typeof responder_credential_id !== 'number' || !Number.isInteger(responder_credential_id)) {
      res.status(400).json(problemJson(400, 'invalid-parameter', 'responder_credential_id must be a valid integer'));
      return;
    }

    const swap = swapService.initiateSwap(
      initiator_address,
      responder_address,
      initiator_credential_id,
      responder_credential_id,
    );

    res.status(201).json({ ok: true, data: swap });
  } catch (error: unknown) {
    const msg = error instanceof Error ? error.message : String(error);
    res.status(400).json(problemJson(400, 'swap-error', msg));
  }
});

/**
 * POST /api/atomic-swap/:swap_id/lock
 * Locks a participant's credential
 */
router.post('/:swap_id/lock', (req: Request, res: Response) => {
  try {
    const swap_id = (req.params.swap_id || '') as string;
    const { participant_address } = req.body as Record<string, unknown>;

    if (!participant_address || typeof participant_address !== 'string') {
      res.status(400).json(problemJson(400, 'invalid-parameter', 'participant_address is required'));
      return;
    }

    const swap = swapService.lockCredential(swap_id, participant_address);
    res.json({ ok: true, data: swap });
  } catch (error: unknown) {
    const msg = error instanceof Error ? error.message : String(error);
    res.status(400).json(problemJson(400, 'swap-error', msg));
  }
});

/**
 * POST /api/atomic-swap/:swap_id/complete
 * Completes an atomic swap
 */
router.post('/:swap_id/complete', (req: Request, res: Response) => {
  try {
    const swap_id = (req.params.swap_id || '') as string;
    const { initiator_signature, responder_signature } = req.body as Record<string, unknown>;

    if (!initiator_signature || typeof initiator_signature !== 'string') {
      res.status(400).json(problemJson(400, 'invalid-parameter', 'initiator_signature is required'));
      return;
    }

    if (!responder_signature || typeof responder_signature !== 'string') {
      res.status(400).json(problemJson(400, 'invalid-parameter', 'responder_signature is required'));
      return;
    }

    const swap = swapService.completeSwap(swap_id, initiator_signature, responder_signature);
    res.json({ ok: true, data: swap });
  } catch (error: unknown) {
    const msg = error instanceof Error ? error.message : String(error);
    res.status(400).json(problemJson(400, 'swap-error', msg));
  }
});

/**
 * POST /api/atomic-swap/:swap_id/cancel
 * Cancels an ongoing swap
 */
router.post('/:swap_id/cancel', (req: Request, res: Response) => {
  try {
    const swap_id = (req.params.swap_id || '') as string;
    const { canceller_address } = req.body as Record<string, unknown>;

    if (!canceller_address || typeof canceller_address !== 'string') {
      res.status(400).json(problemJson(400, 'invalid-parameter', 'canceller_address is required'));
      return;
    }

    const swap = swapService.cancelSwap(swap_id, canceller_address);
    res.json({ ok: true, data: swap });
  } catch (error: unknown) {
    const msg = error instanceof Error ? error.message : String(error);
    res.status(400).json(problemJson(400, 'swap-error', msg));
  }
});

/**
 * GET /api/atomic-swap/:swap_id
 * Gets the current state of a swap
 */
router.get('/:swap_id', (req: Request, res: Response) => {
  try {
    const swap_id = (req.params.swap_id || '') as string;
    const swap = swapService.getSwap(swap_id);
    res.json({ ok: true, data: swap });
  } catch (error: unknown) {
    const msg = error instanceof Error ? error.message : String(error);
    res.status(404).json(problemJson(404, 'not-found', msg));
  }
});

/**
 * GET /api/atomic-swap/participant/:address
 * Lists all swaps for a participant
 */
router.get('/participant/:address', (req: Request, res: Response) => {
  try {
    const address = (req.params.address || '') as string;
    const swaps = swapService.listSwapsForParticipant(address);
    res.json({ ok: true, data: swaps, count: swaps.length });
  } catch (error: unknown) {
    const msg = error instanceof Error ? error.message : String(error);
    res.status(400).json(problemJson(400, 'error', msg));
  }
});

/**
 * GET /api/atomic-swap/stats
 * Gets swap statistics
 */
router.get('/stats', (req: Request, res: Response) => {
  try {
    const stats = swapService.getSwapStats();
    res.json({ ok: true, data: stats });
  } catch (error: unknown) {
    const msg = error instanceof Error ? error.message : String(error);
    res.status(400).json(problemJson(400, 'error', msg));
  }
});

export default router;
