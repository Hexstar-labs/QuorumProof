/**
 * ShareCredentialDialog.test.tsx
 * Tests for the credential sharing UI — issue #319
 */

import { describe, it, expect, vi, beforeEach } from 'vitest';
import { render, screen, fireEvent, act } from '@testing-library/react';
import '@testing-library/jest-dom';
import { ShareCredentialDialog } from '../components/ShareCredentialDialog';

// ── Fixtures ──────────────────────────────────────────────────────────────────

// 56-char Stellar address (G + 55 chars)
const VALID_ADDRESS = 'GAAZI4TCR3TY5OJHCTJC2A4QSY6CJWJH5IAJTGKIN2ER7LBNVKOCCWNN';
const SHORT_ADDRESS = 'GABC123';
const CRED_ID = '42';

function renderDialog(onClose = vi.fn()) {
  return render(
    <ShareCredentialDialog credentialId={CRED_ID} onClose={onClose} />
  );
}

async function fillAndAdd(address: string) {
  await act(async () => {
    fireEvent.change(screen.getByPlaceholderText('G…'), { target: { value: address } });
  });
  await act(async () => {
    fireEvent.click(screen.getByText('Add'));
  });
}

// ── Address validation (pure logic mirrored from component) ──────────────────

function isValidStellarAddress(addr: string): boolean {
  return addr.startsWith('G') && addr.length >= 56;
}

describe('isValidStellarAddress', () => {
  it('accepts a valid Stellar address', () => {
    expect(isValidStellarAddress(VALID_ADDRESS)).toBe(true);
  });

  it('rejects an address that does not start with G', () => {
    expect(isValidStellarAddress('X' + VALID_ADDRESS.slice(1))).toBe(false);
  });

  it('rejects an address shorter than 56 chars', () => {
    expect(isValidStellarAddress(SHORT_ADDRESS)).toBe(false);
  });

  it('rejects an empty string', () => {
    expect(isValidStellarAddress('')).toBe(false);
  });
});

// ── Rendering ────────────────────────────────────────────────────────────────

describe('ShareCredentialDialog rendering', () => {
  beforeEach(() => {
    Object.assign(navigator, {
      clipboard: { writeText: vi.fn().mockResolvedValue(undefined) },
    });
  });

  it('renders the dialog with the credential ID in the title', () => {
    renderDialog();
    expect(screen.getByRole('dialog')).toBeInTheDocument();
    expect(screen.getByText(`Share Credential #${CRED_ID}`)).toBeInTheDocument();
  });

  it('shows the public verification link', () => {
    renderDialog();
    expect(screen.getByText(/\/verify\?id=42/)).toBeInTheDocument();
  });

  it('renders the address input and permission select', () => {
    renderDialog();
    expect(screen.getByPlaceholderText('G…')).toBeInTheDocument();
    expect(screen.getByLabelText('Permission level')).toBeInTheDocument();
  });

  it('calls onClose when the close button is clicked', () => {
    const onClose = vi.fn();
    renderDialog(onClose);
    fireEvent.click(screen.getByLabelText('Close share dialog'));
    expect(onClose).toHaveBeenCalledOnce();
  });

  it('calls onClose when clicking the overlay backdrop', () => {
    const onClose = vi.fn();
    renderDialog(onClose);
    fireEvent.click(screen.getByRole('dialog'));
    expect(onClose).toHaveBeenCalledOnce();
  });
});

// ── Adding shares ─────────────────────────────────────────────────────────────

describe('ShareCredentialDialog — adding addresses', () => {
  it('shows an error for an invalid address', async () => {
    renderDialog();
    await fillAndAdd(SHORT_ADDRESS);
    expect(screen.getByRole('alert')).toHaveTextContent(/valid Stellar address/i);
  });

  it('adds a valid address to the shared list', async () => {
    renderDialog();
    await fillAndAdd(VALID_ADDRESS);
    expect(screen.getByText(new RegExp(VALID_ADDRESS.slice(0, 8)))).toBeInTheDocument();
    expect(screen.queryByRole('alert')).not.toBeInTheDocument();
  });

  it('clears the input after a successful add', async () => {
    renderDialog();
    const input = screen.getByPlaceholderText('G…') as HTMLInputElement;
    await fillAndAdd(VALID_ADDRESS);
    expect(input.value).toBe('');
  });

  it('shows an error when adding a duplicate address', async () => {
    renderDialog();
    await fillAndAdd(VALID_ADDRESS);
    await fillAndAdd(VALID_ADDRESS);
    expect(screen.getByRole('alert')).toHaveTextContent(/already added/i);
  });
});

// ── Permission management ─────────────────────────────────────────────────────

describe('ShareCredentialDialog — permission management', () => {
  it('defaults to "view" permission', () => {
    renderDialog();
    const select = screen.getByLabelText('Permission level') as HTMLSelectElement;
    expect(select.value).toBe('view');
  });

  it('allows changing permission before adding', async () => {
    renderDialog();
    await act(async () => {
      fireEvent.change(screen.getByLabelText('Permission level'), { target: { value: 'full' } });
    });
    await fillAndAdd(VALID_ADDRESS);
    const entrySelects = screen.getAllByRole('combobox') as HTMLSelectElement[];
    // The per-entry select is the last one rendered (after the "add" select)
    const entrySelect = entrySelects[entrySelects.length - 1];
    expect(entrySelect.value).toBe('full');
  });

  it('allows changing permission of an existing entry', async () => {
    renderDialog();
    await fillAndAdd(VALID_ADDRESS);
    const entrySelects = screen.getAllByRole('combobox') as HTMLSelectElement[];
    const entrySelect = entrySelects[entrySelects.length - 1];
    await act(async () => {
      fireEvent.change(entrySelect, { target: { value: 'verify' } });
    });
    expect(entrySelect.value).toBe('verify');
  });
});

// ── Removing shares ───────────────────────────────────────────────────────────

describe('ShareCredentialDialog — removing addresses', () => {
  it('removes an address from the list', async () => {
    renderDialog();
    await fillAndAdd(VALID_ADDRESS);
    const truncated = VALID_ADDRESS.slice(0, 8);
    expect(screen.getByText(new RegExp(truncated))).toBeInTheDocument();

    await act(async () => {
      fireEvent.click(screen.getByLabelText(new RegExp(`Remove ${truncated}`, 'i')));
    });
    expect(screen.queryByText(new RegExp(truncated))).not.toBeInTheDocument();
  });
});

// ── Copy link ─────────────────────────────────────────────────────────────────

describe('ShareCredentialDialog — copy link', () => {
  beforeEach(() => {
    Object.assign(navigator, {
      clipboard: { writeText: vi.fn().mockResolvedValue(undefined) },
    });
  });

  it('calls clipboard.writeText with the verification URL', () => {
    renderDialog();
    fireEvent.click(screen.getByLabelText('Copy verification link'));
    expect(navigator.clipboard.writeText).toHaveBeenCalledWith(
      expect.stringContaining(`/verify?id=${CRED_ID}`)
    );
  });
});
