export type StellarNetwork = 'testnet' | 'mainnet' | 'futurenet' | 'standalone';

export const VALID_NETWORKS: StellarNetwork[] = ['testnet', 'mainnet', 'futurenet', 'standalone'];

export interface EnvConfig {
  network: StellarNetwork;
  rpcUrl: string;
}

const rawNetwork = import.meta.env.VITE_STELLAR_NETWORK as string | undefined;

let network: StellarNetwork;
if (!rawNetwork) {
  network = 'testnet';
} else if ((VALID_NETWORKS as string[]).includes(rawNetwork)) {
  network = rawNetwork as StellarNetwork;
} else {
  console.warn(`Unknown VITE_STELLAR_NETWORK value "${rawNetwork}"; falling back to testnet`);
  network = 'testnet';
}

const rpcUrl: string =
  (import.meta.env.VITE_STELLAR_RPC_URL as string | undefined) ??
  'https://soroban-testnet.stellar.org';

export const envConfig: EnvConfig = { network, rpcUrl };
