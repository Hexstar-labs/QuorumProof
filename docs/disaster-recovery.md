# Disaster Recovery Procedures

## Overview

This document covers recovery procedures, backup strategy, and recovery testing for QuorumProof. Because core credential data lives on the Stellar blockchain, recovery focuses on restoring contract access, operator keys, and off-chain supporting infrastructure.

---

## 1. Recovery Procedures

### 1.1 Lost Deployer / Admin Key

1. If a backup key was pre-registered as a secondary admin via the contract's admin management, invoke `set_admin(new_admin)` from the backup key.
2. If no backup key exists, the contract is unrecoverable — redeploy all three contracts and re-issue credentials from source institutions.
3. Update `.env` and GitHub secrets (`STELLAR_SECRET_KEY`) with the new key immediately.

### 1.2 Contract Redeployment

Use this procedure when a contract must be redeployed (e.g. critical bug, key compromise):

```bash
# 1. Build fresh WASM artifacts
./scripts/build.sh

# 2. Deploy to the target network
./scripts/deploy_testnet.sh   # or deploy_mainnet.sh for production

# 3. Update contract addresses in .env
CONTRACT_QUORUM_PROOF=<new-id>
CONTRACT_SBT_REGISTRY=<new-id>
CONTRACT_ZK_VERIFIER=<new-id>

# 4. Update frontend/dashboard env files and redeploy frontend
```

> Existing on-chain SBTs issued under the old contract address are not migrated automatically. Coordinate with attestors to re-attest affected credentials.

### 1.3 RPC / Network Outage

- Switch `STELLAR_RPC_URL` to an alternate RPC endpoint (e.g. Horizon public API or a self-hosted Stellar node).
- Testnet: `https://soroban-testnet.stellar.org`
- Mainnet fallback: `https://horizon.stellar.org`
- No contract redeployment is needed; only the client configuration changes.

### 1.4 Frontend / Dashboard Outage

1. Redeploy from the latest tagged release on the `main` branch via the CI/CD pipeline (`workflow_dispatch` on `deploy.yml`).
2. If the hosting provider is unavailable, deploy to an alternate static host using `npm run build` output from `frontend/` or `dashboard/`.

---

## 2. Backup Strategy

| Asset | What to Back Up | Where | Frequency |
|---|---|---|---|
| Deployer secret key | Stellar secret key (`S...`) | Encrypted cold storage + GitHub secret | On creation / rotation |
| Contract IDs | `CONTRACT_QUORUM_PROOF`, `CONTRACT_SBT_REGISTRY`, `CONTRACT_ZK_VERIFIER` | `.env`, repo wiki, team password manager | After every deployment |
| Environment config | `.env` values (non-secret portions) | `.env.example` kept up to date in repo | On every config change |
| WASM artifacts | Built `.wasm` files | GitHub Actions artifacts (retained 90 days) | Every CI run on `main` |
| On-chain state | Credential and attestation records | Inherently replicated by Stellar network | Continuous (blockchain) |

**Key rotation policy**: Rotate the deployer key every 90 days or immediately after any suspected compromise.

---

## 3. Recovery Testing

Run recovery drills on testnet. Do **not** use mainnet for drills.

### 3.1 Key Recovery Drill (quarterly)

1. Generate a temporary test key: `stellar keys generate dr-test --network testnet`
2. Register it as a secondary admin on the testnet contract.
3. Revoke the primary test key and confirm `dr-test` can call admin-gated functions.
4. Clean up: remove `dr-test` and restore primary key.

### 3.2 Contract Redeployment Drill (per release)

1. On testnet, run `./scripts/deploy_testnet.sh` from a clean environment (no cached `.env`).
2. Verify all three contract IDs are returned and functional via `cargo test`.
3. Confirm the CI deploy workflow (`deploy.yml`) completes successfully end-to-end.

### 3.3 RPC Failover Drill (quarterly)

1. Point `STELLAR_RPC_URL` at the fallback endpoint in `.env`.
2. Run `cargo test` and confirm all contract interactions succeed.
3. Restore the primary RPC URL.

### 3.4 Checklist

- [ ] Deployer key backup verified in cold storage
- [ ] Contract IDs recorded and accessible to the team
- [ ] Secondary admin key registered on-chain
- [ ] CI deploy workflow tested via `workflow_dispatch`
- [ ] RPC failover endpoint confirmed reachable
- [ ] Recovery drill results logged with date and outcome
