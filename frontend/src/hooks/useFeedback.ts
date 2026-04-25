import { useState } from 'react';

export interface FeedbackPayload {
  rating: number;
  category: string;
  message: string;
  walletAddress?: string;
}

export type FeedbackStatus = 'idle' | 'submitting' | 'success' | 'error';

export interface UseFeedbackResult {
  status: FeedbackStatus;
  error: string | null;
  submit: (payload: FeedbackPayload) => Promise<void>;
  reset: () => void;
}

/**
 * Handles feedback form submission.
 * Stores feedback in localStorage (no backend required).
 * Replace the `_persist` body with a real API call when a backend is available.
 */
export function useFeedback(): UseFeedbackResult {
  const [status, setStatus] = useState<FeedbackStatus>('idle');
  const [error, setError] = useState<string | null>(null);

  async function submit(payload: FeedbackPayload): Promise<void> {
    setStatus('submitting');
    setError(null);
    try {
      await _persist(payload);
      setStatus('success');
    } catch (err) {
      setError(err instanceof Error ? err.message : 'Submission failed.');
      setStatus('error');
    }
  }

  function reset() {
    setStatus('idle');
    setError(null);
  }

  return { status, error, submit, reset };
}

/** Persists feedback locally. Swap for a fetch() call when a backend exists. */
async function _persist(payload: FeedbackPayload): Promise<void> {
  const existing = JSON.parse(localStorage.getItem('qp_feedback') ?? '[]') as FeedbackPayload[];
  existing.push({ ...payload, _ts: Date.now() } as FeedbackPayload & { _ts: number });
  localStorage.setItem('qp_feedback', JSON.stringify(existing));
}
