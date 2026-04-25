/**
 * Typed contract client for the `quorum_proof` Soroban contract.
 *
 * Contract address is read from VITE_CONTRACT_QUORUM_PROOF env var.
 * All methods map 1:1 to the on-chain function signatures.
 */

import { invokeContract } from './rpc'
import type { Credential, QuorumSlice, ProofRequest } from './types'
import { ClaimType } from './types'

const CONTRACT_ID = import.meta.env.VITE_CONTRACT_QUORUM_PROOF as string

if (!CONTRACT_ID) {
  console.warn('[QuorumProof] VITE_CONTRACT_QUORUM_PROOF is not set.')
}

// ── Credential management ────────────────────────────────────────────────────

/**
 * Issue a new credential.
 * @returns The new credential ID.
 */
export async function issueCredential(
  subject: string,
  credentialType: number,
  metadataHash: string,
  expiresAt?: bigint,
): Promise<bigint> {
  return invokeContract<bigint>({
    contractId: CONTRACT_ID,
    method: 'issue_credential',
    args: [subject, credentialType, metadataHash, expiresAt ?? null],
  })
}

/** Fetch a credential by ID. Returns `null` if not found. */
export async function getCredential(credentialId: bigint): Promise<Credential | null> {
  return invokeContract<Credential | null>({
    contractId: CONTRACT_ID,
    method: 'get_credential',
    args: [credentialId],
  })
}

/**
 * Revoke a credential. Caller must be the subject or issuer.
 * @param caller - The address authorising the revocation.
 */
export async function revokeCredential(caller: string, credentialId: bigint): Promise<void> {
  return invokeContract<void>({
    contractId: CONTRACT_ID,
    method: 'revoke_credential',
    args: [caller, credentialId],
    source: caller,
  })
}

// ── Quorum slices ────────────────────────────────────────────────────────────

/**
 * Create a new quorum slice.
 * @returns The new slice ID.
 */
export async function createSlice(
  creator: string,
  attestors: string[],
  weights: number[],
  threshold: number,
): Promise<bigint> {
  return invokeContract<bigint>({
    contractId: CONTRACT_ID,
    method: 'create_slice',
    args: [creator, attestors, weights, threshold],
    source: creator,
  })
}

/** Fetch a quorum slice by ID. */
export async function getSlice(sliceId: bigint): Promise<QuorumSlice | null> {
  return invokeContract<QuorumSlice | null>({
    contractId: CONTRACT_ID,
    method: 'get_slice',
    args: [sliceId],
  })
}

/**
 * Add an attestor to an existing slice.
 * @param caller - Must be the slice creator.
 */
export async function addAttestor(
  caller: string,
  sliceId: bigint,
  attestor: string,
  weight: number,
): Promise<void> {
  return invokeContract<void>({
    contractId: CONTRACT_ID,
    method: 'add_attestor',
    args: [caller, sliceId, attestor, weight],
    source: caller,
  })
}

// ── Attestation ──────────────────────────────────────────────────────────────

/**
 * Submit an attestation for a credential against a slice.
 * @param attestor - The attesting address.
 */
export async function attest(
  attestor: string,
  credentialId: bigint,
  sliceId: bigint,
): Promise<void> {
  return invokeContract<void>({
    contractId: CONTRACT_ID,
    method: 'attest',
    args: [attestor, credentialId, sliceId],
    source: attestor,
  })
}

/** Returns true if the credential has met its attestation threshold. */
export async function isAttested(credentialId: bigint): Promise<boolean> {
  return invokeContract<boolean>({
    contractId: CONTRACT_ID,
    method: 'is_attested',
    args: [credentialId],
  })
}

/** Returns the list of attestor addresses for a credential. */
export async function getAttestors(credentialId: bigint): Promise<string[]> {
  return invokeContract<string[]>({
    contractId: CONTRACT_ID,
    method: 'get_attestors',
    args: [credentialId],
  })
}

// ── Proof requests ───────────────────────────────────────────────────────────

/**
 * Generate a proof request for a credential and claim type.
 * Delegates to the zk_verifier contract internally.
 */
export async function generateProofRequest(
  verifier: string,
  credentialId: bigint,
  claimTypes: ClaimType[],
): Promise<ProofRequest> {
  return invokeContract<ProofRequest>({
    contractId: CONTRACT_ID,
    method: 'generate_proof_request',
    args: [verifier, credentialId, claimTypes],
    source: verifier,
  })
}
