import { Navbar } from '../components/Navbar';
import { QuorumSliceBuilder } from '../components/QuorumSliceBuilder';
import { useFreighter } from '../lib/hooks/useFreighter';

function formatAddress(addr: string) {
  if (!addr || addr.length < 10) return addr;
  return addr.slice(0, 8) + '…' + addr.slice(-6);
}

export default function QuorumSlice() {
  const { address, isInitializing, connect, hasFreighter } = useFreighter();

  return (
    <div id="app">
      <Navbar />
      <main className="dashboard-main">
        <div className="container" style={{ maxWidth: 600 }}>
          <div className="dashboard-header" style={{ marginBottom: 32 }}>
            <div>
              <h1 className="dashboard-title">Quorum Slice Builder</h1>
              <p className="dashboard-subtitle">
                Compose your attestor quorum, set the threshold, and deploy the slice on-chain.
              </p>
            </div>
          </div>

          {isInitializing ? (
            <div className="loading-state">
              <div className="spinner" />
              <span>Connecting wallet…</span>
            </div>
          ) : !address ? (
            <div className="wallet-guard-card" style={{ margin: '0 auto' }}>
              <div className="wallet-guard__icon">🔐</div>
              <h2 className="wallet-guard__title">Connect Your Wallet</h2>
              <p className="wallet-guard__sub">
                You need a connected Freighter wallet to create a quorum slice on-chain.
              </p>
              <button className="btn btn--primary" onClick={connect}>
                {hasFreighter ? 'Connect Freighter' : 'Install Freighter'}
              </button>
            </div>
          ) : (
            <div className="search-card">
              <div style={{ display: 'flex', justifyContent: 'space-between', alignItems: 'center', marginBottom: 24 }}>
                <span className="detail-card__title">Building as</span>
                <span className="wallet-pill" title={address}>
                  <span className="wallet-pill__dot" aria-hidden="true" />
                  {formatAddress(address)}
                </span>
              </div>
              <QuorumSliceBuilder creatorAddress={address} />
            </div>
          )}
        </div>
      </main>
    </div>
  );
}
