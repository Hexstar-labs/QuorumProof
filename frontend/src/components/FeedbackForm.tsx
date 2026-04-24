import { useState } from 'react';
import type { FormEvent, ChangeEvent } from 'react';
import { useFeedback } from '../hooks/useFeedback';

const CATEGORIES = ['General', 'Credential Issuance', 'Verification', 'Attestation', 'Other'] as const;

interface FormState {
  rating: number;
  category: string;
  message: string;
}

interface FormErrors {
  rating?: string;
  category?: string;
  message?: string;
}

function validate(form: FormState): FormErrors {
  const errs: FormErrors = {};
  if (!form.rating) errs.rating = 'Please select a rating.';
  if (!form.category) errs.category = 'Please select a category.';
  if (!form.message.trim()) errs.message = 'Feedback message is required.';
  else if (form.message.trim().length < 10) errs.message = 'Message must be at least 10 characters.';
  return errs;
}

export function FeedbackForm({ walletAddress }: { walletAddress?: string }) {
  const { status, error, submit, reset } = useFeedback();
  const [form, setForm] = useState<FormState>({ rating: 0, category: '', message: '' });
  const [errors, setErrors] = useState<FormErrors>({});

  async function handleSubmit(e: FormEvent<HTMLFormElement>) {
    e.preventDefault();
    const errs = validate(form);
    if (Object.keys(errs).length > 0) { setErrors(errs); return; }
    setErrors({});
    await submit({ ...form, walletAddress });
  }

  function handleChange<K extends keyof FormState>(field: K, value: FormState[K]) {
    setForm(prev => ({ ...prev, [field]: value }));
    if (errors[field]) setErrors(prev => ({ ...prev, [field]: undefined }));
  }

  if (status === 'success') {
    return (
      <div className="status-banner status-banner--valid" role="status" aria-live="polite">
        <div className="status-banner__icon">✅</div>
        <div>
          <div className="status-banner__title">Thank you for your feedback!</div>
          <div className="status-banner__sub">Your response has been recorded.</div>
        </div>
        <button className="btn btn--ghost" onClick={reset} style={{ marginLeft: 'auto' }}>
          Submit another
        </button>
      </div>
    );
  }

  return (
    <form
      className="issue-form"
      onSubmit={handleSubmit}
      noValidate
      aria-label="Feedback Form"
    >
      {/* Star rating */}
      <div className="form-row">
        <fieldset style={{ border: 'none', padding: 0, margin: 0 }}>
          <legend className="form-label">Rating</legend>
          <div role="group" aria-label="Rating" style={{ display: 'flex', gap: '0.5rem' }}>
            {[1, 2, 3, 4, 5].map(n => (
              <button
                key={n}
                type="button"
                aria-label={`${n} star${n > 1 ? 's' : ''}`}
                aria-pressed={form.rating === n}
                onClick={() => handleChange('rating', n)}
                style={{
                  fontSize: '1.5rem',
                  background: 'none',
                  border: 'none',
                  cursor: 'pointer',
                  opacity: form.rating >= n ? 1 : 0.3,
                }}
              >
                ★
              </button>
            ))}
          </div>
          {errors.rating && <p className="issue-form__field-error" role="alert">{errors.rating}</p>}
        </fieldset>
      </div>

      {/* Category */}
      <div className="form-row">
        <label htmlFor="fb-category" className="form-label">Category</label>
        <div className="input-wrap">
          <select
            id="fb-category"
            value={form.category}
            onChange={(e: ChangeEvent<HTMLSelectElement>) => handleChange('category', e.target.value)}
            aria-invalid={!!errors.category}
          >
            <option value="">Select a category…</option>
            {CATEGORIES.map(c => <option key={c} value={c}>{c}</option>)}
          </select>
        </div>
        {errors.category && <p className="issue-form__field-error" role="alert">{errors.category}</p>}
      </div>

      {/* Message */}
      <div className="form-row">
        <label htmlFor="fb-message" className="form-label">Message</label>
        <textarea
          id="fb-message"
          rows={4}
          placeholder="Tell us about your experience…"
          value={form.message}
          onChange={(e: ChangeEvent<HTMLTextAreaElement>) => handleChange('message', e.target.value)}
          aria-invalid={!!errors.message}
          aria-describedby={errors.message ? 'fb-message-err' : undefined}
          style={{ width: '100%', resize: 'vertical', padding: '0.5rem', borderRadius: '0.375rem' }}
        />
        {errors.message && <p id="fb-message-err" className="issue-form__field-error" role="alert">{errors.message}</p>}
      </div>

      {error && (
        <div className="error-card" role="alert">
          <span className="error-card__icon">⚠️</span>
          <div className="error-card__msg">{error}</div>
        </div>
      )}

      <button
        type="submit"
        className="btn btn--primary issue-form__submit"
        disabled={status === 'submitting'}
        aria-busy={status === 'submitting'}
      >
        {status === 'submitting' ? 'Submitting…' : 'Submit Feedback'}
      </button>
    </form>
  );
}
