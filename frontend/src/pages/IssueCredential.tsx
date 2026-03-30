import { Navbar } from '../components/Navbar';
import { WalletGuard } from '../components/WalletGate';
import { IssueCredentialForm } from '../components/IssueCredentialForm';
import { useWallet } from '../hooks';

function formatAddress(addr: string) {
  if (!addr || addr.length < 10) return addr;
  return addr.slice(0, 8) + '…' + addr.slice(-6);
}

export default function IssueCredential() {
  const { address } = useWallet();

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

          <div className="search-card">
            <div className="detail-card__header" style={{ marginBottom: 24, padding: 0, background: 'none', border: 'none' }}>
              <span className="detail-card__title">Issuing as</span>
              <span
                className="wallet-pill"
                title={address!}
                aria-label={`Connected wallet: ${address}`}
              >
                <span className="wallet-pill__dot" aria-hidden="true" />
                {formatAddress(address!)}
              </span>
            </div>
            <IssueCredentialForm issuerAddress={address!} />
          </div>
        </div>
      </main>
    </div>
  );
}
