/**
 * sbtRegistry.ts — Typed contract client for the SBT Registry contract.
 * Soulbound tokens are non-transferable by design; no transfer method is exposed.
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
// Types
// ---------------------------------------------------------------------------

export interface SoulboundToken {
  id: bigint;
  owner: string;
  credential_id: bigint;
  metadata_uri: Uint8Array;
}

export interface Delegation {
  token_id: bigint;
  delegatee: string;
  expires_at: bigint;
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
  const id = import.meta.env.VITE_CONTRACT_SBT_REGISTRY ?? '';
  if (!id) throw new Error('VITE_CONTRACT_SBT_REGISTRY is not set');
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
// NOTE: Transfer is intentionally omitted — SBTs are non-transferable.
// ---------------------------------------------------------------------------

/**
 * Mint a soulbound token bound to `owner`.
 * Non-transferability is enforced at the contract level; no transfer wrapper exists.
 */
export async function mintSbt(
  owner: string,
  credentialId: bigint | number,
  metadataUri: Uint8Array,
): Promise<bigint> {
  return simulate<bigint>('mint', [
    addr(owner),
    u64(credentialId),
    xdr.ScVal.scvBytes(metadataUri),
  ]);
}

/** Retrieve a soulbound token by ID. */
export async function getToken(tokenId: bigint | number): Promise<SoulboundToken> {
  return simulate<SoulboundToken>('get_token', [u64(tokenId)]);
}

/** Return the owner address of a token. */
export async function ownerOf(tokenId: bigint | number): Promise<string> {
  return simulate<string>('owner_of', [u64(tokenId)]);
}

/** Return all token IDs owned by a given address. */
export async function getTokensByOwner(owner: string): Promise<bigint[]> {
  return simulate<bigint[]>('get_tokens_by_owner', [addr(owner)]);
}

/** Delegate rights for a specific SBT to another address until a timestamp expires. */
export async function delegateSbtRights(
  owner: string,
  tokenId: bigint | number,
  delegatee: string,
  expiresAt: bigint | number,
): Promise<void> {
  return simulate<void>('delegate_sbt_rights', [
    addr(owner),
    u64(tokenId),
    addr(delegatee),
    u64(expiresAt),
  ]);
}

/** Retrieve delegation details for a token. */
export async function getDelegation(tokenId: bigint | number): Promise<Delegation> {
  return simulate<Delegation>('get_delegation', [u64(tokenId)]);
}

/** Check whether a delegatee currently holds active rights for the token. */
export async function isDelegateActive(
  tokenId: bigint | number,
  delegatee: string,
): Promise<boolean> {
  return simulate<boolean>('is_delegate_active', [u64(tokenId), addr(delegatee)]);
}
