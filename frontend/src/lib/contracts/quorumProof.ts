/**
 * quorumProof.ts — Typed contract client for the QuorumProof contract.
 * All RPC calls are centralised here; UI components must not call RPC directly.
 */

import {
  Contract,
  Networks,
  rpc as StellarRpc,
  scValToNative,
  nativeToScVal,
  Address,
  Account,
  Keypair,
  TransactionBuilder,
  BASE_FEE,
  xdr,
} from '@stellar/stellar-sdk';

// ---------------------------------------------------------------------------
// Types mirroring the on-chain contract structs
// ---------------------------------------------------------------------------

export interface Credential {
  id: bigint;
  subject: string;
  issuer: string;
  credential_type: number;
  metadata_hash: Uint8Array;
  revoked: boolean;
  expires_at: bigint | null;
}

export interface QuorumSlice {
  id: bigint;
  creator: string;
  attestors: string[];
  threshold: number;
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
  const id = import.meta.env.VITE_CONTRACT_QUORUM_PROOF ?? '';
  if (!id) throw new Error('VITE_CONTRACT_QUORUM_PROOF is not set');
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

function addr(stellarAddress: string): xdr.ScVal {
  return new Address(stellarAddress).toScVal();
}

// ---------------------------------------------------------------------------
// Public API
// ---------------------------------------------------------------------------

/** Issue a new credential. Returns the new credential ID. */
export async function issueCredential(
  issuer: string,
  subject: string,
  credentialType: number,
  metadataHash: Uint8Array,
  expiresAt: bigint | null = null,
): Promise<bigint> {
  return simulate<bigint>('issue_credential', [
    addr(issuer),
    addr(subject),
    nativeToScVal(credentialType, { type: 'u32' }),
    xdr.ScVal.scvBytes(metadataHash),
    expiresAt !== null
      ? xdr.ScVal.scvVec([u64(expiresAt)])
      : xdr.ScVal.scvVoid(),
  ]);
}

/** Retrieve a credential by ID. */
export async function getCredential(credentialId: bigint | number): Promise<Credential> {
  return simulate<Credential>('get_credential', [u64(credentialId)]);
}

/** Revoke a credential. Caller must be the issuer or subject. */
export async function revokeCredential(
  caller: string,
  credentialId: bigint | number,
): Promise<void> {
  return simulate<void>('revoke_credential', [addr(caller), u64(credentialId)]);
}

/** Create a quorum slice. Returns the new slice ID. */
export async function createSlice(
  creator: string,
  attestors: string[],
  threshold: number,
): Promise<bigint> {
  const attestorVec = xdr.ScVal.scvVec(attestors.map(addr));
  return simulate<bigint>('create_slice', [
    addr(creator),
    attestorVec,
    nativeToScVal(threshold, { type: 'u32' }),
  ]);
}

/** Add an attestor to an existing slice. Only the slice creator can call this. */
export async function addAttestor(
  creator: string,
  sliceId: bigint | number,
  attestor: string,
): Promise<void> {
  return simulate<void>('add_attestor', [addr(creator), u64(sliceId), addr(attestor)]);
}

/** Attest a credential using a quorum slice. */
export async function attest(
  attestor: string,
  credentialId: bigint | number,
  sliceId: bigint | number,
): Promise<void> {
  return simulate<void>('attest', [addr(attestor), u64(credentialId), u64(sliceId)]);
}

/** Check if a credential has met its quorum threshold. */
export async function isAttested(
  credentialId: bigint | number,
  sliceId: bigint | number,
): Promise<boolean> {
  return simulate<boolean>('is_attested', [u64(credentialId), u64(sliceId)]);
}

/** Get all attestor addresses for a credential. */
export async function getAttestors(credentialId: bigint | number): Promise<string[]> {
  return simulate<string[]>('get_attestors', [u64(credentialId)]);
}

/** Get all credential IDs issued to a subject address. */
export async function getCredentialsBySubject(subject: string): Promise<bigint[]> {
  return simulate<bigint[]>('get_credentials_by_subject', [addr(subject)]);
}

/** Check whether a credential is expired. */
export async function isExpired(credentialId: bigint | number): Promise<boolean> {
  return simulate<boolean>('is_expired', [u64(credentialId)]);
}

/** Retrieve a quorum slice by ID. Returns { id, creator, attestors, threshold }. */
export async function getSlice(sliceId: bigint | number): Promise<QuorumSlice> {
  return simulate<QuorumSlice>('get_slice', [u64(sliceId)]);
}
