import { describe, it, expect, vi, beforeEach } from 'vitest';
import { render, screen } from '@testing-library/react';
import { BrowserRouter } from 'react-router-dom';
import QuorumSlice from '../QuorumSlice';

// Mock useFreighter hook
vi.mock('../../lib/hooks/useFreighter', () => ({
  useFreighter: vi.fn(),
}));

// Mock QuorumSliceBuilder component
vi.mock('../../components/QuorumSliceBuilder', () => ({
  QuorumSliceBuilder: ({ creatorAddress }: { creatorAddress: string }) => (
    <div data-testid="quorum-slice-builder" data-creator-address={creatorAddress}>
      QuorumSliceBuilder
    </div>
  ),
}));

// Mock Navbar
vi.mock('../../components/Navbar', () => ({
  Navbar: () => <div>Navbar</div>,
}));

describe('QuorumSlice page (#236)', () => {
  beforeEach(() => {
    vi.clearAllMocks();
  });

  it('passes creatorAddress from wallet to QuorumSliceBuilder', () => {
    const { useFreighter } = require('../../lib/hooks/useFreighter');
    const testAddress = 'GBRPYHIL2CI3WHZDTOOQFC6EB4CGQOFSNQB37HNU7F5V4Z5SHEOSVBQ';

    useFreighter.mockReturnValue({
      address: testAddress,
      isInitializing: false,
      connect: vi.fn(),
      hasFreighter: true,
      disconnect: vi.fn(),
    });

    render(
      <BrowserRouter>
        <QuorumSlice />
      </BrowserRouter>
    );

    const builder = screen.getByTestId('quorum-slice-builder');
    expect(builder).toHaveAttribute('data-creator-address', testAddress);
  });

  it('shows connect wallet prompt when no address is available', () => {
    const { useFreighter } = require('../../lib/hooks/useFreighter');

    useFreighter.mockReturnValue({
      address: null,
      isInitializing: false,
      connect: vi.fn(),
      hasFreighter: true,
      disconnect: vi.fn(),
    });

    render(
      <BrowserRouter>
        <QuorumSlice />
      </BrowserRouter>
    );

    expect(screen.getByText('Connect Your Wallet')).toBeInTheDocument();
    expect(screen.getByText(/You need a connected Freighter wallet/)).toBeInTheDocument();
  });

  it('shows loading state while wallet is initializing', () => {
    const { useFreighter } = require('../../lib/hooks/useFreighter');

    useFreighter.mockReturnValue({
      address: null,
      isInitializing: true,
      connect: vi.fn(),
      hasFreighter: true,
      disconnect: vi.fn(),
    });

    render(
      <BrowserRouter>
        <QuorumSlice />
      </BrowserRouter>
    );

    expect(screen.getByText('Connecting wallet…')).toBeInTheDocument();
  });

  it('does not render QuorumSliceBuilder when address is undefined', () => {
    const { useFreighter } = require('../../lib/hooks/useFreighter');

    useFreighter.mockReturnValue({
      address: undefined,
      isInitializing: false,
      connect: vi.fn(),
      hasFreighter: true,
      disconnect: vi.fn(),
    });

    render(
      <BrowserRouter>
        <QuorumSlice />
      </BrowserRouter>
    );

    expect(screen.queryByTestId('quorum-slice-builder')).not.toBeInTheDocument();
  });
});
