/**
 * zkVerifier.ts — Typed contract client for the ZK Verifier contract.
 */

import {
  Contract,
  Networks,
  rpc as StellarRpc,
  scValToNative,
  nativeToScVal,
  Account,
  Keypair,
  TransactionBuilder,
  BASE_FEE,
  xdr,
} from '@stellar/stellar-sdk';

// ---------------------------------------------------------------------------
// Types
// ---------------------------------------------------------------------------

/** Mirrors the on-chain ClaimType enum variants. */
export type ClaimType = 'HasDegree' | 'HasLicense' | 'HasEmploymentHistory';

export interface ProofRequest {
  credential_id: bigint;
  claim_type: ClaimType;
  nonce: bigint;
}

// ---------------------------------------------------------------------------
// Internal helpers
// ---------------------------------------------------------------------------

const RPC_URL =
  import.meta.env.VITE_STELLAR_RPC_URL ?? 'https://soroban-testnet.stellar.org';
const NETWORK = (import.meta.env.VITE_STELLAR_NETWORK ?? 'testnet') as string;

const PASSPHRASES: Record<string, string> = {
  testnet: Networks.TESTNET,
  mainnet: Networks.PUBLIC,
  futurenet: Networks.FUTURENET,
};

function getPassphrase(): string {
  return PASSPHRASES[NETWORK] ?? Networks.TESTNET;
}

function getServer(): StellarRpc.Server {
  return new StellarRpc.Server(RPC_URL, { allowHttp: false });
}

function getContractId(): string {
  const id = import.meta.env.VITE_CONTRACT_ZK_VERIFIER ?? '';
  if (!id) throw new Error('VITE_CONTRACT_ZK_VERIFIER is not set');
  return id;
}

async function simulate<T>(method: string, args: xdr.ScVal[] = []): Promise<T> {
  const contractId = getContractId();
  const server = getServer();
  const contract = new Contract(contractId);

  const dummyKeypair = Keypair.random();
  const dummyAccount = new Account(dummyKeypair.publicKey(), '0');

  const tx = new TransactionBuilder(dummyAccount, {
    fee: BASE_FEE,
    networkPassphrase: getPassphrase(),
  })
    .addOperation(contract.call(method, ...args))
    .setTimeout(30)
    .build();

  const result = await server.simulateTransaction(tx);

  if (StellarRpc.Api.isSimulationError(result)) {
    throw new Error(result.error ?? 'Simulation failed');
  }
  if (!result.result) throw new Error('No result returned from simulation');

  return scValToNative(result.result.retval) as T;
}

function u64(value: bigint | number): xdr.ScVal {
  return nativeToScVal(BigInt(value), { type: 'u64' });
}

/** Encode a ClaimType string as the matching on-chain enum ScVal. */
function claimTypeToScVal(claimType: ClaimType): xdr.ScVal {
  return xdr.ScVal.scvVec([xdr.ScVal.scvSymbol(claimType)]);
}

/** Convert a hex string to Uint8Array. */
function hexToBytes(hex: string): Uint8Array {
  const clean = hex.replace(/^0x/, '');
  if (clean.length % 2 !== 0) throw new Error('Invalid hex string');
  const bytes = new Uint8Array(clean.length / 2);
  for (let i = 0; i < bytes.length; i++) {
    bytes[i] = parseInt(clean.substring(i * 2, i * 2 + 2), 16);
  }
  return bytes;
}

// ---------------------------------------------------------------------------
// Public API
// ---------------------------------------------------------------------------

/**
 * Generate a proof request for a given credential and claim type.
 * Returns a ProofRequest containing a nonce tied to the current ledger sequence.
 */
export async function generateProofRequest(
  credentialId: bigint | number,
  claimType: ClaimType,
): Promise<ProofRequest> {
  return simulate<ProofRequest>('generate_proof_request', [
    u64(credentialId),
    claimTypeToScVal(claimType),
  ]);
}

/**
 * Verify a ZK proof for a claim.
 * @param proof - raw proof bytes or a hex-encoded string
 */
export async function verifyClaim(
  credentialId: bigint | number,
  claimType: ClaimType,
  proof: Uint8Array | string,
): Promise<boolean> {
  const proofBytes = typeof proof === 'string' ? hexToBytes(proof) : proof;
  return simulate<boolean>('verify_claim', [
    u64(credentialId),
    claimTypeToScVal(claimType),
    xdr.ScVal.scvBytes(proofBytes),
  ]);
}
