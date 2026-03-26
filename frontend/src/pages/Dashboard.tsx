import { useState, useEffect, useCallback } from 'react';
import { Navbar } from '../components/Navbar';
import { WalletGate } from '../components/WalletGate';
import { CredentialCard } from '../components/CredentialCard';
import { EmptyState } from '../components/EmptyState';
import { useWallet } from '../hooks';
import {
  getCredentialsBySubject,
  getCredential,
  isAttested,
  getAttestors,
  getSlice,
  isExpired,
} from '../stellar';
import { type CredCardData } from '../lib/credentialUtils';

export default function Dashboard() {
  const { address, hasFreighter, isInitializing, connect, disconnect } = useWallet();
  const [cards, setCards] = useState<CredCardData[]>([]);
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [retryKey, setRetryKey] = useState(0);

  const fetchCredentials = useCallback(async (walletAddress: string) => {
    setLoading(true);
    setError(null);
    setCards([]);

    const sliceIdRaw = localStorage.getItem('qp-slice-id');
    const sliceId = sliceIdRaw ? BigInt(sliceIdRaw) : null;

    try {
      const ids: bigint[] = await getCredentialsBySubject(walletAddress);

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
              attested = await isAttested(id, sliceId).catch((err) => {
                console.error(`isAttested failed for credential ${id}:`, err);
                return false;
              });
              try {
                slice = await getSlice(sliceId);
              } catch (err) {
                console.error(`getSlice failed for slice ${sliceId}:`, err);
                sliceError = true;
              }
            } else {
              const attestors: string[] = await getAttestors(id).catch(() => []);
              attested = attestors.length > 0;
            }

            return { credential, attested, slice, expired, sliceError, credError: null };
          } catch (err) {
            // Per-card error — return a placeholder so the grid still renders
            const msg = err instanceof Error ? err.message : 'Failed to load credential';
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
              credError: msg,
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
  }, []);

  useEffect(() => {
    if (!address) return;
    fetchCredentials(address);
  }, [address, retryKey, fetchCredentials]);

  const sliceIdRaw = localStorage.getItem('qp-slice-id');
  const sliceId = sliceIdRaw ? BigInt(sliceIdRaw) : null;

  return (
    <>
      <Navbar />
      <main className="container container--wide dashboard-main">
        <header className="dashboard-header">
          <div>
            <h1 className="dashboard-title">Credential Dashboard</h1>
            <p className="dashboard-subtitle">Your verifiable credentials on Stellar Soroban</p>
          </div>
          {address && (
            <div className="wallet-sim-card">
              <div className="wallet-sim__label">Connected Address</div>
              <div className="mono" style={{ fontSize: '12px', wordBreak: 'break-all' }}>
                {address}
              </div>
              <button
                className="btn btn--ghost btn--sm"
                style={{ marginTop: '8px' }}
                onClick={disconnect}
              >
                Disconnect
              </button>
            </div>
          )}
        </header>

        <div className="dashboard-content">
          {/* Wallet initializing */}
          {isInitializing && (
            <div className="loading-state">
              <div className="spinner" />
              <p>Checking wallet…</p>
            </div>
          )}

          {/* No wallet connected */}
          {!isInitializing && !address && (
            <WalletGate hasFreighter={hasFreighter} connect={connect} />
          )}

          {/* Loading credentials */}
          {address && loading && (
            <div className="loading-state">
              <div className="spinner" />
              <p>Loading your credentials…</p>
            </div>
          )}

          {/* Top-level fetch error */}
          {address && !loading && error && (
            <div className="error-card">
              <div className="error-card__icon">⚠️</div>
              <div>
                <div className="error-card__title">Could Not Load Credentials</div>
                <div className="error-card__msg">{error}</div>
                <button
                  className="btn btn--ghost btn--sm"
                  style={{ marginTop: '12px' }}
                  onClick={() => setRetryKey((k: number) => k + 1)}
                >
                  Retry
                </button>
              </div>
            </div>
          )}

          {/* Empty state */}
          {address && !loading && !error && cards.length === 0 && (
            <EmptyState address={address} />
          )}

          {/* Credential grid */}
          {address && !loading && !error && cards.length > 0 && (
            <div className="dashboard-grid">
              {cards.map((card: CredCardData) => (
                <CredentialCard
                  key={card.credential.id.toString()}
                  data={card}
                  sliceId={sliceId}
                />
              ))}
            </div>
          )}
        </div>
      </main>

      <footer className="footer">
        <div className="container">
          Powered by{' '}
          <a href="https://stellar.org" target="_blank" rel="noopener">
            Stellar Soroban
          </a>{' '}
          ·{' '}
          <a
            href="https://github.com/Phantomcall/QuorumProof"
            target="_blank"
            rel="noopener"
          >
            QuorumProof
          </a>
        </div>
      </footer>
    </>
  );
}
