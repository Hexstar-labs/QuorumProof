import { ReactNode } from 'react';
import { useWallet } from '../hooks';

interface WalletGuardProps {
  children: ReactNode;
}

export function WalletGuard({ children }: WalletGuardProps) {
  const { address, hasFreighter, isInitializing, connect } = useWallet();

  if (isInitializing) {
    return (
      <div className="loading-state">
        <div className="spinner" />
        <p>Checking wallet…</p>
      </div>
    );
  }

  if (!hasFreighter) {
    return (
      <div className="wallet-guard-card">
        <div className="wallet-guard__icon">🔐</div>
        <h2 className="wallet-guard__title">Freighter Required</h2>
        <p className="wallet-guard__sub">
          Install the Freighter browser extension to use this feature.
        </p>
        <a
          href="https://freighter.app"
          target="_blank"
          rel="noopener noreferrer"
          className="btn btn--primary"
        >
          Install Freighter
        </a>
      </div>
    );
  }

  if (!address) {
    return (
      <div className="wallet-guard-card">
        <div className="wallet-guard__icon">🔐</div>
        <h2 className="wallet-guard__title">Connect Your Stellar Wallet</h2>
        <p className="wallet-guard__sub">
          Connect your Stellar wallet to continue.
        </p>
        <button className="btn btn--primary" onClick={connect}>
          Connect Wallet
        </button>
      </div>
    );
  }

  return <>{children}</>;
}
