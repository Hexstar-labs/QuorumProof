# Pull Request: Fix #21 - Add Issuer Revocation Support

## 🎯 Issue
Fixes #21 - Only the credential subject could revoke their own credential. Universities or licensing bodies (issuers) had no mechanism to revoke fraudulent credentials they issued.

## 📝 Description
This PR implements issuer-initiated credential revocation, allowing both the credential subject and the issuer to revoke credentials. This is critical for real-world use cases where institutions need to revoke fraudulent or invalid credentials.

## 🔧 Changes Made

### 1. Enhanced `revoke_credential()` Function
- **Added proper authentication**: Changed from `env.invoker()` to explicit `caller: Address` parameter with `caller.require_auth()`
- **Dual authorization**: Both credential subject AND issuer can now revoke credentials
- **Better error handling**: Clear error message "only subject or issuer can revoke" for unauthorized attempts
- **Security improvement**: Follows Soroban best practices for authentication

**Before:**
```rust
pub fn revoke_credential(env: Env, credential_id: u64) {
    let caller = env.invoker();
    // ...
}
```

**After:**
```rust
pub fn revoke_credential(env: Env, caller: Address, credential_id: u64) {
    caller.require_auth();
    // ...
    assert!(
        caller == credential.subject || caller == credential.issuer,
        "only subject or issuer can revoke"
    );
}
```

### 2. Comprehensive Test Coverage
Added three new test cases:

- ✅ **`test_issuer_revoke_credential`**: Verifies issuer can revoke credentials they issued
- ✅ **`test_subject_revoke_credential`**: Verifies subject can still revoke their own credentials
- ✅ **`test_unauthorized_revoke_credential`**: Verifies unauthorized parties cannot revoke (should panic)

### 3. Documentation
- Updated function documentation to clarify dual authorization
- Added `ISSUE_21_FIX_SUMMARY.md` with detailed implementation notes

## ✅ Testing

### Test Results
All tests pass successfully:
```
running 5 tests
test tests::test_subject_revoke_credential ... ok
test tests::test_issue_and_get_credential ... ok
test tests::test_issuer_revoke_credential ... ok
test tests::test_quorum_slice_and_attestation ... ok
test tests::test_unauthorized_revoke_credential - should panic ... ok

test result: ok. 5 passed; 0 failed; 0 ignored
```

### Build Status
✅ All contracts build successfully:
- `quorum_proof.wasm` - 12,831 bytes
- `sbt_registry.wasm` - 7,617 bytes  
- `zk_verifier.wasm` - 4,848 bytes

## 📋 Checklist
- [x] Code follows project style guidelines
- [x] All tests pass
- [x] New tests added for new functionality
- [x] Contract builds successfully to WASM
- [x] Documentation updated
- [x] Security best practices followed
- [x] Breaking changes documented (function signature changed)

## 🔒 Security Considerations
- Uses proper authentication with `require_auth()`
- Implements principle of least privilege (only subject or issuer)
- Clear error messages prevent information leakage
- Comprehensive test coverage ensures security guarantees

## 🚨 Breaking Changes
⚠️ **Function Signature Change**: The `revoke_credential` function now requires a `caller` parameter:
```rust
// Old
revoke_credential(env, credential_id)

// New  
revoke_credential(env, caller, credential_id)
```

Clients calling this function will need to update their code to pass the caller address.

## 📦 Files Changed
- `contracts/quorum_proof/src/lib.rs` - Core implementation
- `contracts/quorum_proof/test_snapshots/tests/*.json` - Test snapshots
- `ISSUE_21_FIX_SUMMARY.md` - Implementation documentation

## 🎓 Use Cases Enabled
1. **University Credential Revocation**: Universities can revoke degrees if fraud is discovered
2. **License Revocation**: Licensing bodies can revoke professional licenses for misconduct
3. **Certificate Invalidation**: Issuers can invalidate certificates that were issued in error
4. **Self-Revocation**: Users can still revoke their own credentials if needed

## 🔍 Review Focus Areas
- Security of the authorization logic
- Test coverage completeness
- Breaking change impact on existing integrations
- Error message clarity

---

**Branch**: `fix/issue-21-issuer-revocation`  
**Base**: `main`  
**Category**: Smart Contract - Bug Fix  
**Priority**: High  
**Estimated Time**: 1 hour (as specified in issue)
