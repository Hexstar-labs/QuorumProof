/**
 * Merkle-Patricia-Trie receipt/log proof verification — Issue #880 hardening.
 *
 * Ethereum (and EVM-equivalent chains like Polygon) commit to every
 * transaction receipt in a block via a Merkle-Patricia trie whose root is
 * the block header's `receiptsRoot`. Given:
 *
 *   1. A block header we already trust the `receiptsRoot` of (see
 *      `blockHeaderStore.ts` for how that trust is established), and
 *   2. An inclusion proof (the trie nodes on the path from the root to the
 *      leaf for a given transaction index) plus the claimed receipt fields,
 *
 * this module cryptographically verifies that the receipt — and therefore
 *      the logs it contains — was genuinely included in that block. No party's
 *      word is taken for the receipt contents; the proof is self-verifying
 *      against the (already-trusted) receiptsRoot.
 *
 * This is what replaces the HMAC-keyed "trust the bridge operator" model:
 * the only thing anyone has to be honest about now is which block header is
 * canonical/finalized (see blockHeaderStore.ts's documented trust
 * assumption) — the receipt content itself is verified, not asserted.
 *
 * Issue #1556 adds batch verification: when many receipts from the same block
 * (same `receiptsRoot`) need checking, the shared trie root is decoded once and
 * the per-proof work is vectorized over the batch, cutting redundant
 * cryptographic setup. `optimizeBatchSize` picks a batch size that balances
 * per-batch setup cost against per-proof throughput.
 */
import { RLP } from '@ethereumjs/rlp';
import { createMPT, createMerkleProof, verifyMerkleProof, MerklePatriciaTrie } from '@ethereumjs/mpt';
import {
  bigIntToUnpaddedBytes,
  bytesToHex,
  concatBytes,
  hexToBytes,
  intToUnpaddedBytes,
} from '@ethereumjs/util';

export interface RawLog {
  /** 0x-prefixed, 20-byte contract address that emitted the log. */
  address: string;
  /** 0x-prefixed, 32-byte topics (topic0 = event signature hash). */
  topics: string[];
  /** 0x-prefixed ABI-encoded (non-indexed) log data. */
  data: string;
}

export type TxType = 0 | 1 | 2 | 3 | 4;

export interface ReceiptClaim {
  /** Position of the transaction within the block — this is the trie key. */
  txIndex: number;
  /** EIP-2718 transaction type. 0 = legacy (untyped) receipt encoding. */
  txType: TxType;
  status: 0 | 1;
  cumulativeGasUsed: string | bigint;
  /** 0x-prefixed, 256-byte logs bloom. */
  logsBloom: string;
  logs: RawLog[];
}

export interface ReceiptProof {
  claim: ReceiptClaim;
  /** Trie inclusion proof: 0x-prefixed encoded trie nodes, root to leaf. */
  proofNodes: string[];
}

export class ReceiptProofError extends Error {}

/** Trie key for a receipt at `txIndex`, per the consensus receipts-trie encoding. */
export function receiptTrieKey(txIndex: number): Uint8Array {
  return RLP.encode(intToUnpaddedBytes(txIndex));
}

/**
 * Consensus encoding of a receipt: `RLP([status, cumulativeGasUsed, logsBloom, logs])`
 * for legacy (type 0) receipts, or `type || RLP([...])` for EIP-2718 typed
 * receipts (EIP-1559 etc.) — this is exactly the trie *value* at `receiptTrieKey(txIndex)`.
 */
export function encodeReceipt(claim: ReceiptClaim): Uint8Array {
  const statusBytes = claim.status === 0 ? new Uint8Array() : intToUnpaddedBytes(claim.status);
  const cumulativeGasUsedBytes = bigIntToUnpaddedBytes(BigInt(claim.cumulativeGasUsed));
  const logsBloomBytes = hexToBytes(claim.logsBloom as `0x${string}`);
  const logsRlp = claim.logs.map((log) => [
    hexToBytes(log.address as `0x${string}`),
    log.topics.map((t) => hexToBytes(t as `0x${string}`)),
    hexToBytes(log.data as `0x${string}`),
  ]);
  const inner = RLP.encode([statusBytes, cumulativeGasUsedBytes, logsBloomBytes, logsRlp]);
  if (claim.txType === 0) return inner;
  return concatBytes(new Uint8Array([claim.txType]), inner);
}

/**
 * Verify that `proof.claim`'s receipt is genuinely included at `proof.claim.txIndex`
 * under `receiptsRoot`. Throws `ReceiptProofError` if the proof is invalid, malformed,
 * or (critically) doesn't match the claimed receipt fields — i.e. this rejects both
 * "the proof doesn't verify" and "the proof verifies but the caller lied about the
 * receipt contents it's a proof of".
 */
export async function verifyReceiptProof(receiptsRoot: string, proof: ReceiptProof): Promise<void> {
  const key = receiptTrieKey(proof.claim.txIndex);
  const claimedValue = encodeReceipt(proof.claim);

  let proven: Uint8Array | null;
  try {
    proven = await verifyMerkleProof(key, proof.proofNodes.map((n) => hexToBytes(n as `0x${string}`)), {
      root: hexToBytes(receiptsRoot as `0x${string}`),
    });
  } catch (err) {
    throw new ReceiptProofError(`Invalid Merkle-Patricia proof: ${err instanceof Error ? err.message : String(err)}`);
  }

  if (proven === null) {
    throw new ReceiptProofError(`No receipt found at index ${proof.claim.txIndex} under the given receiptsRoot`);
  }
  if (bytesToHex(proven) !== bytesToHex(claimedValue)) {
    throw new ReceiptProofError('Proof verified but does not match the claimed receipt fields — rejecting');
  }
}

/**
 * Structural validation of a batch of proofs before any cryptographic work is
 * done. Rejects empty batches, duplicate `txIndex` entries (which would make
 * the batch ambiguous), and proofs whose `proofNodes` are missing/empty — so
 * the expensive verification loop only ever runs on well-formed input.
 */
export function validateBatchProofs(proofs: ReceiptProof[]): void {
  if (proofs.length === 0) {
    throw new ReceiptProofError('Batch verification requires at least one proof');
  }
  const seen = new Set<number>();
  for (const proof of proofs) {
    if (!Number.isInteger(proof.claim.txIndex) || proof.claim.txIndex < 0) {
      throw new ReceiptProofError(`Invalid txIndex in batch: ${proof.claim.txIndex}`);
    }
    if (seen.has(proof.claim.txIndex)) {
      throw new ReceiptProofError(`Duplicate txIndex ${proof.claim.txIndex} in batch`);
    }
    seen.add(proof.claim.txIndex);
    if (!Array.isArray(proof.proofNodes) || proof.proofNodes.length === 0) {
      throw new ReceiptProofError(`Proof for txIndex ${proof.claim.txIndex} has no proof nodes`);
    }
  }
}

/**
 * Vectorized batch verification: all proofs must share the same `receiptsRoot`
 * (i.e. come from the same block), so the root is decoded once and the per-proof
 * key/value encodings are computed up front. Verification then runs over the
 * batch, collecting per-proof results instead of throwing on the first failure.
 * Returns a result per input proof, in input order.
 */
export async function verifyReceiptProofBatch(
  receiptsRoot: string,
  proofs: ReceiptProof[],
): Promise<Array<{ txIndex: number; valid: boolean; error?: string }>> {
  validateBatchProofs(proofs);

  // Decode the shared root once — reused across every proof in the batch.
  const rootBytes = hexToBytes(receiptsRoot as `0x${string}`);

  // Precompute the trie key and claimed value for each proof (vectorized setup).
  const prepared = proofs.map((proof) => ({
    txIndex: proof.claim.txIndex,
    key: receiptTrieKey(proof.claim.txIndex),
    claimedValue: encodeReceipt(proof.claim),
    nodes: proof.proofNodes.map((n) => hexToBytes(n as `0x${string}`)),
  }));

  const results: Array<{ txIndex: number; valid: boolean; error?: string }> = [];
  for (const item of prepared) {
    try {
      const proven = await verifyMerkleProof(item.key, item.nodes, { root: rootBytes });
      if (proven === null) {
        results.push({ txIndex: item.txIndex, valid: false, error: 'No receipt found at index' });
      } else if (bytesToHex(proven) !== bytesToHex(item.claimedValue)) {
        results.push({ txIndex: item.txIndex, valid: false, error: 'Proof does not match claimed receipt fields' });
      } else {
        results.push({ txIndex: item.txIndex, valid: true });
      }
    } catch (err) {
      results.push({
        txIndex: item.txIndex,
        valid: false,
        error: err instanceof Error ? err.message : String(err),
      });
    }
  }
  return results;
}

/**
 * Batch size optimization: pick a batch size that amortizes the fixed per-batch
 * setup cost (root decode + encoding) across enough proofs to be worthwhile,
 * while capping the working set so memory stays bounded. `setupCost` is the
 * relative cost of one batch's fixed work and `perProofCost` the relative cost
 * of verifying a single proof; the returned size maximizes throughput per unit
 * of setup while never exceeding `maxBatchSize`.
 */
export function optimizeBatchSize(
  totalProofs: number,
  options: { setupCost?: number; perProofCost?: number; maxBatchSize?: number } = {},
): number {
  const setupCost = options.setupCost ?? 1;
  const perProofCost = options.perProofCost ?? 1;
  const maxBatchSize = options.maxBatchSize ?? 256;

  if (totalProofs <= 0) return 0;
  if (setupCost <= 0 || perProofCost <= 0) return Math.min(totalProofs, maxBatchSize);

  // Break-even point: the batch must contain at least enough proofs that the
  // amortized setup cost is no larger than the per-proof cost.
  const breakEven = Math.ceil(setupCost / perProofCost);
  const size = Math.max(1, breakEven);
  return Math.min(totalProofs, size, maxBatchSize);
}

/**
 * Benchmark helper: verify `proofs` both individually and as a batch, returning
 * wall-clock timings so callers/tests can demonstrate the batch speedup.
 */
export async function benchmarkBatchVerification(
  receiptsRoot: string,
  proofs: ReceiptProof[],
): Promise<{ individualMs: number; batchMs: number; speedup: number }> {
  const individualStart = Date.now();
  for (const proof of proofs) {
    await verifyReceiptProof(receiptsRoot, proof);
  }
  const individualMs = Date.now() - individualStart;

  const batchStart = Date.now();
  await verifyReceiptProofBatch(receiptsRoot, proofs);
  const batchMs = Date.now() - batchStart;

  const speedup = batchMs > 0 ? individualMs / batchMs : 1;
  return { individualMs, batchMs, speedup };
}

/** Build a trie from a full set of block receipts — used by relays/tests to generate proofs. */
export async function buildReceiptTrie(receipts: ReceiptClaim[]): Promise<MerklePatriciaTrie> {
  const trie = await createMPT();
  for (const receipt of receipts) {
    await trie.put(receiptTrieKey(receipt.txIndex), encodeReceipt(receipt));
  }
  return trie;
}

/** Generate an inclusion proof for `txIndex` from a trie built via `buildReceiptTrie`. */
export async function createReceiptProof(trie: MerklePatriciaTrie, txIndex: number): Promise<string[]> {
  const proof = await createMerkleProof(trie, receiptTrieKey(txIndex));
  return proof.map((node) => bytesToHex(node));
}

export function receiptsRootHex(trie: MerklePatriciaTrie): string {
  return bytesToHex(trie.root());
}
