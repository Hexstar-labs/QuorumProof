import React from "react";

// ─── Types ────────────────────────────────────────────────────────────────────

interface AppLayoutProps {
  /** Current pathname, e.g. "/dashboard" */
  currentPath: string;
  /** Connected Stellar wallet address (full G… address) */
  walletAddress?: string;
  /** Function to connect wallet */
  onConnectWallet?: () => void;
  /** Current network */
  network?: string;
  children: React.ReactNode;
}

// ─── Helpers ──────────────────────────────────────────────────────────────────

/** Truncate a Stellar address: GABC...XYZ */
function truncateAddress(addr: string): string {
  if (addr.length <= 10) return addr;
  return `${addr.slice(0, 4)}...${addr.slice(-4)}`;
}

// ─── Component ────────────────────────────────────────────────────────────────

/**
 * AppLayout — shared layout with top nav bar.
 */
export function AppLayout({ currentPath, walletAddress, onConnectWallet, network = "Testnet", children }: AppLayoutProps) {
  const isActive = (href: string) => currentPath === href;

  return (
    <div className="flex flex-col h-screen bg-slate-900 text-slate-100">
      {/* Top Nav Bar */}
      <header className="flex items-center justify-between h-14 px-4 border-b border-slate-700 bg-slate-800">
        {/* Logo */}
        <div className="text-base font-bold tracking-tight text-white">
          ⬡ QuorumProof
        </div>

        {/* Nav Links - hidden on mobile, show on md+ */}
        <nav className="hidden md:flex space-x-4">
          <a href="/dashboard" className={`px-3 py-2 rounded ${isActive('/dashboard') ? 'bg-indigo-600 text-white' : 'text-slate-300 hover:bg-slate-700'}`}>
            Dashboard
          </a>
          <a href="/verify" className={`px-3 py-2 rounded ${isActive('/verify') ? 'bg-indigo-600 text-white' : 'text-slate-300 hover:bg-slate-700'}`}>
            Verify
          </a>
          <a href="/slice/new" className={`px-3 py-2 rounded ${isActive('/slice/new') ? 'bg-indigo-600 text-white' : 'text-slate-300 hover:bg-slate-700'}`}>
            New Slice
          </a>
        </nav>

        {/* Wallet and Network */}
        <div className="flex items-center space-x-4">
          <span className="text-sm text-slate-400">{network}</span>
          {walletAddress ? (
            <span className="text-sm font-mono text-slate-300">{truncateAddress(walletAddress)}</span>
          ) : (
            <button
              onClick={onConnectWallet}
              className="px-3 py-1 bg-indigo-600 text-white rounded hover:bg-indigo-700"
            >
              Connect Wallet
            </button>
          )}
        </div>
      </header>

      {/* Main Content */}
      <main className="flex-1 overflow-y-auto p-4">
        {children}
      </main>
    </div>
  );
}
