import { useState, useEffect, useRef } from 'react';
import { useSearchParams } from 'react-router-dom';
import { Navbar } from '../components/Navbar';
import {
  getCredential,
  getCredentialsBySubject,
  getAttestors,
  isExpired,
  isAttested,
} from '../lib/contracts/quorumProof';
import type { Credential } from '../lib/contracts/quorumProof';
import { verifyClaim } from '../lib/contracts/zkVerifier';
import type { ClaimType } from '../lib/contracts/zkVerifier';
import { decodeMetadataHash, CONTRACT_ID, RPC_URL, NETWORK } from '../stellar';

export const DEFAULT_SLICE_ID = 1n;

const CREDENTIAL_TYPES: Record<number, string> = {
  1: '🎓 Degree', 2: '🏛️ License', 3: '💼 Employment',
  4: '📜 Certification', 5: '🔬 Research',
};

const CLAIM_TYPE_OPTIONS: { value: ClaimType; label: string }[] = [
  { value: 'HasDegree', label: '🎓 Degree' },
  { value: 'HasLicense', label: '🏛️ License' },
  { value: 'HasEmploymentHistory', label: '💼 Employment History' },
];

const ZK_PRIVACY_TOOLTIP =
  'Zero-knowledge proofs confirm a property of a credential (e.g. holds a degree) without revealing the credential data itself. The proof is verified entirely on-chain.';

export function credTypeLabel(n: number | bigint): string {
  return CREDENTIAL_TYPES[Number(n)] || `Type ${n}`;
}

export function formatTimestamp(ts: number | bigint | null | undefined): string {
  if (!ts) return 'Never';
  return new Date(Number(ts) * 1000).toLocaleDateString(undefined, {
    year: 'numeric', month: 'short', day: 'numeric',
  });
}

export function formatAddress(addr: string): string {
  if (!addr || addr.length < 10) return addr || '—';
  return addr.slice(0, 8) + '…' + addr.slice(-6);
}

export function buildShareUrl(id: bigint): string {
  return `${window.location.origin}/verify?id=${id}`;
}

export function parseIdFromUrl(url: string): bigint | null {
  try {
    const u = new URL(url);
    const raw = u.searchParams.get('id');
    if (!raw) return null;
    const n = parseInt(raw, 10);
    if (isNaN(n) || n < 1) return null;
    return BigInt(n);
  } catch {
    return null;
  }
}

export type StatusClass = 'valid' | 'revoked' | 'expired' | 'pending' | 'warning';

export interface StatusInfo {
  statusClass: StatusClass;
  statusIcon: string;
  statusTitle: string;
  statusSub: string;
}

export function deriveStatus(
  revoked: boolean,
  expired: boolean,
  attested: boolean | null,
  attestorCount: number,
  expiresAt?: bigint | null,
): StatusInfo {
  if (revoked) return { statusClass: 'revoked', statusIcon: '🚫', statusTitle: 'Credential Revoked', statusSub: 'This credential has been officially revoked.' };
  if (expired) return { statusClass: 'expired', statusIcon: '⏰', statusTitle: 'Credential Expired', statusSub: `This credential expired on ${formatTimestamp(expiresAt)}.` };
  if (attested === true || attestorCount > 0) return { statusClass: 'valid', statusIcon: '✅', statusTitle: 'Credential Verified', statusSub: `Attested by ${attestorCount} trusted node${attestorCount !== 1 ? 's' : ''}.` };
  if (attested === null) return { statusClass: 'warning', statusIcon: '⚠️', statusTitle: 'Attestation Status Unconfirmed', statusSub: 'Could not confirm quorum attestation. The credential may still be valid.' };
  return { statusClass: 'pending', statusIcon: '⏳', statusTitle: 'Awaiting Attestation', statusSub: 'No attestors have signed this credential yet.' };
}

export interface VerifyResult {
  credential: Credential;
  attestors: string[];
  expired: boolean;
  attested: boolean | null;
}

type ZkResult = { kind: 'verified'; value: boolean } | { kind: 'error'; message: string };


function ZkClaimPanel({ credentialId }: { credentialId: bigint }) {
  const [claimType, setClaimType] = useState<ClaimType>('HasDegree');
  const [proofHex, setProofHex] = useState('');
  const [zkResult, setZkResult] = useState<ZkResult | null>(null);
  const [zkLoading, setZkLoading] = useState(false);

  const handleVerify = async () => {
    if (!proofHex.trim()) { setZkResult({ kind: 'error', message: '⚠️ Please paste proof bytes.' }); return; }
    setZkLoading(true); setZkResult(null);
    try {
      const verified = await verifyClaim(credentialId, claimType, proofHex.trim());
      setZkResult({ kind: 'verified', value: verified });
    } catch (err: unknown) {
      setZkResult({ kind: 'error', message: err instanceof Error ? err.message : 'ZK verification failed.' });
    } finally { setZkLoading(false); }
  };

  return (
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
          <select value={claimType} onChange={e => setClaimType(e.target.value as ClaimType)} style={{ paddingLeft: 16 }}>
            {CLAIM_TYPE_OPTIONS.map(opt => (
              <option key={opt.value} value={opt.value}>{opt.label}</option>
            ))}
          </select>
        </div>
        <div className="form-row">
          <label className="form-label">ZK Proof (hex-encoded bytes)</label>
          <textarea placeholder="Paste hex-encoded proof bytes…" value={proofHex} onChange={e => setProofHex(e.target.value)} />
        </div>
        <div style={{ display: 'flex', gap: 10, alignItems: 'center', flexWrap: 'wrap' }}>
          <button className="btn btn--primary" onClick={handleVerify} disabled={zkLoading}>
            {zkLoading ? '⏳ Verifying…' : '🔐 Verify Claim'}
          </button>
          <button className="btn btn--ghost btn--sm" onClick={() => { setProofHex(''); setZkResult(null); }}>Clear</button>
          <span style={{ fontSize: 12, color: 'var(--text-muted)' }}>No wallet required</span>
        </div>
        {zkResult && (
          <div
            className={`zk-result zk-result--${zkResult.kind === 'verified' ? (zkResult.value ? 'success' : 'fail') : 'error'}`}
            role="alert"
          >
            {zkResult.kind === 'verified'
              ? (zkResult.value ? '✅ Claim Verified' : '❌ Claim Not Verified')
              : `⚠️ ${zkResult.message}`}
            {zkResult.kind === 'verified' && (
              <span style={{ marginLeft: 8, cursor: 'help' }} title={ZK_PRIVACY_TOOLTIP} aria-label="About zero-knowledge proofs">ℹ️</span>
            )}
          </div>
        )}
      </div>
    </div>
  );
}

function CredentialResult({ credential, attestors, expired, attested }: VerifyResult) {
  const metaStr = decodeMetadataHash(credential.metadata_hash);
  const shareUrl = buildShareUrl(credential.id);
  const isRevoked = credential.revoked;
  const { statusClass, statusIcon, statusTitle, statusSub } = deriveStatus(isRevoked, expired, attested, attestors.length, credential.expires_at);

  return (
    <div className="result-section">
      <div className={`status-banner status-banner--${statusClass}`}>
        <div className="status-banner__icon">{statusIcon}</div>
        <div>
          <div className="status-banner__title">{statusTitle}</div>
          <div className="status-banner__sub">{statusSub}</div>
        </div>
      </div>
      <div className="share-bar">
        <span style={{ fontSize: 13, color: 'var(--text-muted)' }}>🔗 Share:</span>
        <span className="share-bar__url">{shareUrl}</span>
        <button className="btn btn--ghost btn--sm" onClick={() => navigator.clipboard.writeText(shareUrl)}>Copy</button>
      </div>
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
      <div className="detail-card">
        <div className="detail-card__header">
          <span className="detail-card__title">ATTESTORS</span>
          <span className={`badge badge--${attestors.length > 0 ? 'green' : 'gray'}`}>{attestors.length} node{attestors.length !== 1 ? 's' : ''}</span>
        </div>
        <div className="detail-card__body">
          {attestors.length === 0
            ? <div style={{ color: 'var(--text-muted)', fontSize: 14, textAlign: 'center', padding: '20px 0' }}>No attestors have signed this credential yet.</div>
            : <div className="attestor-list">{attestors.map(addr => (
                <div key={addr} className="attestor-item">
                  <div className="attestor-item__avatar">🏛️</div>
                  <div className="attestor-item__addr" title={addr}>{addr}</div>
                  <span className="attestor-item__badge">✓ Signed</span>
                </div>
              ))}</div>
          }
        </div>
      </div>
      <ZkClaimPanel credentialId={credential.id} />
    </div>
  );
}


export default function Verify() {
  const [searchParams, setSearchParams] = useSearchParams();
  const [activeTab, setActiveTab] = useState<'id' | 'addr'>('id');
  const [credInput, setCredInput] = useState(searchParams.get('id') || '');
  const [addrInput, setAddrInput] = useState('');
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [result, setResult] = useState<VerifyResult | null>(null);
  const [addrResults, setAddrResults] = useState<bigint[] | null>(null);
  const autoTriggered = useRef(false);

  const fetchCred = async (id: bigint) => {
    setLoading(true); setError(null); setResult(null); setAddrResults(null);
    setSearchParams({ id: id.toString() });
    try {
      const [credential, attestors, expired, attested] = await Promise.all([
        getCredential(id),
        getAttestors(id).catch(() => [] as string[]),
        isExpired(id).catch(() => false),
        isAttested(id, DEFAULT_SLICE_ID).catch(() => null),
      ]);
      setResult({ credential, attestors, expired, attested });
    } catch (err: unknown) {
      setError(err instanceof Error ? err.message : 'Failed to fetch credential.');
    } finally { setLoading(false); }
  };

  const handleVerifyId = async () => {
    const id = parseInt(credInput, 10);
    if (isNaN(id) || id < 1) { setError('Please enter a valid credential ID (positive integer).'); return; }
    await fetchCred(BigInt(id));
  };

  const handleVerifyAddr = async () => {
    const addr = addrInput.trim();
    if (!addr.startsWith('G') || addr.length < 56) { setError('Please enter a valid Stellar address (starts with G, 56+ characters).'); return; }
    setLoading(true); setError(null); setResult(null); setAddrResults(null);
    try {
      const ids: bigint[] = await getCredentialsBySubject(addr);
      setAddrResults(ids || []);
    } catch (err: unknown) {
      setError(err instanceof Error ? err.message : 'Failed to look up address.');
    } finally { setLoading(false); }
  };

  useEffect(() => {
    const preId = searchParams.get('id');
    if (preId && !autoTriggered.current) {
      autoTriggered.current = true;
      const id = parseInt(preId, 10);
      if (!isNaN(id) && id > 0) fetchCred(BigInt(id));
      else setError('Invalid credential ID in URL.');
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
              <button className="btn btn--primary" onClick={handleVerifyId} disabled={loading} style={{ minWidth: 120 }}>{loading ? 'Verifying…' : 'Verify'}</button>
            </div>
          )}
          {activeTab === 'addr' && (
            <div className="input-group">
              <div className="input-wrap">
                <span className="input-icon">G</span>
                <input type="text" placeholder="Enter Stellar address (GABC…)" value={addrInput} onChange={e => setAddrInput(e.target.value)} onKeyDown={e => e.key === 'Enter' && handleVerifyAddr()} aria-label="Stellar address" spellCheck={false} />
              </div>
              <button className="btn btn--primary" onClick={handleVerifyAddr} disabled={loading} style={{ minWidth: 120 }}>{loading ? 'Looking up…' : 'Look Up'}</button>
            </div>
          )}
          <div style={{ marginTop: 16, display: 'flex', gap: 8, flexWrap: 'wrap' }}>
            <span className="badge badge--gray">🌐 {NETWORK}</span>
            <span className="badge badge--gray" style={{ fontSize: 10, fontFamily: 'var(--font-mono)', maxWidth: 300, overflow: 'hidden', textOverflow: 'ellipsis' }} title={RPC_URL}>{RPC_URL}</span>
            {CONTRACT_ID
              ? <span className="badge badge--blue" style={{ fontSize: 10, fontFamily: 'var(--font-mono)' }} title={`Contract: ${CONTRACT_ID}`}>📄 {formatAddress(CONTRACT_ID)}</span>
              : <span className="badge badge--red">⚠ Contract not configured</span>
            }
          </div>
        </div>
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
                    {addrResults.map(id => (
                      <div key={id.toString()} className="cred-list-item" role="button" tabIndex={0} onClick={() => fetchCred(id)} onKeyDown={e => (e.key === 'Enter' || e.key === ' ') && fetchCred(id)}>
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
