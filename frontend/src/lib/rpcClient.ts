/**
 * Soroban RPC Client
 * Singleton instance for consistent RPC communication
 */

import { rpc as StellarRpc } from '@stellar/stellar-sdk';
import { STELLAR_RPC_URL } from '../config/env';

/**
 * Singleton SorobanRpc.Server instance
 * Initialized once and reused throughout the application
 */
export const rpcClient = new StellarRpc.Server(STELLAR_RPC_URL, {
  allowHttp: false
});