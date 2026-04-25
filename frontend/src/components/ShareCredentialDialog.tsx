import { useState } from 'react';

export type SharePermission = 'view' | 'verify' | 'full';

export interface ShareEntry {
  address: string;
  permission: SharePermission;
}

interface Props {
  credentialId: string;
  onClose: () => void;
}

const PERMISSION_LABELS: Record<SharePermission, { label: string; desc: string }> = {
  view:   { label: 'View',   desc: 'Can see credential details' },
  verify: { label: 'Verify', desc: 'Can verify and attest' },
  full:   { label: 'Full',   desc: 'Full access including export' },
};

function isValidStellarAddress(addr: string): boolean {
  return addr.startsWith('G') && addr.length >= 56;
}

export function ShareCredentialDialog({ credentialId, onClose }: Props) {
  const [shares, setShares] = useState<ShareEntry[]>([]);
  const [address, setAddress] = useState('');
  const [permission, setPermission] = useState<SharePermission>('view');
  const [error, setError] = useState('');
  const [copied, setCopied] = useState(false);

  const shareUrl = `${window.location.origin}/verify?id=${credentialId}`;

  function handleAdd() {
    const trimmed = address.trim();
    if (!isValidStellarAddress(trimmed)) {
      setError('Enter a valid Stellar address (starts with G, 56+ chars)');
      return;
    }
    if (shares.some((s) => s.address === trimmed)) {
      setError('Address already added');
      return;
    }
    setShares((prev) => [...prev, { address: trimmed, permission }]);
    setAddress('');
    setError('');
  }

  function handleRemove(addr: string) {
    setShares((prev) => prev.filter((s) => s.address !== addr));
  }

  function handlePermissionChange(addr: string, perm: SharePermission) {
    setShares((prev) =>
      prev.map((s) => (s.address === addr ? { ...s, permission: perm } : s))
    );
  }

  function handleCopyLink() {
    navigator.clipboard.writeText(shareUrl).then(() => {
      setCopied(true);
      setTimeout(() => setCopied(false), 2000);
    });
  }

  return (
    <div
      className="share-dialog-overlay"
      role="dialog"
      aria-modal="true"
      aria-label="Share credential"
      onClick={(e) => { if (e.target === e.currentTarget) onClose(); }}
    >
      <div className="share-dialog">
        <div className="share-dialog__header">
          <h2 className="share-dialog__title">Share Credential #{credentialId}</h2>
          <button
            className="share-dialog__close"
            onClick={onClose}
            aria-label="Close share dialog"
          >
            ✕
          </button>
        </div>

        {/* Public link */}
        <div className="share-dialog__section">
          <label className="share-dialog__label">Public verification link</label>
          <div className="share-dialog__link-row">
            <span className="share-dialog__link-url" title={shareUrl}>{shareUrl}</span>
            <button
              className="btn btn--sm btn--ghost"
              onClick={handleCopyLink}
              aria-label="Copy verification link"
            >
              {copied ? '✅ Copied' : '📋 Copy'}
            </button>
          </div>
        </div>

        {/* Add address */}
        <div className="share-dialog__section">
          <label className="share-dialog__label" htmlFor="share-address">
            Share with Stellar address
          </label>
          <div className="share-dialog__add-row">
            <input
              id="share-address"
              className="share-dialog__input"
              type="text"
              placeholder="G…"
              value={address}
              onChange={(e) => { setAddress(e.target.value); setError(''); }}
              aria-describedby={error ? 'share-error' : undefined}
            />
            <select
              className="share-dialog__select"
              value={permission}
              onChange={(e) => setPermission(e.target.value as SharePermission)}
              aria-label="Permission level"
            >
              {(Object.keys(PERMISSION_LABELS) as SharePermission[]).map((p) => (
                <option key={p} value={p}>{PERMISSION_LABELS[p].label}</option>
              ))}
            </select>
            <button className="btn btn--sm btn--primary" onClick={handleAdd}>
              Add
            </button>
          </div>
          {error && (
            <p id="share-error" className="share-dialog__error" role="alert">{error}</p>
          )}
        </div>

        {/* Shared with list */}
        {shares.length > 0 && (
          <div className="share-dialog__section">
            <label className="share-dialog__label">Shared with</label>
            <ul className="share-dialog__list" aria-label="Shared addresses">
              {shares.map(({ address: addr, permission: perm }) => (
                <li key={addr} className="share-dialog__list-item">
                  <span className="share-dialog__addr mono" title={addr}>
                    {addr.slice(0, 8)}…{addr.slice(-6)}
                  </span>
                  <select
                    className="share-dialog__select share-dialog__select--sm"
                    value={perm}
                    onChange={(e) => handlePermissionChange(addr, e.target.value as SharePermission)}
                    aria-label={`Permission for ${addr.slice(0, 8)}…`}
                  >
                    {(Object.keys(PERMISSION_LABELS) as SharePermission[]).map((p) => (
                      <option key={p} value={p} title={PERMISSION_LABELS[p].desc}>
                        {PERMISSION_LABELS[p].label}
                      </option>
                    ))}
                  </select>
                  <button
                    className="share-dialog__remove"
                    onClick={() => handleRemove(addr)}
                    aria-label={`Remove ${addr.slice(0, 8)}…`}
                  >
                    ✕
                  </button>
                </li>
              ))}
            </ul>
          </div>
        )}

        <div className="share-dialog__footer">
          <button className="btn btn--ghost btn--sm" onClick={onClose}>Close</button>
        </div>
      </div>
    </div>
  );
}
