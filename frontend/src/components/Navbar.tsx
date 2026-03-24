import { Link, useLocation } from 'react-router-dom';
import { useFreighter } from '../lib/hooks/useFreighter';

const NETWORK = import.meta.env.VITE_STELLAR_NETWORK || 'testnet';

function formatAddress(addr: string) {
  if (!addr || addr.length < 10) return addr;
  return addr.slice(0, 8) + '…' + addr.slice(-6);
}

export function Navbar() {
  const location = useLocation();
  const { address, isInitializing, connect, disconnect } = useFreighter();

  return (
    <nav className="navbar">
      <div className="container navbar__inner">
        <Link to="/dashboard" className="navbar__logo">
          <div className="navbar__logo-icon">⬡</div>
          QuorumProof
        </Link>

        <div className="navbar__links">
          <Link
            to="/dashboard"
            className={`nav-link${location.pathname === '/dashboard' ? ' active' : ''}`}
          >
            Dashboard
          </Link>
          <Link
            to="/verify"
            className={`nav-link${location.pathname === '/verify' ? ' active' : ''}`}
          >
            Verify
          </Link>
        </div>

        <div className="navbar__right">
          <span className="navbar__badge">{NETWORK}</span>
          {isInitializing ? (
            <span className="navbar__badge" style={{ opacity: 0.5 }}>Connecting…</span>
          ) : address ? (
            <div style={{ display: 'flex', alignItems: 'center', gap: '8px' }}>
              <span className="navbar__badge" title={address} style={{ fontFamily: 'var(--font-mono)', fontSize: '12px' }}>
                {formatAddress(address)}
              </span>
              <button className="btn btn--ghost" style={{ padding: '4px 10px', fontSize: '12px' }} onClick={disconnect}>
                Disconnect
              </button>
            </div>
          ) : (
            <button className="btn btn--primary" style={{ padding: '6px 14px', fontSize: '13px' }} onClick={connect}>
              Connect Wallet
            </button>
          )}
        </div>
      </div>
    </nav>
  );
}
