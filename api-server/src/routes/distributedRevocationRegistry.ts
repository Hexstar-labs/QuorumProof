/**
 * Distributed Revocation Registry Routes (Issue #1581)
 *
 * Endpoints for decentralized revocation with Byzantine fault tolerance
 */

import { Router, Request, Response } from 'express';
import { DistributedRevocationRegistry } from '../services/distributedRevocationRegistry.js';
import { problemJson } from '../middleware/problemDetails.js';

const router = Router();

// Create singleton instance (in production, would be persisted to database)
let registry: DistributedRevocationRegistry | null = null;

function getRegistry(): DistributedRevocationRegistry {
  if (!registry) {
    registry = new DistributedRevocationRegistry();
  }
  return registry;
}

/**
 * POST /api/distributed-revocation/revoke
 * Adds a revocation entry to the distributed registry
 */
router.post('/revoke', (req: Request, res: Response) => {
  try {
    const { credential_id, revoked_by, reason, locked_until } = req.body as Record<string, unknown>;

    if (typeof credential_id !== 'number' || !Number.isInteger(credential_id)) {
      res.status(400).json(problemJson(400, 'invalid-parameter', 'credential_id must be a valid integer'));
      return;
    }

    if (!revoked_by || typeof revoked_by !== 'string') {
      res.status(400).json(problemJson(400, 'invalid-parameter', 'revoked_by is required'));
      return;
    }

    const reg = getRegistry();
    const record = reg.addRevocationEntry(
      credential_id,
      revoked_by,
      typeof reason === 'string' ? reason : undefined,
      typeof locked_until === 'string' ? locked_until : undefined
    );

    res.status(201).json({ ok: true, data: record });
  } catch (error: unknown) {
    const msg = error instanceof Error ? error.message : String(error);
    res.status(400).json(problemJson(400, 'error', msg));
  }
});

/**
 * GET /api/distributed-revocation/status/:credential_id
 * Checks revocation status with Byzantine fault tolerance
 */
router.get('/status/:credential_id', (req: Request, res: Response) => {
  try {
    const credential_id = (req.params.credential_id || '') as string;
    const credId = parseInt(credential_id, 10);

    if (!Number.isInteger(credId)) {
      res.status(400).json(problemJson(400, 'invalid-parameter', 'credential_id must be an integer'));
      return;
    }

    const reg = getRegistry();
    const status = reg.getRevocationStatus(credId);

    res.json({ ok: true, data: status });
  } catch (error: unknown) {
    const msg = error instanceof Error ? error.message : String(error);
    res.status(400).json(problemJson(400, 'error', msg));
  }
});

/**
 * POST /api/distributed-revocation/peer/register
 * Registers a peer node in the network
 */
router.post('/peer/register', (req: Request, res: Response) => {
  try {
    const { peer_id, peer_address } = req.body as Record<string, unknown>;

    if (!peer_id || typeof peer_id !== 'string') {
      res.status(400).json(problemJson(400, 'invalid-parameter', 'peer_id is required'));
      return;
    }

    if (!peer_address || typeof peer_address !== 'string') {
      res.status(400).json(problemJson(400, 'invalid-parameter', 'peer_address is required'));
      return;
    }

    const reg = getRegistry();
    const peer = reg.registerPeer(peer_id, peer_address);

    res.status(201).json({ ok: true, data: peer });
  } catch (error: unknown) {
    const msg = error instanceof Error ? error.message : String(error);
    res.status(400).json(problemJson(400, 'error', msg));
  }
});

/**
 * POST /api/distributed-revocation/peer/:peer_id/unreachable
 * Marks a peer as unreachable (network partition)
 */
router.post('/peer/:peer_id/unreachable', (req: Request, res: Response) => {
  try {
    const peer_id = (req.params.peer_id || '') as string;

    const reg = getRegistry();
    reg.markPeerUnreachable(peer_id);

    res.json({ ok: true, message: `Peer ${peer_id} marked as unreachable` });
  } catch (error: unknown) {
    const msg = error instanceof Error ? error.message : String(error);
    res.status(400).json(problemJson(400, 'error', msg));
  }
});

/**
 * POST /api/distributed-revocation/peer/:peer_id/reachable
 * Marks a peer as reachable again
 */
router.post('/peer/:peer_id/reachable', (req: Request, res: Response) => {
  try {
    const peer_id = (req.params.peer_id || '') as string;

    const reg = getRegistry();
    reg.markPeerReachable(peer_id);

    res.json({ ok: true, message: `Peer ${peer_id} marked as reachable` });
  } catch (error: unknown) {
    const msg = error instanceof Error ? error.message : String(error);
    res.status(400).json(problemJson(400, 'error', msg));
  }
});

/**
 * POST /api/distributed-revocation/sync/:peer_id
 * Syncs with a peer node
 */
router.post('/sync/:peer_id', (req: Request, res: Response) => {
  try {
    const peer_id = (req.params.peer_id || '') as string;

    const reg = getRegistry();
    const success = reg.syncWithPeer(peer_id);

    res.json({
      ok: true,
      data: {
        peer_id,
        sync_success: success,
      },
    });
  } catch (error: unknown) {
    const msg = error instanceof Error ? error.message : String(error);
    res.status(400).json(problemJson(400, 'error', msg));
  }
});

/**
 * GET /api/distributed-revocation/topology
 * Gets current network topology
 */
router.get('/topology', (req: Request, res: Response) => {
  try {
    const reg = getRegistry();
    const topology = reg.getNetworkTopology();

    res.json({ ok: true, data: topology });
  } catch (error: unknown) {
    const msg = error instanceof Error ? error.message : String(error);
    res.status(400).json(problemJson(400, 'error', msg));
  }
});

/**
 * GET /api/distributed-revocation/stats
 * Gets registry statistics
 */
router.get('/stats', (req: Request, res: Response) => {
  try {
    const reg = getRegistry();
    const stats = reg.getRegistryStats();

    res.json({ ok: true, data: stats });
  } catch (error: unknown) {
    const msg = error instanceof Error ? error.message : String(error);
    res.status(400).json(problemJson(400, 'error', msg));
  }
});

/**
 * GET /api/distributed-revocation/sync-log
 * Gets sync log for audit trail
 */
router.get('/sync-log', (req: Request, res: Response) => {
  try {
    const limit = Math.min(parseInt((req.query.limit as string) ?? '100', 10), 1000);

    const reg = getRegistry();
    const logs = reg.getSyncLog(limit);

    res.json({
      ok: true,
      data: logs,
      count: logs.length,
    });
  } catch (error: unknown) {
    const msg = error instanceof Error ? error.message : String(error);
    res.status(400).json(problemJson(400, 'error', msg));
  }
});

/**
 * POST /api/distributed-revocation/commit
 * Commits current state to permanent storage
 */
router.post('/commit', (req: Request, res: Response) => {
  try {
    const reg = getRegistry();
    reg.commitState();
    const stats = reg.getRegistryStats();

    res.json({
      ok: true,
      message: 'State committed successfully',
      data: stats,
    });
  } catch (error: unknown) {
    const msg = error instanceof Error ? error.message : String(error);
    res.status(400).json(problemJson(400, 'error', msg));
  }
});

export default router;
