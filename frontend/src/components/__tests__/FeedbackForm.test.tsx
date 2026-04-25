import { describe, it, expect, vi, beforeEach } from 'vitest';
import { render, screen, fireEvent, waitFor } from '@testing-library/react';
import '@testing-library/jest-dom';
import { FeedbackForm } from '../../components/FeedbackForm';

// Mock useFeedback so we control submission state
vi.mock('../../hooks/useFeedback', () => ({
  useFeedback: vi.fn(),
}));

function mockHook(overrides = {}) {
  const { useFeedback } = require('../../hooks/useFeedback');
  useFeedback.mockReturnValue({
    status: 'idle',
    error: null,
    submit: vi.fn(),
    reset: vi.fn(),
    ...overrides,
  });
}

describe('FeedbackForm (#323)', () => {
  beforeEach(() => {
    vi.clearAllMocks();
    localStorage.clear();
  });

  // ── Rendering ──────────────────────────────────────────────────────────────

  it('renders the form with all fields', () => {
    mockHook();
    render(<FeedbackForm />);
    expect(screen.getByRole('form', { name: 'Feedback Form' })).toBeInTheDocument();
    expect(screen.getByRole('group', { name: 'Rating' })).toBeInTheDocument();
    expect(screen.getByLabelText('Category')).toBeInTheDocument();
    expect(screen.getByLabelText('Message')).toBeInTheDocument();
    expect(screen.getByRole('button', { name: 'Submit Feedback' })).toBeInTheDocument();
  });

  it('renders 5 star buttons', () => {
    mockHook();
    render(<FeedbackForm />);
    for (let i = 1; i <= 5; i++) {
      expect(screen.getByRole('button', { name: `${i} star${i > 1 ? 's' : ''}` })).toBeInTheDocument();
    }
  });

  // ── Validation ─────────────────────────────────────────────────────────────

  it('shows validation errors when submitted empty', async () => {
    mockHook();
    render(<FeedbackForm />);
    fireEvent.submit(screen.getByRole('form', { name: 'Feedback Form' }));
    expect(await screen.findByText('Please select a rating.')).toBeInTheDocument();
    expect(await screen.findByText('Please select a category.')).toBeInTheDocument();
    expect(await screen.findByText('Feedback message is required.')).toBeInTheDocument();
  });

  it('shows message-too-short error', async () => {
    mockHook();
    render(<FeedbackForm />);
    fireEvent.change(screen.getByLabelText('Message'), { target: { value: 'short' } });
    fireEvent.submit(screen.getByRole('form', { name: 'Feedback Form' }));
    expect(await screen.findByText('Message must be at least 10 characters.')).toBeInTheDocument();
  });

  it('clears field error when user corrects input', async () => {
    mockHook();
    render(<FeedbackForm />);
    fireEvent.submit(screen.getByRole('form', { name: 'Feedback Form' }));
    expect(await screen.findByText('Feedback message is required.')).toBeInTheDocument();
    fireEvent.change(screen.getByLabelText('Message'), { target: { value: 'This is valid feedback text' } });
    expect(screen.queryByText('Feedback message is required.')).not.toBeInTheDocument();
  });

  // ── Submission ─────────────────────────────────────────────────────────────

  it('calls submit with correct payload on valid form', async () => {
    const submit = vi.fn().mockResolvedValue(undefined);
    mockHook({ submit });
    render(<FeedbackForm walletAddress="GABC123" />);

    fireEvent.click(screen.getByRole('button', { name: '3 stars' }));
    fireEvent.change(screen.getByLabelText('Category'), { target: { value: 'Verification' } });
    fireEvent.change(screen.getByLabelText('Message'), { target: { value: 'Great experience overall!' } });
    fireEvent.submit(screen.getByRole('form', { name: 'Feedback Form' }));

    await waitFor(() => {
      expect(submit).toHaveBeenCalledWith({
        rating: 3,
        category: 'Verification',
        message: 'Great experience overall!',
        walletAddress: 'GABC123',
      });
    });
  });

  it('does not call submit when validation fails', () => {
    const submit = vi.fn();
    mockHook({ submit });
    render(<FeedbackForm />);
    fireEvent.submit(screen.getByRole('form', { name: 'Feedback Form' }));
    expect(submit).not.toHaveBeenCalled();
  });

  it('disables submit button while submitting', () => {
    mockHook({ status: 'submitting' });
    render(<FeedbackForm />);
    const btn = screen.getByRole('button', { name: /Submitting/ });
    expect(btn).toBeDisabled();
    expect(btn).toHaveAttribute('aria-busy', 'true');
  });

  // ── Success state ──────────────────────────────────────────────────────────

  it('shows success message after successful submission', () => {
    mockHook({ status: 'success' });
    render(<FeedbackForm />);
    expect(screen.getByText('Thank you for your feedback!')).toBeInTheDocument();
    expect(screen.getByRole('button', { name: 'Submit another' })).toBeInTheDocument();
  });

  it('calls reset when "Submit another" is clicked', () => {
    const reset = vi.fn();
    mockHook({ status: 'success', reset });
    render(<FeedbackForm />);
    fireEvent.click(screen.getByRole('button', { name: 'Submit another' }));
    expect(reset).toHaveBeenCalled();
  });

  // ── Error state ────────────────────────────────────────────────────────────

  it('shows submission error when hook returns error', () => {
    mockHook({ status: 'error', error: 'Network error' });
    render(<FeedbackForm />);
    expect(screen.getByRole('alert')).toHaveTextContent('Network error');
  });

  // ── Accessibility ──────────────────────────────────────────────────────────

  it('star buttons have aria-pressed reflecting current rating', () => {
    mockHook();
    render(<FeedbackForm />);
    const star2 = screen.getByRole('button', { name: '2 stars' });
    expect(star2).toHaveAttribute('aria-pressed', 'false');
    fireEvent.click(star2);
    expect(star2).toHaveAttribute('aria-pressed', 'true');
  });

  it('message textarea has aria-invalid when there is an error', async () => {
    mockHook();
    render(<FeedbackForm />);
    fireEvent.submit(screen.getByRole('form', { name: 'Feedback Form' }));
    await screen.findByText('Feedback message is required.');
    expect(screen.getByLabelText('Message')).toHaveAttribute('aria-invalid', 'true');
  });
});
