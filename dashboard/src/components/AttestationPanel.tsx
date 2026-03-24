import React, { useState, useCallback } from 'react'
import {
  Award,
  FileText,
  Briefcase,
  Trophy,
  CheckCircle,
  Clock,
  AlertCircle,
  Users,
  Signature,
} from 'lucide-react'
import { formatDistanceToNow, format } from 'date-fns'
import clsx from 'clsx'
import { Credential, CredentialType } from '../types/credential'
import { Attestor, AttestationThreshold } from '../types/attestor'
import '../styles/attestationPanel.css'

export interface AttestationPanelProps {
  /**
   * The credential to attest
   */
  credential: Credential

  /**
   * List of attestors in the quorum slice
   */
  attestors: Attestor[]

  /**
   * Attestation threshold requirements
   */
  threshold: AttestationThreshold

  /**
   * Current connected wallet address
   */
  connectedWalletAddress?: string

  /**
   * Callback when attestation is submitted
   */
  onAttest?: (credentialId: string) => Promise<void>

  /**
   * Optional CSS class for additional styling
   */
  className?: string
}

type AttestationState = 'idle' | 'signing' | 'success' | 'error'

/**
 * AttestationPanel Component
 *
 * Displays credential details and allows attestors to review and submit their attestation.
 * Features:
 * - Shows credential details
 * - Displays attestor list with signing status
 * - Shows threshold progress bar
 * - Provides "Attest" button (disabled if wallet not in quorum)
 * - Shows confirmation state after successful attestation
 *
 * @example
 * ```tsx
 * <AttestationPanel
 *   credential={credential}
 *   attestors={attestors}
 *   threshold={{ signed: 2, required: 3 }}
 *   connectedWalletAddress="0x..."
 *   onAttest={handleAttest}
 * />
 * ```
 */
export const AttestationPanel: React.FC<AttestationPanelProps> = ({
  credential,
  attestors,
  threshold,
  connectedWalletAddress,
  onAttest,
  className,
}) => {
  const [attestationState, setAttestationState] = useState<AttestationState>('idle')
  const [errorMessage, setErrorMessage] = useState<string>('')

  /**
   * Check if the connected wallet is in the quorum slice
   */
  const isWalletInQuorum = useCallback(() => {
    if (!connectedWalletAddress) return false
    return attestors.some(
      (attestor) =>
        attestor.address.toLowerCase() === connectedWalletAddress.toLowerCase()
    )
  }, [connectedWalletAddress, attestors])

  /**
   * Check if this wallet has already signed
   */
  const hasAlreadySigned = useCallback(() => {
    if (!connectedWalletAddress) return false
    const attestor = attestors.find(
      (a) => a.address.toLowerCase() === connectedWalletAddress.toLowerCase()
    )
    return attestor?.hasSigned || false
  }, [connectedWalletAddress, attestors])

  /**
   * Handle attestation submission
   */
  const handleAttestClick = useCallback(async () => {
    if (!isWalletInQuorum() || !onAttest) return

    try {
      setAttestationState('signing')
      setErrorMessage('')
      await onAttest(credential.id)
      setAttestationState('success')
      // Reset success state after 3 seconds
      setTimeout(() => setAttestationState('idle'), 3000)
    } catch (error) {
      setAttestationState('error')
      setErrorMessage(
        error instanceof Error ? error.message : 'Failed to submit attestation'
      )
    }
  }, [isWalletInQuorum, onAttest, credential.id])

  /**
   * Get credential type icon
   */
  const getCredentialIcon = (type: CredentialType) => {
    const iconProps = { width: 32, height: 32 }
    switch (type) {
      case 'degree':
        return <Award aria-hidden="true" {...iconProps} />
      case 'license':
        return <FileText aria-hidden="true" {...iconProps} />
      case 'employment':
        return <Briefcase aria-hidden="true" {...iconProps} />
      case 'achievement':
        return <Trophy aria-hidden="true" {...iconProps} />
      default:
        return <Award aria-hidden="true" {...iconProps} />
    }
  }

  const isThresholdMet = threshold.signed >= threshold.required
  const progressPercentage = Math.min(
    (threshold.signed / threshold.required) * 100,
    100
  )

  return (
    <div
      className={clsx('attestation-panel', className)}
      role="region"
      aria-label="Credential Attestation Panel"
    >
      {/* Header */}
      <div className="attestation-panel__header">
        <div className="attestation-panel__icon-container">
          {getCredentialIcon(credential.type)}
        </div>
        <div className="attestation-panel__header-content">
          <h2 className="attestation-panel__title">{credential.title}</h2>
          <p className="attestation-panel__issuer">
            Issued by: {credential.issuer.name}
          </p>
        </div>
      </div>

      {/* Credential Details */}
      <div className="attestation-panel__details">
        <div className="attestation-panel__detail-item">
          <span className="attestation-panel__detail-label">Credential ID:</span>
          <code className="attestation-panel__detail-value">
            {credential.id.substring(0, 10)}...{credential.id.substring(-8)}
          </code>
        </div>
        <div className="attestation-panel__detail-item">
          <span className="attestation-panel__detail-label">Subject Address:</span>
          <code className="attestation-panel__detail-value">
            {credential.subjectAddress.substring(0, 6)}...
            {credential.subjectAddress.substring(-4)}
          </code>
        </div>
        <div className="attestation-panel__detail-item">
          <span className="attestation-panel__detail-label">Issued:</span>
          <span className="attestation-panel__detail-value">
            {format(credential.issuanceDate, 'MMM d, yyyy')} (
            {formatDistanceToNow(credential.issuanceDate, { addSuffix: true })})
          </span>
        </div>
      </div>

      {/* Attestation Threshold Progress */}
      <div className="attestation-panel__threshold">
        <div className="attestation-panel__threshold-header">
          <h3 className="attestation-panel__threshold-title">
            <Users width={18} height={18} aria-hidden="true" />
            Attestation Threshold
          </h3>
          <span
            className={clsx(
              'attestation-panel__threshold-badge',
              isThresholdMet && 'attestation-panel__threshold-badge--met'
            )}
          >
            {threshold.signed} of {threshold.required}
          </span>
        </div>

        <div className="attestation-panel__progress-bar">
          <div
            className="attestation-panel__progress-fill"
            style={{ width: `${progressPercentage}%` }}
            roll="progressbar"
            aria-valuenow={threshold.signed}
            aria-valuemin={0}
            aria-valuemax={threshold.required}
          />
        </div>

        {isThresholdMet && (
          <div className="attestation-panel__threshold-met">
            <CheckCircle width={18} height={18} aria-hidden="true" />
            <span>Threshold reached! Credential is now attested.</span>
          </div>
        )}
      </div>

      {/* Attestors List */}
      <div className="attestation-panel__attestors">
        <h3 className="attestation-panel__attestors-title">Attestors</h3>
        <div className="attestation-panel__attestors-list">
          {attestors.map((attestor) => (
            <div
              key={attestor.address}
              className={clsx(
                'attestation-panel__attestor-item',
                attestor.hasSigned && 'attestation-panel__attestor-item--signed',
                connectedWalletAddress?.toLowerCase() === attestor.address.toLowerCase() &&
                  'attestation-panel__attestor-item--current'
              )}
            >
              <div className="attestation-panel__attestor-address">
                {attestor.name || `${attestor.address.substring(0, 6)}...${attestor.address.substring(-4)}`}
              </div>
              <div className="attestation-panel__attestor-status">
                {attestor.hasSigned ? (
                  <span className="attestation-panel__attestor-signed">
                    <CheckCircle width={16} height={16} aria-hidden="true" />
                    Signed {attestor.signedAt && `on ${format(attestor.signedAt, 'MMM d')}`}
                  </span>
                ) : (
                  <span className="attestation-panel__attestor-pending">
                    <Clock width={16} height={16} aria-hidden="true" />
                    Pending
                  </span>
                )}
              </div>
            </div>
          ))}
        </div>
      </div>

      {/* Attest Button */}
      <div className="attestation-panel__actions">
        {!connectedWalletAddress ? (
          <div className="attestation-panel__warning">
            <AlertCircle width={18} height={18} aria-hidden="true" />
            <span>Connect your wallet to submit an attestation</span>
          </div>
        ) : !isWalletInQuorum() ? (
          <div className="attestation-panel__warning">
            <AlertCircle width={18} height={18} aria-hidden="true" />
            <span>Your wallet is not in the quorum slice for this credential</span>
          </div>
        ) : hasAlreadySigned() ? (
          <div className="attestation-panel__success">
            <CheckCircle width={18} height={18} aria-hidden="true" />
            <span>You have already attested to this credential</span>
          </div>
        ) : (
          <button
            className={clsx(
              'attestation-panel__button',
              attestationState === 'signing' && 'attestation-panel__button--loading',
              attestationState === 'error' && 'attestation-panel__button--error',
              attestationState === 'success' && 'attestation-panel__button--success'
            )}
            onClick={handleAttestClick}
            disabled={!isWalletInQuorum() || attestationState === 'signing'}
            aria-label="Submit attestation to blockchain"
          >
            {attestationState === 'success' ? (
              <>
                <CheckCircle width={18} height={18} aria-hidden="true" />
                <span>Attestation Submitted!</span>
              </>
            ) : attestationState === 'signing' ? (
              <>
                <span className="attestation-panel__spinner" aria-hidden="true" />
                <span>Signing...</span>
              </>
            ) : (
              <>
                <Signature width={18} height={18} aria-hidden="true" />
                <span>Submit Attestation</span>
              </>
            )}
          </button>
        )}

        {attestationState === 'error' && (
          <div className="attestation-panel__error-message" role="alert">
            <AlertCircle width={16} height={16} aria-hidden="true" />
            <span>{errorMessage}</span>
          </div>
        )}
      </div>
    </div>
  )
}
