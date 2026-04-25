# Security Audit Checklist — QuorumProof Contracts

Covers all three Soroban contracts: `quorum_proof`, `sbt_registry`, `zk_verifier`.  
Work through each section in order. Mark each item ✅ pass, ❌ fail, or ⚠️ needs review.

---

## 1. Authentication & Authorization

| # | Check | Contract(s) | Status |
|---|-------|-------------|--------|
| 1.1 | Every state-mutating function calls `caller.require_auth()` before any storage write | all | |
| 1.2 | Admin-only functions verify the caller matches the stored `DataKey::Admin` address | all | |
| 1.3 | `revoke_credential` only allows the original issuer (not subject, not third party) | quorum_proof | |
| 1.4 | `pause` / `unpause` are gated to admin only | quorum_proof | |
| 1.5 | `verify_claim` (ZK stub) is gated to admin only until real ZK is implemented | zk_verifier | |
| 1.6 | `mint` in sbt_registry requires owner auth and cross-validates credential via quorum_proof | sbt_registry | |
| 1.7 | Recovery execution (`execute_recovery`) is restricted to the original issuer | quorum_proof | |
| 1.8 | Blacklist add/remove is restricted to the issuer who created the entry | quorum_proof | |

**Remediation**: Any function missing `require_auth()` must have it added before the first storage read or write. Admin checks must compare against the value stored at `DataKey::Admin`, not a constructor argument.

---

## 2. Input Validation

| # | Check | Contract(s) | Status |
|---|-------|-------------|--------|
| 2.1 | `credential_type` is validated `> 0` before issuance | quorum_proof | |
| 2.2 | `metadata_hash` / `metadata_uri` are validated non-empty before storage | quorum_proof, sbt_registry | |
| 2.3 | `threshold` is validated `> 0` and `<= attestors.len()` in `create_slice` | quorum_proof | |
| 2.4 | Attestor and slice arrays are bounded by `MAX_ATTESTORS_PER_SLICE = 20` | quorum_proof | |
| 2.5 | Batch operations are bounded by `MAX_BATCH_SIZE = 50` | quorum_proof | |
| 2.6 | Multisig approver arrays are bounded by `MAX_MULTISIG_SIGNERS = 10` | quorum_proof | |
| 2.7 | All `Address` inputs pass `require_valid_address` before use | quorum_proof | |
| 2.8 | ZK proof `Bytes` are validated non-empty in `verify_claim` | zk_verifier | |
| 2.9 | `holder_commitment` is validated non-empty in `generate_anonymous_proof_request` | zk_verifier | |

**Remediation**: Add explicit bounds checks and `assert!` / `panic_with_error!` guards at the top of each function. Never rely on downstream storage operations to surface invalid inputs.

---

## 3. Soulbound Token Enforcement

| # | Check | Contract(s) | Status |
|---|-------|-------------|--------|
| 3.1 | `transfer` always panics with `SoulboundNonTransferable` — no code path allows transfer | sbt_registry | |
| 3.2 | `mint` checks `OwnerCredential(owner, credential_id)` key to prevent duplicate SBTs | sbt_registry | |
| 3.3 | `mint` cross-calls `quorum_proof.is_revoked` and panics if credential is revoked | sbt_registry | |
| 3.4 | `burn_sbt` is restricted to the token owner or contract admin | sbt_registry | |
| 3.5 | No function in `sbt_registry` updates the `owner` field of a `SoulboundToken` outside of admin-gated recovery | sbt_registry | |

**Remediation**: The `transfer` function must remain a permanent panic. Any future refactor that adds a transfer path must be treated as a critical security regression.

---

## 4. Credential Lifecycle Integrity

| # | Check | Contract(s) | Status |
|---|-------|-------------|--------|
| 4.1 | Revoked credentials cannot be attested | quorum_proof | |
| 4.2 | Revoked credentials cannot have new SBTs minted against them | sbt_registry | |
| 4.3 | Double revocation is rejected with `"credential already revoked"` | quorum_proof | |
| 4.4 | Expired credentials (`expires_at` in the past) are treated as invalid in `is_attested` | quorum_proof | |
| 4.5 | `DuplicateCredential` error is raised when the same issuer issues the same type to the same subject twice | quorum_proof | |
| 4.6 | Credential recovery cannot be initiated for a revoked credential | quorum_proof | |
| 4.7 | Only one pending recovery per credential (`RecoveryAlreadyExists` guard) | quorum_proof | |

**Remediation**: Any path that skips the `revoked` check before a state mutation is a critical bug. Add a dedicated test for each lifecycle transition.

---

## 5. Quorum Slice & Attestation Logic

| # | Check | Contract(s) | Status |
|---|-------|-------------|--------|
| 5.1 | An attestor cannot attest the same credential+slice pair twice (`DuplicateAttestor`) | quorum_proof | |
| 5.2 | An attestor must be a member of the slice to attest (`NotInSlice`) | quorum_proof | |
| 5.3 | Attestation time windows are enforced: attestations outside the window are rejected | quorum_proof | |
| 5.4 | Fork detection fires when two attestors submit conflicting boolean values for the same slice | quorum_proof | |
| 5.5 | `is_attested` correctly counts weighted attestations against the slice threshold | quorum_proof | |
| 5.6 | Attestation expiry (`expires_at`) is respected in `is_attested` | quorum_proof | |

**Remediation**: Weighted threshold logic must be reviewed for integer overflow. Use `saturating_add` for weight accumulation (already present — verify it is not bypassed in any code path).

---

## 6. Storage & TTL Management

| # | Check | Contract(s) | Status |
|---|-------|-------------|--------|
| 6.1 | Every `storage().instance().set()` call is followed by `extend_ttl(STANDARD_TTL, EXTENDED_TTL)` | all | |
| 6.2 | Persistent storage entries (`Token`, `Owner`, `OwnerTokens`) have TTL extended after write | sbt_registry | |
| 6.3 | No storage entry can be silently evicted during normal operation (TTL covers expected credential lifetime) | all | |
| 6.4 | `initialize` is guarded against double-initialization (`already initialized` assert) | all | |

**Remediation**: Missing `extend_ttl` calls cause silent data loss after ledger eviction. Audit every `set()` call and confirm a corresponding `extend_ttl` follows it.

---

## 7. Cross-Contract Call Safety

| # | Check | Contract(s) | Status |
|---|-------|-------------|--------|
| 7.1 | `sbt_registry.mint` validates the `quorum_proof_id` is set before making cross-contract calls | sbt_registry | |
| 7.2 | Cross-contract calls use the stored `DataKey::QuorumProofId`, not a caller-supplied address | sbt_registry | |
| 7.3 | `quorum_proof` calls to `zk_verifier` pass the admin address from storage, not from the caller | quorum_proof | |
| 7.4 | No cross-contract call passes unvalidated user input as a contract address | all | |

**Remediation**: Never allow a caller to supply the target contract address for a cross-contract call. Always read it from initialized storage to prevent contract substitution attacks.

---

## 8. ZK Verifier Stub Risk

| # | Check | Contract(s) | Status |
|---|-------|-------------|--------|
| 8.1 | `verify_claim` is admin-gated and cannot be called by arbitrary users | zk_verifier | |
| 8.2 | All call sites of `verify_claim` in `quorum_proof` pass the stored admin address | quorum_proof | |
| 8.3 | The README and contract doc comment clearly warn that the stub accepts any non-empty proof | zk_verifier | |
| 8.4 | No production credential decision relies solely on `verify_claim` output until v1.1 | quorum_proof | |

**Remediation**: Until Groth16/PLONK verification is implemented (tracked in issue #ZK-IMPL), treat any `verify_claim` result as untrusted. Do not gate access-control decisions on it.

---

## 9. Pause / Emergency Stop

| # | Check | Contract(s) | Status |
|---|-------|-------------|--------|
| 9.1 | `pause` blocks `issue_credential`, `attest`, `revoke_credential`, and `mint` | quorum_proof, sbt_registry | |
| 9.2 | Read-only functions (`get_credential`, `is_attested`, `get_slice`) remain accessible while paused | quorum_proof | |
| 9.3 | `unpause` is restricted to admin only | quorum_proof | |
| 9.4 | There is no way to permanently brick the contract (admin can always unpause) | quorum_proof | |

**Remediation**: Verify `require_not_paused()` is called at the top of every state-mutating function. Read paths must not call it.

---

## 10. Audit Procedures

### Pre-Audit Preparation
1. Run `cargo test` — all tests must pass with zero failures.
2. Run `./scripts/mutation_test.sh` — mutation score must be ≥ 80%.
3. Run `cargo clippy -- -D warnings` — zero warnings.
4. Confirm WASM binary sizes are within expected bounds (quorum_proof < 200 KB, others < 50 KB).

### Manual Review Steps
1. For each public function: verify auth check → input validation → business logic → storage write → TTL extension order.
2. Trace every cross-contract call: confirm the target address comes from storage, not caller input.
3. Review every `assert!` and `panic_with_error!`: confirm the error variant is appropriate and the message is not information-leaking.
4. Check all integer arithmetic for overflow: look for `+`, `-`, `*` on `u32`/`u64` without `saturating_*` or `checked_*`.
5. Verify the ZK stub is unreachable without admin auth.

### Automated Checks
```bash
# Full test suite
cargo test

# Mutation testing
./scripts/mutation_test.sh

# Lint
cargo clippy -- -D warnings

# Check for integer overflow patterns (manual grep)
grep -n '\bu32\b.*+\|\bu64\b.*+' contracts/*/src/lib.rs | grep -v saturating | grep -v checked
```

---

## 11. Remediation Severity Guide

| Severity | Definition | SLA |
|----------|-----------|-----|
| **Critical** | Unauthorized state mutation, bypass of `require_auth`, SBT transfer enabled, ZK stub exposed without admin gate | Fix before any mainnet deployment |
| **High** | Missing TTL extension (data loss risk), missing `require_not_paused`, double-initialization possible | Fix before next release |
| **Medium** | Missing input bounds check, integer arithmetic without overflow protection, cross-contract address from caller | Fix within 2 sprints |
| **Low** | Missing doc comment, inconsistent error message, non-critical lint warning | Fix in next maintenance window |

---

## 12. Sign-Off

| Role | Name | Date | Signature |
|------|------|------|-----------|
| Lead Auditor | | | |
| Contract Author | | | |
| Security Reviewer | | | |

All **Critical** and **High** findings must be resolved and re-verified before sign-off.
