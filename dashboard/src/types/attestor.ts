/**
 * Attestor types and interfaces for the QuorumProof dashboard
 */

export interface Attestor {
  /**
   * Attestor wallet address
   */
  address: string

  /**
   * Whether this attestor has signed the credential
   */
  hasSigned: boolean

  /**
   * Optional timestamp of when the attestation was signed
   */
  signedAt?: Date

  /**
   * Optional attestor name or display name
   */
  name?: string
}

export interface AttestationThreshold {
  /**
   * Number of attestors who have signed
   */
  signed: number

  /**
   * Total number of attestors required
   */
  required: number
}
