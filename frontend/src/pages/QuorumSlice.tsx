import { Navbar } from '../components/Navbar';
import { WalletGuard } from '../components/WalletGate';
import { QuorumSliceBuilder } from '../components/QuorumSliceBuilder';
import { useWallet } from '../hooks';

function formatAddress(addr: string) {
  if (!addr || addr.length < 10) return addr;
  return addr.slice(0, 8) + '…' + addr.slice(-6);
}

export default function QuorumSlice() {
  const { address } = useWallet();

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

          <div className="search-card">
            <div style={{ display: 'flex', justifyContent: 'space-between', alignItems: 'center', marginBottom: 24 }}>
              <span className="detail-card__title">Building as</span>
              <span className="wallet-pill" title={address!}>
                <span className="wallet-pill__dot" aria-hidden="true" />
                {formatAddress(address!)}
              </span>
            </div>
            <QuorumSliceBuilder creatorAddress={address!} />
          </div>
        </div>
      </main>
    </div>
  );
}
