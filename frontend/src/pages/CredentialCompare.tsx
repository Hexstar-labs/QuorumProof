import { useState } from 'react';
import { Navbar } from '../components/Navbar';
import { getCredential, getAttestors, isExpired } from '../lib/contracts/quorumProof';
import type { Credential } from '../lib/contracts/quorumProof';
import { credTypeLabel, formatAddress, formatTimestamp } from '../lib/credentialUtils';

// ── Types ─────────────────────────────────────────────────────────────────────

interface CredentialInfo {
  credential: Credential;
  attestors: string[];
  expired: boolean;
}

type FieldKey = 'type' | 'subject' | 'issuer' | 'expires' | 'revoked' | 'attestors';

interface Field {
  key: FieldKey;
  label: string;
  value: (info: CredentialInfo) => string;
}

// ── Field definitions ─────────────────────────────────────────────────────────

const FIELDS: Field[] = [
  { key: 'type',      label: 'Type',       value: (i) => credTypeLabel(i.credential.credential_type) },
  { key: 'subject',   label: 'Subject',    value: (i) => formatAddress(i.credential.subject) },
  { key: 'issuer',    label: 'Issuer',     value: (i) => formatAddress(i.credential.issuer) },
  { key: 'expires',   label: 'Expires',    value: (i) => formatTimestamp(i.credential.expires_at) },
  { key: 'revoked',   label: 'Revoked',    value: (i) => i.credential.revoked ? 'Yes' : 'No' },
  { key: 'attestors', label: 'Attestors',  value: (i) => i.attestors.length.toString() },
];

// ── Helpers ───────────────────────────────────────────────────────────────────

async function loadCredentialInfo(id: string): Promise<CredentialInfo> {
  const credId = BigInt(id.trim());
  const [credential, expired, attestors] = await Promise.all([
    getCredential(credId),
    isExpired(credId).catch(() => false),
    getAttestors(credId).catch(() => [] as string[]),
  ]);
  return { credential, expired, attestors };
}

// ── Component ─────────────────────────────────────────────────────────────────

export default function CredentialCompare() {
  const [idA, setIdA] = useState('');
  const [idB, setIdB] = useState('');
  const [infoA, setInfoA] = useState<CredentialInfo | null>(null);
  const [infoB, setInfoB] = useState<CredentialInfo | null>(null);
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);

  async function handleCompare() {
    const trimA = idA.trim();
    const trimB = idB.trim();
    if (!trimA || !trimB) { setError('Enter both credential IDs.'); return; }
    if (trimA === trimB)  { setError('Enter two different credential IDs.'); return; }

    setLoading(true);
    setError(null);
    setInfoA(null);
    setInfoB(null);

    try {
      const [a, b] = await Promise.all([
        loadCredentialInfo(trimA),
        loadCredentialInfo(trimB),
      ]);
      setInfoA(a);
      setInfoB(b);
    } catch (err) {
      setError(err instanceof Error ? err.message : 'Failed to load credentials.');
    } finally {
      setLoading(false);
    }
  }

  const hasDiff = (field: Field) =>
    infoA && infoB && field.value(infoA) !== field.value(infoB);

  return (
    <>
      <Navbar />
      <main className="container" style={{ paddingTop: '40px', maxWidth: '900px', paddingBottom: '64px' }}>
        <h1 className="compare-title">Compare Credentials</h1>
        <p className="compare-subtitle">Enter two credential IDs to compare them side-by-side.</p>

        {/* Input row */}
        <div className="compare-inputs">
          <div className="compare-input-group">
            <label className="compare-input-label" htmlFor="cred-id-a">Credential A</label>
            <input
              id="cred-id-a"
              className="compare-input"
              type="text"
              placeholder="e.g. 1"
              value={idA}
              onChange={(e) => setIdA(e.target.value)}
              aria-label="Credential A ID"
            />
          </div>
          <div className="compare-vs" aria-hidden="true">VS</div>
          <div className="compare-input-group">
            <label className="compare-input-label" htmlFor="cred-id-b">Credential B</label>
            <input
              id="cred-id-b"
              className="compare-input"
              type="text"
              placeholder="e.g. 2"
              value={idB}
              onChange={(e) => setIdB(e.target.value)}
              aria-label="Credential B ID"
            />
          </div>
          <button
            className="btn btn--primary"
            onClick={handleCompare}
            disabled={loading}
            aria-label="Compare credentials"
          >
            {loading ? 'Loading…' : 'Compare'}
          </button>
        </div>

        {error && (
          <p className="compare-error" role="alert">{error}</p>
        )}

        {/* Comparison table */}
        {infoA && infoB && (
          <div className="compare-table" role="table" aria-label="Credential comparison">
            {/* Header */}
            <div className="compare-row compare-row--header" role="row">
              <div className="compare-cell compare-cell--label" role="columnheader">Field</div>
              <div className="compare-cell" role="columnheader">
                Credential #{infoA.credential.id.toString()}
              </div>
              <div className="compare-cell" role="columnheader">
                Credential #{infoB.credential.id.toString()}
              </div>
            </div>

            {FIELDS.map((field) => {
              const diff = hasDiff(field);
              return (
                <div
                  key={field.key}
                  className={`compare-row${diff ? ' compare-row--diff' : ''}`}
                  role="row"
                  aria-label={diff ? `${field.label}: values differ` : undefined}
                >
                  <div className="compare-cell compare-cell--label" role="cell">
                    {field.label}
                    {diff && <span className="compare-diff-badge" aria-label="Different">≠</span>}
                  </div>
                  <div className="compare-cell" role="cell" data-testid={`field-a-${field.key}`}>
                    {field.value(infoA)}
                  </div>
                  <div className="compare-cell" role="cell" data-testid={`field-b-${field.key}`}>
                    {field.value(infoB)}
                  </div>
                </div>
              );
            })}
          </div>
        )}
      </main>

      <footer className="footer">
        <div className="container">
          Powered by{' '}
          <a href="https://stellar.org" target="_blank" rel="noopener">Stellar Soroban</a>
          {' · '}
          <a href="https://github.com/Phantomcall/QuorumProof" target="_blank" rel="noopener">QuorumProof</a>
        </div>
      </footer>
    </>
  );
}
