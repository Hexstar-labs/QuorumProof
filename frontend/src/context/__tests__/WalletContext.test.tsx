import { render, screen, waitFor } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { WalletProvider, useWallet } from '../WalletContext';
import * as FreighterApi from '@stellar/freighter-api';

jest.mock('@stellar/freighter-api');

const TestComponent = () => {
  const { address, isConnected, error, disconnect } = useWallet();
  return (
    <div>
      <div data-testid="address">{address || 'Not connected'}</div>
      <div data-testid="is-connected">{isConnected ? 'Connected' : 'Disconnected'}</div>
      <div data-testid="error">{error || 'No error'}</div>
      <button onClick={disconnect}>Disconnect</button>
    </div>
  );
};

describe('WalletContext', () => {
  beforeEach(() => {
    jest.clearAllMocks();
    localStorage.clear();
  });

  it('should disconnect and clear error state', async () => {
    (FreighterApi.isConnected as jest.Mock).mockResolvedValue(true);
    (FreighterApi.isAllowed as jest.Mock).mockResolvedValue(true);
    (FreighterApi.getPublicKey as jest.Mock).mockResolvedValue('GTEST123');

    render(
      <WalletProvider>
        <TestComponent />
      </WalletProvider>
    );

    await waitFor(() => {
      expect(screen.getByTestId('address')).toHaveTextContent('GTEST123');
    });

    const disconnectBtn = screen.getByText('Disconnect');
    await userEvent.click(disconnectBtn);

    await waitFor(() => {
      expect(screen.getByTestId('is-connected')).toHaveTextContent('Disconnected');
      expect(screen.getByTestId('error')).toHaveTextContent('No error');
    });
  });

  it('should surface connection errors', async () => {
    (FreighterApi.isConnected as jest.Mock).mockRejectedValue(new Error('Connection failed'));

    render(
      <WalletProvider>
        <TestComponent />
      </WalletProvider>
    );

    await waitFor(() => {
      expect(screen.getByTestId('error')).toHaveTextContent('Connection failed');
    });
  });
});
