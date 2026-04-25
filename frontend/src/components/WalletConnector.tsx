import { useState } from 'react';
import { isConnected, requestAccess } from '@stellar/freighter-api';

type WalletState =
  | { status: 'disconnected' }
  | { status: 'connecting' }
  | { status: 'connected'; publicKey: string }
  | { status: 'error'; message: string };

export function WalletConnector() {
  const [walletState, setWalletState] = useState<WalletState>({ status: 'disconnected' });
  const [freighterAbsent, setFreighterAbsent] = useState(false);

  async function handleConnect() {
    setWalletState({ status: 'connecting' });

    try {
      const connected = await isConnected();
      if (!connected) {
        setFreighterAbsent(true);
        setWalletState({ status: 'disconnected' });
        return;
      }
    } catch {
      setFreighterAbsent(true);
      setWalletState({ status: 'disconnected' });
      return;
    }

    try {
      const result = await requestAccess();
      if ('error' in result && result.error) {
        setWalletState({ status: 'error', message: result.error });
      } else if ('publicKey' in result) {
        setWalletState({ status: 'connected', publicKey: result.publicKey });
      } else {
        setWalletState({ status: 'error', message: 'Connection failed' });
      }
    } catch (err: unknown) {
      const message = err instanceof Error ? err.message : 'Connection failed';
      setWalletState({ status: 'error', message });
    }
  }

  function handleDisconnect() {
    setWalletState({ status: 'disconnected' });
  }

  if (freighterAbsent) {
    return <span>Freighter extension is required.</span>;
  }

  if (walletState.status === 'connecting') {
    return (
      <div>
        <span>Connecting…</span>
        <button disabled>Connect Wallet</button>
      </div>
    );
  }

  if (walletState.status === 'connected') {
    return (
      <div>
        <code>{walletState.publicKey}</code>
        <button onClick={handleDisconnect}>Disconnect</button>
      </div>
    );
  }

  if (walletState.status === 'error') {
    return (
      <div>
        <span>{walletState.message}</span>
        <button onClick={handleConnect}>Connect Wallet</button>
      </div>
    );
  }

  // disconnected
  return <button onClick={handleConnect}>Connect Wallet</button>;
}
