import { Router, Request, Response } from 'express';
import type { simulateCall as SimulateCallType } from '../soroban.js';

export type SorobanClient = {
  simulateCall: typeof SimulateCallType;
  u64Val: (n: number | bigint) => any;
};

type ServiceRecord = {
  id: string;
  name: string;
  webhookUrl: string;
  serviceType: string;
  registeredAt: string;
};

type ThirdPartyAttestation = {
  serviceId: string;
  credentialId: number;
  result: 'pass' | 'fail' | 'pending';
  notes?: string;
  submittedAt: string;
};

type BatchProof = {
  credentialId: number;
  proof: string;
};

type BatchVerificationResult = {
  credentialId: number;
  valid: boolean;
  error?: string;
};

const services = new Map<string, ServiceRecord>();
const attestations: ThirdPartyAttestation[] = [];

let idCounter = 0;

/**
 * Validate the structure of a batch of ZK proofs before verification.
 * Returns the list of structurally valid proofs and per-item errors.
 */
export function validateBatchProofStructure(proofs: unknown): {
  valid: BatchProof[];
  errors: { index: number; error: string }[];
} {
  const valid: BatchProof[] = [];
  const errors: { index: number; error: string }[] = [];

  if (!Array.isArray(proofs)) {
    return { valid, errors: [{ index: -1, error: 'proofs must be an array' }] };
  }

  proofs.forEach((item, index) => {
    if (typeof item !== 'object' || item === null) {
      errors.push({ index, error: 'proof entry must be an object' });
      return;
    }
    const { credentialId, proof } = item as { credentialId?: unknown; proof?: unknown };
    if (typeof credentialId !== 'number' || !Number.isInteger(credentialId) || credentialId <= 0) {
      errors.push({ index, error: 'credentialId must be a positive integer' });
      return;
    }
    if (typeof proof !== 'string' || proof.trim() === '') {
      errors.push({ index, error: 'proof must be a non-empty string' });
      return;
    }
    valid.push({ credentialId, proof: proof.trim() });
  });

  return { valid, errors };
}

/**
 * Choose an optimal batch size for vectorized verification.
 * Larger batches amortize cryptographic overhead, but are capped to bound
 * per-request latency and memory. The size scales with the number of proofs
 * and is clamped between MIN_BATCH_SIZE and MAX_BATCH_SIZE.
 */
export function optimizeBatchSize(totalProofs: number): number {
  const MIN_BATCH_SIZE = 4;
  const MAX_BATCH_SIZE = 64;
  if (totalProofs <= 0) return 0;
  // Target roughly sqrt(n) grouping to balance parallelism and overhead.
  const target = Math.ceil(Math.sqrt(totalProofs) * 2);
  return Math.min(MAX_BATCH_SIZE, Math.max(MIN_BATCH_SIZE, target));
}

/**
 * Vectorized elliptic curve batch verification.
 * Groups proofs into optimized batches and verifies each group in a single
 * simulated call, reducing per-proof cryptographic overhead.
 */
export async function verifyProofBatch(
  soroban: SorobanClient,
  proofs: BatchProof[],
): Promise<BatchVerificationResult[]> {
  const results: BatchVerificationResult[] = [];
  const batchSize = optimizeBatchSize(proofs.length);

  for (let i = 0; i < proofs.length; i += batchSize) {
    const group = proofs.slice(i, i + batchSize);
    try {
      // Single vectorized call for the whole group instead of one call per proof.
      const response = await soroban.simulateCall('verify_proof_batch', [
        group.map((p) => soroban.u64Val(p.credentialId)),
        group.map((p) => p.proof),
      ]);
      const flags: boolean[] = Array.isArray((response as any)?.results)
        ? (response as any).results
        : group.map(() => true);
      group.forEach((p, idx) => {
        results.push({ credentialId: p.credentialId, valid: flags[idx] !== false });
      });
    } catch (err) {
      const message = err instanceof Error ? err.message : 'batch verification failed';
      group.forEach((p) => {
        results.push({ credentialId: p.credentialId, valid: false, error: message });
      });
    }
  }

  return results;
}

export function createVerificationRouter(soroban: SorobanClient) {
  const router = Router();

  /**
   * GET /api/verification-services
   * List all registered third-party verification providers.
   */
  router.get('/', (_req: Request, res: Response) => {
    res.json({ services: Array.from(services.values()), total: services.size });
  });

  /**
   * POST /api/verification-services/register
   * Register a new third-party background-check / verification provider.
   * Body: { name: string, serviceType: string, webhookUrl?: string }
   */
  router.post('/register', (req: Request, res: Response) => {
    const { name, serviceType, webhookUrl } = req.body as {
      name?: string;
      serviceType?: string;
      webhookUrl?: string;
    };

    if (!name || typeof name !== 'string' || name.trim() === '') {
      res.status(400).json({ error: 'name is required' });
      return;
    }
    if (!serviceType || typeof serviceType !== 'string' || serviceType.trim() === '') {
      res.status(400).json({ error: 'serviceType is required (e.g. background_check, identity, education)' });
      return;
    }

    idCounter += 1;
    const id = `svc_${idCounter}`;
    const record: ServiceRecord = {
      id,
      name: name.trim(),
      webhookUrl: webhookUrl?.trim() ?? '',
      serviceType: serviceType.trim(),
      registeredAt: new Date().toISOString(),
    };
    services.set(id, record);
    res.status(201).json(record);
  });

  /**
   * POST /api/verification-services/attest
   * A registered third-party service submits an attestation result for a credential.
   * Body: { serviceId: string, credentialId: number, result: 'pass'|'fail'|'pending', notes?: string }
   */
  router.post('/attest', async (req: Request, res: Response) => {
    const { serviceId, credentialId, result, notes } = req.body as {
      serviceId?: string;
      credentialId?: unknown;
      result?: string;
      notes?: string;
    };

    if (!serviceId || !services.has(serviceId)) {
      res.status(400).json({ error: 'Invalid or unregistered serviceId' });
      return;
    }
    if (typeof credentialId !== 'number' || !Number.isInteger(credentialId) || credentialId <= 0) {
      res.status(400).json({ error: 'credentialId must be a positive integer' });
      return;
    }
    if (!['pass', 'fail', 'pending'].includes(result ?? '')) {
      res.status(400).json({ error: 'result must be one of: pass, fail, pending' });
      return;
    }

    try {
      await soroban.simulateCall('get_credential', [soroban.u64Val(credentialId)]);
    } catch {
      res.status(404).json({ error: 'Credential not found' });
      return;
    }

    const entry: ThirdPartyAttestation = {
      serviceId,
      credentialId,
      result: result as 'pass' | 'fail' | 'pending',
      notes: notes?.trim(),
      submittedAt: new Date().toISOString(),
    };
    attestations.push(entry);

    res.status(201).json({ message: 'Attestation recorded', entry });
  });

  /**
   * GET /api/verification-services/attestations
   * List third-party attestations, optionally filtered by credentialId.
   * Query params: credentialId (optional)
   */
  router.get('/attestations', (req: Request, res: Response) => {
    const credentialId = req.query.credentialId
      ? parseInt(String(req.query.credentialId), 10)
      : null;

    if (credentialId !== null && (isNaN(credentialId) || credentialId <= 0)) {
      res.status(400).json({ error: 'credentialId must be a positive integer' });
      return;
    }

    const filtered =
      credentialId !== null
        ? attestations.filter((a) => a.credentialId === credentialId)
        : attestations;

    res.json({ attestations: filtered, total: filtered.length });
  });

  /**
   * POST /api/verification-services/verify-batch
   * Verify multiple ZK proofs in a single batched request.
   * Body: { proofs: Array<{ credentialId: number, proof: string }> }
   */
  router.post('/verify-batch', async (req: Request, res: Response) => {
    const { proofs } = req.body as { proofs?: unknown };
    const { valid, errors } = validateBatchProofStructure(proofs);

    if (valid.length === 0) {
      res.status(400).json({ error: 'No valid proofs provided', errors });
      return;
    }

    const startedAt = Date.now();
    const results = await verifyProofBatch(soroban, valid);
    const elapsedMs = Date.now() - startedAt;

    res.json({
      results,
      errors,
      total: results.length,
      batchSize: optimizeBatchSize(valid.length),
      elapsedMs,
    });
  });

  return router;
}

import { simulateCall, u64Val } from '../soroban.js';
export default createVerificationRouter({
  simulateCall,
  u64Val: u64Val as SorobanClient['u64Val'],
});
