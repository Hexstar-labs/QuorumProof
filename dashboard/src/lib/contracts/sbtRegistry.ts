/**
 * Typed contract client for the `sbt_registry` Soroban contract.
 *
 * Contract address is read from VITE_CONTRACT_SBT_REGISTRY env var.
 */

import { invokeContract } from './rpc'
import type { SoulboundToken, Delegation } from './types'

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

/** Delegate rights for a specific SBT to another address until a timestamp expires. */
export async function delegateSbtRights(
  owner: string,
  tokenId: bigint,
  delegatee: string,
  expiresAt: bigint,
): Promise<void> {
  return invokeContract<void>({
    contractId: CONTRACT_ID,
    method: 'delegate_sbt_rights',
    args: [owner, tokenId, delegatee, expiresAt],
    source: owner,
  })
}

/** Retrieve delegation details for a token. */
export async function getDelegation(tokenId: bigint): Promise<Delegation> {
  return invokeContract<Delegation>({
    contractId: CONTRACT_ID,
    method: 'get_delegation',
    args: [tokenId],
  })
}

/** Check whether a delegatee currently holds active rights for the token. */
export async function isDelegateActive(
  tokenId: bigint,
  delegatee: string,
): Promise<boolean> {
  return invokeContract<boolean>({
    contractId: CONTRACT_ID,
    method: 'is_delegate_active',
    args: [tokenId, delegatee],
  })
}
