/**
 * dashboard.js — Credential Dashboard Page Logic
 *
 * Handles the /dashboard route:
 *  - Simulates a connected wallet (via input address)
 *  - Fetches and displays all credentials issued to that address
 *  - Displays attestation status and attestor lists for each credential
 */

import { navigateTo } from './main.js';
import {
  getCredentialsBySubject,
  getCredential,
  isExpired,
  getAttestors,
  decodeMetadataHash,
  NETWORK,
} from './stellar.js';

// ── Credential type labels ──────────────────────────────────────────────────
const CREDENTIAL_TYPES = {
  1: '🎓 Degree',
  2: '🏛️ License',
  3: '💼 Employment',
  4: '📜 Certification',
  5: '🔬 Research',
};

function credTypeLabel(n) {
  return CREDENTIAL_TYPES[Number(n)] || `Type ${n}`;
}

// ── Format helpers ────────────────────────────────────────────────────────
function formatTimestamp(ts) {
  if (!ts) return 'Never';
  const d = new Date(Number(ts) * 1000);
  return d.toLocaleDateString(undefined, {
    year: 'numeric', month: 'short', day: 'numeric',
  });
}

function formatAddress(addr) {
  if (!addr || addr.length < 10) return addr || '—';
  return addr.slice(0, 8) + '…' + addr.slice(-6);
}

// ── Shared navbar builder ────────────────────────────────────────────────
export function renderNavbar(activeRoute) {
  return `
    <nav class="navbar">
      <div class="container navbar__inner">
        <a href="/dashboard" class="navbar__logo" data-route="/dashboard">
          <div class="navbar__logo-icon">⬡</div>
          QuorumProof
        </a>
        <div class="navbar__links">
          <a href="/dashboard" class="nav-link ${activeRoute === '/dashboard' ? 'active' : ''}" data-route="/dashboard">Dashboard</a>
          <a href="/verify" class="nav-link ${activeRoute === '/verify' ? 'active' : ''}" data-route="/verify">Verify</a>
        </div>
        <div class="navbar__right">
          <span class="navbar__badge">${NETWORK}</span>
        </div>
      </div>
    </nav>
  `;
}

export function bindNavbarLinks(container) {
  container.querySelectorAll('a[data-route]').forEach(link => {
    link.addEventListener('click', (e) => {
      if (e.ctrlKey || e.metaKey || e.shiftKey) return;
      e.preventDefault();
      navigateTo(link.dataset.route);
    });
  });
}

// ── Main Dashboard Render ──────────────────────────────────────────────────
export function renderDashboardPage(container) {
  // We use local storage to simulate a "connected wallet"
  const savedAddress = localStorage.getItem('qp-wallet-address') || '';

  container.innerHTML = `
    ${renderNavbar('/dashboard')}

    <main class="container container--wide dashboard-main">
      <header class="dashboard-header">
        <div>
          <h1 class="dashboard-title">Credential Dashboard</h1>
          <p class="dashboard-subtitle">Manage and track your verifiable credentials</p>
        </div>
        
        <!-- Wallet Connection Simulation -->
        <div class="wallet-sim-card">
          <div class="wallet-sim__label">Connected Wallet (Simulated)</div>
          <div class="input-group">
            <div class="input-wrap">
              <span class="input-icon">G</span>
              <input id="input-wallet" type="text" placeholder="Enter your Stellar address (GABC…)" value="${savedAddress}" />
            </div>
            <button class="btn btn--primary" id="btn-connect">
              ${savedAddress ? 'Switch Wallet' : 'Connect'}
            </button>
            ${savedAddress ? '<button class="btn btn--ghost" id="btn-disconnect">Disconnect</button>' : ''}
          </div>
        </div>
      </header>

      <div id="dashboard-content" class="dashboard-content">
        ${savedAddress ? `
          <div class="loading-state">
            <div class="spinner"></div>
            <p>Loading your credentials…</p>
          </div>
        ` : `
          <div class="empty-state">
            <div class="empty-state__icon">👛</div>
            <div class="empty-state__title">No wallet connected</div>
            <p>Connect your Stellar address to view your credentials.</p>
          </div>
        `}
      </div>
    </main>

    <footer class="footer">
      <div class="container">
        Powered by <a href="https://stellar.org" target="_blank" rel="noopener">Stellar Soroban</a>
        · <a href="https://github.com/Phantomcall/QuorumProof" target="_blank" rel="noopener">QuorumProof</a>
      </div>
    </footer>
  `;

  bindNavbarLinks(container);

  const btnConnect = container.querySelector('#btn-connect');
  const btnDisconnect = container.querySelector('#btn-disconnect');
  const inputWallet = container.querySelector('#input-wallet');

  btnConnect.addEventListener('click', () => {
    const addr = inputWallet.value.trim();
    if (!addr.startsWith('G') || addr.length < 56) {
      alert('Please enter a valid Stellar address (starts with G, 56+ characters).');
      return;
    }
    localStorage.setItem('qp-wallet-address', addr);
    renderDashboardPage(container); // Re-render page
  });

  inputWallet.addEventListener('keydown', (e) => {
    if (e.key === 'Enter') btnConnect.click();
  });

  if (btnDisconnect) {
    btnDisconnect.addEventListener('click', () => {
      localStorage.removeItem('qp-wallet-address');
      renderDashboardPage(container);
    });
  }

  // Load credentials if connected
  if (savedAddress) {
    loadCredentials(savedAddress, container.querySelector('#dashboard-content'));
  }
}

// ── Load and Render Credentials Grid ─────────────────────────────────────────
async function loadCredentials(address, contentEl) {
  try {
    const ids = await getCredentialsBySubject(address);

    if (!ids || ids.length === 0) {
      contentEl.innerHTML = `
        <div class="empty-state" style="margin-top: 48px; border: 1px dashed var(--border); border-radius: var(--radius-lg);">
          <div class="empty-state__icon">📭</div>
          <div class="empty-state__title">No credentials found</div>
          <p>You haven't been issued any credentials yet.</p>
        </div>
      `;
      return;
    }

    contentEl.innerHTML = `
      <div class="dashboard-grid">
        ${ids.map(id => `<div id="cred-card-${id}" class="cred-card skeleton-loader"></div>`).join('')}
      </div>
    `;

    // Fetch details for all credentials in parallel
    await Promise.all(ids.map(id => renderDashboardCard(id, document.getElementById(`cred-card-${id}`))));

  } catch (err) {
    contentEl.innerHTML = `
      <div class="error-card">
        <div class="error-card__icon">⚠️</div>
        <div>
          <div class="error-card__title">Could Not Load Credentials</div>
          <div class="error-card__msg">${err.message || 'Failed to fetch credentials from the network.'}</div>
        </div>
      </div>
    `;
  }
}

async function renderDashboardCard(credId, cardEl) {
  try {
    const [credential, attestors, expired] = await Promise.all([
      getCredential(credId),
      getAttestors(credId),
      isExpired(credId).catch(() => false),
    ]);

    const isRevoked = credential.revoked;
    const metaStr = decodeMetadataHash(credential.metadata_hash);
    
    let statusClass, statusLabel, statusIcon;
    if (isRevoked) {
      statusClass = 'revoked'; statusIcon = '🚫'; statusLabel = 'Revoked';
    } else if (expired) {
      statusClass = 'expired'; statusIcon = '⏰'; statusLabel = 'Expired';
    } else if (attestors.length === 0) {
      statusClass = 'pending'; statusIcon = '⏳'; statusLabel = 'Pending Attestation';
    } else {
      statusClass = 'valid'; statusIcon = '✅'; statusLabel = 'Attested';
    }

    // Remove skeleton class
    cardEl.classList.remove('skeleton-loader');
    
    cardEl.innerHTML = `
      <div class="cred-card__header cred-card__header--${statusClass}">
        <div class="cred-card__type">${credTypeLabel(credential.credential_type)}</div>
        <div class="badge badge--${statusClass}">
          ${statusIcon} ${statusLabel}
        </div>
      </div>
      
      <div class="cred-card__body">
        <h3 class="cred-card__id">Credential #${credential.id}</h3>
        
        <div class="cred-card__meta">
          <div class="meta-row">
            <span class="meta-label">Issuer</span>
            <span class="meta-value mono" title="${credential.issuer}">${formatAddress(credential.issuer)}</span>
          </div>
          <div class="meta-row">
            <span class="meta-label">Metadata</span>
            <span class="meta-value mono">${metaStr}</span>
          </div>
          ${credential.expires_at ? `
          <div class="meta-row">
            <span class="meta-label">Expires</span>
            <span class="meta-value">${formatTimestamp(credential.expires_at)}</span>
          </div>` : ''}
        </div>

        <div class="cred-card__attestors">
          <div class="attestors-header">
            <span class="meta-label">Quorum Slice Attestors</span>
            <span class="badge badge--${attestors.length > 0 ? 'gray' : 'red'}" style="font-size:10px;">
              ${attestors.length} Node${attestors.length !== 1 ? 's' : ''}
            </span>
          </div>
          
          ${attestors.length === 0 
            ? `<div class="attestors-empty">Awaiting slice signatures</div>`
            : `<div class="attestor-mini-list">
                 ${attestors.map(addr => `
                   <div class="attestor-mini-item">
                     <span>🏛️</span>
                     <span class="mono" title="${addr}">${formatAddress(addr)}</span>
                   </div>
                 `).join('')}
               </div>`
          }
        </div>
      </div>
      
      <div class="cred-card__footer">
        <a href="/verify?credentialId=${credential.id}" class="btn btn--sm btn--ghost" style="width:100%;" data-route="/verify?credentialId=${credential.id}">
          View Public Page →
        </a>
      </div>
    `;

    // Bind the public page link to SPA router
    cardEl.querySelector('a').addEventListener('click', (e) => {
      if (e.ctrlKey || e.metaKey || e.shiftKey) return;
      e.preventDefault();
      navigateTo(`/verify?credentialId=${credential.id}`);
    });

  } catch (err) {
    cardEl.classList.remove('skeleton-loader');
    cardEl.innerHTML = `
      <div class="cred-card__body" style="justify-content:center; align-items:center; text-align:center;">
        <div style="font-size:24px; margin-bottom:8px;">⚠️</div>
        <div style="color:var(--red); font-size:13px;">Failed to load data</div>
        <div style="color:var(--text-muted); font-size:11px; margin-top:4px;">${err.message}</div>
      </div>
    `;
  }
}
