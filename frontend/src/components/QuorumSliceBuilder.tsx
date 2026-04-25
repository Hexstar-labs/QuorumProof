import { useState } from 'react';
import type { ChangeEvent, FormEvent } from 'react';
import { createSlice } from '../lib/contracts/quorumProof';
import { useToast } from '../context/ToastContext';

const ROLES = ['University', 'Licensing Body', 'Employer', 'Other'] as const;
type Role = (typeof ROLES)[number];

const ROLE_ICONS: Record<Role, string> = {
  University: '🎓',
  'Licensing Body': '🏛️',
  Employer: '💼',
  Other: '🔹',
};

interface Attestor {
  id: string;
  address: string;
  role: Role;
}

function isValidStellarAddress(addr: string): boolean {
  return /^G[A-Z2-7]{55}$/.test(addr.trim());
}

function formatAddress(addr: string) {
  return addr.slice(0, 8) + '…' + addr.slice(-6);
}

interface SuccessState {
  sliceId: bigint;
}

export function QuorumSliceBuilder({ creatorAddress }: { creatorAddress: string }) {
  const { addToast, removeToast } = useToast();
  // Add-attestor form state
  const [addrInput, setAddrInput] = useState('');
  const [roleInput, setRoleInput] = useState<Role>('University');
  const [addrError, setAddrError] = useState('');

  // Slice state
  const [attestors, setAttestors] = useState<Attestor[]>([]);
  const [threshold, setThreshold] = useState(1);
  const [thresholdError, setThresholdError] = useState('');

  // Submit state
  const [submitting, setSubmitting] = useState(false);
  const [submitError, setSubmitError] = useState('');
  const [success, setSuccess] = useState<SuccessState | null>(null);

  function handleAddAttestor(e: FormEvent) {
    e.preventDefault();
    const trimmed = addrInput.trim();
    if (!trimmed) { setAddrError('Address is required.'); return; }
    if (!isValidStellarAddress(trimmed)) { setAddrError('Must be a valid Stellar address (G…, 56 chars).'); return; }
    if (attestors.some((a) => a.address === trimmed)) { setAddrError('This address is already in the slice.'); return; }
    setAddrError('');
    setAttestors((prev) => [...prev, { id: crypto.randomUUID(), address: trimmed, role: roleInput }]);
    setAddrInput('');
  }

  function handleRemove(id: string) {
    setAttestors((prev) => {
      const next = prev.filter((a) => a.id !== id);
      if (threshold > next.length && next.length > 0) setThreshold(next.length);
      return next;
    });
  }

  function handleThresholdChange(e: ChangeEvent<HTMLInputElement>) {
    const val = Number(e.target.value);
    setThreshold(val);
    if (attestors.length > 0 && val > attestors.length) {
      setThresholdError(`Threshold cannot exceed number of attestors (${attestors.length}).`);
    } else if (val < 1) {
      setThresholdError('Threshold must be at least 1.');
    } else {
      setThresholdError('');
    }
  }

  async function handleSubmit(e: FormEvent) {
    e.preventDefault();
    if (attestors.length === 0) return;
    if (threshold < 1 || threshold > attestors.length) return;
    setSubmitError('');
    setSubmitting(true);
    const pendingId = addToast({ type: 'pending', message: 'Transaction pending…' });
    try {
      const sliceId = await createSlice(
        creatorAddress,
        attestors.map((a) => a.address),
        threshold,
      );
      removeToast(pendingId);
      addToast({
        type: 'success',
        message: 'Transaction confirmed — quorum slice created.',
        explorerUrl: `https://stellar.expert/explorer/testnet/tx/${sliceId.toString()}`,
      });
      setSuccess({ sliceId });
    } catch (err: unknown) {
      removeToast(pendingId);
      const msg = err instanceof Error ? err.message : 'Failed to create slice.';
      addToast({ type: 'error', message: `Transaction failed: ${msg}` });
      setSubmitError(msg);
    } finally {
      setSubmitting(false);
    }
  }

  function handleReset() {
    setAttestors([]);
    setThreshold(1);
    setThresholdError('');
    setSubmitError('');
    setSuccess(null);
    setAddrInput('');
  }

  if (success) {
    return (
      <div className="qsb-success" role="status" aria-live="polite">
        <div className="status-banner status-banner--valid">
          <div className="status-banner__icon">✅</div>
          <div>
            <div className="status-banner__title">Quorum Slice Created</div>
            <div className="status-banner__sub">
              Slice #{success.sliceId.toString()} is live on-chain with {attestors.length} attestor{attestors.length !== 1 ? 's' : ''} and a threshold of {threshold}.
            </div>
          </div>
        </div>
        <div className="qsb-success__actions">
          <button className="btn btn--ghost" onClick={handleReset}>Build Another Slice</button>
        </div>
      </div>
    );
  }

  const canSubmit = attestors.length > 0 && threshold >= 1 && threshold <= attestors.length && !thresholdError;

  return (
    <div className="qsb">
      {/* ── Add Attestor ── */}
      <section className="qsb__section" aria-label="Add attestor">
        <div className="detail-card__header" style={{ padding: '0 0 12px', background: 'none', border: 'none' }}>
          <span className="detail-card__title">Add Attestor</span>
        </div>
        <form onSubmit={handleAddAttestor} noValidate>
          <div className="form-row">
            <label htmlFor="qsb-addr" className="form-label">Stellar Address</label>
            <div className="input-wrap">
              <span className="input-icon" aria-hidden="true">👤</span>
              <input
                id="qsb-addr"
                type="text"
                placeholder="GABC…XYZ"
                value={addrInput}
                onChange={(e: ChangeEvent<HTMLInputElement>) => { setAddrInput(e.target.value); setAddrError(''); }}
                aria-invalid={!!addrError}
                aria-describedby={addrError ? 'qsb-addr-err' : undefined}
                aria-label="Attestor Stellar address"
                autoComplete="off"
                spellCheck={false}
              />
            </div>
            {addrError && <p id="qsb-addr-err" className="issue-form__field-error" role="alert">{addrError}</p>}
          </div>
          <div className="form-row" style={{ marginBottom: 0 }}>
            <label htmlFor="qsb-role" className="form-label">Role</label>
            <div className="input-wrap">
              <span className="input-icon" aria-hidden="true">{ROLE_ICONS[roleInput]}</span>
              <select
                id="qsb-role"
                value={roleInput}
                onChange={(e: ChangeEvent<HTMLSelectElement>) => setRoleInput(e.target.value as Role)}
                aria-label="Attestor role"
              >
                {ROLES.map((r) => (
                  <option key={r} value={r}>{ROLE_ICONS[r]} {r}</option>
                ))}
              </select>
            </div>
          </div>
          <button
            type="submit"
            className="btn btn--ghost btn--sm"
            style={{ marginTop: 16, width: '100%' }}
          >
            + Add to Slice
          </button>
        </form>
      </section>

      <div className="divider" />

      {/* ── Attestor List ── */}
      <section className="qsb__section" aria-label="Attestor list">
        <div className="detail-card__header" style={{ padding: '0 0 12px', background: 'none', border: 'none' }}>
          <span className="detail-card__title">Attestors ({attestors.length})</span>
        </div>
        {attestors.length === 0 ? (
          <p className="qsb__empty">No attestors added yet. Add at least one above.</p>
        ) : (
          <ul className="qsb__attestor-list" aria-label="Added attestors">
            {attestors.map((a) => (
              <li key={a.id} className="qsb__attestor-item">
                <div className="attestor-mini-item__avatar" aria-hidden="true">
                  {ROLE_ICONS[a.role]}
                </div>
                <div className="qsb__attestor-info">
                  <span className="qsb__attestor-addr mono" title={a.address}>{formatAddress(a.address)}</span>
                  <span className="qsb__attestor-role">{a.role}</span>
                </div>
                <button
                  className="qsb__remove-btn"
                  onClick={() => handleRemove(a.id)}
                  aria-label={`Remove ${a.role} ${formatAddress(a.address)}`}
                >
                  ✕
                </button>
              </li>
            ))}
          </ul>
        )}
      </section>

      <div className="divider" />

      {/* ── Threshold ── */}
      <section className="qsb__section" aria-label="Threshold">
        <div className="detail-card__header" style={{ padding: '0 0 12px', background: 'none', border: 'none' }}>
          <span className="detail-card__title">Attestation Threshold</span>
        </div>
        <div className="form-row" style={{ marginBottom: 0 }}>
          <label htmlFor="qsb-threshold" className="form-label">
            Minimum signatures required
          </label>
          <div className="input-wrap">
            <span className="input-icon" aria-hidden="true">🔢</span>
            <input
              id="qsb-threshold"
              type="number"
              min={1}
              max={attestors.length || 1}
              value={threshold}
              onChange={handleThresholdChange}
              aria-invalid={!!thresholdError}
              aria-describedby={thresholdError ? 'qsb-threshold-err' : 'qsb-threshold-hint'}
              aria-label="Attestation threshold"
            />
          </div>
          {thresholdError
            ? <p id="qsb-threshold-err" className="issue-form__field-error" role="alert">{thresholdError}</p>
            : <p id="qsb-threshold-hint" className="issue-form__hint">Must be between 1 and {attestors.length || '—'} (number of attestors).</p>
          }
        </div>
      </section>

      <div className="divider" />

      {/* ── Preview ── */}
      <section className="qsb__preview" aria-label="Slice preview">
        <div className="detail-card__header" style={{ padding: '0 0 12px', background: 'none', border: 'none' }}>
          <span className="detail-card__title">Preview</span>
        </div>
        <div className="qsb__preview-card">
          <div className="meta-row">
            <span className="meta-label">Creator</span>
            <span className="meta-value mono" title={creatorAddress}>{formatAddress(creatorAddress)}</span>
          </div>
          <div className="meta-row">
            <span className="meta-label">Attestors</span>
            <span className="meta-value">{attestors.length}</span>
          </div>
          <div className="meta-row">
            <span className="meta-label">Threshold</span>
            <span className="meta-value">
              {threshold} / {attestors.length || '—'}
              {attestors.length > 0 && (
                <span className="qsb__threshold-pct">
                  {' '}({Math.round((threshold / attestors.length) * 100)}%)
                </span>
              )}
            </span>
          </div>
          {attestors.length > 0 && (
            <div className="slice-progress" style={{ marginTop: 12 }}>
              <div
                className="slice-progress__bar"
                style={{ width: `${(threshold / attestors.length) * 100}%` }}
                role="progressbar"
                aria-valuenow={threshold}
                aria-valuemin={1}
                aria-valuemax={attestors.length}
              />
            </div>
          )}
        </div>
      </section>

      {/* ── Submit ── */}
      {submitError && (
        <div className="error-card" role="alert">
          <span className="error-card__icon">⚠️</span>
          <div>
            <div className="error-card__title">Transaction Failed</div>
            <div className="error-card__msg">{submitError}</div>
          </div>
        </div>
      )}

      <form onSubmit={handleSubmit}>
        <button
          type="submit"
          className="btn btn--primary"
          style={{ width: '100%', marginTop: 8 }}
          disabled={!canSubmit || submitting}
          aria-busy={submitting}
        >
          {submitting ? (
            <>
              <span className="spinner" style={{ width: 16, height: 16, borderWidth: 2 }} aria-hidden="true" />
              Creating Slice…
            </>
          ) : (
            'Create Quorum Slice'
          )}
        </button>
      </form>
    </div>
  );
}
