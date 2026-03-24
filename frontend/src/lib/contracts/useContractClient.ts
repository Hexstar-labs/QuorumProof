/**
 * useContractClient.ts — Unified React hook that exposes all three contract clients.
 *
 * Usage:
 *   const { quorumProof, sbtRegistry, zkVerifier } = useContractClient();
 *   const cred = await quorumProof.getCredential(1n);
 */

import * as quorumProof from './quorumProof';
import * as sbtRegistry from './sbtRegistry';
import * as zkVerifier from './zkVerifier';

export type { Credential, QuorumSlice } from './quorumProof';
export type { SoulboundToken } from './sbtRegistry';
export type { ClaimType, ProofRequest } from './zkVerifier';

export interface ContractClient {
  quorumProof: typeof quorumProof;
  sbtRegistry: typeof sbtRegistry;
  zkVerifier: typeof zkVerifier;
  /** Contract addresses resolved from env vars (empty string if not set). */
  addresses: {
    quorumProof: string;
    sbtRegistry: string;
    zkVerifier: string;
  };
}

/**
 * Returns stable references to all three typed contract clients.
 * The hook itself is synchronous — individual methods return Promises.
 */
export function useContractClient(): ContractClient {
  return {
    quorumProof,
    sbtRegistry,
    zkVerifier,
    addresses: {
      quorumProof: import.meta.env.VITE_CONTRACT_QUORUM_PROOF ?? '',
      sbtRegistry: import.meta.env.VITE_CONTRACT_SBT_REGISTRY ?? '',
      zkVerifier: import.meta.env.VITE_CONTRACT_ZK_VERIFIER ?? '',
    },
  };
}
