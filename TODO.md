# Issue 49: Add Test - mint SBT for revoked credential should panic

## Steps to Complete (Approved Plan)

### 1. [x] Setup Dependencies&#10;- Cargo.toml already has quorum_proof dep. Good.
- Ensure `contracts/sbt_registry/Cargo.toml` depends on `quorum_proof` crate.

### 2. [ ] Update SBT Registry Contract (`contracts/sbt_registry/src/lib.rs`)
- Add `use quorum_proof::{QuorumProofContractClient, Credential};`
- Update `mint` signature: `mint(env: Env, quorum_proof_id: Address, owner: Address, credential_id: u64, metadata_uri: Bytes) -> u64`
- In `mint`: 
  - `let qp_client = QuorumProofContractClient::new(&env, &quorum_proof_id);`
  - `let cred = qp_client.get_credential(&credential_id);`
  - `if cred.revoked { panic_with_error!(&env, ContractError::CredentialRevoked); }`
- Update all existing test calls to `client.mint(&qp_id, &owner, &cred_id, &uri)`

### 3. [ ] Add New Test
- `#[test] #[should_panic(expected = "CredentialRevoked")] fn test_mint_revoked_credential_panics() { ... }`
  - Deploy QP contract `qp_id`.
  - `let cred_id = qp_client.issue_credential(...);`
  - `qp_client.revoke_credential(&issuer, &cred_id);`
  - `client.mint(&qp_id, &owner, &cred_id, &uri); // panics`

### 4. [ ] Run Tests
- `cd contracts/sbt_registry && cargo test`
- Update snapshots if needed.

### 5. [ ] Commit & PR
- `git add . && git commit -m "Issue 49: Add revoked credential mint panic + test"`
- Use `create_pr.sh` or gh CLI.

**Progress: Starting with TODO.md creation.**

