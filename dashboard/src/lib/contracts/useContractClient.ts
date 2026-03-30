/**
 * useContractClient — unified React hook that exposes all three contract
 * clients to UI components.
 *
 * Usage:
 *   const { quorumProof, sbtRegistry, zkVerifier } = useContractClient()
 *   const credential = await quorumProof.getCredential(1n)
 */

import { useMemo } from 'react'
import * as quorumProof from './quorumProof'
import * as sbtRegistry from './sbtRegistry'
import * as zkVerifier from './zkVerifier'

export interface ContractClient {
  quorumProof: typeof quorumProof
  sbtRegistry: typeof sbtRegistry
  zkVerifier: typeof zkVerifier
}

export function useContractClient(): ContractClient {
  // Memoised so component re-renders don't produce new object references.
  return useMemo(
    () => ({ quorumProof, sbtRegistry, zkVerifier }),
    [],
  )
}
