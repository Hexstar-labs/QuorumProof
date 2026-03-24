/**
 * Credential types and interfaces for the QuorumProof dashboard
 */

export type CredentialStatus = 'attested' | 'pending' | 'revoked'

export type CredentialType = 'degree' | 'license' | 'employment' | 'achievement'

export interface Credential {
  /**
   * Unique identifier for the credential
   */
  id: string

  /**
   * Type of credential
   */
  type: CredentialType

  /**
   * Display name/title of the credential
   */
  title: string

  /**
   * Subject address (e.g., wallet address or email)
   */
  subjectAddress: string

  /**
   * Date when the credential was issued
   */
  issuanceDate: Date

  /**
   * Current status of the credential
   */
  status: CredentialStatus

  /**
   * Optional expiration date
   */
  expirationDate?: Date

  /**
   * Issuer information
   */
  issuer: {
    name: string
    icon?: string
  }

  /**
   * Optional revocation reason (if status is 'revoked')
   */
  revocationReason?: string
}
