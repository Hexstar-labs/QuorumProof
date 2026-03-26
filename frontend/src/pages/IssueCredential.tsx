import { Navbar } from '../components/Navbar';
import { IssueCredentialForm } from '../components/IssueCredentialForm';
import { useFreighter } from '../lib/hooks/useFreighter';

function formatAddress(addr: string) {
  if (!addr || addr.length < 10) return addr;
  return addr.slice(0, 8) + '…' + addr.slice(-6);
}

export function IssueCredential() {
  const { address, isInitializing, connect, hasFreighter } = useFreighter();

  return (
    <div id="app">
      <Navbar />
      <main className="dashboard-main">
        <div className="container" style={{ maxWidth: 600 }}>
          <div className="dashboard-header" style={{ marginBottom: 32 }}>
            <div>
              <h1 className="dashboard-title">Issue Credential</h1>
              <p className="dashboard-subtitle">
                Issue a verifiable on-chain credential to an engineer's Stellar address.
              </p>
            </div>
          </div>

          {isInitializing ? (
            <div className="loading-state">
              <div className="spinner" />
              <span>Connecting wallet…</span>
            </div>
          ) : !address ? (
            <div
              className="wallet-guard-card"
              style={{ margin: '0 auto' }}
              role="region"
              aria-label="Wallet connection required"
            >
              <div className="wallet-guard__icon">🔐</div>
              <h2 className="wallet-guard__title">Connect Your Wallet</h2>
              <p className="wallet-guard__sub">
                You must connect a Freighter wallet to issue credentials as an attestor.
              </p>
              <button className="btn btn--primary" onClick={connect}>
                {hasFreighter ? 'Connect Freighter' : 'Install Freighter'}
              </button>
            </div>
          ) : (
            <div className="search-card">
              <div className="detail-card__header" style={{ marginBottom: 24, padding: 0, background: 'none', border: 'none' }}>
                <span className="detail-card__title">Issuing as</span>
                <span
                  className="wallet-pill"
                  title={address}
                  aria-label={`Connected wallet: ${address}`}
                >
                  <span className="wallet-pill__dot" aria-hidden="true" />
                  {formatAddress(address)}
                </span>
              </div>
              <IssueCredentialForm issuerAddress={address} />
            </div>
          )}
        </div>
      </main>
    </div>
  );
}
