import { useState, useEffect } from 'react';
import { useNavigate } from 'react-router-dom';
import { Navbar } from '../components/Navbar';
import { CredentialCard } from '../components/CredentialCard';
import { CredentialCardSkeleton } from '../components/CredentialCardSkeleton';
import { EmptyState } from '../components/EmptyState';
import { useWallet } from '../hooks';
import {
  getCredentialsBySubject,
  getCredential,
  isAttested,
  getSlice,
  isExpired,
} from '../stellar';
import { formatAddress } from '../lib/credentialUtils';
import type { CredCardData } from '../lib/credentialUtils';

export default function Profile() {
  const { address } = useWallet();
  const navigate = useNavigate();
  const [cards, setCards] = useState<CredCardData[]>([]);
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    if (!address) return;

    (async () => {
      setLoading(true);
      setError(null);

      const sliceIdRaw = localStorage.getItem('qp-slice-id');
      const sliceId = sliceIdRaw ? BigInt(sliceIdRaw) : null;

      try {
        const ids: bigint[] = await getCredentialsBySubject(address);

        if (!ids || ids.length === 0) {
          setCards([]);
          return;
        }

        const results = await Promise.all(
          ids.map(async (id): Promise<CredCardData> => {
            try {
              const [credential, expired] = await Promise.all([
                getCredential(id),
                isExpired(id).catch(() => false),
              ]);

              let attested = false;
              let slice = null;
              let sliceError = false;

              if (sliceId !== null) {
                attested = await isAttested(id, sliceId).catch(() => false);
                try { slice = await getSlice(sliceId); } catch { sliceError = true; }
              }

              return { credential, attested, slice, expired, sliceError, credError: null };
            } catch (err) {
              return {
                credential: {
                  id,
                  subject: '',
                  issuer: '',
                  credential_type: 0,
                  metadata_hash: new Uint8Array(),
                  revoked: false,
                  expires_at: null,
                },
                attested: false,
                slice: null,
                expired: false,
                sliceError: false,
                credError: err instanceof Error ? err.message : 'Failed to load',
              };
            }
          })
        );

        setCards(results);
      } catch (err) {
        setError(err instanceof Error ? err.message : 'Failed to load credentials.');
      } finally {
        setLoading(false);
      }
    })();
  }, [address]);

  const sliceId = (() => {
    const raw = localStorage.getItem('qp-slice-id');
    return raw ? BigInt(raw) : null;
  })();

  const attested = cards.filter((c) => c.attested && !c.credential.revoked).length;
  const revoked  = cards.filter((c) => c.credential.revoked).length;

  return (
    <>
      <Navbar />
      <main className="container" style={{ paddingTop: '40px', maxWidth: '900px', paddingBottom: '64px' }}>

        {/* Profile header */}
        <div className="profile-header">
          <div className="profile-avatar" aria-hidden="true">
            {address ? address.slice(0, 2).toUpperCase() : '??'}
          </div>
          <div className="profile-info">
            <h1 className="profile-name">Credential Holder</h1>
            {address ? (
              <p className="profile-address mono" title={address}>{formatAddress(address)}</p>
            ) : (
              <p className="profile-address" style={{ color: 'var(--text-muted)' }}>No wallet connected</p>
            )}
          </div>
        </div>

        {/* Stats */}
        {!loading && cards.length > 0 && (
          <div className="profile-stats" role="list" aria-label="Credential statistics">
            <div className="profile-stat" role="listitem">
              <span className="profile-stat__value">{cards.length}</span>
              <span className="profile-stat__label">Total</span>
            </div>
            <div className="profile-stat" role="listitem">
              <span className="profile-stat__value" style={{ color: 'var(--green)' }}>{attested}</span>
              <span className="profile-stat__label">Attested</span>
            </div>
            <div className="profile-stat" role="listitem">
              <span className="profile-stat__value" style={{ color: 'var(--red)' }}>{revoked}</span>
              <span className="profile-stat__label">Revoked</span>
            </div>
          </div>
        )}

        {/* Credential list */}
        <section aria-label="Credentials">
          <h2 className="profile-section-title">Credentials</h2>

          {!address && (
            <div className="error-card">
              <div className="error-card__icon">🔌</div>
              <div>
                <div className="error-card__title">Wallet not connected</div>
                <div className="error-card__msg">Connect your wallet to view your credentials.</div>
                <button
                  className="btn btn--ghost btn--sm"
                  style={{ marginTop: '12px' }}
                  onClick={() => navigate('/dashboard')}
                >
                  Go to Dashboard
                </button>
              </div>
            </div>
          )}

          {address && loading && (
            <div className="dashboard-grid">
              {[1, 2, 3].map((i) => <CredentialCardSkeleton key={i} />)}
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
            <EmptyState address={address} />
          )}

          {address && !loading && !error && cards.length > 0 && (
            <div className="dashboard-grid">
              {cards.map((card) => (
                <CredentialCard
                  key={card.credential.id.toString()}
                  data={card}
                  sliceId={sliceId}
                />
              ))}
            </div>
          )}
        </section>
      </main>

      <footer className="footer">
        <div className="container">
          Powered by{' '}
          <a href="https://stellar.org" target="_blank" rel="noopener">Stellar Soroban</a>
          {' · '}
          <a href="https://github.com/Phantomcall/QuorumProof" target="_blank" rel="noopener">QuorumProof</a>
        </div>
      </footer>
    </>
  );
}
