import type { Credential, QuorumSlice } from './contracts/quorumProof';

export type AttestationStatus = 'attested' | 'pending' | 'revoked' | 'expired';

export interface CredCardData {
  credential: Credential;
  attested: boolean;
  slice: QuorumSlice | null;
  expired: boolean;
  sliceError: boolean;
  credError: string | null;
}

export const CREDENTIAL_TYPES: Record<number, string> = {
  1: '🎓 Degree',
  2: '🏛️ License',
  3: '💼 Employment',
  4: '📜 Certification',
  5: '🔬 Research',
};

export const ATTESTOR_ROLES = [
  'Lead Verifier',
  'Co-Verifier',
  'Auditor',
  'Reviewer',
  'Observer',
];

/** Derive attestation status with priority: revoked > expired > attested > pending */
export function deriveStatus(
  revoked: boolean,
  expired: boolean,
  attested: boolean
): AttestationStatus {
  if (revoked) return 'revoked';
  if (expired) return 'expired';
  if (attested) return 'attested';
  return 'pending';
}

/** Truncate a Stellar address to first 8 + last 6 chars */
export function formatAddress(addr: string): string {
  if (!addr || addr.length < 10) return addr || '—';
  return addr.slice(0, 8) + '…' + addr.slice(-6);
}

/** Get role label for an attestor by index */
export function attestorRole(index: number): string {
  return ATTESTOR_ROLES[index] ?? `Member ${index + 1}`;
}

/** Get human-readable credential type label */
export function credTypeLabel(n: number | bigint): string {
  return CREDENTIAL_TYPES[Number(n)] || `Type ${n}`;
}

/** Format a Unix timestamp (seconds) to a readable date string */
export function formatTimestamp(
  ts: number | bigint | null | undefined
): string {
  if (!ts) return 'Never';
  return new Date(Number(ts) * 1000).toLocaleDateString(undefined, {
    year: 'numeric',
    month: 'short',
    day: 'numeric',
  });
}
