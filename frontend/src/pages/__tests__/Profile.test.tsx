/**
 * Profile.test.tsx
 * Tests for the credential holder profile page — issue #320
 */

import { describe, it, expect, vi, beforeEach } from 'vitest';
import { render, screen, act } from '@testing-library/react';
import '@testing-library/jest-dom';
import { MemoryRouter } from 'react-router-dom';

// ── Mocks ─────────────────────────────────────────────────────────────────────

vi.mock('../../hooks', () => ({
  useWallet: vi.fn(),
}));

vi.mock('../../stellar', () => ({
  getCredentialsBySubject: vi.fn(),
  getCredential: vi.fn(),
  isAttested: vi.fn(),
  getSlice: vi.fn(),
  isExpired: vi.fn(),
}));

vi.mock('../../components/Navbar', () => ({
  Navbar: () => <nav data-testid="navbar" />,
}));

vi.mock('../../components/CredentialCard', () => ({
  CredentialCard: ({ data }: { data: { credential: { id: bigint } } }) => (
    <div data-testid="credential-card">{data.credential.id.toString()}</div>
  ),
}));

vi.mock('../../components/CredentialCardSkeleton', () => ({
  CredentialCardSkeleton: () => <div data-testid="skeleton" />,
}));

vi.mock('../../components/EmptyState', () => ({
  EmptyState: () => <div data-testid="empty-state" />,
}));

// ── Helpers ───────────────────────────────────────────────────────────────────

const WALLET_ADDRESS = 'GAAZI4TCR3TY5OJHCTJC2A4QSY6CJWJH5IAJTGKIN2ER7LBNVKOCCWNN';

function makeCredential(id: bigint) {
  return {
    id,
    subject: WALLET_ADDRESS,
    issuer: WALLET_ADDRESS,
    credential_type: 1,
    metadata_hash: new Uint8Array(),
    revoked: false,
    expires_at: null,
  };
}

async function renderProfile() {
  const { default: Profile } = await import('../../pages/Profile');
  let result: ReturnType<typeof render>;
  await act(async () => {
    result = render(
      <MemoryRouter>
        <Profile />
      </MemoryRouter>
    );
  });
  return result!;
}

// ── Tests ─────────────────────────────────────────────────────────────────────

describe('Profile page — no wallet', () => {
  beforeEach(async () => {
    const { useWallet } = await import('../../hooks');
    vi.mocked(useWallet).mockReturnValue({
      address: null,
      isConnected: false,
      hasFreighter: false,
      isInitializing: false,
      network: 'testnet',
      error: null,
      connect: vi.fn(),
      disconnect: vi.fn(),
    });
  });

  it('shows wallet-not-connected message', async () => {
    await renderProfile();
    expect(screen.getByText(/wallet not connected/i)).toBeInTheDocument();
  });

  it('does not show credential cards', async () => {
    await renderProfile();
    expect(screen.queryByTestId('credential-card')).not.toBeInTheDocument();
  });
});

describe('Profile page — wallet connected, loading', () => {
  beforeEach(async () => {
    const { useWallet } = await import('../../hooks');
    vi.mocked(useWallet).mockReturnValue({
      address: WALLET_ADDRESS,
      isConnected: true,
      hasFreighter: true,
      isInitializing: false,
      network: 'testnet',
      error: null,
      connect: vi.fn(),
      disconnect: vi.fn(),
    });

    const { getCredentialsBySubject } = await import('../../stellar');
    // Never resolves during this test
    vi.mocked(getCredentialsBySubject).mockReturnValue(new Promise(() => {}));
  });

  it('shows skeleton cards while loading', async () => {
    await renderProfile();
    expect(screen.getAllByTestId('skeleton').length).toBeGreaterThan(0);
  });
});

describe('Profile page — wallet connected, credentials loaded', () => {
  beforeEach(async () => {
    const { useWallet } = await import('../../hooks');
    vi.mocked(useWallet).mockReturnValue({
      address: WALLET_ADDRESS,
      isConnected: true,
      hasFreighter: true,
      isInitializing: false,
      network: 'testnet',
      error: null,
      connect: vi.fn(),
      disconnect: vi.fn(),
    });

    const stellar = await import('../../stellar');
    vi.mocked(stellar.getCredentialsBySubject).mockResolvedValue([1n, 2n]);
    vi.mocked(stellar.getCredential).mockImplementation((id) =>
      Promise.resolve(makeCredential(id))
    );
    vi.mocked(stellar.isExpired).mockResolvedValue(false);
    vi.mocked(stellar.isAttested).mockResolvedValue(false);
    vi.mocked(stellar.getSlice).mockRejectedValue(new Error('no slice'));

    localStorage.removeItem('qp-slice-id');
  });

  it('renders the profile header with address', async () => {
    await renderProfile();
    expect(screen.getByText('Credential Holder')).toBeInTheDocument();
    expect(screen.getByTitle(WALLET_ADDRESS)).toBeInTheDocument();
  });

  it('renders a credential card for each credential', async () => {
    await renderProfile();
    expect(screen.getAllByTestId('credential-card')).toHaveLength(2);
  });

  it('shows stats with correct total count', async () => {
    await renderProfile();
    const stats = screen.getByRole('list', { name: /credential statistics/i });
    expect(stats).toBeInTheDocument();
    // Total stat is the first listitem value
    const items = screen.getAllByRole('listitem');
    expect(items[0]).toHaveTextContent('2'); // total = 2
  });

  it('does not show skeleton or empty state', async () => {
    await renderProfile();
    expect(screen.queryByTestId('skeleton')).not.toBeInTheDocument();
    expect(screen.queryByTestId('empty-state')).not.toBeInTheDocument();
  });
});

describe('Profile page — wallet connected, no credentials', () => {
  beforeEach(async () => {
    const { useWallet } = await import('../../hooks');
    vi.mocked(useWallet).mockReturnValue({
      address: WALLET_ADDRESS,
      isConnected: true,
      hasFreighter: true,
      isInitializing: false,
      network: 'testnet',
      error: null,
      connect: vi.fn(),
      disconnect: vi.fn(),
    });

    const { getCredentialsBySubject } = await import('../../stellar');
    vi.mocked(getCredentialsBySubject).mockResolvedValue([]);
  });

  it('shows empty state', async () => {
    await renderProfile();
    expect(screen.getByTestId('empty-state')).toBeInTheDocument();
  });
});

describe('Profile page — fetch error', () => {
  beforeEach(async () => {
    const { useWallet } = await import('../../hooks');
    vi.mocked(useWallet).mockReturnValue({
      address: WALLET_ADDRESS,
      isConnected: true,
      hasFreighter: true,
      isInitializing: false,
      network: 'testnet',
      error: null,
      connect: vi.fn(),
      disconnect: vi.fn(),
    });

    const { getCredentialsBySubject } = await import('../../stellar');
    vi.mocked(getCredentialsBySubject).mockRejectedValue(new Error('RPC error'));
  });

  it('shows error message', async () => {
    await renderProfile();
    expect(screen.getByText(/could not load credentials/i)).toBeInTheDocument();
    expect(screen.getByText(/RPC error/i)).toBeInTheDocument();
  });
});
