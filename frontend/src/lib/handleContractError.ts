/**
 * Translate known contract error messages into user-facing friendly text.
 */
const CONTRACT_ERROR_MAP: { [key: string]: string } = {
  'already attested': 'This credential has already been attested by your quorum slice.',
  'credential revoked': 'This credential has been revoked and cannot be used.',
  'not found': 'Requested credential was not found on chain.',
  'unauthorized': 'Action is not authorized. Please check your permissions.',
  'invalid request': 'Contract call was invalid. Please try again with correct data.',
};

export function handleContractError(error: unknown): string {
  const message = error instanceof Error ? error.message : String(error);

  if (!message) {
    return 'An unknown contract error occurred.';
  }

  const normalized = message.toLowerCase();

  for (const [key, userMessage] of Object.entries(CONTRACT_ERROR_MAP)) {
    if (normalized.includes(key)) {
      return userMessage;
    }
  }

  return `Contract error: ${message}`;
}
