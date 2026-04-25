# TODO: Issue #290 — Add Credential Holder Recovery Mechanism

## Plan

### 1. `contracts/quorum_proof/src/lib.rs`
- [x] Add missing `is_multisig_approved` stub (compilation fix)
- [x] Add new constants (`TOPIC_RECOVERY_INITIATED`, `TOPIC_RECOVERY_APPROVED`, `TOPIC_RECOVERY_EXECUTED`)
- [x] Add new `ContractError` variants (`RecoveryNotFound`, `RecoveryAlreadyExists`, `RecoveryNotPending`, `RecoveryAlreadyApproved`, `RecoveryThresholdNotMet`, `NotRecoveryApprover`, `DuplicateRecoveryApproval`)
- [x] Add new structs (`RecoveryStatus`, `RecoveryRequest`, `RecoveryApproval`, `RecoveryInitiatedEventData`, `RecoveryApprovedEventData`, `RecoveryExecutedEventData`)
- [x] Add new `DataKey` variants (`RecoveryRequest(u64)`, `RecoveryRequestCount`, `CredentialRecovery(u64)`, `RecoveryApprovals(u64)`)
- [x] Add new `ActivityType` variant (`CredentialRecovered`)
- [x] Implement `initiate_recovery`
- [x] Implement `approve_recovery`
- [x] Implement `execute_recovery`
- [x] Implement `get_recovery_request`
- [x] Implement `get_recovery_approvals`
- [x] Implement `cancel_recovery`
- [x] Add comprehensive tests (14 tests)

### 2. `contracts/sbt_registry/src/lib.rs`
- [x] Add `recover_sbt` function callable by quorum_proof contract

### 3. Testing & Verification
- [ ] Run `cargo test` in `contracts/quorum_proof`
- [ ] Run `cargo test` in `contracts/sbt_registry`
- [ ] Update snapshots if needed

