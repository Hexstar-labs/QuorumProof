import { describe, it, expect, vi, beforeEach } from 'vitest';
import { render, screen, waitFor } from '@testing-library/react';
import { BrowserRouter } from 'react-router-dom';
import Dashboard from '../Dashboard';

// Mock useWallet hook
vi.mock('../../hooks', () => ({
  useWallet: vi.fn(),
}));

// Mock stellar functions
vi.mock('../../stellar', () => ({
  getCredentialsBySubject: vi.fn(),
  getCredential: vi.fn(),
  isAttested: vi.fn(),
  getAttestors: vi.fn(),
  getSlice: vi.fn(),
  isExpired: vi.fn(),
  decodeMetadataHash: vi.fn(() => 'test-hash'),
}));

// Mock components
vi.mock('../../components/Navbar', () => ({
  Navbar: () => <div>Navbar</div>,
}));

vi.mock('../../components/WalletGate', () => ({
  WalletGate: () => <div>WalletGate</div>,
}));

vi.mock('../../components/CredentialCard', () => ({
  CredentialCard: () => <div>CredentialCard</div>,
}));

vi.mock('../../components/CredentialCardSkeleton', () => ({
  CredentialCardSkeleton: () => <div data-testid="credential-skeleton">Skeleton</div>,
}));

vi.mock('../../components/EmptyState', () => ({
  EmptyState: () => <div>EmptyState</div>,
}));

describe('Dashboard (#239)', () => {
  beforeEach(() => {
    vi.clearAllMocks();
    localStorage.clear();
  });

  it('renders skeleton cards while loading credentials', async () => {
    const { useWallet } = require('../../hooks');
    const { getCredentialsBySubject } = require('../../stellar');

    const testAddress = 'GBRPYHIL2CI3WHZDTOOQFC6EB4CGQOFSNQB37HNU7F5V4Z5SHEOSVBQ';

    useWallet.mockReturnValue({
      address: testAddress,
      hasFreighter: true,
      isInitializing: false,
      connect: vi.fn(),
      disconnect: vi.fn(),
    });

    // Simulate loading delay
    getCredentialsBySubject.mockImplementation(
      () => new Promise((resolve) => setTimeout(() => resolve([]), 100))
    );

    render(
      <BrowserRouter>
        <Dashboard />
      </BrowserRouter>
    );

    // Should show skeletons while loading
    const skeletons = screen.getAllByTestId('credential-skeleton');
    expect(skeletons).toHaveLength(3);

    // Wait for loading to complete
    await waitFor(() => {
      expect(getCredentialsBySubject).toHaveBeenCalledWith(testAddress);
    });
  });

  it('clears skeletons once data resolves', async () => {
    const { useWallet } = require('../../hooks');
    const { getCredentialsBySubject } = require('../../stellar');

    const testAddress = 'GBRPYHIL2CI3WHZDTOOQFC6EB4CGQOFSNQB37HNU7F5V4Z5SHEOSVBQ';

    useWallet.mockReturnValue({
      address: testAddress,
      hasFreighter: true,
      isInitializing: false,
      connect: vi.fn(),
      disconnect: vi.fn(),
    });

    getCredentialsBySubject.mockResolvedValue([]);

    render(
      <BrowserRouter>
        <Dashboard />
      </BrowserRouter>
    );

    // Wait for loading to complete
    await waitFor(() => {
      const skeletons = screen.queryAllByTestId('credential-skeleton');
      expect(skeletons).toHaveLength(0);
    });
  });

  it('shows empty state when no credentials exist', async () => {
    const { useWallet } = require('../../hooks');
    const { getCredentialsBySubject } = require('../../stellar');

    const testAddress = 'GBRPYHIL2CI3WHZDTOOQFC6EB4CGQOFSNQB37HNU7F5V4Z5SHEOSVBQ';

    useWallet.mockReturnValue({
      address: testAddress,
      hasFreighter: true,
      isInitializing: false,
      connect: vi.fn(),
      disconnect: vi.fn(),
    });

    getCredentialsBySubject.mockResolvedValue([]);

    render(
      <BrowserRouter>
        <Dashboard />
      </BrowserRouter>
    );

    await waitFor(() => {
      expect(screen.getByText('EmptyState')).toBeInTheDocument();
    });
  });

  it('does not show skeletons when wallet is not connected', () => {
    const { useWallet } = require('../../hooks');

    useWallet.mockReturnValue({
      address: null,
      hasFreighter: true,
      isInitializing: false,
      connect: vi.fn(),
      disconnect: vi.fn(),
    });

    render(
      <BrowserRouter>
        <Dashboard />
      </BrowserRouter>
    );

    const skeletons = screen.queryAllByTestId('credential-skeleton');
    expect(skeletons).toHaveLength(0);
  });
});
