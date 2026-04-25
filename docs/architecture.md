# Architecture Overview

## Contract Dependency Graph

```
┌─────────────────────────────────────────────────────────────┐
│                        quorum_proof                         │
│  - Issues and manages credentials                           │
│  - Manages quorum slices and attestations                   │
│  - Orchestrates cross-contract verification flows           │
└──────────────┬──────────────────────────┬───────────────────┘
               │ calls                    │ calls
               ▼                          ▼
┌──────────────────────────┐  ┌───────────────────────────────┐
│       sbt_registry       │  │         zk_verifier           │
│  - Mints soulbound tokens│  │  - Verifies ZK claims         │
│  - Enforces non-transfer │  │  - Generates proof requests   │
│  - Calls quorum_proof to │  │  ⚠️  STUB: admin-gated until  │
│    validate credentials  │  │     real ZK is implemented    │
└──────────────┬───────────┘  └───────────────────────────────┘
               │ calls
               ▼
┌─────────────────────────────────────────────────────────────┐
│                        quorum_proof                         │
│  (cross-call: sbt_registry → quorum_proof.get_credential)  │
└─────────────────────────────────────────────────────────────┘
```

## Contract Responsibilities

### `quorum_proof`
The central contract. Owns all credential and quorum slice state.

- Issues, revokes, and renews credentials
- Creates and manages quorum slices (weighted FBA trust model)
- Records attestations from slice members
- Calls `sbt_registry` indirectly via `verify_engineer`
- Calls `zk_verifier` via `verify_engineer` and `verify_claim_batch`

### `sbt_registry`
Manages soulbound tokens (non-transferable NFTs).

- Mints SBTs linked to a `credential_id`
- On `mint`, cross-calls `quorum_proof.get_credential` to verify the credential exists and is not revoked — **SBTs cannot be minted for non-existent or revoked credentials**
- Enforces one SBT per `(owner, credential_id)` pair
- Supports admin-gated ownership transfer (wallet recovery / legal name change)

Depends on: `quorum_proof`

### `zk_verifier`
Handles zero-knowledge proof verification for conditional credential claims.

- Exposes `verify_claim(admin, quorum_proof_id, credential_id, claim_type, proof)`
- **⚠️ Current implementation is a stub**: any non-empty `Bytes` passes. This is admin-gated to prevent misuse until real ZK (Groth16/PLONK) is implemented in v1.1.
- Generates `ProofRequest` nonces for off-chain proof generation

Depends on: nothing (stateless verifier)

## Cross-Contract Call Map

| Caller          | Callee          | Method                  | Purpose                                      |
|-----------------|-----------------|-------------------------|----------------------------------------------|
| `sbt_registry`  | `quorum_proof`  | `get_credential`        | Validate credential exists and is not revoked before minting SBT |
| `quorum_proof`  | `zk_verifier`   | `verify_claim`          | Verify a ZK proof for a specific claim type  |
| `quorum_proof`  | `sbt_registry`  | `get_tokens_by_owner`, `get_token` | Check subject holds an SBT for the credential |

## Deployment Order

Contracts must be deployed and initialized in this order due to the dependency graph:

1. **`quorum_proof`** — no external contract dependencies at deploy time
   ```bash
   stellar contract deploy --wasm quorum_proof.wasm
   stellar contract invoke quorum_proof -- initialize --admin <ADMIN_ADDRESS>
   ```

2. **`zk_verifier`** — no external contract dependencies at deploy time
   ```bash
   stellar contract deploy --wasm zk_verifier.wasm
   stellar contract invoke zk_verifier -- initialize --admin <ADMIN_ADDRESS>
   ```

3. **`sbt_registry`** — requires `quorum_proof` address at initialization
   ```bash
   stellar contract deploy --wasm sbt_registry.wasm
   stellar contract invoke sbt_registry -- initialize \
     --admin <ADMIN_ADDRESS> \
     --quorum-proof-id <QUORUM_PROOF_CONTRACT_ID>
   ```

`sbt_registry` must be initialized last because it stores the `quorum_proof` contract address to use for cross-contract credential validation on every `mint` call.
