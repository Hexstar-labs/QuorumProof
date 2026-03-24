/**
 * dashboard.js — Credential Dashboard Page
 *
 * /dashboard route — wallet-gated via WalletGuard.
 * Fetches all credentials for the connected wallet, shows attestation status
 * per credential via is_attested(credId, sliceId), and renders quorum slice
 * members with role labels via get_slice.
 */

import { navigateTo } from './main.js';
import {
  getCredentialsBySubject,
  getCredential,
  isAttested,
  isExpired,
  getAttestors,
  getSlice,
  decodeMetadataHash,
  NETWORK,
} from './stellar.js';

// ── Constants ────────────────────────────────────────────────────────────────
const WALLET_KEY = 'qp-wallet-address';
const SLICE_KEY  = 'qp-slice-id';

const CREDENTIAL_TYPES = {
  1: '🎓 Degree',
  2: '🏛️ License',
  3: '💼 Employment',
  4: '📜 Certification',
  5: '🔬 Research',
};

// Role labels assigned by attestor index within the slice
const ATTESTOR_ROLES = ['Lead Verifier', 'Co-Verifier', 'Auditor', 'Reviewer', 'Observer'];

// ── Helpers ──────────────────────────────────────────────────────────────────
function credTypeLabel(n) {
  return CREDENTIAL_TYPES[Number(n)] || `Type ${n}`;
}

function formatTimestamp(ts) {
  if (!ts) return 'Never';
  return new Date(Number(ts) * 1000).toLocaleDateString(undefined, {
    year: 'numeric', month: 'short', day: 'numeric',
  });
}

function formatAddress(addr) {
  if (!addr || addr.length < 10) return addr || '—';
  return addr.slice(0, 8) + '…' + addr.slice(-6);
}

function attestorRole(index) {
  return ATTESTOR_ROLES[index] || `Member ${index + 1}`;
}

// ── Shared Navbar ────────────────────────────────────────────────────────────
export function renderNavbar(activeRoute) {
  const walletAddr = localStorage.getItem(WALLET_KEY) || '';
  return `
    <nav class="navbar">
      <div class="container navbar__inner">
        <a href="/dashboard" class="navbar__logo" data-route="/dashboard">
          <div class="navbar__logo-icon">⬡</div>
          QuorumProof
        </a>
        <div class="navbar__links">
          <a href="/dashboard" class="nav-link ${activeRoute === '/dashboard' ? 'active' : ''}" data-route="/dashboard">Dashboard</a>
          <a href="/verify"    class="nav-link ${activeRoute === '/verify'    ? 'active' : ''}" data-route="/verify">Verify</a>
        </div>
        <div class="navbar__right">
          ${walletAddr
            ? `<span class="wallet-pill" title="${walletAddr}">
                 <span class="wallet-pill__dot"></span>
                 ${formatAddress(walletAddr)}
               </span>`
            : ''}
          <span class="navbar__badge">${NETWORK}</span>
        </div>
      </div>
    </nav>
  `;
}

export function bindNavbarLinks(container) {
  container.querySelectorAll('a[data-route]').forEach(link => {
    link.addEventListener('click', e => {
      if (e.ctrlKey || e.metaKey || e.shiftKey) return;
      e.preventDefault();
      navigateTo(link.dataset.route);
    });
  });
}

// ── WalletGuard ──────────────────────────────────────────────────────────────
/**
 * Renders the wallet-connect gate. Returns true if a wallet is already
 * connected (caller should proceed to load content), false otherwise.
 */
function renderWalletGuard(container, onConnected) {
  const saved = localStorage.getItem(WALLET_KEY) || '';
  const savedSlice = localStorage.getItem(SLICE_KEY) || '';

  if (saved) {
    onConnected(saved, savedSlice || null);
    return;
  }

  container.innerHTML = `
    ${renderNavbar('/dashboard')}
    <main class="container dashboard-main" style="display:flex; flex-direction:column; align-items:center; justify-content:center; min-height:70vh;">
      <div class="wallet-guard-card">
        <div class="wallet-guard__icon">🔐</div>
        <h2 class="wallet-guard__title">Connect Your Wallet</h2>
        <p class="wallet-guard__sub">Enter your Stellar address to access your credential dashboard.</p>

        <div class="wallet-guard__form">
          <div class="input-wrap" style="margin-bottom:12px;">
            <span class="input-icon">G</span>
            <input id="guard-input-wallet" type="text"
              placeholder="Stellar address (GABC…56 chars)"
              autocomplete="off" spellcheck="false" />
          </div>
          <div class="input-wrap" style="margin-bottom:16px;">
            <span class="input-icon">#</span>
            <input id="guard-input-slice" type="number" min="1"
              placeholder="Quorum Slice ID (optional)"
              autocomplete="off" />
          </div>
          <button class="btn btn--primary" id="guard-btn-connect" style="width:100%;">
            Connect Wallet
          </button>
        </div>
      </div>
    </main>
    <footer class="footer"><div class="container">
      Powered by <a href="https://stellar.org" target="_blank" rel="noopener">Stellar Soroban</a>
      · <a href="https://github.com/Phantomcall/QuorumProof" target="_blank" rel="noopener">QuorumProof</a>
    </div></footer>
  `;

  bindNavbarLinks(container);

  const inputWallet = container.querySelector('#guard-input-wallet');
  const inputSlice  = container.querySelector('#guard-input-slice');
  const btnConnect  = container.querySelector('#guard-btn-connect');

  const connect = () => {
    const addr = inputWallet.value.trim();
    if (!addr.startsWith('G') || addr.length < 56) {
      inputWallet.style.borderColor = 'var(--red)';
      inputWallet.focus();
      return;
    }
    inputWallet.style.borderColor = '';
    const sliceId = inputSlice.value.trim();
    localStorage.setItem(WALLET_KEY, addr);
    if (sliceId) localStorage.setItem(SLICE_KEY, sliceId);
    else localStorage.removeItem(SLICE_KEY);
    renderDashboardPage(container);
  };

  btnConnect.addEventListener('click', connect);
  inputWallet.addEventListener('keydown', e => { if (e.key === 'Enter') connect(); });
  inputSlice.addEventListener('keydown',  e => { if (e.key === 'Enter') connect(); });
}

// ── Main Dashboard Page ──────────────────────────────────────────────────────
export function renderDashboardPage(container) {
  renderWalletGuard(container, (walletAddress, sliceId) => {
    _renderDashboard(container, walletAddress, sliceId);
  });
}

function _renderDashboard(container, walletAddress, sliceId) {
  container.innerHTML = `
    ${renderNavbar('/dashboard')}
    <main class="container container--wide dashboard-main">
      <header class="dashboard-header">
        <div>
          <h1 class="dashboard-title">Credential Dashboard</h1>
          <p class="dashboard-subtitle">Your verifiable credentials on Stellar Soroban</p>
        </div>
        <div class="dashboard-header__actions">
          <span class="wallet-address-chip" title="${walletAddress}">
            <span class="wallet-pill__dot"></span>
            ${formatAddress(walletAddress)}
          </span>
          ${sliceId ? `<span class="badge badge--gray">Slice #${sliceId}</span>` : ''}
          <button class="btn btn--ghost btn--sm" id="btn-disconnect">Disconnect</button>
        </div>
      </header>

      <div id="dashboard-content">
        <div class="loading-state">
          <div class="spinner"></div>
          <p>Loading credentials…</p>
        </div>
      </div>
    </main>
    <footer class="footer"><div class="container">
      Powered by <a href="https://stellar.org" target="_blank" rel="noopener">Stellar Soroban</a>
      · <a href="https://github.com/Phantomcall/QuorumProof" target="_blank" rel="noopener">QuorumProof</a>
    </div></footer>
  `;

  bindNavbarLinks(container);

  container.querySelector('#btn-disconnect').addEventListener('click', () => {
    localStorage.removeItem(WALLET_KEY);
    localStorage.removeItem(SLICE_KEY);
    renderDashboardPage(container);
  });

  loadCredentials(walletAddress, sliceId, container.querySelector('#dashboard-content'));
}

// ── Load Credentials ─────────────────────────────────────────────────────────
async function loadCredentials(address, sliceId, contentEl) {
  try {
    const ids = await getCredentialsBySubject(address);

    if (!ids || ids.length === 0) {
      contentEl.innerHTML = renderEmptyState();
      bindEmptyStateCTA(contentEl);
      return;
    }

    contentEl.innerHTML = `
      <div class="dashboard-grid" id="cred-grid">
        ${ids.map(id => `
          <div id="cred-card-${id}" class="cred-card skeleton-loader" style="min-height:380px; position:relative;"></div>
        `).join('')}
      </div>
    `;

    await Promise.all(ids.map(id =>
      renderCredentialCard(id, sliceId, document.getElementById(`cred-card-${id}`))
    ));

  } catch (err) {
    contentEl.innerHTML = `
      <div class="error-card">
        <div class="error-card__icon">⚠️</div>
        <div>
          <div class="error-card__title">Could Not Load Credentials</div>
          <div class="error-card__msg">${err.message || 'Failed to fetch from network.'}</div>
        </div>
      </div>
    `;
  }
}

// ── Credential Card ───────────────────────────────────────────────────────────
async function renderCredentialCard(credId, sliceId, cardEl) {
  try {
    // Fetch credential + expiry in parallel; slice + attestation status depend on sliceId
    const [credential, expired] = await Promise.all([
      getCredential(credId),
      isExpired(credId).catch(() => false),
    ]);

    const isRevoked = credential.revoked;
    const metaStr   = decodeMetadataHash(credential.metadata_hash);

    // Fetch slice data and attestation status if a sliceId is known
    let attested = false;
    let slice    = null;
    if (sliceId) {
      [attested, slice] = await Promise.all([
        isAttested(credId, sliceId).catch(() => false),
        getSlice(sliceId).catch(() => null),
      ]);
    } else {
      // Fallback: use attestor count as proxy (no slice context)
      const attestors = await getAttestors(credId).catch(() => []);
      attested = attestors.length > 0;
    }

    // Determine status
    let statusClass, statusLabel, statusIcon;
    if (isRevoked) {
      statusClass = 'revoked'; statusIcon = '🚫'; statusLabel = 'Revoked';
    } else if (expired) {
      statusClass = 'expired'; statusIcon = '⏰'; statusLabel = 'Expired';
    } else if (attested) {
      statusClass = 'valid';   statusIcon = '✅'; statusLabel = 'Attested';
    } else {
      statusClass = 'pending'; statusIcon = '⏳'; statusLabel = 'Pending Attestation';
    }

    cardEl.classList.remove('skeleton-loader');
    cardEl.innerHTML = `
      <div class="cred-card__header cred-card__header--${statusClass}">
        <div class="cred-card__type">${credTypeLabel(credential.credential_type)}</div>
        <div class="badge badge--${statusClass === 'valid' ? 'green' : statusClass === 'revoked' ? 'red' : statusClass === 'expired' ? 'yellow' : 'blue'}">
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
          ${sliceId ? `
          <div class="meta-row">
            <span class="meta-label">Slice</span>
            <span class="meta-value mono">#${sliceId}</span>
          </div>` : ''}
        </div>

        ${renderSliceSection(slice, sliceId)}
      </div>

      <div class="cred-card__footer">
        <a href="/verify?credentialId=${credential.id}" class="btn btn--sm btn--ghost"
           style="width:100%;" data-route="/verify?credentialId=${credential.id}">
          View Public Page →
        </a>
      </div>
    `;

    cardEl.querySelector('a[data-route]').addEventListener('click', e => {
      if (e.ctrlKey || e.metaKey || e.shiftKey) return;
      e.preventDefault();
      navigateTo(`/verify?credentialId=${credential.id}`);
    });

  } catch (err) {
    cardEl.classList.remove('skeleton-loader');
    cardEl.innerHTML = `
      <div class="cred-card__body" style="justify-content:center; align-items:center; text-align:center; padding:32px;">
        <div style="font-size:24px; margin-bottom:8px;">⚠️</div>
        <div style="color:var(--red); font-size:13px;">Failed to load</div>
        <div style="color:var(--text-muted); font-size:11px; margin-top:4px;">${err.message}</div>
      </div>
    `;
  }
}

// ── Quorum Slice Section ──────────────────────────────────────────────────────
function renderSliceSection(slice, sliceId) {
  if (!slice) {
    return `
      <div class="cred-card__attestors">
        <div class="attestors-header">
          <span class="meta-label">Quorum Slice</span>
          ${sliceId
            ? `<span class="badge badge--gray" style="font-size:10px;">Slice #${sliceId}</span>`
            : `<span class="badge badge--gray" style="font-size:10px;">No slice</span>`}
        </div>
        <div class="attestors-empty">No slice data available</div>
      </div>
    `;
  }

  const attestors  = Array.isArray(slice.attestors) ? slice.attestors : [];
  const threshold  = Number(slice.threshold);
  const total      = attestors.length;
  const progress   = Math.min(total, threshold);

  return `
    <div class="cred-card__attestors">
      <div class="attestors-header">
        <span class="meta-label">Quorum Slice #${slice.id}</span>
        <span class="badge badge--gray" style="font-size:10px;">
          ${progress}/${threshold} threshold
        </span>
      </div>

      <div class="slice-progress" title="${progress} of ${threshold} required attestors">
        <div class="slice-progress__bar" style="width:${threshold > 0 ? Math.round((progress / threshold) * 100) : 0}%"></div>
      </div>

      ${attestors.length === 0
        ? `<div class="attestors-empty">No attestors in this slice</div>`
        : `<div class="attestor-mini-list">
             ${attestors.map((addr, i) => `
               <div class="attestor-mini-item">
                 <span class="attestor-mini-item__avatar">${i + 1}</span>
                 <div class="attestor-mini-item__info">
                   <span class="mono" title="${addr}">${formatAddress(addr)}</span>
                   <span class="attestor-mini-item__role">${attestorRole(i)}</span>
                 </div>
               </div>
             `).join('')}
           </div>`
      }

      <div class="slice-creator">
        <span class="meta-label">Creator</span>
        <span class="mono" style="font-size:11px; color:var(--text-muted);" title="${slice.creator}">
          ${formatAddress(slice.creator)}
        </span>
      </div>
    </div>
  `;
}

// ── Empty State ───────────────────────────────────────────────────────────────
function renderEmptyState() {
  return `
    <div class="empty-state empty-state--dashboard">
      <div class="empty-state__icon">📭</div>
      <div class="empty-state__title">No credentials yet</div>
      <p class="empty-state__body">
        You haven't been issued any verifiable credentials.<br>
        Request issuance from a trusted institution to get started.
      </p>
      <div class="empty-state__actions">
        <button class="btn btn--primary" id="btn-request-issuance">
          ✦ Request Credential Issuance
        </button>
        <a href="/verify" class="btn btn--ghost" data-route="/verify">
          Verify an existing credential
        </a>
      </div>
    </div>
  `;
}

function bindEmptyStateCTA(contentEl) {
  contentEl.querySelector('#btn-request-issuance')?.addEventListener('click', () => {
    // Placeholder: in a real app this would open a modal or navigate to a request form
    alert('Credential issuance request flow coming soon.\nContact your institution with your Stellar address.');
  });

  contentEl.querySelector('a[data-route]')?.addEventListener('click', e => {
    if (e.ctrlKey || e.metaKey || e.shiftKey) return;
    e.preventDefault();
    navigateTo('/verify');
  });
}
