import { useState, useEffect, useRef } from 'react';
import { useSearchParams, useNavigate } from 'react-router-dom';
import { Navbar } from '../components/Navbar';
import {
  getCredential,
  getCredentialsBySubject,
  getAttestors,
  isExpired,
  verifyClaim,
  decodeMetadataHash,
  CONTRACT_ID,
  RPC_URL,
  NETWORK,
} from '../stellar';

const CREDENTIAL_TYPES: Record<number, string> = {
  1: '🎓 Degree', 2: '🏛️ License', 3: '💼 Employment',
  4: '📜 Certification', 5: '🔬 Research',
};

function credTypeLabel(n: number | bigint) {
  return CREDENTIAL_TYPES[Number(n)] || `Type ${n}`;
}
function formatTimestamp(ts: number | bigint | null | undefined) {
  if (!ts) return 'Never';
  return new Date(Number(ts) * 1000).toLocaleDateString(undefined, { year: 'numeric', month: 'short', day: 'numeric' });
}
function formatAddress(addr: string) {
  if (!addr || addr.length < 10) return addr || '—';
  return addr.slice(0, 8) + '…' + addr.slice(-6);
}
function hexToBytes (hex: string): Uint8Array {
  const clean = hex.replace(/^0x/, '').replace(/\s/g, '');
  const bytes = new Uint8Array(clean.length / 2);
  for (let i = 0; i < bytes.length; i++) bytes[i] = parseInt(clean.substr(i * 2, 2), 16);
  return bytes;
}

interface CredentialData {
  id: bigint; subject: string; issuer: string; credential_type: number;
  metadata_hash: Uint8Array; revoked: boolean; expires_at: bigint | null;
}

function CredentialResult({ credential, attestors, expired }: {
  credential: CredentialData; attestors: string[]; expired: boolean;
}) {
  const [searchParams] = useSearchParams();
  const navigate = useNavigate();
  const [zkClaimType, setZkClaimType] = useState('has_degree');
  const [zkCustomType, setZkCustomType] = useState('');
  const [zkProof, setZkProof] = useState('');
  const [zkResult, setZkResult] = useState<{ type: string; msg: string } | null>(null);
  const [zkLoading, setZkLoading] = useState(false);

  const isRevoked = credential.revoked;
  const metaStr = decodeMetadataHash(credential.metadata_hash);
  const shareUrl = `${window.location.origin}/verify?credentialId=${credential.id}`;

  let statusClass: string, statusIcon: string, statusTitle: string, statusSub: string;
  if (isRevoked) {
    statusClass = 'revoked'; statusIcon = '🚫'; statusTitle = 'Credential Revoked';
    statusSub = 'This credential has been officially revoked.';
  } else if (expired) {
    statusClass = 'expired'; statusIcon = '⏰'; statusTitle = 'Credential Expired';
    statusSub = `This credential expired on ${formatTimestamp(credential.expires_at)}.`;
  } else if (attestors.length === 0) {
    statusClass = 'pending'; statusIcon = '⏳'; statusTitle = 'Awaiting Attestation';
    statusSub = 'No attestors have signed this credential yet.';
  } else {
    statusClass = 'valid'; statusIcon = '✅'; statusTitle = 'Credential Verified';
    statusSub = `Attested by ${attestors.length} trusted node${attestors.length !== 1 ? 's' : ''}.`;
  }

  const handleZkVerify = async () => {
    const claimType = zkClaimType === 'custom' ? zkCustomType.trim() : zkClaimType;
    if (!claimType) { setZkResult({ type: 'error', msg: '⚠️ Please enter a claim type.' }); return; }
    const proofHex = zkProof.trim().replace(/\s/g, '');
    if (!proofHex) { setZkResult({ type: 'error', msg: '⚠️ Please paste proof bytes.' }); return; }
    setZkLoading(true); setZkResult(null);
    try {
      const result = await verifyClaim(credential.id, claimType, hexToBytes(proofHex));
      setZkResult(result
        ? { type: 'success', msg: `✅ Claim "${claimType}" is valid for credential #${credential.id}.` }
        : { type: 'fail', msg: `❌ Claim "${claimType}" could not be verified.` }
      );
    } catch (err: unknown) {
      setZkResult({ type: 'error', msg: `⚠️ ${err instanceof Error ? err.message : 'ZK verification failed.'}` });
    } finally { setZkLoading(false); }
  };

  return (
    <div className="result-section">
      {/* Status Banner */}
      <div className={`status-banner status-banner--${statusClass}`}>
        <div className="status-banner__icon">{statusIcon}</div>
        <div>
          <div className="status-banner__title">{statusTitle}</div>
          <div className="status-banner__sub">{statusSub}</div>
        </div>
      </div>

      {/* Share Bar */}
      <div className="share-bar">
        <span style={{ fontSize: 13, color: 'var(--text-muted)' }}>🔗 Share:</span>
        <span className="share-bar__url">{shareUrl}</span>
        <button className="btn btn--ghost btn--sm" onClick={() => navigator.clipboard.writeText(shareUrl)}>Copy</button>
      </div>

      {/* Metadata */}
      <div className="detail-card">
        <div className="detail-card__header">
          <span className="detail-card__title">CREDENTIAL DETAILS</span>
          <span className={`badge badge--${isRevoked ? 'red' : expired ? 'gray' : 'green'}`}>
            {isRevoked ? '⛔ Revoked' : expired ? '⏰ Expired' : '✓ Active'}
          </span>
        </div>
        <div className="detail-card__body">
          <div className="meta-grid">
            <div className="meta-item"><div className="meta-item__label">ID</div><div className="meta-item__value meta-item__value--mono">#{credential.id.toString()}</div></div>
            <div className="meta-item"><div className="meta-item__label">Type</div><div className="meta-item__value">{credTypeLabel(credential.credential_type)}</div></div>
            <div className="meta-item" style={{ gridColumn: '1 / -1' }}><div className="meta-item__label">Subject</div><div className="meta-item__value meta-item__value--mono">{credential.subject}</div></div>
            <div className="meta-item" style={{ gridColumn: '1 / -1' }}><div className="meta-item__label">Issuer</div><div className="meta-item__value meta-item__value--mono">{credential.issuer}</div></div>
            <div className="meta-item" style={{ gridColumn: '1 / -1' }}><div className="meta-item__label">Metadata</div><div className="meta-item__value meta-item__value--mono">{metaStr || '—'}</div></div>
            <div className="meta-item"><div className="meta-item__label">Expires</div><div className="meta-item__value">{credential.expires_at ? formatTimestamp(credential.expires_at) : 'Never'}</div></div>
            <div className="meta-item"><div className="meta-item__label">Network</div><div className="meta-item__value">{NETWORK}</div></div>
          </div>
        </div>
      </div>

      {/* Attestors */}
      <div className="detail-card">
        <div className="detail-card__header">
          <span className="detail-card__title">ATTESTORS</span>
          <span className={`badge badge--${attestors.length > 0 ? 'green' : 'gray'}`}>{attestors.length} node{attestors.length !== 1 ? 's' : ''}</span>
        </div>
        <div className="detail-card__body">
          {attestors.length === 0 ? (
            <div style={{ color: 'var(--text-muted)', fontSize: 14, textAlign: 'center', padding: '20px 0' }}>No attestors have signed this credential yet.</div>
          ) : (
            <div className="attestor-list">
              {attestors.map((addr) => (
                <div key={addr} className="attestor-item">
                  <div className="attestor-item__avatar">🏛️</div>
                  <div className="attestor-item__addr" title={addr}>{addr}</div>
                  <span className="attestor-item__badge">✓ Signed</span>
                </div>
              ))}
            </div>
          )}
        </div>
      </div>

      {/* ZK Claim */}
      <div className="zk-card">
        <div className="zk-card__header">
          <span className="zk-card__icon">🔐</span>
          <div>
            <div className="zk-card__title">Zero-Knowledge Claim Verification</div>
            <div className="zk-card__sub">Verify a specific claim without revealing the full credential</div>
          </div>
        </div>
        <div className="zk-card__body">
          <div className="form-row">
            <label className="form-label">Claim Type</label>
            <select value={zkClaimType} onChange={e => setZkClaimType(e.target.value)} style={{ paddingLeft: 16 }}>
              <option value="has_degree">🎓 Has Engineering Degree</option>
              <option value="license_valid">🏛️ License Is Valid</option>
              <option value="employer_verified">💼 Employer Verified</option>
              <option value="certification_active">📜 Certification Active</option>
              <option value="custom">✏️ Custom claim type…</option>
            </select>
          </div>
          {zkClaimType === 'custom' && (
            <div className="form-row">
              <label className="form-label">Custom Claim Type</label>
              <div className="input-wrap">
                <span className="input-icon">✏️</span>
                <input type="text" placeholder="e.g. masters_degree" value={zkCustomType} onChange={e => setZkCustomType(e.target.value)} />
              </div>
            </div>
          )}
          <div className="form-row">
            <label className="form-label">ZK Proof (hex-encoded bytes)</label>
            <textarea placeholder="Paste hex-encoded proof bytes…" value={zkProof} onChange={e => setZkProof(e.target.value)} />
          </div>
          <div style={{ display: 'flex', gap: 10, alignItems: 'center', flexWrap: 'wrap' }}>
            <button className="btn btn--primary" onClick={handleZkVerify} disabled={zkLoading}>
              {zkLoading ? '⏳ Verifying…' : '🔐 Verify Claim'}
            </button>
            <button className="btn btn--ghost btn--sm" onClick={() => { setZkProof(''); setZkResult(null); }}>Clear</button>
            <span style={{ fontSize: 12, color: 'var(--text-muted)' }}>No wallet required</span>
          </div>
          {zkResult && <div className={`zk-result zk-result--${zkResult.type}`} role="alert">{zkResult.msg}</div>}
        </div>
      </div>
    </div>
  );
}

export function Verify() {
  const [searchParams, setSearchParams] = useSearchParams();
  const [activeTab, setActiveTab] = useState<'id' | 'addr'>('id');
  const [credInput, setCredInput] = useState(searchParams.get('credentialId') || '');
  const [addrInput, setAddrInput] = useState('');
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [result, setResult] = useState<{ credential: CredentialData; attestors: string[]; expired: boolean } | null>(null);
  const [addrResults, setAddrResults] = useState<bigint[] | null>(null);
  const [selectedCredId, setSelectedCredId] = useState<bigint | null>(null);
  const autoTriggered = useRef(false);

  const fetchCred = async (id: bigint) => {
    setLoading(true); setError(null); setResult(null); setAddrResults(null);
    setSearchParams({ credentialId: id.toString() });
    try {
      const [credential, attestors, expired] = await Promise.all([
        getCredential(id),
        getAttestors(id),
        isExpired(id).catch(() => false),
      ]);
      setResult({ credential, attestors, expired });
    } catch (err: unknown) {
      setError(err instanceof Error ? err.message : 'Failed to fetch credential.');
    } finally { setLoading(false); }
  };

  const handleVerifyId = async () => {
    const id = parseInt(credInput);
    if (isNaN(id) || id < 1) { setError('Please enter a valid credential ID (positive integer).'); return; }
    await fetchCred(BigInt(id));
  };

  const handleVerifyAddr = async () => {
    const addr = addrInput.trim();
    if (!addr.startsWith('G') || addr.length < 56) { setError('Please enter a valid Stellar address.'); return; }
    setLoading(true); setError(null); setResult(null); setAddrResults(null);
    try {
      const ids: bigint[] = await getCredentialsBySubject(addr);
      setAddrResults(ids || []);
    } catch (err: unknown) {
      setError(err instanceof Error ? err.message : 'Failed to look up address.');
    } finally { setLoading(false); }
  };

  // Auto-verify from query param on mount
  useEffect(() => {
    const preId = searchParams.get('credentialId');
    if (preId && !autoTriggered.current) {
      autoTriggered.current = true;
      const id = parseInt(preId);
      if (!isNaN(id) && id > 0) fetchCred(BigInt(id));
    }
  }, []);

  return (
    <>
      <Navbar />
      <main className="container" style={{ paddingTop: 0, paddingBottom: 64 }}>
        <div className="verify-hero">
          <div className="verify-hero__eyebrow">⚡ Instant On-Chain Verification</div>
          <h1 className="verify-hero__title">Verify Engineering Credentials</h1>
          <p className="verify-hero__subtitle">
            Confirm an engineer's credentials are authentic, attested by a quorum of trusted institutions, and have not been revoked — without connecting a wallet.
          </p>
        </div>

        <div className="search-card" id="search-card">
          <div className="search-card__label">SEARCH BY</div>
          <div className="search-card__tabs" role="tablist">
            <button className={`tab-btn${activeTab === 'id' ? ' active' : ''}`} role="tab" aria-selected={activeTab === 'id'} onClick={() => { setActiveTab('id'); setError(null); }}>🔑 Credential ID</button>
            <button className={`tab-btn${activeTab === 'addr' ? ' active' : ''}`} role="tab" aria-selected={activeTab === 'addr'} onClick={() => { setActiveTab('addr'); setError(null); }}>🌐 Stellar Address</button>
          </div>

          {activeTab === 'id' && (
            <div className="input-group">
              <div className="input-wrap">
                <span className="input-icon">#</span>
                <input type="number" min="1" placeholder="Enter credential ID (e.g. 42)" value={credInput} onChange={e => setCredInput(e.target.value)} onKeyDown={e => e.key === 'Enter' && handleVerifyId()} aria-label="Credential ID" />
              </div>
              <button className="btn btn--primary" onClick={handleVerifyId} disabled={loading} style={{ minWidth: 120 }}>
                {loading ? 'Verifying…' : 'Verify'}
              </button>
            </div>
          )}

          {activeTab === 'addr' && (
            <div className="input-group">
              <div className="input-wrap">
                <span className="input-icon">G</span>
                <input type="text" placeholder="Enter Stellar address (GABC…)" value={addrInput} onChange={e => setAddrInput(e.target.value)} onKeyDown={e => e.key === 'Enter' && handleVerifyAddr()} aria-label="Stellar address" spellCheck={false} />
              </div>
              <button className="btn btn--primary" onClick={handleVerifyAddr} disabled={loading} style={{ minWidth: 120 }}>
                {loading ? 'Looking up…' : 'Look Up'}
              </button>
            </div>
          )}

          {/* Network info */}
          <div style={{ marginTop: 16, display: 'flex', gap: 8, flexWrap: 'wrap' }}>
            <span className="badge badge--gray">🌐 {NETWORK}</span>
            <span className="badge badge--gray" style={{ fontSize: 10, fontFamily: 'var(--font-mono)', maxWidth: 300, overflow: 'hidden', textOverflow: 'ellipsis' }} title={RPC_URL}>{RPC_URL}</span>
            {CONTRACT_ID
              ? <span className="badge badge--blue" style={{ fontSize: 10, fontFamily: 'var(--font-mono)' }} title={`Contract: ${CONTRACT_ID}`}>📄 {formatAddress(CONTRACT_ID)}</span>
              : <span className="badge badge--red">⚠ Contract not configured</span>
            }
          </div>
        </div>

        {/* Results */}
        <div id="results-area">
          {loading && <div className="loading-state"><div className="spinner" /><p>Verifying on-chain…</p></div>}
          {error && (
            <div className="error-card">
              <div className="error-card__icon">⚠️</div>
              <div><div className="error-card__title">Could Not Verify</div><div className="error-card__msg">{error}</div></div>
            </div>
          )}
          {result && <CredentialResult {...result} />}
          {addrResults && addrResults.length === 0 && (
            <div className="empty-state">
              <div className="empty-state__icon">🔍</div>
              <div className="empty-state__title">No credentials found</div>
              <p>This address has no credentials recorded on-chain.</p>
            </div>
          )}
          {addrResults && addrResults.length > 0 && (
            <div className="result-section">
              <div className="detail-card" style={{ marginBottom: 20 }}>
                <div className="detail-card__header">
                  <span className="detail-card__title">CREDENTIALS FOR ADDRESS</span>
                  <span className="badge badge--blue">{addrResults.length} found</span>
                </div>
                <div className="detail-card__body">
                  <div className="cred-list">
                    {addrResults.map((id) => (
                      <div key={id.toString()} className="cred-list-item" role="button" tabIndex={0}
                        onClick={() => { setSelectedCredId(id); fetchCred(id); }}
                        onKeyDown={e => (e.key === 'Enter' || e.key === ' ') && fetchCred(id)}
                      >
                        <div>
                          <div className="cred-list-item__id">Credential #{id.toString()}</div>
                          <div style={{ fontSize: 12, color: 'var(--text-muted)', marginTop: 2 }}>Click to view full details</div>
                        </div>
                        <span style={{ color: 'var(--text-muted)' }}>→</span>
                      </div>
                    ))}
                  </div>
                </div>
              </div>
            </div>
          )}
        </div>
      </main>
      <footer className="footer">
        <div className="container">
          Powered by <a href="https://stellar.org" target="_blank" rel="noopener">Stellar Soroban</a>
          {' · '}
          <a href="https://github.com/Phantomcall/QuorumProof" target="_blank" rel="noopener">QuorumProof</a>
        </div>
      </footer>
    </>
  );
}
