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

const RPC_URL =
  import.meta.env.VITE_STELLAR_RPC_URL ||
  'https://soroban-testnet.stellar.org';

const NETWORK = import.meta.env.VITE_STELLAR_NETWORK || 'testnet';

const CONTRACT_ID =
  import.meta.env.VITE_CONTRACT_QUORUM_PROOF || '';

const ZK_CONTRACT_ID =
  import.meta.env.VITE_CONTRACT_ZK_VERIFIER || '';

/** Stellar network passphrase map */
const PASSPHRASES = {
  testnet: Networks.TESTNET,
  mainnet: Networks.PUBLIC,
  futurenet: Networks.FUTURENET,
};

const networkPassphrase = PASSPHRASES[NETWORK] || Networks.TESTNET;

/** Build an RPC server instance */
function getServer() {
  return new StellarRpc.Server(RPC_URL, { allowHttp: false });
}

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

  const server = getServer();
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

  const result = await server.simulateTransaction(tx);

  if (StellarRpc.Api.isSimulationError(result)) {
    throw new Error(result.error || 'Simulation failed');
  }

  if (!result.result) {
    throw new Error('No result returned from simulation');
  }

  return scValToNative(result.result.retval);
}

/**
 * Retrieve a credential by numeric ID.
 * Returns a plain JS object with fields: id, subject, issuer, credential_type,
 * metadata_hash, revoked, expires_at.
 * Throws if the credential does not exist.
 */
export async function getCredential(credentialId) {
  const idVal = nativeToScVal(BigInt(credentialId), { type: 'u64' });
  return simulate(CONTRACT_ID, 'get_credential', [idVal]);
}

/**
 * Get all credential IDs issued to a Stellar address (subject lookup).
 * Returns an array of BigInt credential IDs (may be empty).
 */
export async function getCredentialsBySubject(stellarAddress) {
  const addressVal = new Address(stellarAddress).toScVal();
  return simulate(CONTRACT_ID, 'get_credentials_by_subject', [addressVal]);
}

/**
 * Check whether a credential has reached its quorum threshold.
 * @param {number|string} credentialId
 * @param {number|string} sliceId
 * @returns {Promise<boolean>}
 */
export async function isAttested(credentialId, sliceId) {
  const credVal = nativeToScVal(BigInt(credentialId), { type: 'u64' });
  const sliceVal = nativeToScVal(BigInt(sliceId), { type: 'u64' });
  return simulate(CONTRACT_ID, 'is_attested', [credVal, sliceVal]);
}

/**
 * Retrieve a quorum slice by ID.
 * Returns a plain JS object with fields: id, creator, attestors, threshold.
 * @param {number|string} sliceId
 * @returns {Promise<{id: bigint, creator: string, attestors: string[], threshold: number}>}
 */
export async function getSlice(sliceId) {
  const sliceVal = nativeToScVal(BigInt(sliceId), { type: 'u64' });
  return simulate(CONTRACT_ID, 'get_slice', [sliceVal]);
}

/**
 * Get all attestor addresses for a credential.
 * @returns {Promise<string[]>}
 */
export async function getAttestors(credentialId) {
  const credVal = nativeToScVal(BigInt(credentialId), { type: 'u64' });
  return simulate(CONTRACT_ID, 'get_attestors', [credVal]);
}

/**
 * Check whether a credential is expired.
 * @returns {Promise<boolean>}
 */
export async function isExpired(credentialId) {
  const credVal = nativeToScVal(BigInt(credentialId), { type: 'u64' });
  return simulate(CONTRACT_ID, 'is_expired', [credVal]);
}

/**
 * Verify a ZK claim against the ZK verifier contract.
 * @param {number|string} credentialId
 * @param {string} claimType  e.g. "has_degree"
 * @param {string} proofHex   hex-encoded proof bytes
 * @returns {Promise<boolean>}
 */
export async function verifyClaim(credentialId, claimType, proofHex) {
  if (!ZK_CONTRACT_ID) {
    throw new Error(
      'ZK Contract ID not configured. Set VITE_CONTRACT_ZK_VERIFIER in .env'
    );
  }
  const credVal = nativeToScVal(BigInt(credentialId), { type: 'u64' });
  // Encode ClaimType as a Soroban enum variant: scvVec([scvSymbol("HasDegree")])
  const claimVal = xdr.ScVal.scvVec([xdr.ScVal.scvSymbol(claimType)]);
  const proofBytes = hexToBytes(proofHex);
  const proofVal = xdr.ScVal.scvBytes(proofBytes);
  return simulate(ZK_CONTRACT_ID, 'verify_claim', [credVal, claimVal, proofVal]);
}

/** Utility: hex string → Uint8Array */
export function hexToBytes(hex) {
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

export { RPC_URL, NETWORK, CONTRACT_ID };
