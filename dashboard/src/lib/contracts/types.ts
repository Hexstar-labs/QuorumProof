/**
 * Shared types mirroring the Soroban contract structs.
 * These map 1:1 to the on-chain data shapes returned by RPC calls.
 */

// ── quorum_proof ────────────────────────────────────────────────────────────

export interface Credential {
  id: bigint
  subject: string
  issuer: string
  credential_type: number
  metadata_hash: string
  revoked: boolean
  expires_at: bigint | null
}

export interface QuorumSlice {
  id: bigint
  creator: string
  attestors: string[]
  weights: number[]
  threshold: number
}

export interface ProofRequest {
  id: bigint
  credential_id: bigint
  verifier: string
  requested_at: bigint
  claim_types: ClaimType[]
}

// ── zk_verifier ─────────────────────────────────────────────────────────────

export enum ClaimType {
  HasDegree = 'HasDegree',
  HasLicense = 'HasLicense',
  HasEmploymentHistory = 'HasEmploymentHistory',
  HasCertification = 'HasCertification',
  HasResearchPublication = 'HasResearchPublication',
}

// ── sbt_registry ─────────────────────────────────────────────────────────────

export interface SoulboundToken {
  id: bigint
  owner: string
  credential_id: bigint
  metadata_uri: string
}

export interface Delegation {
  token_id: bigint
  delegatee: string
  expires_at: bigint
}

export interface Delegation {
  token_id: bigint
  delegatee: string
  expires_at: bigint
}

export enum DisputeStatus {
  Open = 0,
  Upheld = 1,
  Dismissed = 2,
}

export interface Dispute {
  id: bigint
  token_id: bigint
  initiator: string
  accused: string
  status: DisputeStatus
  uphold_votes: string[]
  dismiss_votes: string[]
}
