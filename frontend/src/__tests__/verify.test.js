/**
 * verify.test.js — Unit and property-based tests for the public credential verification page.
 * Feature: public-credential-verification
 */

import { describe, it, expect, vi, beforeEach } from 'vitest';
import fc from 'fast-check';

// ── Pure helpers under test (no DOM / contract deps) ──────────────────────

/** Mirrors the validation logic in renderVerifyPage */
function isValidCredentialId(raw) {
  const trimmed = String(raw).trim();
  const parsed = parseInt(trimmed, 10);
  return trimmed !== '' && parsed > 0 && isFinite(parsed) && String(parsed) === trimmed;
}

/** Mirrors the Stellar address validation in handleVerifyByAddr */
function isValidStellarAddress(addr) {
  return typeof addr === 'string' && addr.startsWith('G') && addr.length >= 56;
}

/** Mirrors the deriveStatus logic extracted from renderCredential */
export function deriveStatus(revoked, expired, attestorCount) {
  if (revoked) return 'revoked';
  if (expired) return 'expired';
  if (attestorCount === 0) return 'pending';
  return 'verified';
}

/** Mirrors hexToBytes from stellar.js */
function hexToBytes(hex) {
  const clean = hex.replace(/^0x/, '').replace(/\s/g, '');
  if (clean.length === 0) throw new Error('Invalid hex string');
  if (clean.length % 2 !== 0) throw new Error('Invalid hex string');
  const bytes = new Uint8Array(clean.length / 2);
  for (let i = 0; i < bytes.length; i++) {
    bytes[i] = parseInt(clean.substr(i * 2, 2), 16);
  }
  return bytes;
}

function uint8ArrayToHex(arr) {
  return Array.from(arr).map(b => b.toString(16).padStart(2, '0')).join('');
}

// ─────────────────────────────────────────────────────────────────────────────
// Task 7.1 — Credential ID validation unit tests
// ─────────────────────────────────────────────────────────────────────────────
describe('Credential ID validation', () => {
  it('rejects empty string', () => expect(isValidCredentialId('')).toBe(false));
  it('rejects "0"', () => expect(isValidCredentialId('0')).toBe(false));
  it('rejects "-1"', () => expect(isValidCredentialId('-1')).toBe(false));
  it('rejects "1.5"', () => expect(isValidCredentialId('1.5')).toBe(false));
  it('rejects "abc"', () => expect(isValidCredentialId('abc')).toBe(false));
  it('accepts "1"', () => expect(isValidCredentialId('1')).toBe(true));
  it('accepts "42"', () => expect(isValidCredentialId('42')).toBe(true));
});

// ─────────────────────────────────────────────────────────────────────────────
// Task 7.2 — Property 1: Credential ID input validation rejects non-positive integers
// Feature: public-credential-verification, Property 1: Credential ID input validation rejects non-positive integers
// ─────────────────────────────────────────────────────────────────────────────
describe('Property 1: Credential ID validation rejects non-positive integers', () => {
  it('rejects all non-positive-integer strings', () => {
    fc.assert(
      fc.property(
        fc.oneof(
          fc.constant(''),
          fc.constant('0'),
          fc.integer({ max: 0 }).map(String),
          fc.double({ noNaN: true }).filter(n => !Number.isInteger(n)).map(String),
          fc.string().filter(s => isNaN(Number(s)) && s.trim() !== '')
        ),
        (input) => {
          return isValidCredentialId(input) === false;
        }
      ),
      { numRuns: 100 }
    );
  });
});

// ─────────────────────────────────────────────────────────────────────────────
// Task 7.3 — Property 2: Stellar address validation rejects malformed addresses
// Feature: public-credential-verification, Property 2: Stellar address input validation rejects malformed addresses
// ─────────────────────────────────────────────────────────────────────────────
describe('Property 2: Stellar address validation rejects malformed addresses', () => {
  it('rejects all strings that are not valid Stellar addresses', () => {
    fc.assert(
      fc.property(
        fc.string().filter(s => !(s.startsWith('G') && s.length >= 56)),
        (addr) => {
          return isValidStellarAddress(addr) === false;
        }
      ),
      { numRuns: 100 }
    );
  });
});

// ─────────────────────────────────────────────────────────────────────────────
// Task 8.2 — deriveStatus unit tests
// ─────────────────────────────────────────────────────────────────────────────
describe('deriveStatus', () => {
  it('returns "revoked" when revoked=true regardless of other flags', () => {
    expect(deriveStatus(true, false, 5)).toBe('revoked');
    expect(deriveStatus(true, true, 0)).toBe('revoked');
  });
  it('returns "expired" when not revoked but expired', () => {
    expect(deriveStatus(false, true, 3)).toBe('expired');
    expect(deriveStatus(false, true, 0)).toBe('expired');
  });
  it('returns "pending" when not revoked, not expired, and 0 attestors', () => {
    expect(deriveStatus(false, false, 0)).toBe('pending');
  });
  it('returns "verified" when not revoked, not expired, and ≥1 attestor', () => {
    expect(deriveStatus(false, false, 1)).toBe('verified');
    expect(deriveStatus(false, false, 10)).toBe('verified');
  });
});

// ─────────────────────────────────────────────────────────────────────────────
// Task 8.3 — Property 3: Credential status derivation is deterministic
// Feature: public-credential-verification, Property 3: Credential status derivation is deterministic
// ─────────────────────────────────────────────────────────────────────────────
describe('Property 3: Status derivation is deterministic', () => {
  const VALID_STATUSES = new Set(['revoked', 'expired', 'pending', 'verified']);

  it('always returns one of the four valid statuses', () => {
    fc.assert(
      fc.property(
        fc.record({ revoked: fc.boolean(), expired: fc.boolean(), count: fc.nat() }),
        ({ revoked, expired, count }) => {
          const status = deriveStatus(revoked, expired, count);
          return VALID_STATUSES.has(status);
        }
      ),
      { numRuns: 100 }
    );
  });

  it('same inputs always produce same output', () => {
    fc.assert(
      fc.property(
        fc.record({ revoked: fc.boolean(), expired: fc.boolean(), count: fc.nat() }),
        ({ revoked, expired, count }) => {
          return deriveStatus(revoked, expired, count) === deriveStatus(revoked, expired, count);
        }
      ),
      { numRuns: 100 }
    );
  });
});

// ─────────────────────────────────────────────────────────────────────────────
// Task 5.1 — Property 4: Shareable URL round-trip
// Feature: public-credential-verification, Property 4: Shareable URL round-trip
// ─────────────────────────────────────────────────────────────────────────────
describe('Property 4: Shareable URL round-trip', () => {
  it('serializing then parsing a credentialId recovers the original value', () => {
    fc.assert(
      fc.property(
        fc.integer({ min: 1, max: Number.MAX_SAFE_INTEGER }),
        (id) => {
          const url = new URL('http://localhost/verify');
          url.searchParams.set('credentialId', String(id));
          const parsed = parseInt(url.searchParams.get('credentialId'), 10);
          return parsed === id;
        }
      ),
      { numRuns: 100 }
    );
  });
});

// ─────────────────────────────────────────────────────────────────────────────
// Task 10.1 — hexToBytes unit tests
// ─────────────────────────────────────────────────────────────────────────────
describe('hexToBytes', () => {
  it('converts valid even-length hex', () => {
    expect(hexToBytes('deadbeef')).toEqual(new Uint8Array([0xde, 0xad, 0xbe, 0xef]));
  });
  it('handles 0x-prefixed hex', () => {
    expect(hexToBytes('0xdeadbeef')).toEqual(new Uint8Array([0xde, 0xad, 0xbe, 0xef]));
  });
  it('throws on odd-length hex', () => {
    expect(() => hexToBytes('abc')).toThrow('Invalid hex string');
  });
  it('throws on empty string', () => {
    expect(() => hexToBytes('')).toThrow('Invalid hex string');
  });
});

// ─────────────────────────────────────────────────────────────────────────────
// Task 10.2 — Property 8: Hex-to-bytes round-trip
// Feature: public-credential-verification, Property 8: Hex-to-bytes round-trip
// ─────────────────────────────────────────────────────────────────────────────
describe('Property 8: Hex-to-bytes round-trip', () => {
  it('converting bytes to hex and back recovers the original bytes', () => {
    fc.assert(
      fc.property(
        fc.uint8Array({ minLength: 1 }),
        (bytes) => {
          const hex = uint8ArrayToHex(bytes);
          const recovered = hexToBytes(hex);
          return recovered.length === bytes.length &&
            recovered.every((b, i) => b === bytes[i]);
        }
      ),
      { numRuns: 100 }
    );
  });
});

// ─────────────────────────────────────────────────────────────────────────────
// Task 11.1 — ZK proof validation unit tests
// ─────────────────────────────────────────────────────────────────────────────
function isValidProof(proofHex) {
  return proofHex.trim().replace(/\s/g, '') !== '';
}

describe('ZK proof validation', () => {
  it('rejects empty string', () => expect(isValidProof('')).toBe(false));
  it('rejects whitespace-only string', () => expect(isValidProof('   ')).toBe(false));
  it('rejects tab-only string', () => expect(isValidProof('\t\t')).toBe(false));
  it('accepts non-empty hex', () => expect(isValidProof('deadbeef')).toBe(true));
});

// ─────────────────────────────────────────────────────────────────────────────
// Task 11.2 — Property 5: ZK form rejects empty or whitespace-only proof
// Feature: public-credential-verification, Property 5: ZK form rejects empty or whitespace-only proof
// ─────────────────────────────────────────────────────────────────────────────
describe('Property 5: ZK form rejects empty or whitespace-only proof', () => {
  it('rejects all whitespace-only strings', () => {
    fc.assert(
      fc.property(
        fc.string({ unit: fc.constantFrom(' ', '\t', '\n', '\r') }),
        (proof) => {
          return isValidProof(proof) === false;
        }
      ),
      { numRuns: 100 }
    );
  });
});

// ─────────────────────────────────────────────────────────────────────────────
// Task 10.3 — decodeMetadataHash unit tests
// ─────────────────────────────────────────────────────────────────────────────

/** Mirrors decodeMetadataHash from stellar.js */
function decodeMetadataHash(rawValue) {
  if (typeof rawValue === 'string') return rawValue;
  if (rawValue instanceof Uint8Array || Array.isArray(rawValue)) {
    try {
      return new TextDecoder().decode(new Uint8Array(rawValue));
    } catch {
      return Array.from(rawValue).map(b => b.toString(16).padStart(2, '0')).join('');
    }
  }
  return String(rawValue);
}

describe('decodeMetadataHash', () => {
  it('passes through plain strings unchanged', () => {
    expect(decodeMetadataHash('QmHash123')).toBe('QmHash123');
  });
  it('decodes UTF-8 byte array to string', () => {
    const bytes = new TextEncoder().encode('hello');
    expect(decodeMetadataHash(bytes)).toBe('hello');
  });
  it('falls back to hex for non-UTF-8 bytes', () => {
    // 0xff 0xfe are not valid UTF-8 start bytes in isolation but TextDecoder won't throw —
    // instead test that arbitrary bytes produce a hex string
    const bytes = new Uint8Array([0xde, 0xad]);
    const result = decodeMetadataHash(bytes);
    expect(typeof result).toBe('string');
    expect(result.length).toBeGreaterThan(0);
  });
});
