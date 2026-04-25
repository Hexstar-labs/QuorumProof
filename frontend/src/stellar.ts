/**
 * stellar.js — Soroban RPC read-only wrapper for QuorumProof.
 *
 * All functions simulate contract calls without a wallet (no auth needed
 * for read-only methods).  Results are parsed from XDR ScVal.
 */

import {
  Contract,
  Networks,
  rpc as StellarRpc,
  scValToNative,
  xdr,
  nativeToScVal,
  Address,
} from '@stellar/stellar-sdk';
import {
  STELLAR_NETWORK,
  CONTRACT_QUORUM_PROOF,
  CONTRACT_ZK_VERIFIER,
} from './config/env';
import { rpcClient } from './lib/rpcClient';
import { handleContractError } from './lib/handleContractError';

/** Stellar network passphrase map */
const PASSPHRASES = {
  testnet: Networks.TESTNET,
  mainnet: Networks.PUBLIC,
  futurenet: Networks.FUTURENET,
};

const networkPassphrase = PASSPHRASES[STELLAR_NETWORK] || Networks.TESTNET;

/**
 * Simulate a read-only contract call and return the parsed native JS value.
 * @param {string} contractId
 * @param {string} method
 * @param {xdr.ScVal[]} args
 */
async function simulate(contractId, method, args = []) {
  if (!contractId) {
    throw new Error(
      'Contract ID not configured. Set VITE_CONTRACT_QUORUM_PROOF in .env'
    );
  }

  const contract = new Contract(contractId);

  // Build a transaction to simulate (no source account needed for simulation)
  const { SorobanDataBuilder, TransactionBuilder, Keypair, Account, BASE_FEE, Operation } =
    await import('@stellar/stellar-sdk');

  // Use a dummy source account for simulation
  const dummyKeypair = Keypair.random();
  const dummyAccount = new Account(dummyKeypair.publicKey(), '0');

  const tx = new TransactionBuilder(dummyAccount, {
    fee: BASE_FEE,
    networkPassphrase,
  })
    .addOperation(contract.call(method, ...args))
    .setTimeout(30)
    .build();

  try {
    const result = await rpcClient.simulateTransaction(tx);

    if (StellarRpc.Api.isSimulationError(result)) {
      throw new Error(result.error || 'Simulation failed');
    }

    if (!result.result) {
      throw new Error('No result returned from simulation');
    }

    return scValToNative(result.result.retval);
  } catch (error) {
    throw new Error(handleContractError(error));
  }
}

/**
 * Retrieve a credential by numeric ID.
 * Returns a plain JS object with fields: id, subject, issuer, credential_type,
 * metadata_hash, revoked, expires_at.
 * Throws if the credential does not exist.
 */
export async function getCredential(credentialId) {
  try {
    const idVal = nativeToScVal(BigInt(credentialId), { type: 'u64' });
    return await simulate(CONTRACT_ID, 'get_credential', [idVal]);
  } catch (error) {
    throw new Error(handleContractError(error));
  }
}

/**
 * Get all credential IDs issued to a Stellar address (subject lookup).
 * Returns an array of BigInt credential IDs (may be empty).
 */
export async function getCredentialsBySubject(stellarAddress) {
  try {
    const addressVal = new Address(stellarAddress).toScVal();
    return await simulate(CONTRACT_ID, 'get_credentials_by_subject', [addressVal]);
  } catch (error) {
    throw new Error(handleContractError(error));
  }
}

/**
 * Check whether a credential has reached its quorum threshold.
 * @param {number|string} credentialId
 * @param {number|string} sliceId
 * @returns {Promise<boolean>}
 */
export async function isAttested(credentialId, sliceId) {
  try {
    const credVal = nativeToScVal(BigInt(credentialId), { type: 'u64' });
    const sliceVal = nativeToScVal(BigInt(sliceId), { type: 'u64' });
    return await simulate(CONTRACT_ID, 'is_attested', [credVal, sliceVal]);
  } catch (error) {
    throw new Error(handleContractError(error));
  }
}

/**
 * Get all attestor addresses for a credential.
 * @returns {Promise<string[]>}
 */
export async function getAttestors(credentialId) {
  try {
    const credVal = nativeToScVal(BigInt(credentialId), { type: 'u64' });
    return await simulate(CONTRACT_ID, 'get_attestors', [credVal]);
  } catch (error) {
    throw new Error(handleContractError(error));
  }
}

/**
 * Check whether a credential is expired.
 * @returns {Promise<boolean>}
 */
export async function isExpired(credentialId) {
  try {
    const credVal = nativeToScVal(BigInt(credentialId), { type: 'u64' });
    return await simulate(CONTRACT_ID, 'is_expired', [credVal]);
  } catch (error) {
    throw new Error(handleContractError(error));
  }
}

/**
 * Retrieve a quorum slice by ID.
 * Returns the QuorumSlice struct: { id, creator, attestors, threshold }
 * @returns {Promise<{id: bigint, creator: string, attestors: string[], threshold: number}>}
 */
export async function getSlice(sliceId) {
  try {
    const sliceVal = nativeToScVal(BigInt(sliceId), { type: 'u64' });
    return await simulate(CONTRACT_ID, 'get_slice', [sliceVal]);
  } catch (error) {
    throw new Error(handleContractError(error));
  }
}

/**
 * Verify a ZK claim against the ZK verifier contract.
 * @param {number|string} credentialId
 * @param {string} claimType  e.g. "has_degree"
 * @param {string} proofHex   hex-encoded proof bytes
 * @returns {Promise<boolean>}
 */
export async function verifyClaim(credentialId, claimType, proofHex) {
  try {
    if (!CONTRACT_ZK_VERIFIER) {
      throw new Error(
        'ZK Contract ID not configured. Set VITE_CONTRACT_ZK_VERIFIER in .env'
      );
    }
    const { nativeToScVal: n, xdr: x } = await import('@stellar/stellar-sdk');
    const credVal = nativeToScVal(BigInt(credentialId), { type: 'u64' });
    const claimVal = nativeToScVal(claimType, { type: 'string' });
    const proofBytes = hexToBytes(proofHex);
    const proofVal = xdr.ScVal.scvBytes(proofBytes);
    return await simulate(CONTRACT_ZK_VERIFIER, 'verify_claim', [credVal, claimVal, proofVal]);
  } catch (error) {
    throw new Error(handleContractError(error));
  }
}

/** Utility: hex string → Uint8Array */
function hexToBytes(hex) {
  const clean = hex.replace(/^0x/, '').replace(/\s/g, '');
  if (clean.length % 2 !== 0) throw new Error('Invalid hex string');
  const bytes = new Uint8Array(clean.length / 2);
  for (let i = 0; i < bytes.length; i++) {
    bytes[i] = parseInt(clean.substr(i * 2, 2), 16);
  }
  return bytes;
}

/** Metadata hash bytes → readable string (utf8 or hex fallback) */
export function decodeMetadataHash(rawValue) {
  if (typeof rawValue === 'string') return rawValue;
  if (rawValue instanceof Uint8Array || Array.isArray(rawValue)) {
    try {
      return new TextDecoder().decode(new Uint8Array(rawValue));
    } catch {
      return uint8ArrayToHex(new Uint8Array(rawValue));
    }
  }
  return String(rawValue);
}

function uint8ArrayToHex(arr) {
  return Array.from(arr)
    .map((b) => b.toString(16).padStart(2, '0'))
    .join('');
}

export { STELLAR_NETWORK as NETWORK, CONTRACT_QUORUM_PROOF as CONTRACT_ID, STELLAR_RPC_URL as RPC_URL };
