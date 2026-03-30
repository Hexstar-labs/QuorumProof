/**
 * Typed contract client for the `sbt_registry` Soroban contract.
 *
 * Contract address is read from VITE_CONTRACT_SBT_REGISTRY env var.
 */

import { invokeContract } from './rpc'
import type { SoulboundToken } from './types'

const CONTRACT_ID = import.meta.env.VITE_CONTRACT_SBT_REGISTRY as string

if (!CONTRACT_ID) {
  console.warn('[QuorumProof] VITE_CONTRACT_SBT_REGISTRY is not set.')
}

/**
 * Mint a soulbound token for a credential.
 * The `(owner, credential_id)` pair must be unique — the contract panics on duplicates.
 * @returns The new token ID.
 */
export async function mint(
  owner: string,
  credentialId: bigint,
  metadataUri: string,
): Promise<bigint> {
  return invokeContract<bigint>({
    contractId: CONTRACT_ID,
    method: 'mint',
    args: [owner, credentialId, metadataUri],
    source: owner,
  })
}

/** Fetch a soulbound token by ID. Returns `null` if not found. */
export async function getToken(tokenId: bigint): Promise<SoulboundToken | null> {
  return invokeContract<SoulboundToken | null>({
    contractId: CONTRACT_ID,
    method: 'get_token',
    args: [tokenId],
  })
}

/** Returns the owner address of a token. Returns `null` if the token doesn't exist. */
export async function ownerOf(tokenId: bigint): Promise<string | null> {
  return invokeContract<string | null>({
    contractId: CONTRACT_ID,
    method: 'owner_of',
    args: [tokenId],
  })
}

/** Returns all token IDs owned by an address. */
export async function getTokensByOwner(owner: string): Promise<bigint[]> {
  return invokeContract<bigint[]>({
    contractId: CONTRACT_ID,
    method: 'get_tokens_by_owner',
    args: [owner],
  })
}
