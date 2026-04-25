import { envConfig, StellarNetwork } from '../envConfig';

const NETWORK_LABELS: Record<StellarNetwork, string> = {
  testnet: 'Testnet',
  mainnet: 'Mainnet',
  futurenet: 'Futurenet',
  standalone: 'Standalone',
};

export function NetworkIndicator() {
  const { network } = envConfig;
  return (
    <span className={`network-badge network-badge--${network}`}>
      {NETWORK_LABELS[network]}
    </span>
  );
}
