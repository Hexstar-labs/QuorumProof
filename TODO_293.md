# TODO: Issue #293 — Implement Credential Holder Blacklist

## Plan

### 1. Data Structures & Error Handling

#### New Error Variants in ContractError
- [ ] `HolderBlacklisted` - Holder is blacklisted
- [ ] `AlreadyBlacklisted` - Holder already on blacklist
- [ ] `NotBlacklisted` - Holder not on blacklist

#### New Structures
- [ ] `BlacklistEntry` - Stores blacklist record with issuer, holder, timestamp, reason

#### DataKey Variants
- [ ] `Blacklist(Address, Address)` - Maps (issuer, holder) to blacklist entry
- [ ] `IssuerBlacklist(Address)` - All blacklisted holders for an issuer
- [ ] `HolderBlacklists(Address)` - All issuers who have blacklisted a holder

### 2. Core Functions to Implement

#### add_holder_to_blacklist
- [ ] Function signature: `(issuer, holder, reason) -> void`
- [ ] Only issuer can add
- [ ] Check not already blacklisted
- [ ] Store entry with timestamp
- [ ] Emit event
- [ ] Add to issuer's blacklist
- [ ] Add to holder's recorded blacklists

#### is_holder_blacklisted
- [ ] Function signature: `(issuer, holder) -> bool`
- [ ] Query for specific issuer-holder pair
- [ ] Return true/false

#### remove_holder_from_blacklist
- [ ] Function signature: `(issuer, holder) -> void`
- [ ] Only issuer can remove
- [ ] Check holder is blacklisted
- [ ] Remove entry
- [ ] Emit event
- [ ] Update blacklist stores

#### Helper/Query Functions
- [ ] `get_blacklisted_by_issuer(issuer) -> Vec<Address>` - Get all holders blacklisted by issuer
- [ ] `get_blacklist_entries_for_holder(holder) -> Vec<(Address, BlacklistEntry)>` - Get all issuers who blacklisted
- [ ] `get_blacklist_entry(issuer, holder) -> BlacklistEntry` - Get specific entry details

### 3. Integration with Existing Features

#### Issue Credential
- [ ] Add check: reject if holder is blacklisted by issuer
- [ ] Return appropriate error

#### Batch Issue Credentials
- [ ] Add blacklist check for each subject
- [ ] Reject entire batch if any subject is blacklisted

#### Attestation
- [ ] Consider: should attestors see if holder is blacklisted?
- [ ] May want to warn/inform attestors

### 4. Tests

#### Basic Blacklist Operations (3 tests)
- [ ] test_add_holder_to_blacklist
- [ ] test_is_holder_blacklisted
- [ ] test_remove_holder_from_blacklist

#### Error Cases (4 tests)
- [ ] test_add_already_blacklisted_panics
- [ ] test_remove_not_blacklisted_panics
- [ ] test_add_blacklist_unauthorized_panics
- [ ] test_remove_blacklist_unauthorized_panics

#### Credential Issuance Integration (3 tests)
- [ ] test_issue_credential_to_blacklisted_holder_fails
- [ ] test_batch_issue_rejects_blacklisted_subject
- [ ] test_issue_after_removal_succeeds

#### Query Functions (3 tests)
- [ ] test_get_blacklisted_by_issuer
- [ ] test_get_blacklist_entries_for_holder
- [ ] test_get_blacklist_entry_details

#### Multiple Issuers (2 tests)
- [ ] test_different_issuers_independent_blacklists
- [ ] test_holder_blacklisted_by_multiple_issuers

#### Edge Cases (2 tests)
- [ ] test_blacklist_operations_when_paused
- [ ] test_blacklist_time_tracking

### 5. Events

- [ ] Define `HolderBlacklistedEventData`
- [ ] Define `HolderUnblacklistedEventData`
- [ ] Emit on add_holder_to_blacklist
- [ ] Emit on remove_holder_from_blacklist

## Progress Tracking
- Status: NOT_STARTED
