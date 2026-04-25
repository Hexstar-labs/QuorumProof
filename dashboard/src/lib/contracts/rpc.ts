/**
 * Minimal Soroban RPC client wrapper.
 * Reads network config from Vite env vars and exposes a single `invokeContract`
 * helper used by all three contract clients.
 */

const RPC_URL = import.meta.env.VITE_STELLAR_RPC_URL as string
const NETWORK = import.meta.env.VITE_STELLAR_NETWORK as string

if (!RPC_URL) {
  console.warn('[QuorumProof] VITE_STELLAR_RPC_URL is not set.')
}

export interface InvokeParams {
  contractId: string
  method: string
  args?: unknown[]
  /** Stellar account address that signs the transaction (for write calls). */
  source?: string
}

/**
 * Call a Soroban contract function via JSON-RPC.
 *
 * For read-only (simulation) calls, `source` can be omitted.
 * For state-mutating calls the caller must supply a `source` address and
 * handle signing externally (e.g. via Freighter) before submitting.
 *
 * Returns the decoded XDR result as a plain JS value.
 */
export async function invokeContract<T = unknown>({
  contractId,
  method,
  args = [],
}: InvokeParams): Promise<T> {
  const body = {
    jsonrpc: '2.0',
    id: 1,
    method: 'simulateTransaction',
    params: {
      transaction: buildTransactionEnvelope(contractId, method, args),
      network: NETWORK,
    },
  }

  const res = await fetch(RPC_URL, {
    method: 'POST',
    headers: { 'Content-Type': 'application/json' },
    body: JSON.stringify(body),
  })

  if (!res.ok) {
    throw new Error(`RPC HTTP error ${res.status}: ${res.statusText}`)
  }

  const json = await res.json()

  if (json.error) {
    throw new Error(`RPC error [${json.error.code}]: ${json.error.message}`)
  }

  return decodeResult<T>(json.result)
}

// ── Internal helpers ─────────────────────────────────────────────────────────

/**
 * Builds a minimal transaction envelope string for simulation.
 * In a production integration this would use the Stellar SDK to construct
 * a proper XDR-encoded InvokeHostFunctionOp.
 *
 * Placeholder: returns a JSON-serialised descriptor that the RPC layer can
 * interpret. Replace with `@stellar/stellar-sdk` TransactionBuilder when
 * the SDK is added as a dependency.
 */
function buildTransactionEnvelope(
  contractId: string,
  method: string,
  args: unknown[],
): string {
  return JSON.stringify({ contractId, method, args })
}

/**
 * Decodes the XDR result from a simulateTransaction response.
 * Replace with proper XDR decoding via `@stellar/stellar-sdk` SorobanDataBuilder.
 */
function decodeResult<T>(result: unknown): T {
  // Passthrough until real XDR decoding is wired up.
  return result as T
}
