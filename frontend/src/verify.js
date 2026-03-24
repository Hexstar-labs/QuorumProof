/**
 * verify.js — Credential Verification Page Logic
 *
 * Handles the /verify route:
 *  - Tab switching between Credential ID and Stellar Address modes
 *  - Pre-fills from ?credentialId= query param (shareable URL)
 *  - Calls stellar.js helpers, renders result card with metadata & attestors
 *  - ZK claim verification form
 */

import {
  getCredential,
  getCredentialsBySubject,
  isAttested,
  getAttestors,
  isExpired,
  verifyClaim,
  decodeMetadataHash,
  CONTRACT_ID,
  RPC_URL,
  NETWORK,
} from './stellar.js';

import { renderNavbar, bindNavbarLinks } from './dashboard.js';

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

/**
 * Derive the verification status from credential flags.
 * Priority: revoked → expired → pending (0 attestors) → verified
 * @param {boolean} revoked
 * @param {boolean} expired
 * @param {number} attestorCount
 * @returns {'revoked'|'expired'|'pending'|'verified'}
 */
export function deriveStatus(revoked, expired, attestorCount) {
  if (revoked) return 'revoked';
  if (expired) return 'expired';
  if (attestorCount === 0) return 'pending';
  return 'verified';
}

// ── Format timestamps ────────────────────────────────────────────────────────
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

// ── Main render entry ────────────────────────────────────────────────────────
export function renderVerifyPage(container) {
  // Read ?credentialId from URL
  const params = new URLSearchParams(window.location.search);
  const prefilledId = params.get('credentialId') || '';

  container.innerHTML = `
    ${renderNavbar('/verify')}

    <main class="container" style="padding-top: 0; padding-bottom: 64px;">
      <!-- Hero -->
      <div class="verify-hero">
        <div class="verify-hero__eyebrow">⚡ Instant On-Chain Verification</div>
        <h1 class="verify-hero__title">Verify Engineering Credentials</h1>
        <p class="verify-hero__subtitle">
          Confirm an engineer's credentials are authentic, attested by a quorum
          of trusted institutions, and have not been revoked — without connecting a wallet.
        </p>
      </div>

      <!-- Search Card -->
      <div class="search-card" id="search-card">
        <div class="search-card__label">SEARCH BY</div>

        <div class="search-card__tabs" role="tablist">
          <button class="tab-btn active" id="tab-id" role="tab" aria-selected="true"
                  data-tab="id">🔑 Credential ID</button>
          <button class="tab-btn" id="tab-addr" role="tab" aria-selected="false"
                  data-tab="addr">🌐 Stellar Address</button>
        </div>

        <!-- Credential ID mode -->
        <div id="panel-id">
          <div class="input-group">
            <div class="input-wrap">
              <span class="input-icon">#</span>
              <input id="input-cred-id"
                     type="number"
                     min="1"
                     placeholder="Enter credential ID (e.g. 42)"
                     value="${prefilledId}"
                     aria-label="Credential ID"
                     autocomplete="off" />
            </div>
            <button class="btn btn--primary" id="btn-verify-id" style="min-width:120px;">
              <span id="btn-verify-id-text">Verify</span>
            </button>
          </div>
        </div>

        <!-- Stellar Address mode -->
        <div id="panel-addr" style="display:none;">
          <div class="input-group">
            <div class="input-wrap">
              <span class="input-icon">G</span>
              <input id="input-addr"
                     type="text"
                     placeholder="Enter Stellar address (GABC…)"
                     aria-label="Stellar address"
                     autocomplete="off"
                     spellcheck="false" />
            </div>
            <button class="btn btn--primary" id="btn-verify-addr" style="min-width:120px;">
              <span id="btn-verify-addr-text">Look Up</span>
            </button>
          </div>
        </div>

        <!-- Network info -->
        <div style="margin-top:16px; display:flex; gap:8px; flex-wrap:wrap;">
          <span class="badge badge--gray">🌐 ${NETWORK}</span>
          <span class="badge badge--gray" style="font-size:10px; font-family:var(--font-mono); max-width:300px; overflow:hidden; text-overflow:ellipsis;"
                title="${RPC_URL}">${RPC_URL}</span>
          ${CONTRACT_ID
            ? `<span class="badge badge--blue" style="font-size:10px; font-family:var(--font-mono);"
                     title="Contract: ${CONTRACT_ID}">📄 ${formatAddress(CONTRACT_ID)}</span>`
            : `<span class="badge badge--red">⚠ Contract not configured</span>`}
        </div>
      </div>

      <!-- Results Area -->
      <div id="results-area"></div>
    </main>

    <footer class="footer">
      <div class="container">
        Powered by <a href="https://stellar.org" target="_blank" rel="noopener">Stellar Soroban</a>
        · <a href="https://github.com/Phantomcall/QuorumProof" target="_blank" rel="noopener">QuorumProof</a>
      </div>
    </footer>
  `;

  bindNavbarLinks(container);

  // ── Wire up tabs ────────────────────────────────────────────────────────
  const tabId   = container.querySelector('#tab-id');
  const tabAddr = container.querySelector('#tab-addr');
  const panelId   = container.querySelector('#panel-id');
  const panelAddr = container.querySelector('#panel-addr');

  function activateTab(tab) {
    if (tab === 'id') {
      tabId.classList.add('active');   tabId.setAttribute('aria-selected', 'true');
      tabAddr.classList.remove('active'); tabAddr.setAttribute('aria-selected', 'false');
      panelId.style.display = '';
      panelAddr.style.display = 'none';
    } else {
      tabAddr.classList.add('active');  tabAddr.setAttribute('aria-selected', 'true');
      tabId.classList.remove('active'); tabId.setAttribute('aria-selected', 'false');
      panelAddr.style.display = '';
      panelId.style.display = 'none';
    }
    container.querySelector('#results-area').innerHTML = '';
  }

  tabId.addEventListener('click', () => activateTab('id'));
  tabAddr.addEventListener('click', () => activateTab('addr'));

  // ── Verify by Credential ID ──────────────────────────────────────────────
  const btnVerifyId = container.querySelector('#btn-verify-id');
  const inputCredId = container.querySelector('#input-cred-id');

  async function handleVerifyById() {
    const rawId = inputCredId.value.trim();
    if (!rawId || isNaN(rawId) || Number(rawId) < 1) {
      showError('Please enter a valid credential ID (a positive integer).');
      return;
    }
    const credId = Number(rawId);
    updateShareableUrl(credId);
    showLoading('Looking up credential on-chain…');
    btnVerifyId.disabled = true;
    container.querySelector('#btn-verify-id-text').textContent = 'Verifying…';

    try {
      await renderCredential(credId);
    } catch (err) {
      showError(err.message || 'Failed to fetch credential.');
    } finally {
      btnVerifyId.disabled = false;
      container.querySelector('#btn-verify-id-text').textContent = 'Verify';
    }
  }

  btnVerifyId.addEventListener('click', handleVerifyById);
  inputCredId.addEventListener('keydown', (e) => { if (e.key === 'Enter') handleVerifyById(); });

  // ── Verify by Stellar Address ────────────────────────────────────────────
  const btnVerifyAddr = container.querySelector('#btn-verify-addr');
  const inputAddr = container.querySelector('#input-addr');

  async function handleVerifyByAddr() {
    const addr = inputAddr.value.trim();
    if (!addr.startsWith('G') || addr.length < 56) {
      showError('Please enter a valid Stellar address (starts with G, 56+ characters).');
      return;
    }
    showLoading('Looking up credentials for this address…');
    btnVerifyAddr.disabled = true;
    container.querySelector('#btn-verify-addr-text').textContent = 'Looking up…';

    try {
      const ids = await getCredentialsBySubject(addr);
      const resultsEl = container.querySelector('#results-area');

      if (!ids || ids.length === 0) {
        resultsEl.innerHTML = `
          <div class="empty-state">
            <div class="empty-state__icon">🔍</div>
            <div class="empty-state__title">No credentials found</div>
            <p>This address has no credentials recorded on-chain.</p>
          </div>`;
        return;
      }

      resultsEl.innerHTML = `
        <div class="result-section">
          <div class="detail-card" style="margin-bottom:20px;">
            <div class="detail-card__header">
              <span class="detail-card__title">CREDENTIALS FOR ADDRESS</span>
              <span class="badge badge--blue">${ids.length} found</span>
            </div>
            <div class="detail-card__body">
              <div class="cred-list" id="cred-list">
                ${ids.map(id => `
                  <div class="cred-list-item" data-cred-id="${id}" tabindex="0" role="button" aria-label="View credential ${id}">
                    <div>
                      <div class="cred-list-item__id">Credential #${id}</div>
                      <div style="font-size:12px;color:var(--text-muted);margin-top:2px;">Click to view full details</div>
                    </div>
                    <span style="color:var(--text-muted);">→</span>
                  </div>
                `).join('')}
              </div>
            </div>
          </div>
          <div id="selected-cred-details"></div>
        </div>`;

      resultsEl.querySelectorAll('.cred-list-item').forEach(item => {
        const handler = async () => {
          const credId = Number(item.dataset.credId);
          updateShareableUrl(credId);
          const detailEl = resultsEl.querySelector('#selected-cred-details');
          detailEl.innerHTML = `<div class="loading-state"><div class="spinner"></div><p>Loading credential #${credId}…</p></div>`;
          try {
            await renderCredential(credId, detailEl);
          } catch (err) {
            detailEl.innerHTML = buildErrorHTML(err.message || 'Failed to load credential.');
          }
        };
        item.addEventListener('click', handler);
        item.addEventListener('keydown', (e) => { if (e.key === 'Enter' || e.key === ' ') handler(); });
      });

    } catch (err) {
      showError(err.message || 'Failed to look up address.');
    } finally {
      btnVerifyAddr.disabled = false;
      container.querySelector('#btn-verify-addr-text').textContent = 'Look Up';
    }
  }

  btnVerifyAddr.addEventListener('click', handleVerifyByAddr);
  inputAddr.addEventListener('keydown', (e) => { if (e.key === 'Enter') handleVerifyByAddr(); });

  // ── Auto-verify if credentialId in query param ────────────────────────────
  const parsedId = parseInt(prefilledId, 10);
  const isValidQueryId = prefilledId.trim() !== '' &&
    parsedId > 0 &&
    isFinite(parsedId) &&
    String(parsedId) === prefilledId.trim();

  if (isValidQueryId) {
    // Trigger after DOM is ready
    setTimeout(() => handleVerifyById(), 50);
  }

  // ── Helper: render a credential into a target element ─────────────────────
  async function renderCredential(credId, targetEl = null) {
    const resultsEl = targetEl || container.querySelector('#results-area');

    // Fetch all data in parallel
    const [credential, attestors, expired] = await Promise.all([
      getCredential(credId),
      getAttestors(credId),
      isExpired(credId).catch(() => false),
    ]);

    const isRevoked = credential.revoked;
    const metaStr = decodeMetadataHash(credential.metadata_hash);
    const expiresAt = credential.expires_at;

    // Determine overall status
    const status = deriveStatus(isRevoked, expired, attestors.length);
    let statusClass, statusIcon, statusTitle, statusSub;
    if (status === 'revoked') {
      statusClass = 'revoked'; statusIcon = '🚫';
      statusTitle = 'Credential Revoked';
      statusSub = 'This credential has been officially revoked by the subject or issuer.';
    } else if (status === 'expired') {
      statusClass = 'expired'; statusIcon = '⏰';
      statusTitle = 'Credential Expired';
      statusSub = `This credential expired on ${formatTimestamp(expiresAt)}.`;
    } else if (status === 'pending') {
      statusClass = 'pending'; statusIcon = '⏳';
      statusTitle = 'Awaiting Attestation';
      statusSub = 'No attestors have signed this credential yet.';
    } else {
      statusClass = 'valid'; statusIcon = '✅';
      statusTitle = 'Credential Verified';
      statusSub = `Attested by ${attestors.length} trusted node${attestors.length !== 1 ? 's' : ''}.`;
    }

    const shareUrl = buildShareableUrl(credId);

    resultsEl.innerHTML = `
      <div class="result-section">
        <!-- Status Banner -->
        <div class="status-banner status-banner--${statusClass}">
          <div class="status-banner__icon">${statusIcon}</div>
          <div>
            <div class="status-banner__title">${statusTitle}</div>
            <div class="status-banner__sub">${statusSub}</div>
          </div>
        </div>

        <!-- Share bar -->
        <div class="share-bar">
          <span style="font-size:13px;color:var(--text-muted);">🔗 Share:</span>
          <span class="share-bar__url" id="share-url" title="${shareUrl}">${shareUrl}</span>
          <button class="btn btn--ghost btn--sm" id="btn-copy-url">Copy</button>
        </div>

        <!-- Credential Metadata -->
        <div class="detail-card">
          <div class="detail-card__header">
            <span class="detail-card__title">CREDENTIAL DETAILS</span>
            <span class="badge badge--${isRevoked ? 'red' : expired ? 'gray' : 'green'}">
              ${isRevoked ? '⛔ Revoked' : expired ? '⏰ Expired' : '✓ Active'}
            </span>
          </div>
          <div class="detail-card__body">
            <div class="meta-grid">
              <div class="meta-item">
                <div class="meta-item__label">Credential ID</div>
                <div class="meta-item__value meta-item__value--mono">#${credential.id}</div>
              </div>
              <div class="meta-item">
                <div class="meta-item__label">Type</div>
                <div class="meta-item__value">${credTypeLabel(credential.credential_type)}</div>
              </div>
              <div class="meta-item" style="grid-column: 1 / -1;">
                <div class="meta-item__label">Subject (Engineer)</div>
                <div class="meta-item__value meta-item__value--mono" title="${credential.subject}">${credential.subject}</div>
              </div>
              <div class="meta-item" style="grid-column: 1 / -1;">
                <div class="meta-item__label">Issuer</div>
                <div class="meta-item__value meta-item__value--mono" title="${credential.issuer}">${credential.issuer}</div>
              </div>
              <div class="meta-item" style="grid-column: 1 / -1;">
                <div class="meta-item__label">Metadata / IPFS Hash</div>
                <div class="meta-item__value meta-item__value--mono">${metaStr || '—'}</div>
              </div>
              <div class="meta-item">
                <div class="meta-item__label">Expires</div>
                <div class="meta-item__value">${expiresAt ? formatTimestamp(expiresAt) : 'Never'}</div>
              </div>
              <div class="meta-item">
                <div class="meta-item__label">Network</div>
                <div class="meta-item__value">${NETWORK}</div>
              </div>
            </div>
          </div>
        </div>

        <!-- Attestors -->
        <div class="detail-card">
          <div class="detail-card__header">
            <span class="detail-card__title">ATTESTORS</span>
            <span class="badge badge--${attestors.length > 0 ? 'green' : 'gray'}">${attestors.length} node${attestors.length !== 1 ? 's' : ''}</span>
          </div>
          <div class="detail-card__body">
            ${attestors.length === 0
              ? `<div style="color:var(--text-muted);font-size:14px;text-align:center;padding:20px 0;">
                   No attestors have signed this credential yet.
                 </div>`
              : `<div class="attestor-list">
                  ${attestors.map((addr, i) => `
                    <div class="attestor-item">
                      <div class="attestor-item__avatar">🏛️</div>
                      <div class="attestor-item__addr" title="${addr}">${addr}</div>
                      <span class="attestor-item__badge">✓ Signed</span>
                    </div>
                  `).join('')}
                </div>`
            }
          </div>
        </div>

        <!-- ZK Claim Verification -->
        ${buildZkClaimHTML(credId)}
      </div>
    `;

    // Copy URL button
    resultsEl.querySelector('#btn-copy-url')?.addEventListener('click', async () => {
      try {
        await navigator.clipboard.writeText(shareUrl);
        const btn = resultsEl.querySelector('#btn-copy-url');
        btn.textContent = '✓ Copied!';
        setTimeout(() => { btn.textContent = 'Copy'; }, 1500);
      } catch { /* silent */ }
    });

    // Wire up ZK form
    wireZkForm(resultsEl, credId);
  }

  // ── Helpers ───────────────────────────────────────────────────────────────
  function showLoading(msg = 'Verifying on-chain…') {
    container.querySelector('#results-area').innerHTML = `
      <div class="loading-state">
        <div class="spinner"></div>
        <p>${msg}</p>
      </div>`;
  }

  function showError(msg) {
    container.querySelector('#results-area').innerHTML = buildErrorHTML(msg);
  }

  function buildErrorHTML(msg) {
    return `
      <div class="error-card">
        <div class="error-card__icon">⚠️</div>
        <div>
          <div class="error-card__title">Could Not Verify</div>
          <div class="error-card__msg">${msg}</div>
        </div>
      </div>`;
  }

  function buildShareableUrl(credId) {
    const url = new URL(window.location.href);
    url.search = '';
    url.searchParams.set('credentialId', credId);
    return url.toString();
  }

  function updateShareableUrl(credId) {
    const url = new URL(window.location.href);
    url.searchParams.set('credentialId', credId);
    window.history.replaceState({}, '', url.toString());
  }
}

// ── ZK Claim Form ─────────────────────────────────────────────────────────
function buildZkClaimHTML(credId) {
  const zkNotConfigured = !import.meta.env.VITE_CONTRACT_ZK_VERIFIER;
  if (zkNotConfigured) {
    return `
      <div class="zk-card">
        <div class="zk-card__header">
          <span class="zk-card__icon">🔐</span>
          <div>
            <div class="zk-card__title">Zero-Knowledge Claim Verification</div>
            <div class="zk-card__sub">Verify a specific claim without revealing the full credential</div>
          </div>
        </div>
        <div class="zk-card__body">
          <div class="badge badge--red" style="font-size:13px;padding:10px 14px;">
            ⚠ ZK Verifier contract not configured. Set VITE_CONTRACT_ZK_VERIFIER in your .env file.
          </div>
        </div>
      </div>`;
  }

  return `
    <div class="zk-card">
      <div class="zk-card__header">
        <span class="zk-card__icon">🔐</span>
        <div>
          <div class="zk-card__title">Zero-Knowledge Claim Verification</div>
          <div class="zk-card__sub">Verify a specific claim without revealing the full credential</div>
        </div>
      </div>
      <div class="zk-card__body">
        <div class="form-row">
          <label class="form-label" for="zk-claim-type">Claim Type</label>
          <select id="zk-claim-type" style="padding-left:16px;">
            <option value="HasDegree">🎓 Has Engineering Degree</option>
            <option value="HasLicense">🏛️ License Is Valid</option>
            <option value="HasEmploymentHistory">💼 Employer Verified</option>
          </select>
        </div>
        <div class="form-row">
          <label class="form-label" for="zk-proof">ZK Proof (hex-encoded bytes)</label>
          <textarea id="zk-proof"
                    placeholder="Paste the hex-encoded proof bytes provided by the engineer…&#10;e.g. 0x4a3f09c2…"></textarea>
        </div>
        <div style="display:flex; gap:10px; align-items:center; flex-wrap:wrap;">
          <button class="btn btn--primary" id="btn-zk-verify" data-cred-id="${credId}">
            🔐 Verify Claim
          </button>
          <button class="btn btn--ghost btn--sm" id="btn-zk-clear">Clear</button>
          <span style="font-size:12px; color:var(--text-muted);">No wallet required</span>
        </div>
        <div id="zk-result"></div>
      </div>
    </div>`;
}

function wireZkForm(container, credId) {
  const claimTypeEl = container.querySelector('#zk-claim-type');
  const proofEl     = container.querySelector('#zk-proof');
  const btnVerify   = container.querySelector('#btn-zk-verify');
  const btnClear    = container.querySelector('#btn-zk-clear');
  const resultEl    = container.querySelector('#zk-result');

  btnClear?.addEventListener('click', () => {
    proofEl.value = '';
    resultEl.innerHTML = '';
  });

  btnVerify?.addEventListener('click', async () => {
    const claimType = claimTypeEl.value; // already a valid enum string

    const proofHex = proofEl.value.trim().replace(/\s/g, '');
    if (!proofHex) {
      resultEl.innerHTML = zkResultHTML('error', '⚠️ Please paste the proof bytes.');
      return;
    }

    btnVerify.disabled = true;
    btnVerify.textContent = '⏳ Verifying…';
    resultEl.innerHTML = '';

    try {
      const result = await verifyClaim(credId, claimType, proofHex);
      if (result) {
        resultEl.innerHTML = zkResultHTML('success', `✅ Claim Verified`, 'This claim was proven cryptographically without revealing the full credential details.');
      } else {
        resultEl.innerHTML = zkResultHTML('fail', `❌ Claim Not Verified`, 'The submitted proof did not satisfy the claim. No credential data was revealed.');
      }
    } catch (err) {
      const msg = err.message || 'ZK verification failed.';
      // If the ZK contract isn't configured, show a helpful message
      if (msg.includes('not configured')) {
        resultEl.innerHTML = zkResultHTML('error', `⚠️ ZK Verifier contract not configured. Set VITE_CONTRACT_ZK_VERIFIER in your .env file.`);
      } else {
        resultEl.innerHTML = zkResultHTML('error', `⚠️ ${msg}`);
      }
    } finally {
      btnVerify.disabled = false;
      btnVerify.textContent = '🔐 Verify Claim';
    }
  });
}

function zkResultHTML(type, msg, tooltip = '') {
  return `<div class="zk-result zk-result--${type}" role="alert"${tooltip ? ` title="${tooltip}"` : ''}>${msg}</div>`;
}
