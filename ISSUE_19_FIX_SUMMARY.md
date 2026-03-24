# Issue #19 Fix Summary: TTL Management for Instance Storage

## Changes Made
- Added `STANDARD_TTL = 16_384` and `EXTENDED_TTL = 524_288` constants.
- Added comprehensive doc comment explaining TTL strategy.
- Inserted `env.storage().instance().extend_ttl(STANDARD_TTL, EXTENDED_TTL)` after every `.set()` call in:
  - `issue_credential` (2 calls)
  - `revoke_credential` (1 call)
  - `create_slice` (2 calls)
  - `attest` (1 call)
- Added new test `test_storage_persists_across_ledgers()` that advances ledger sequence by 20,000 ledgers and verifies data persistence.

## Verification
- `cargo test` passes (5 tests + new TTL test).
- Snapshot files updated automatically.
- No breaking changes to existing functionality.

## PR Ready
Branch: `blackboxai/issue-19-ttl-fix`
Run `./create_pr.sh` to create PR for #19.
