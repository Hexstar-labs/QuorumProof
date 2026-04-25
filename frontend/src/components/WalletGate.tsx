import { ReactNode } from 'react';
import { useWallet } from '../hooks';

interface WalletGateProps {
  hasFreighter: boolean;
  connect: () => Promise<void>;
}

/** Legacy display-only prompt — kept for backward compatibility */
export function WalletGate({ hasFreighter, connect }: WalletGateProps) {
  return (
    <div className="wallet-guard-card" role="region" aria-label="Wallet connection required">
      <div className="wallet-guard__icon">🔐</div>
      <h2 className="wallet-guard__title">Connect your Stellar wallet to continue</h2>

      {hasFreighter ? (
        <>
          <p className="wallet-guard__sub">
            Connect your Freighter wallet to access this page.
          </p>
          <button className="btn btn--primary" onClick={connect}>
            Connect Wallet
          </button>
        </>
      ) : (
        <>
          <p className="wallet-guard__sub">
            Freighter is not installed. Install it to use QuorumProof.
          </p>
          <a
            href="https://freighter.app"
            target="_blank"
            rel="noopener noreferrer"
            className="btn btn--primary"
          >
            Install Freighter
          </a>
        </>
      )}
    </div>
  );
}

interface WalletGuardProps {
  children: ReactNode;
}

/**
 * WalletGuard — wrap any page that requires a connected wallet.
 * Shows an onboarding prompt when Freighter is absent or not connected.
 */
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

  if (!address) {
    return (
      <div className="wallet-guard-card" role="region" aria-label="Wallet connection required">
        <div className="wallet-guard__icon">🔐</div>
        <h2 className="wallet-guard__title">Connect your Stellar wallet to continue</h2>

        {hasFreighter ? (
          <>
            <p className="wallet-guard__sub">
              Connect your Freighter wallet to access this page.
            </p>
            <button className="btn btn--primary" onClick={connect}>
              Connect Wallet
            </button>
          </>
        ) : (
          <>
            <p className="wallet-guard__sub">
              Freighter is not installed. Install it to use QuorumProof.
            </p>
            <a
              href="https://freighter.app"
              target="_blank"
              rel="noopener noreferrer"
              className="btn btn--primary"
            >
              Install Freighter
            </a>
          </>
        )}
      </div>
    );
  }

  return <>{children}</>;
}
