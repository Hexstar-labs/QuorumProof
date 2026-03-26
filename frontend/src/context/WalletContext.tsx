import React, { createContext, useContext, useState, useEffect, useCallback, ReactNode } from 'react';
import {
  isConnected,
  isAllowed,
  setAllowed,
  getPublicKey,
} from '@stellar/freighter-api';

interface WalletState {
  address: string | null;
  isConnected: boolean;
  hasFreighter: boolean;
  isInitializing: boolean;
  network: string;
  connect: () => Promise<void>;
  disconnect: () => void;
}

const WalletContext = createContext<WalletState | undefined>(undefined);

const STORAGE_KEY = 'quorum-proof-wallet-address';

interface WalletProviderProps {
  children: ReactNode;
}

export function WalletProvider({ children }: WalletProviderProps) {
  const [address, setAddress] = useState<string | null>(null);
  const [hasFreighter, setHasFreighter] = useState(false);
  const [isInitializing, setIsInitializing] = useState(true);
  const [network] = useState('Testnet'); // For now, hardcoded; could be configurable later

  useEffect(() => {
    const init = async () => {
      try {
        const connected = await isConnected();
        setHasFreighter(connected);
        if (connected) {
          const allowed = await isAllowed();
          if (allowed) {
            const pubKey = await getPublicKey();
            setAddress(pubKey);
            // Persist the address
            localStorage.setItem(STORAGE_KEY, pubKey);
          } else {
            // If not allowed, but we have stored address, clear it
            localStorage.removeItem(STORAGE_KEY);
          }
        } else {
          localStorage.removeItem(STORAGE_KEY);
        }
      } catch (err) {
        console.error('Error checking Freighter connection:', err);
        localStorage.removeItem(STORAGE_KEY);
      } finally {
        setIsInitializing(false);
      }
    };
    init();
  }, []);

  const connect = useCallback(async () => {
    if (!hasFreighter) {
      window.open('https://freighter.app', '_blank');
      return;
    }
    try {
      await setAllowed();
      const pubKey = await getPublicKey();
      setAddress(pubKey);
      localStorage.setItem(STORAGE_KEY, pubKey);
    } catch (err) {
      console.error('User rejected connection or error occurred:', err);
    }
  }, [hasFreighter]);

  const disconnect = useCallback(() => {
    setAddress(null);
    localStorage.removeItem(STORAGE_KEY);
  }, []);

  const value: WalletState = {
    address,
    isConnected: address !== null,
    hasFreighter,
    isInitializing,
    network,
    connect,
    disconnect,
  };

  return (
    <WalletContext.Provider value={value}>
      {children}
    </WalletContext.Provider>
  );
}

export function useWallet(): WalletState {
  const context = useContext(WalletContext);
  if (context === undefined) {
    throw new Error('useWallet must be used within a WalletProvider');
  }
  return context;
}