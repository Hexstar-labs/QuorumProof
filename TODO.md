# Issue #19: Fix TTL Management in QuorumProofContract

## Plan Breakdown & Progress

### 1. [x] Checkout new branch `blackboxai/issue-19-ttl-fix`
### 2. [x] Add TTL constants and documentation to `contracts/quorum_proof/src/lib.rs`
### 3. [x] Insert `extend_ttl` calls after all 7 storage writes
### 4. [x] Add new test `test_storage_persists_across_ledgers`
### 5. [x] Run `cargo test` to verify (passes)
### 6. [] Create PR
### 6. [] Create PR

Estimated Time: 2 hours  
Priority: High
