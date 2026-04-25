/**
 * CredentialCompare.test.tsx
 * Tests for the credential comparison tool — issue #322
 */

import { describe, it, expect, vi, beforeEach } from 'vitest';
import { render, screen, fireEvent, act } from '@testing-library/react';
import '@testing-library/jest-dom';
import { MemoryRouter } from 'react-router-dom';

// ── Mocks ─────────────────────────────────────────────────────────────────────

vi.mock('../../lib/contracts/quorumProof', () => ({
  getCredential: vi.fn(),
  getAttestors: vi.fn(),
  isExpired: vi.fn(),
}));

vi.mock('../../components/Navbar', () => ({
  Navbar: () => <nav data-testid="navbar" />,
}));

// ── Helpers ───────────────────────────────────────────────────────────────────

function makeCredential(id: bigint, overrides: Record<string, unknown> = {}) {
  return {
    id,
    subject: 'GAAZI4TCR3TY5OJHCTJC2A4QSY6CJWJH5IAJTGKIN2ER7LBNVKOCCWNN',
    issuer:  'GBAZI4TCR3TY5OJHCTJC2A4QSY6CJWJH5IAJTGKIN2ER7LBNVKOCCWNN',
    credential_type: 1,
    metadata_hash: new Uint8Array(),
    revoked: false,
    expires_at: null,
    ...overrides,
  };
}

async function renderCompare() {
  const { default: CredentialCompare } = await import('../../pages/CredentialCompare');
  let result: ReturnType<typeof render>;
  await act(async () => {
    result = render(
      <MemoryRouter>
        <CredentialCompare />
      </MemoryRouter>
    );
  });
  return result!;
}

async function fillAndCompare(idA: string, idB: string) {
  await act(async () => {
    fireEvent.change(screen.getByLabelText('Credential A ID'), { target: { value: idA } });
    fireEvent.change(screen.getByLabelText('Credential B ID'), { target: { value: idB } });
  });
  await act(async () => {
    fireEvent.click(screen.getByLabelText('Compare credentials'));
  });
}

// ── Tests ─────────────────────────────────────────────────────────────────────

describe('CredentialCompare — rendering', () => {
  it('renders the page title and inputs', async () => {
    await renderCompare();
    expect(screen.getByText('Compare Credentials')).toBeInTheDocument();
    expect(screen.getByLabelText('Credential A ID')).toBeInTheDocument();
    expect(screen.getByLabelText('Credential B ID')).toBeInTheDocument();
    expect(screen.getByLabelText('Compare credentials')).toBeInTheDocument();
  });

  it('does not show the comparison table before comparing', async () => {
    await renderCompare();
    expect(screen.queryByRole('table')).not.toBeInTheDocument();
  });
});

describe('CredentialCompare — validation', () => {
  it('shows error when both IDs are empty', async () => {
    await renderCompare();
    await act(async () => {
      fireEvent.click(screen.getByLabelText('Compare credentials'));
    });
    expect(screen.getByRole('alert')).toHaveTextContent(/enter both credential ids/i);
  });

  it('shows error when IDs are the same', async () => {
    await renderCompare();
    await fillAndCompare('5', '5');
    expect(screen.getByRole('alert')).toHaveTextContent(/two different/i);
  });
});

describe('CredentialCompare — identical credentials', () => {
  beforeEach(async () => {
    const contracts = await import('../../lib/contracts/quorumProof');
    vi.mocked(contracts.getCredential).mockResolvedValue(makeCredential(1n));
    vi.mocked(contracts.getAttestors).mockResolvedValue([]);
    vi.mocked(contracts.isExpired).mockResolvedValue(false);
  });

  it('renders the comparison table', async () => {
    await renderCompare();
    await fillAndCompare('1', '2');
    expect(screen.getByRole('table', { name: /credential comparison/i })).toBeInTheDocument();
  });

  it('shows no diff badges when credentials are identical', async () => {
    await renderCompare();
    await fillAndCompare('1', '2');
    expect(screen.queryByText('≠')).not.toBeInTheDocument();
  });

  it('renders all expected field rows', async () => {
    await renderCompare();
    await fillAndCompare('1', '2');
    for (const label of ['Type', 'Subject', 'Issuer', 'Expires', 'Revoked', 'Attestors']) {
      expect(screen.getByText(label)).toBeInTheDocument();
    }
  });
});

describe('CredentialCompare — different credentials', () => {
  beforeEach(async () => {
    const contracts = await import('../../lib/contracts/quorumProof');
    vi.mocked(contracts.getCredential)
      .mockResolvedValueOnce(makeCredential(1n, { credential_type: 1 }))  // Degree
      .mockResolvedValueOnce(makeCredential(2n, { credential_type: 2 })); // License
    vi.mocked(contracts.getAttestors).mockResolvedValue([]);
    vi.mocked(contracts.isExpired).mockResolvedValue(false);
  });

  it('shows a diff badge for the differing field', async () => {
    await renderCompare();
    await fillAndCompare('1', '2');
    expect(screen.getByText('≠')).toBeInTheDocument();
  });

  it('highlights the differing row', async () => {
    await renderCompare();
    await fillAndCompare('1', '2');
    const typeRow = screen.getByText('Type').closest('.compare-row');
    expect(typeRow).toHaveClass('compare-row--diff');
  });

  it('shows correct values for each credential', async () => {
    await renderCompare();
    await fillAndCompare('1', '2');
    expect(screen.getByTestId('field-a-type')).toHaveTextContent('🎓 Degree');
    expect(screen.getByTestId('field-b-type')).toHaveTextContent('🏛️ License');
  });
});

describe('CredentialCompare — revoked credential', () => {
  beforeEach(async () => {
    const contracts = await import('../../lib/contracts/quorumProof');
    vi.mocked(contracts.getCredential)
      .mockResolvedValueOnce(makeCredential(1n, { revoked: false }))
      .mockResolvedValueOnce(makeCredential(2n, { revoked: true }));
    vi.mocked(contracts.getAttestors).mockResolvedValue([]);
    vi.mocked(contracts.isExpired).mockResolvedValue(false);
  });

  it('shows Yes/No for revoked field and highlights the diff', async () => {
    await renderCompare();
    await fillAndCompare('1', '2');
    expect(screen.getByTestId('field-a-revoked')).toHaveTextContent('No');
    expect(screen.getByTestId('field-b-revoked')).toHaveTextContent('Yes');
    const revokedRow = screen.getByText('Revoked').closest('.compare-row');
    expect(revokedRow).toHaveClass('compare-row--diff');
  });
});

describe('CredentialCompare — fetch error', () => {
  beforeEach(async () => {
    const contracts = await import('../../lib/contracts/quorumProof');
    vi.mocked(contracts.getCredential).mockRejectedValue(new Error('RPC timeout'));
  });

  it('shows an error message on fetch failure', async () => {
    await renderCompare();
    await fillAndCompare('1', '2');
    expect(screen.getByRole('alert')).toHaveTextContent(/RPC timeout/i);
  });
});
