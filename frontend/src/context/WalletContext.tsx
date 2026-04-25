import React, { createContext, useContext, useState, useEffect, useCallback, ReactNode } from 'react';
import {
  isConnected,
  isAllowed,
  setAllowed,
  getPublicKey,
} from '@stellar/freighter-api';
import { STELLAR_NETWORK } from '../config/env';

interface WalletState {
  address: string | null;
  isConnected: boolean;
  hasFreighter: boolean;
  isInitializing: boolean;
  network: string;
  error: string | null;
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
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    const init = async () => {
      try {
        setError(null);
        const connected = await isConnected();
        setHasFreighter(connected);
        if (connected) {
          const allowed = await isAllowed();
          if (allowed) {
            const pubKey = await getPublicKey();
            setAddress(pubKey);
            localStorage.setItem(STORAGE_KEY, pubKey);
          } else {
            localStorage.removeItem(STORAGE_KEY);
          }
        } else {
          localStorage.removeItem(STORAGE_KEY);
        }
      } catch (err) {
        const errorMsg = err instanceof Error ? err.message : 'Failed to initialize wallet';
        setError(errorMsg);
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
      setError(null);
      await setAllowed();
      const pubKey = await getPublicKey();
      setAddress(pubKey);
      localStorage.setItem(STORAGE_KEY, pubKey);
    } catch (err) {
      const errorMsg = err instanceof Error ? err.message : 'Failed to connect wallet';
      setError(errorMsg);
      console.error('User rejected connection or error occurred:', err);
    }
  }, [hasFreighter]);

  const disconnect = useCallback(() => {
    setAddress(null);
    setError(null);
    localStorage.removeItem(STORAGE_KEY);
  }, []);

  const value: WalletState = {
    address,
    isConnected: address !== null,
    hasFreighter,
    isInitializing,
    network: STELLAR_NETWORK,
    error,
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