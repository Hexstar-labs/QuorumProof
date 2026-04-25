import { useNavigate } from 'react-router-dom';
import {
  type CredCardData,
  deriveStatus,
  formatAddress,
  credTypeLabel,
} from '../lib/credentialUtils';

interface CredentialCardProps {
  data: CredCardData;
  sliceId: bigint | null;
}

const STATUS_CONFIG = {
  attested: { label: 'Attested', icon: '✅', badgeClass: 'badge--green', headerMod: 'valid'   },
  pending:  { label: 'Pending',  icon: '⏳', badgeClass: 'badge--blue',  headerMod: 'pending' },
  revoked:  { label: 'Revoked',  icon: '🚫', badgeClass: 'badge--red',   headerMod: 'revoked' },
  expired:  { label: 'Expired',  icon: '⏰', badgeClass: 'badge--gray',  headerMod: 'expired' },
};

export function CredentialCard({ data, sliceId }: CredentialCardProps) {
  const { credential, attested, slice, expired, sliceError, credError } = data;
  const navigate = useNavigate();

  const status = deriveStatus(credential.revoked, expired, attested);
  const { label, icon, badgeClass, headerMod } = STATUS_CONFIG[status];
  const idStr = credential.id.toString();
  const truncId = idStr.length > 14 ? idStr.slice(0, 6) + '…' + idStr.slice(-4) : idStr;
  const isRevoked = status === 'revoked';

  function handleNavigate() {
    navigate(`/credential/${credential.id}`);
  }

  function handleKeyDown(e: React.KeyboardEvent) {
    if (e.key === 'Enter' || e.key === ' ') {
      e.preventDefault();
      handleNavigate();
    }
  }

  return (
    <div
      className={`cred-card${isRevoked ? ' cred-card--revoked' : ''}`}
      role="article"
      tabIndex={0}
      aria-label={`${credTypeLabel(credential.credential_type)} credential ${truncId}, status: ${label}`}
      onClick={handleNavigate}
      onKeyDown={handleKeyDown}
      style={{ cursor: 'pointer' }}
    >
      {/* Header */}
      <div className={`cred-card__header cred-card__header--${headerMod}`}>
        <div className="cred-card__type">{credTypeLabel(credential.credential_type)}</div>
        <div
          className={`badge ${badgeClass}`}
          role="status"
          aria-label={`Attestation status: ${label}`}
        >
          {icon} {label}
        </div>
      </div>

      {/* Body */}
      {credError ? (
        <div className="cred-card__body cred-card__body--error">
          <div style={{ fontSize: '24px', marginBottom: '8px' }}>⚠️</div>
          <div style={{ color: 'var(--red)', fontSize: '13px' }}>Failed to load</div>
          <div style={{ color: 'var(--text-muted)', fontSize: '11px', marginTop: '4px' }}>{credError}</div>
        </div>
      ) : (
        <div className="cred-card__body">
          <h3 className="cred-card__id">#{truncId}</h3>

          <div className="cred-card__meta">
            <div className="meta-row">
              <span className="meta-label">Subject</span>
              <span className="meta-value mono" title={credential.subject}>
                {formatAddress(credential.subject)}
              </span>
            </div>
            <div className="meta-row">
              <span className="meta-label">Issuer</span>
              <span className="meta-value mono" title={credential.issuer}>
                {formatAddress(credential.issuer)}
              </span>
            </div>
            {credential.expires_at && (
              <div className="meta-row">
                <span className="meta-label">Expires</span>
                <span className="meta-value">
                  {new Date(Number(credential.expires_at) * 1000).toLocaleDateString(undefined, {
                    year: 'numeric', month: 'short', day: 'numeric',
                  })}
                </span>
              </div>
            )}
            {sliceId && (
              <div className="meta-row">
                <span className="meta-label">Slice</span>
                <span className="meta-value mono">#{sliceId.toString()}</span>
              </div>
            )}
          </div>

          {/* Quorum Slice Section */}
          <div className="cred-card__attestors">
            <div className="attestors-header">
              <span className="meta-label">Quorum Slice</span>
              {slice && (
                <span className="badge badge--gray" style={{ fontSize: '10px' }}>
                  {slice.attestors.length}/{slice.threshold} threshold
                </span>
              )}
            </div>
            {sliceError ? (
              <div className="attestors-empty">Slice unavailable</div>
            ) : !slice ? (
              <div className="attestors-empty">No slice data</div>
            ) : slice.attestors.length === 0 ? (
              <div className="attestors-empty">No attestors assigned</div>
            ) : (
              <div className="attestor-mini-list">
                {slice.attestors.slice(0, 3).map((addr, i) => (
                  <div key={addr} className="attestor-mini-item">
                    <span className="attestor-mini-item__avatar">{i + 1}</span>
                    <span className="mono" title={addr} style={{ fontSize: '11px' }}>
                      {formatAddress(addr)}
                    </span>
                  </div>
                ))}
                {slice.attestors.length > 3 && (
                  <div className="attestors-empty">+{slice.attestors.length - 3} more</div>
                )}
              </div>
            )}
          </div>
        </div>
      )}

      {/* Footer */}
      <div className="cred-card__footer" style={{ fontSize: '12px', color: 'var(--text-muted)', textAlign: 'center' }}>
        Click to view details →
      </div>
    </div>
  );
}
