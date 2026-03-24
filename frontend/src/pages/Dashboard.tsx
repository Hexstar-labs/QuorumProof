import { useState, useEffect } from 'react';
import { useNavigate } from 'react-router-dom';
import { Navbar } from '../components/Navbar';
import { useFreighter } from '../lib/hooks/useFreighter';
import {
  getCredentialsBySubject,
  getCredential,
  isExpired,
  getAttestors,
  decodeMetadataHash,
} from '../stellar';

const CREDENTIAL_TYPES: Record<number, string> = {
  1: '🎓 Degree',
  2: '🏛️ License',
  3: '💼 Employment',
  4: '📜 Certification',
  5: '🔬 Research',
};

function credTypeLabel(n: number | bigint) {
  return CREDENTIAL_TYPES[Number(n)] || `Type ${n}`;
}

function formatTimestamp(ts: number | bigint | null | undefined) {
  if (!ts) return 'Never';
  const d = new Date(Number(ts) * 1000);
  return d.toLocaleDateString(undefined, { year: 'numeric', month: 'short', day: 'numeric' });
}

function formatAddress(addr: string) {
  if (!addr || addr.length < 10) return addr || '—';
  return addr.slice(0, 8) + '…' + addr.slice(-6);
}

interface CredentialData {
  id: bigint;
  subject: string;
  issuer: string;
  credential_type: number;
  metadata_hash: Uint8Array;
  revoked: boolean;
  expires_at: bigint | null;
}

interface CredCard {
  credential: CredentialData;
  attestors: string[];
  expired: boolean;
}

function CredentialCard({ card }: { card: CredCard }) {
  const navigate = useNavigate();
  const { credential, attestors, expired } = card;
  const metaStr = decodeMetadataHash(credential.metadata_hash);

  let statusClass: string, statusLabel: string, statusIcon: string;
  if (credential.revoked) {
    statusClass = 'revoked'; statusIcon = '🚫'; statusLabel = 'Revoked';
  } else if (expired) {
    statusClass = 'expired'; statusIcon = '⏰'; statusLabel = 'Expired';
  } else if (attestors.length === 0) {
    statusClass = 'pending'; statusIcon = '⏳'; statusLabel = 'Pending Attestation';
  } else {
    statusClass = 'valid'; statusIcon = '✅'; statusLabel = 'Attested';
  }

  return (
    <div className="cred-card">
      <div className={`cred-card__header cred-card__header--${statusClass}`}>
        <div className="cred-card__type">{credTypeLabel(credential.credential_type)}</div>
        <div className={`badge badge--${statusClass}`}>{statusIcon} {statusLabel}</div>
      </div>
      <div className="cred-card__body">
        <h3 className="cred-card__id">Credential #{credential.id.toString()}</h3>
        <div className="cred-card__meta">
          <div className="meta-row">
            <span className="meta-label">Issuer</span>
            <span className="meta-value mono" title={credential.issuer}>{formatAddress(credential.issuer)}</span>
          </div>
          <div className="meta-row">
            <span className="meta-label">Metadata</span>
            <span className="meta-value mono">{metaStr}</span>
          </div>
          {credential.expires_at && (
            <div className="meta-row">
              <span className="meta-label">Expires</span>
              <span className="meta-value">{formatTimestamp(credential.expires_at)}</span>
            </div>
          )}
        </div>
        <div className="cred-card__attestors">
          <div className="attestors-header">
            <span className="meta-label">Quorum Slice Attestors</span>
            <span className={`badge badge--${attestors.length > 0 ? 'gray' : 'red'}`} style={{ fontSize: '10px' }}>
              {attestors.length} Node{attestors.length !== 1 ? 's' : ''}
            </span>
          </div>
          {attestors.length === 0 ? (
            <div className="attestors-empty">Awaiting slice signatures</div>
          ) : (
            <div className="attestor-mini-list">
              {attestors.map((addr) => (
                <div key={addr} className="attestor-mini-item">
                  <span>🏛️</span>
                  <span className="mono" title={addr}>{formatAddress(addr)}</span>
                </div>
              ))}
            </div>
          )}
        </div>
      </div>
      <div className="cred-card__footer">
        <button
          className="btn btn--sm btn--ghost"
          style={{ width: '100%' }}
          onClick={() => navigate(`/verify?credentialId=${credential.id}`)}
        >
          View Public Page →
        </button>
      </div>
    </div>
  );
}

export function Dashboard() {
  const { address, isInitializing, connect } = useFreighter();
  const [cards, setCards] = useState<CredCard[]>([]);
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    if (!address) return;
    setLoading(true);
    setError(null);

    const fetchAll = async () => {
      try {
        const ids: bigint[] = await getCredentialsBySubject(address);
        if (!ids || ids.length === 0) {
          setCards([]);
          return;
        }
        const results = await Promise.all(
          ids.map(async (id) => {
            const [credential, attestors, expired] = await Promise.all([
              getCredential(id),
              getAttestors(id),
              isExpired(id).catch(() => false),
            ]);
            return { credential, attestors, expired } as CredCard;
          })
        );
        setCards(results);
      } catch (err: unknown) {
        setError(err instanceof Error ? err.message : 'Failed to load credentials.');
      } finally {
        setLoading(false);
      }
    };

    fetchAll();
  }, [address]);

  return (
    <>
      <Navbar />
      <main className="container container--wide dashboard-main">
        <header className="dashboard-header">
          <div>
            <h1 className="dashboard-title">Credential Dashboard</h1>
            <p className="dashboard-subtitle">Manage and track your verifiable credentials</p>
          </div>

          {/* Wallet state */}
          {!isInitializing && !address && (
            <div className="wallet-sim-card">
              <div className="wallet-sim__label">Connect your Freighter wallet to view credentials</div>
              <button className="btn btn--primary" onClick={connect}>Connect Wallet</button>
            </div>
          )}
          {address && (
            <div className="wallet-sim-card">
              <div className="wallet-sim__label">Connected Address</div>
              <div className="mono" style={{ fontSize: '13px', wordBreak: 'break-all' }}>{address}</div>
            </div>
          )}
        </header>

        <div className="dashboard-content">
          {!address && !isInitializing && (
            <div className="empty-state">
              <div className="empty-state__icon">👛</div>
              <div className="empty-state__title">No wallet connected</div>
              <p>Connect your Freighter wallet to view your credentials.</p>
            </div>
          )}

          {isInitializing && (
            <div className="loading-state">
              <div className="spinner" />
              <p>Checking wallet…</p>
            </div>
          )}

          {address && loading && (
            <div className="loading-state">
              <div className="spinner" />
              <p>Loading your credentials…</p>
            </div>
          )}

          {address && !loading && error && (
            <div className="error-card">
              <div className="error-card__icon">⚠️</div>
              <div>
                <div className="error-card__title">Could Not Load Credentials</div>
                <div className="error-card__msg">{error}</div>
              </div>
            </div>
          )}

          {address && !loading && !error && cards.length === 0 && (
            <div className="empty-state" style={{ marginTop: 48, border: '1px dashed var(--border)', borderRadius: 'var(--radius-lg)' }}>
              <div className="empty-state__icon">📭</div>
              <div className="empty-state__title">No credentials found</div>
              <p>You haven't been issued any credentials yet.</p>
            </div>
          )}

          {cards.length > 0 && (
            <div className="dashboard-grid">
              {cards.map((card) => (
                <CredentialCard key={card.credential.id.toString()} card={card} />
              ))}
            </div>
          )}
        </div>
      </main>
      <footer className="footer">
        <div className="container">
          Powered by <a href="https://stellar.org" target="_blank" rel="noopener">Stellar Soroban</a>
          {' · '}
          <a href="https://github.com/Phantomcall/QuorumProof" target="_blank" rel="noopener">QuorumProof</a>
        </div>
      </footer>
    </>
  );
}
