import React, { useState } from "react";

// ─── Types ────────────────────────────────────────────────────────────────────

interface NavItem {
  label: string;
  href: string;
  icon: React.ReactNode;
}

interface AppLayoutProps {
  /** Current pathname, e.g. "/dashboard" */
  currentPath: string;
  /** Connected Stellar wallet address (full G… address) */
  walletAddress?: string;
  children: React.ReactNode;
}

// ─── Icons (inline SVG — no extra dep) ───────────────────────────────────────

const IconDashboard = () => (
  <svg xmlns="http://www.w3.org/2000/svg" className="h-5 w-5" viewBox="0 0 24 24" fill="none"
    stroke="currentColor" strokeWidth={2} strokeLinecap="round" strokeLinejoin="round" aria-hidden="true">
    <rect x="3" y="3" width="7" height="7" /><rect x="14" y="3" width="7" height="7" />
    <rect x="14" y="14" width="7" height="7" /><rect x="3" y="14" width="7" height="7" />
  </svg>
);

const IconVerify = () => (
  <svg xmlns="http://www.w3.org/2000/svg" className="h-5 w-5" viewBox="0 0 24 24" fill="none"
    stroke="currentColor" strokeWidth={2} strokeLinecap="round" strokeLinejoin="round" aria-hidden="true">
    <path d="M12 22s8-4 8-10V5l-8-3-8 3v7c0 6 8 10 8 10z" />
    <polyline points="9 12 11 14 15 10" />
  </svg>
);

const IconSlice = () => (
  <svg xmlns="http://www.w3.org/2000/svg" className="h-5 w-5" viewBox="0 0 24 24" fill="none"
    stroke="currentColor" strokeWidth={2} strokeLinecap="round" strokeLinejoin="round" aria-hidden="true">
    <circle cx="12" cy="12" r="10" />
    <line x1="12" y1="8" x2="12" y2="16" />
    <line x1="8" y1="12" x2="16" y2="12" />
  </svg>
);

const IconSettings = () => (
  <svg xmlns="http://www.w3.org/2000/svg" className="h-5 w-5" viewBox="0 0 24 24" fill="none"
    stroke="currentColor" strokeWidth={2} strokeLinecap="round" strokeLinejoin="round" aria-hidden="true">
    <circle cx="12" cy="12" r="3" />
    <path d="M19.4 15a1.65 1.65 0 0 0 .33 1.82l.06.06a2 2 0 0 1-2.83 2.83l-.06-.06a1.65 1.65 0 0 0-1.82-.33
      1.65 1.65 0 0 0-1 1.51V21a2 2 0 0 1-4 0v-.09A1.65 1.65 0 0 0 9 19.4a1.65 1.65 0 0 0-1.82.33l-.06.06
      a2 2 0 0 1-2.83-2.83l.06-.06A1.65 1.65 0 0 0 4.68 15a1.65 1.65 0 0 0-1.51-1H3a2 2 0 0 1 0-4h.09
      A1.65 1.65 0 0 0 4.6 9a1.65 1.65 0 0 0-.33-1.82l-.06-.06a2 2 0 0 1 2.83-2.83l.06.06
      A1.65 1.65 0 0 0 9 4.68a1.65 1.65 0 0 0 1-1.51V3a2 2 0 0 1 4 0v.09a1.65 1.65 0 0 0 1 1.51
      1.65 1.65 0 0 0 1.82-.33l.06-.06a2 2 0 0 1 2.83 2.83l-.06.06A1.65 1.65 0 0 0 19.4 9
      a1.65 1.65 0 0 0 1.51 1H21a2 2 0 0 1 0 4h-.09a1.65 1.65 0 0 0-1.51 1z" />
  </svg>
);

const IconChevron = ({ collapsed }: { collapsed: boolean }) => (
  <svg xmlns="http://www.w3.org/2000/svg" className={`h-4 w-4 transition-transform ${collapsed ? "rotate-180" : ""}`}
    viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth={2}
    strokeLinecap="round" strokeLinejoin="round" aria-hidden="true">
    <polyline points="15 18 9 12 15 6" />
  </svg>
);

// ─── Helpers ──────────────────────────────────────────────────────────────────

/** Truncate a Stellar address: GABC...XYZ */
function truncateAddress(addr: string): string {
  if (addr.length <= 10) return addr;
  return `${addr.slice(0, 4)}...${addr.slice(-4)}`;
}

// ─── Nav items ────────────────────────────────────────────────────────────────

const NAV_ITEMS: NavItem[] = [
  { label: "Dashboard",          href: "/dashboard", icon: <IconDashboard /> },
  { label: "Verify Credential",  href: "/verify",    icon: <IconVerify /> },
  { label: "My Quorum Slice",    href: "/slice",     icon: <IconSlice /> },
  { label: "Settings",           href: "/settings",  icon: <IconSettings /> },
];

// ─── Component ────────────────────────────────────────────────────────────────

/**
 * AppLayout — responsive shell for QuorumProof pages.
 *
 * Breakpoints:
 *   mobile  (<md)  → top header + bottom navigation bar
 *   tablet  (md)   → collapsed icon-only sidebar
 *   desktop (lg+)  → full sidebar with labels
 */
export function AppLayout({ currentPath, walletAddress, children }: AppLayoutProps) {
  // Sidebar collapsed state (tablet icon-only mode)
  const [collapsed, setCollapsed] = useState<boolean>(false);

  const isActive = (href: string) => currentPath === href;

  // Shared nav link classes
  const linkBase =
    "flex items-center gap-3 rounded-lg px-3 py-2 text-sm font-medium transition-colors focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-indigo-500";
  const linkActive = "bg-indigo-600 text-white";
  const linkInactive = "text-slate-300 hover:bg-slate-700 hover:text-white";

  return (
    <div className="flex h-screen overflow-hidden bg-slate-900 text-slate-100">

      {/* ── Desktop / Tablet Sidebar ─────────────────────────────────────── */}
      <aside
        aria-label="Main navigation"
        className={`
          hidden md:flex flex-col border-r border-slate-700 bg-slate-800
          transition-all duration-200
          ${collapsed ? "w-16" : "w-56"}
        `}
      >
        {/* Logo + collapse toggle */}
        <div className="flex h-14 items-center justify-between px-3 border-b border-slate-700">
          {!collapsed && (
            <span className="text-base font-bold tracking-tight text-white">
              ⬡ QuorumProof
            </span>
          )}
          <button
            onClick={() => setCollapsed((c: boolean) => !c)}
            aria-label={collapsed ? "Expand sidebar" : "Collapse sidebar"}
            className="ml-auto rounded p-1 text-slate-400 hover:bg-slate-700 hover:text-white focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-indigo-500"
          >
            <IconChevron collapsed={collapsed} />
          </button>
        </div>

        {/* Nav links */}
        <nav aria-label="Sidebar navigation" className="flex-1 overflow-y-auto px-2 py-4 space-y-1">
          {NAV_ITEMS.map((item) => (
            <a
              key={item.href}
              href={item.href}
              aria-label={item.label}
              aria-current={isActive(item.href) ? "page" : undefined}
              className={`${linkBase} ${isActive(item.href) ? linkActive : linkInactive} ${collapsed ? "justify-center" : ""}`}
            >
              <span className="shrink-0">{item.icon}</span>
              {!collapsed && <span>{item.label}</span>}
            </a>
          ))}
        </nav>

        {/* Wallet address */}
        {walletAddress && (
          <div className="border-t border-slate-700 px-3 py-3">
            {collapsed ? (
              <div
                title={walletAddress}
                aria-label={`Connected wallet: ${walletAddress}`}
                className="flex justify-center"
              >
                <span className="h-2 w-2 rounded-full bg-emerald-400" aria-hidden="true" />
              </div>
            ) : (
              <div className="flex items-center gap-2">
                <span className="h-2 w-2 shrink-0 rounded-full bg-emerald-400" aria-hidden="true" />
                <span
                  className="truncate text-xs text-slate-400 font-mono"
                  title={walletAddress}
                  aria-label={`Connected wallet: ${walletAddress}`}
                >
                  {truncateAddress(walletAddress)}
                </span>
              </div>
            )}
          </div>
        )}
      </aside>

      {/* ── Main area ────────────────────────────────────────────────────── */}
      <div className="flex flex-1 flex-col overflow-hidden">

        {/* Mobile top header */}
        <header className="flex md:hidden h-14 items-center justify-between border-b border-slate-700 bg-slate-800 px-4">
          <span className="text-base font-bold tracking-tight text-white">⬡ QuorumProof</span>
          <div className="flex items-center gap-3">
            {walletAddress && (
              <span
                className="text-xs font-mono text-slate-400"
                title={walletAddress}
                aria-label={`Connected wallet: ${walletAddress}`}
              >
                {truncateAddress(walletAddress)}
              </span>
            )}
            {/* Wallet indicator dot */}
            {walletAddress && (
              <span className="h-2 w-2 rounded-full bg-emerald-400" aria-hidden="true" />
            )}
          </div>
        </header>

        {/* Page content */}
        <main
          id="main-content"
          tabIndex={-1}
          className="flex-1 overflow-y-auto p-4 md:p-6 pb-20 md:pb-6"
        >
          {children}
        </main>

        {/* Mobile bottom navigation */}
        <nav
          aria-label="Bottom navigation"
          className="md:hidden fixed bottom-0 inset-x-0 flex border-t border-slate-700 bg-slate-800"
        >
          {NAV_ITEMS.map((item) => (
            <a
              key={item.href}
              href={item.href}
              aria-label={item.label}
              aria-current={isActive(item.href) ? "page" : undefined}
              className={`
                flex flex-1 flex-col items-center justify-center gap-1 py-2 text-xs font-medium
                transition-colors focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-inset focus-visible:ring-indigo-500
                ${isActive(item.href) ? "text-indigo-400" : "text-slate-400 hover:text-white"}
              `}
            >
              {item.icon}
              <span>{item.label.split(" ")[0]}</span>
            </a>
          ))}
        </nav>
      </div>
    </div>
  );
}
