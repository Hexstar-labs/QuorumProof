# Implementation Plan: Public Credential Verification Page

## Overview

Verify.tsx is already substantially implemented. These tasks cover the remaining fixes and additions: migrating imports from the legacy `stellar.ts` to typed contract clients, correcting validation and ZK dropdown bugs, adding accessibility attributes, implementing graceful degradation for attestor errors, adding the ZK privacy note, auditing CSS coverage, and writing the full property-based and unit test suite.

## Tasks

- [ ] 1. Migrate imports from `../stellar` to typed contract clients
  - Replace all imports from `../stellar` with named imports from `../lib/contracts/quorumProof` and `../lib/contracts/zkVerifier`
  - Remove the local `hexToBytes` helper — `zkVerifier.verifyClaim` accepts a hex string directly
  - Replace `decodeMetadataHash` with the inline `credentialUtils.ts` equivalent or a local helper
  - Replace `CONTRACT_ID`, `RPC_URL`, `NETWORK` imports with `import.meta.env` reads matching the pattern in `quorumProof.ts`
  - Update `handleZkVerify` to call `verifyClaim(credential.id, zkClaimType as ClaimType, zkProof)` — no manual hex decoding needed
  - _Requirements: 2.2, 2.3, 2.4, 8.4_

  - [ ]* 1.1 Write unit test confirming `verifyClaim` is called with a `ClaimType` value (not a plain string)
    - Mock `zkVerifier.verifyClaim` and assert the second argument is one of `HasDegree | HasLicense | HasEmploymentHistory`
    - _Requirements: 8.4_

- [ ] 2. Fix ZK claim type dropdown — exactly 3 options
  - Replace the current 5-option `<select>` (including `license_valid`, `employer_verified`, `certification_active`, `custom`) with exactly 3 options
  - Option values must be the `ClaimType` literals: `HasDegree`, `HasLicense`, `HasEmploymentHistory`
  - Display labels: `🎓 Degree`, `🏛️ License`, `💼 Employment History`
  - Remove the conditional "Custom claim type" input row entirely
  - Update `zkClaimType` state type to `ClaimType` (imported from `../lib/contracts/zkVerifier`)
  - _Requirements: 8.2_

  - [ ]* 2.1 Write unit test asserting the ZK dropdown has exactly 3 options with correct values
    - Render `CredentialResult` with a mock credential and assert `select` has options `HasDegree`, `HasLicense`, `HasEmploymentHistory` only
    - _Requirements: 8.2_

- [ ] 3. Fix Stellar address validation — `addr.length !== 56`
  - In `handleVerifyAddr`, change `addr.length < 56` to `addr.length !== 56`
  - _Requirements: 3.5_

  - [ ]* 3.1 Write property test for invalid Stellar address rejection (Property 4)
    - `// Feature: public-verify-page, Property 4: Invalid Stellar address is rejected without contract calls`
    - Generator: `fc.oneof(fc.string().filter(s => !s.startsWith('G')), fc.string({ minLength: 57 }).map(s => 'G' + s), fc.string({ maxLength: 54 }).map(s => 'G' + s))`
    - Assert: validation returns false for all generated inputs
    - **Property 4: Invalid Stellar address is rejected without contract calls**
    - **Validates: Requirements 3.5**

  - [ ]* 3.2 Write property test for valid Stellar address acceptance (Property 3)
    - `// Feature: public-verify-page, Property 3: Valid Stellar address triggers subject lookup`
    - Generator: `fc.string({ minLength: 55, maxLength: 55 }).map(s => 'G' + s)`
    - Assert: validation returns true for all generated inputs
    - **Property 3: Valid Stellar address triggers subject lookup and displays results**
    - **Validates: Requirements 3.2, 3.3**

- [ ] 4. Add `aria-label` to the status banner element
  - In `CredentialResult`, add `aria-label={statusTitle}` to the `<div className={`status-banner status-banner--${statusClass}`}>` element
  - _Requirements: 5.6_

  - [ ]* 4.1 Write property test for status banner aria-label (Property 7)
    - `// Feature: public-verify-page, Property 7: Status banner always has aria-label matching the status title`
    - Generator: `fc.record({ revoked: fc.boolean(), expired: fc.boolean(), attestorCount: fc.nat() })`
    - Render `CredentialResult` with derived state and assert `status-banner` element has `aria-label` equal to the displayed title text
    - **Property 7: Status banner always has aria-label matching the status title**
    - **Validates: Requirements 5.6**

- [ ] 5. Add `attestorsError` state and graceful degradation when `getAttestors` fails
  - Add `attestorsError: boolean` to the `CredLookupResult` interface in `Verify.tsx`
  - In `fetchCred`, change `getAttestors(id)` to `getAttestors(id).catch((e) => { console.error(e); return null; })` and set `attestorsError = result === null`
  - Pass `attestorsError` as a prop to `CredentialResult`
  - In the attestor section of `CredentialResult`, when `attestorsError` is true, render `"Attestor data unavailable"` instead of the attestor list
  - The status banner and metadata card must still render when `attestorsError` is true
  - _Requirements: 10.4_

  - [ ]* 5.1 Write property test for attestors failure graceful degradation (Property 15)
    - `// Feature: public-verify-page, Property 15: get_attestors failure degrades gracefully without blocking credential display`
    - Mock `getAttestors` to throw, mock `getCredential` to return a valid credential
    - Assert: metadata card renders, attestor section shows "Attestor data unavailable"
    - **Property 15: get_attestors failure degrades gracefully without blocking credential display**
    - **Validates: Requirements 10.4**

- [ ] 6. Add ZK privacy tooltip/note
  - In the ZK form body, add a `<p>` or `<span>` element with the text: `"ZK verification confirms a claim without revealing the underlying credential data."`
  - Place it below the claim type dropdown and above the proof textarea
  - Style with `style={{ fontSize: 12, color: 'var(--text-muted)' }}` or equivalent muted class
  - _Requirements: 8.6, 9.6_

  - [ ]* 6.1 Write unit test asserting the ZK privacy note is present
    - Render `CredentialResult` and assert the privacy note text is in the document
    - _Requirements: 8.6_

- [ ] 7. Audit and add missing CSS classes in `frontend/src/styles.css`
  - Cross-reference every CSS class used in `Verify.tsx` against the class inventory in the design doc
  - Add any missing class definitions to `frontend/src/styles.css`
  - Classes to verify are present: all classes listed in the design doc CSS inventory (layout, inputs, buttons, status, cards, attestors, ZK, badges, states, lists, share, forms)
  - _Requirements: 1.3, 1.4_

- [ ] 8. Write property-based tests for correctness properties P1, P2, P5, P6, P8, P9, P10, P11, P12, P13, P14
  - Add tests to `frontend/src/__tests__/Verify.pbt.test.tsx` (new file, TypeScript)
  - Use `fast-check` with `numRuns: 100` minimum per property
  - Each test must include the comment: `// Feature: public-verify-page, Property N: <property_text>`

  - [ ]* 8.1 Write property test P1: Valid credential ID triggers all contract calls
    - `// Feature: public-verify-page, Property 1: Valid credential ID triggers all contract calls`
    - Generator: `fc.bigInt({ min: 1n })`
    - Mock `getCredential`, `getAttestors`, `isExpired`; submit ID; assert all three mocks called and Result_Panel rendered
    - **Property 1: Valid credential ID triggers all contract calls**
    - **Validates: Requirements 2.2, 2.3, 2.4**

  - [ ]* 8.2 Write property test P2: Invalid credential ID is rejected without contract calls
    - `// Feature: public-verify-page, Property 2: Invalid credential ID is rejected without contract calls`
    - Generator: `fc.oneof(fc.constant('0'), fc.integer({ max: 0 }).map(String), fc.constant(''), fc.string().filter(s => isNaN(Number(s))))`
    - Assert: error message shown, no contract mocks called
    - **Property 2: Invalid credential ID is rejected without contract calls**
    - **Validates: Requirements 2.5**

  - [ ]* 8.3 Write property test P5: Result_Panel displays all required credential metadata fields
    - `// Feature: public-verify-page, Property 5: Result_Panel displays all required credential metadata fields`
    - Generator: `fc.record({ id: fc.bigInt({ min: 1n }), subject: fc.string({ minLength: 56, maxLength: 56 }).map(s => 'G' + s.slice(1)), issuer: fc.string({ minLength: 56, maxLength: 56 }).map(s => 'G' + s.slice(1)), credential_type: fc.integer({ min: 1, max: 5 }), expires_at: fc.option(fc.bigInt({ min: 1n })) })`
    - Assert: credential ID, type label, subject, issuer, and expiration all present in rendered output
    - **Property 5: Result_Panel displays all required credential metadata fields**
    - **Validates: Requirements 4.1, 4.2, 4.3, 4.4, 4.5, 4.6**

  - [ ]* 8.4 Write property test P6: Attestation status derivation is correct for all input combinations
    - `// Feature: public-verify-page, Property 6: Attestation status derivation is correct for all input combinations`
    - Generator: `fc.record({ revoked: fc.boolean(), expired: fc.boolean(), attestorCount: fc.nat() })`
    - Assert correct status text per the priority rules: revoked → "Credential Revoked", expired (not revoked) → "Credential Expired", attestorCount > 0 → "Credential Verified", else → "Awaiting Attestation"
    - **Property 6: Attestation status derivation is correct for all input combinations**
    - **Validates: Requirements 5.2, 5.3, 5.4, 5.5**

  - [ ]* 8.5 Write property test P8: Attestor list renders all addresses with correct count and badges
    - `// Feature: public-verify-page, Property 8: Attestor list renders all addresses with correct count and badges`
    - Generator: `fc.array(fc.string({ minLength: 56, maxLength: 56 }).map(s => 'G' + s.slice(1)), { minLength: 1, maxLength: 10 })`
    - Assert: each address appears in full, each has a "✓ Signed" badge, count badge equals array length
    - **Property 8: Attestor list renders all addresses with correct count and badges**
    - **Validates: Requirements 6.1, 6.2, 6.3**

  - [ ]* 8.6 Write property test P9: Shareable URL always encodes the current credential ID
    - `// Feature: public-verify-page, Property 9: Shareable URL always encodes the current credential ID`
    - Generator: `fc.bigInt({ min: 1n, max: BigInt(Number.MAX_SAFE_INTEGER) })`
    - Render `CredentialResult` with the generated ID; assert share bar URL contains `credentialId=<id>`
    - **Property 9: Shareable URL always encodes the current credential ID**
    - **Validates: Requirements 7.3**

  - [ ]* 8.7 Write property test P10: Query param auto-triggers lookup for any valid credential ID
    - `// Feature: public-verify-page, Property 10: Query param auto-triggers lookup for any valid credential ID`
    - Generator: `fc.bigInt({ min: 1n, max: BigInt(Number.MAX_SAFE_INTEGER) })`
    - Render `Verify` with `?credentialId=<id>` in the URL; assert `getCredential` mock called with that ID without user interaction
    - **Property 10: Query param auto-triggers lookup for any valid credential ID**
    - **Validates: Requirements 7.2**

  - [ ]* 8.8 Write property test P11: ZK form calls verifyClaim with correct typed arguments
    - `// Feature: public-verify-page, Property 11: ZK form calls verifyClaim with correct typed arguments`
    - Generator: `fc.tuple(fc.constantFrom('HasDegree', 'HasLicense', 'HasEmploymentHistory'), fc.hexaString({ minLength: 2 }))`
    - Select claim type, enter proof, submit; assert `verifyClaim` called with `(credential.id, claimType, proofHex)`
    - **Property 11: ZK form calls verifyClaim with correct typed arguments**
    - **Validates: Requirements 8.4**

  - [ ]* 8.9 Write property test P12: Empty ZK proof is rejected without calling verifyClaim
    - `// Feature: public-verify-page, Property 12: Empty ZK proof is rejected without calling verifyClaim`
    - Generator: `fc.stringOf(fc.constantFrom(' ', '\t', '\n', '\r'))`
    - Enter whitespace-only proof, submit; assert error shown and `verifyClaim` not called
    - **Property 12: Empty ZK proof is rejected without calling verifyClaim**
    - **Validates: Requirements 8.5**

  - [ ]* 8.10 Write property test P13: ZK result shows exactly one state at a time
    - `// Feature: public-verify-page, Property 13: ZK result shows exactly one state at a time`
    - Generator: `fc.oneof(fc.constant({ outcome: 'true' }), fc.constant({ outcome: 'false' }), fc.string().map(msg => ({ outcome: 'error', msg })))`
    - Mock `verifyClaim` to return true/false/throw; assert exactly one of `.zk-result--success`, `.zk-result--fail`, `.zk-result--error` is present
    - **Property 13: ZK result shows exactly one state at a time**
    - **Validates: Requirements 9.3, 9.5**

  - [ ]* 8.11 Write property test P14: get_credential failure shows error card and suppresses Result_Panel
    - `// Feature: public-verify-page, Property 14: get_credential failure shows error card and suppresses Result_Panel`
    - Generator: `fc.string({ minLength: 1 })` (error message)
    - Mock `getCredential` to throw with the generated message; submit a valid ID; assert error card contains the message and `.result-section` is absent
    - **Property 14: get_credential failure shows error card and suppresses Result_Panel**
    - **Validates: Requirements 10.1, 10.2**

- [ ] 9. Write unit tests for specific edge cases
  - Add to `frontend/src/__tests__/Verify.unit.test.tsx` (new file, TypeScript)

  - [ ]* 9.1 Assert no WalletGate component is rendered on the Verify page
    - Render `<Verify />` and assert `WalletGate` is not in the tree
    - _Requirements: 1.2_

  - [ ]* 9.2 Assert search card is immediately visible without authentication
    - Render `<Verify />` and assert `.search-card` is present in the document
    - _Requirements: 1.3_

  - [ ]* 9.3 Assert `formatTimestamp(null)` returns `"Never"`
    - Import `formatTimestamp` from `credentialUtils.ts` and assert `formatTimestamp(null) === 'Never'`
    - _Requirements: 4.6_

  - [ ]* 9.4 Assert empty attestor list shows the "no attestors" message
    - Render `CredentialResult` with `attestors=[]` and assert "No attestors have signed this credential yet." is present
    - _Requirements: 6.4_

  - [ ]* 9.5 Assert address lookup returning `[]` shows the empty state
    - Mock `getCredentialsBySubject` to return `[]`; submit a valid address; assert empty state message is shown
    - _Requirements: 3.4_

  - [ ]* 9.6 Assert `verify_claim` returning `true` shows "✅ Claim Verified"
    - Mock `verifyClaim` to return `true`; submit ZK form; assert "✅ Claim Verified" text is present
    - _Requirements: 9.1_

  - [ ]* 9.7 Assert `verify_claim` returning `false` shows "❌ Claim Not Verified"
    - Mock `verifyClaim` to return `false`; submit ZK form; assert "❌ Claim Not Verified" text is present
    - _Requirements: 9.2_

  - [ ]* 9.8 Assert loading state disables the submit button
    - Trigger a credential lookup and assert the verify button has `disabled` attribute while loading
    - _Requirements: 10.6_

  - [ ]* 9.9 Assert clipboard copy writes the correct shareable URL
    - Mock `navigator.clipboard.writeText`; click the copy button; assert it was called with `<origin>/verify?credentialId=<id>`
    - _Requirements: 7.4_

- [ ] 10. Final checkpoint — Ensure all tests pass
  - Run `vitest --run` from the `frontend/` directory and confirm all tests pass. Ask the user if any questions arise.
