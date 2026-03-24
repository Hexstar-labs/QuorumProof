import React, { useCallback } from 'react'
import {
  Award,
  FileText,
  Briefcase,
  Trophy,
  CheckCircle,
  Clock,
  AlertCircle,
  ChevronRight,
} from 'lucide-react'
import { formatDistanceToNow, format } from 'date-fns'
import clsx from 'clsx'
import { Credential, CredentialStatus, CredentialType } from '../types/credential'
import '../styles/credentialCard.css'

export interface CredentialCardProps {
  /**
   * The credential data to display
   */
  credential: Credential

  /**
   * Callback when the card is clicked/navigated to
   */
  onNavigate?: (credentialId: string) => void

  /**
   * Optional CSS class for additional styling
   */
  className?: string

  /**
   * Whether the card is part of a list or gallery
   */
  isInteractive?: boolean
}

/**
 * CredentialCard Component
 *
 * A reusable, accessible card display for credentials with:
 * - Credential type icon with color coding
 * - Truncated credential ID
 * - Subject address
 * - Issuance date
 * - Status badge (Attested / Pending / Revoked)
 * - Keyboard navigation and ARIA labels
 * - Dark mode support
 * - Accessibility features (keyboard nav, screen reader support)
 *
 * @example
 * ```tsx
 * <CredentialCard
 *   credential={credential}
 *   onNavigate={(id) => navigate(`/credentials/${id}`)}
 * />
 * ```
 */
export const CredentialCard: React.FC<CredentialCardProps> = ({
  credential,
  onNavigate,
  className,
  isInteractive = true,
}) => {
  const isRevoked = credential.status === 'revoked'

  /**
   * Get icon component based on credential type
   */
  const getCredentialIcon = (type: CredentialType) => {
    const iconProps = { width: 24, height: 24 }
    const iconClass = clsx(
      'credential-card__icon',
      `credential-card__icon--${type}`
    )

    switch (type) {
      case 'degree':
        return <Award className={iconClass} aria-hidden="true" {...iconProps} />
      case 'license':
        return <FileText className={iconClass} aria-hidden="true" {...iconProps} />
      case 'employment':
        return <Briefcase className={iconClass} aria-hidden="true" {...iconProps} />
      case 'achievement':
        return <Trophy className={iconClass} aria-hidden="true" {...iconProps} />
      default:
        return <Award className={iconClass} aria-hidden="true" {...iconProps} />
    }
  }

  /**
   * Get status badge icon and label
   */
  const getStatusBadge = (status: CredentialStatus) => {
    const baseProps = {
      width: 12,
      height: 12,
      className: 'credential-card__status-dot',
      'aria-hidden': 'true',
    }

    switch (status) {
      case 'attested':
        return {
          icon: <CheckCircle {...baseProps} />,
          label: 'Attested',
          className: 'credential-card__status-badge--attested',
        }
      case 'pending':
        return {
          icon: <Clock {...baseProps} />,
          label: 'Pending',
          className: 'credential-card__status-badge--pending',
        }
      case 'revoked':
        return {
          icon: <AlertCircle {...baseProps} />,
          label: 'Revoked',
          className: 'credential-card__status-badge--revoked',
        }
      default:
        return {
          icon: <CheckCircle {...baseProps} />,
          label: 'Unknown',
          className: '',
        }
    }
  }

  /**
   * Truncate credential ID for display (show first 8 and last 8 chars)
   */
  const truncateId = (id: string): string => {
    if (id.length <= 16) return id
    return `${id.slice(0, 8)}...${id.slice(-8)}`
  }

  /**
   * Handle card click and keyboard navigation
   */
  const handleClick = useCallback(() => {
    if (isInteractive && onNavigate && !isRevoked) {
      onNavigate(credential.id)
    }
  }, [credential.id, onNavigate, isInteractive, isRevoked])

  /**
   * Handle keyboard navigation (Enter/Space to activate)
   */
  const handleKeyDown = useCallback(
    (e: React.KeyboardEvent<HTMLDivElement>) => {
      if ((e.key === 'Enter' || e.key === ' ') && isInteractive && onNavigate) {
        e.preventDefault()
        handleClick()
      }
    },
    [handleClick, isInteractive, onNavigate]
  )

  const statusBadge = getStatusBadge(credential.status)
  const issuanceDateFormatted = format(credential.issuanceDate, 'MMM d, yyyy')
  const issuanceDateRelative = formatDistanceToNow(credential.issuanceDate, {
    addSuffix: true,
  })

  return (
    <div
      className={clsx(
        'credential-card',
        isRevoked && 'credential-card--revoked',
        className
      )}
      onClick={handleClick}
      onKeyDown={handleKeyDown}
      tabIndex={isInteractive ? 0 : -1}
      role={isInteractive ? 'button' : 'article'}
      aria-pressed={isInteractive}
      aria-label={`${credential.title} credential, status: ${credential.status}`}
    >
      {/* Header: Icon and Status Badge */}
      <div className="credential-card__header">
        <div className="credential-card__icon-wrapper">
          {getCredentialIcon(credential.type)}
        </div>

        <div className="credential-card__header-content">
          <h3 className="credential-card__title" title={credential.title}>
            {credential.title}
          </h3>
          <p className="credential-card__id" title={credential.id}>
            {truncateId(credential.id)}
          </p>
        </div>

        <div
          className={clsx(
            'credential-card__status-badge',
            statusBadge.className
          )}
          aria-label={`Credential status: ${statusBadge.label}`}
        >
          {statusBadge.icon}
          <span>{statusBadge.label}</span>
        </div>
      </div>

      {/* Body: Details */}
      <div className="credential-card__body">
        {/* Subject Address */}
        <div className="credential-card__detail">
          <span className="credential-card__detail-label">Subject</span>
          <span
            className="credential-card__detail-value credential-card__detail-value--mono"
            title={credential.subjectAddress}
          >
            {credential.subjectAddress}
          </span>
        </div>

        {/* Issuance Date */}
        <div className="credential-card__detail">
          <span className="credential-card__detail-label">Issued</span>
          <time
            className="credential-card__detail-value"
            dateTime={credential.issuanceDate.toISOString()}
            title={issuanceDateFormatted}
          >
            {issuanceDateRelative}
          </time>
        </div>

        {/* Issuer */}
        <div className="credential-card__issuer">
          {credential.issuer.icon && (
            <div className="credential-card__issuer-icon">
              {credential.issuer.icon}
            </div>
          )}
          <span>{credential.issuer.name}</span>
        </div>

        {/* Revocation Reason (if revoked) */}
        {isRevoked && credential.revocationReason && (
          <div className="credential-card__detail">
            <span className="credential-card__detail-label">Reason</span>
            <span className="credential-card__detail-value">
              {credential.revocationReason}
            </span>
          </div>
        )}

        {/* Expiration Date (if exists) */}
        {credential.expirationDate && (
          <div className="credential-card__detail">
            <span className="credential-card__detail-label">Expires</span>
            <time
              className="credential-card__detail-value"
              dateTime={credential.expirationDate.toISOString()}
              title={format(credential.expirationDate, 'MMM d, yyyy')}
            >
              {format(credential.expirationDate, 'MMM d, yyyy')}
            </time>
          </div>
        )}
      </div>

      {/* Navigation Indicator (for interactive cards) */}
      {isInteractive && !isRevoked && (
        <div
          style={{
            display: 'flex',
            justifyContent: 'flex-end',
            marginTop: '0.5rem',
            opacity: 0.5,
          }}
          aria-hidden="true"
        >
          <ChevronRight width={16} height={16} />
        </div>
      )}
    </div>
  )
}

export default CredentialCard
