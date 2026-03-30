/**
 * Typed contract client for the `zk_verifier` Soroban contract.
 *
 * Contract address is read from VITE_CONTRACT_ZK_VERIFIER env var.
 *
 * ⚠️  The on-chain verify_claim is a STUB in v1.0 — it accepts any non-empty
 *     bytes as a valid proof and provides no cryptographic guarantees.
 *     Real ZK (Groth16/PLONK) is planned for v1.1.
 */

import { invokeContract } from './rpc'
import { ClaimType } from './types'
import type { ProofRequest } from './types'

export { ClaimType }

const CONTRACT_ID = import.meta.env.VITE_CONTRACT_ZK_VERIFIER as string

if (!CONTRACT_ID) {
  console.warn('[QuorumProof] VITE_CONTRACT_ZK_VERIFIER is not set.')
}

/**
 * Generate a proof request for a given credential and claim type.
 * Returns a `ProofRequest` containing a nonce tied to the current ledger sequence.
 */
export async function generateProofRequest(
  credentialId: bigint,
  claimType: ClaimType,
): Promise<ProofRequest> {
  return invokeContract<ProofRequest>({
    contractId: CONTRACT_ID,
    method: 'generate_proof_request',
    args: [credentialId, claimType],
  })
}

/**
 * Verify a ZK claim.
 *
 * @param admin        - Admin address (required by the stub gate).
 * @param credentialId - The credential being verified.
 * @param claimType    - The specific claim to verify.
 * @param proof        - Proof bytes. Any non-empty value passes in v1.0.
 * @returns `true` if the claim is considered valid.
 *
 * ⚠️  STUB: no real cryptographic verification occurs until v1.1.
 */
export async function verifyClaim(
  admin: string,
  credentialId: bigint,
  claimType: ClaimType,
  proof: Uint8Array,
): Promise<boolean> {
  return invokeContract<boolean>({
    contractId: CONTRACT_ID,
    method: 'verify_claim',
    args: [admin, credentialId, claimType, Array.from(proof)],
    source: admin,
  })
}
