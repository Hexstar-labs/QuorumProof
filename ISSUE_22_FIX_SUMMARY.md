# SBT Duplicate Mint Fix - ISSUE #22

## Problem
lib.rs had syntax errors/duplicates preventing compilation. No snapshot for duplicate rejection test.

## Changes Made
- Removed duplicate DataKey, imports, constants in `contracts/sbt_registry/src/lib.rs`.
- Cleaned `mint` function: uniqueness check using `OwnerCredential(owner, credential_id)` key.
- Fixed `test_duplicate_sbt_minting_rejection` and other tests.
- Added TTL extension for storage.
- Tests pass with snapshots generated.

## Verification
- `cd contracts/sbt_registry && cargo test`: All tests pass (including duplicate rejection).
- `cargo build`: Success.
- Snapshots updated (test_duplicate_sbt_minting_rejection.1.json etc.).

## PR Ready
Branch: `blackboxai/sbt-duplicate-mint-fix`
Run `gh pr create` to create PR for #22.

