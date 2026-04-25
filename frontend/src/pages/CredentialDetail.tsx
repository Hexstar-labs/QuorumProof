import { useState, useEffect } from 'react';
import { useParams, useNavigate } from 'react-router-dom';
import { Navbar } from '../components/Navbar';
import { ShareCredentialDialog } from '../components/ShareCredentialDialog';
import { AuditTrail } from '../components/AuditTrail';
import {
  getCredential,
  getAttestors,
  isExpired,
  getSlice,
} from '../lib/contracts/quorumProof';
import type { Credential, QuorumSlice } from '../lib/contracts/quorumProof';
import { decodeMetadataHash } from '../stellar';
import { credTypeLabel, formatTimestamp, formatAddress } from './Verify';
import { attestorRole, deriveStatus } from '../lib/credentialUtils';

const STATUS_CONFIG = {
  attested: { label: 'Attested', icon: '✅', bannerMod: 'valid' },
  pending:  { label: 'Pending',  icon: '⏳', bannerMod: 'pending' },
  revoked:  { label: 'Revoked',  icon: '🚫', bannerMod: 'revoked' },
  expired:  { label: 'Expired',  icon: '⏰', bannerMod: 'expired' },
};

export default function CredentialDetail() {
  const { id } = useParams<{ id: string }>();
  const navigate = useNavigate();
  const [credential, setCredential] = useState<Credential | null>(null);
  const [attestors, setAttestors] = useState<string[]>([]);
  const [slice, setSlice] = useState<QuorumSlice | null>(null);
  const [isExpiredFlag, setIsExpiredFlag] = useState(false);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);
  const [copied, setCopied] = useState(false);
  const [showShare, setShowShare] = useState(false);

  useEffect(() => {
    if (!id) { setError('No credential ID provided'); setLoading(false); return; }

    (async () => {
      try {
        const credId = BigInt(id);
        const [cred, expired, attestorList] = await Promise.all([
          getCredential(credId),
          isExpired(credId).catch(() => false),
          getAttestors(credId).catch(() => [] as string[]),
        ]);
        setCredential(cred);
        setIsExpiredFlag(expired);
        setAttestors(attestorList ?? []);

        // Try to load slice from localStorage (same pattern as Dashboard)
        const sliceIdRaw = localStorage.getItem('qp-slice-id');
        if (sliceIdRaw) {
          try { setSlice(await getSlice(BigInt(sliceIdRaw))); } catch { /* no slice */ }
        }
      } catch (err) {
        setError(err instanceof Error ? err.message : 'Failed to load credential');
      } finally {
        setLoading(false);
      }
    })();
  }, [id]);

  function copyShareLink() {
    const url = `${window.location.origin}/verify?id=${id}`;
    navigator.clipboard.writeText(url).then(() => {
      setCopied(true);
      setTimeout(() => setCopied(false), 2000);
    });
  }

  if (loading) {
    return (
      <>
        <Navbar />
        <main className="container" style={{ paddingTop: '40px' }}>
          <div className="loading-state"><div className="spinner" /><p>Loading credential…</p></div>
        </main>
      </>
    );
  }

  if (error || !credential) {
    return (
      <>
        <Navbar />
        <main className="container" style={{ paddingTop: 40 }}>
          <div className="error-card">
            <div className="error-card__icon">⚠️</div>
            <div>
              <div className="error-card__title">Could Not Load Credential</div>
              <div className="error-card__msg">{error ?? 'Credential not found'}</div>
              <button className="btn btn--ghost btn--sm" style={{ marginTop: '12px' }} onClick={() => navigate('/dashboard')}>
                Back to Dashboard
              </button>
            </div>
          </div>
        </main>
      </>
    );
  }

  const status = deriveStatus(credential.revoked, isExpiredFlag, attestors.length > 0);
  const { label, icon, bannerMod } = STATUS_CONFIG[status];
  const metaStr = decodeMetadataHash(credential.metadata_hash);

  // Threshold progress
  const threshold = slice?.threshold ?? attestors.length;
  const attestedCount = attestors.length;
  const fullyAttested = attestedCount >= threshold && threshold > 0;
  const thresholdLabel = threshold > 0
    ? `${attestedCount} of ${threshold} — ${fullyAttested ? 'Fully Attested' : 'Pending'}`
    : `${attestedCount} attestor${attestedCount !== 1 ? 's' : ''}`;

  const shareUrl = `${window.location.origin}/verify?id=${id}`;

  return (
    <>
      <Navbar />
      <main className="container" style={{ paddingTop: '40px', maxWidth: '800px', paddingBottom: '64px' }}>
        {/* Back */}
        <button className="btn btn--ghost btn--sm" style={{ marginBottom: '24px' }} onClick={() => navigate('/dashboard')}>
          ← Back to Dashboard
        </button>

        {/* Status Banner */}
        <div className={`status-banner status-banner--${bannerMod}`} role="status" aria-label={`Credential status: ${label}`}>
          <div className="status-banner__icon" aria-hidden="true">{icon}</div>
          <div>
            <div className="status-banner__title">{label}</div>
            <div className="status-banner__sub">
              Credential #{id} · {credTypeLabel(credential.credential_type)}
              {credential.revoked && ' · Revoked'}
            </div>
          </div>
        </div>

        {/* Share Bar */}
        <div className="share-bar" style={{ marginBottom: '20px' }}>
          <span className="share-bar__url" aria-label="Verification link">{shareUrl}</span>
          <button
            className="btn btn--sm btn--ghost"
            onClick={copyShareLink}
            aria-label="Copy verification link to clipboard"
          >
            {copied ? '✅ Copied' : '📋 Copy'}
          </button>
          <button
            className="btn btn--sm btn--primary"
            onClick={() => setShowShare(true)}
            aria-label="Open share dialog"
          >
            🔗 Share
          </button>
        </div>

        {/* Credential Details */}
        <div className="detail-card" style={{ marginBottom: '20px' }}>
          <div className="detail-card__header">
            <span className="detail-card__title">Credential Details</span>
          </div>
          <div className="detail-card__body">
            <div className="meta-grid">
              <div>
                <div className="meta-item__label">Type</div>
                <div className="meta-item__value">{credTypeLabel(credential.credential_type)}</div>
              </div>
              <div>
                <div className="meta-item__label">Credential ID</div>
                <div className="meta-item__value meta-item__value--mono">#{id}</div>
              </div>
              <div>
                <div className="meta-item__label">Subject</div>
                <div className="meta-item__value meta-item__value--mono" title={credential.subject}>
                  {formatAddress(credential.subject)}
                </div>
              </div>
              <div>
                <div className="meta-item__label">Issuer</div>
                <div className="meta-item__value meta-item__value--mono" title={credential.issuer}>
                  {formatAddress(credential.issuer)}
                </div>
              </div>
              {metaStr && (
                <div style={{ gridColumn: '1 / -1' }}>
                  <div className="meta-item__label">Metadata</div>
                  <div className="meta-item__value meta-item__value--mono">{metaStr}</div>
                </div>
              )}
              {credential.expires_at && (
                <div>
                  <div className="meta-item__label">Expires</div>
                  <div className="meta-item__value">{formatTimestamp(credential.expires_at)}</div>
                </div>
              )}
              {credential.revoked && (
                <div>
                  <div className="meta-item__label">Status</div>
                  <div className="meta-item__value" style={{ color: 'var(--red)' }}>Revoked</div>
                </div>
              )}
            </div>
          </div>
        </div>

        {/* Quorum Slice & Attestation */}
        <div className="detail-card">
          <div className="detail-card__header">
            <span className="detail-card__title">Attestation History</span>            <span
              className={`badge ${fullyAttested ? 'badge--green' : 'badge--blue'}`}
              role="status"
              aria-label={`Threshold progress: ${thresholdLabel}`}
            >
              {thresholdLabel}
            </span>
          </div>
          <div className="detail-card__body">
            {/* Threshold progress bar */}
            {threshold > 0 && (
              <div style={{ marginBottom: '20px' }}>
                <div style={{ display: 'flex', justifyContent: 'space-between', fontSize: '12px', color: 'var(--text-secondary)', marginBottom: '6px' }}>
                  <span>Quorum Progress</span>
                  <span aria-live="polite">{attestedCount}/{threshold}</span>
                </div>
                <div
                  role="progressbar"
                  aria-valuenow={attestedCount}
                  aria-valuemin={0}
                  aria-valuemax={threshold}
                  aria-label={`Attestation progress: ${attestedCount} of ${threshold}`}
                  style={{
                    height: '6px',
                    background: 'var(--bg-surface)',
                    borderRadius: '3px',
                    overflow: 'hidden',
                  }}
                >
                  <div style={{
                    height: '100%',
                    width: `${Math.min(100, (attestedCount / threshold) * 100)}%`,
                    background: fullyAttested ? 'var(--green)' : 'var(--accent-primary)',
                    borderRadius: '3px',
                    transition: 'width 0.4s ease',
                  }} />
                </div>
              </div>
            )}

            {attestors.length === 0 ? (
              <div className="attestors-empty" style={{ padding: '16px 0' }}>No attestors yet</div>
            ) : (
              <ol className="attestor-list" aria-label="Attestor timeline" style={{ listStyle: 'none', padding: 0 }}>
                {attestors.map((addr, idx) => (
                  <li key={addr} className="attestor-item">
                    <div className="attestor-item__avatar" aria-hidden="true">{idx + 1}</div>
                    <div style={{ flex: 1, minWidth: 0 }}>
                      <div className="attestor-item__addr" title={addr}>{formatAddress(addr)}</div>
                      <div style={{ fontSize: '11px', color: 'var(--text-muted)', marginTop: '2px' }}>
                        {attestorRole(idx)}
                      </div>
                    </div>
                    <span
                      className="attestor-item__badge"
                      role="status"
                      aria-label={`${attestorRole(idx)} attestation confirmed`}
                    >
                      ✓ Attested
                    </span>
                  </li>
                ))}
              </ol>
            )}
          </div>
        </div>

        {/* Audit Trail */}
        <div className="detail-card" style={{ marginTop: '20px' }}>
          <div className="detail-card__header">
            <span className="detail-card__title">Audit Trail</span>
          </div>
          <div className="detail-card__body">
            <AuditTrail
              credential={credential}
              attestors={attestors}
              expired={isExpiredFlag}
            />
          </div>
        </div>
      </main>

      <footer className="footer">
        <div className="container">
          Powered by{' '}
          <a href="https://stellar.org" target="_blank" rel="noopener">Stellar Soroban</a>
          {' · '}
          <a href="https://github.com/Phantomcall/QuorumProof" target="_blank" rel="noopener">QuorumProof</a>
        </div>
      </footer>

      {showShare && id && (        <ShareCredentialDialog
          credentialId={id}
          onClose={() => setShowShare(false)}
        />
      )}
    </>
  );
}
