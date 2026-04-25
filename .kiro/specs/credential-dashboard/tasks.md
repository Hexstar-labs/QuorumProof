# Implementation Plan: Credential Dashboard

## Overview

Rewrite `Dashboard.tsx` with full `getSlice` support, decompose into `WalletGate`, `CredentialCard`, and `EmptyState` components, add `getSlice` to the contract clients, and cover all correctness properties with property-based and unit tests.

## Tasks

- [x] 1. Add `getSlice` to contract clients
  - [x] 1.1 Add `getSlice(sliceId)` to `frontend/src/stellar.ts`
    - Export `async function getSlice(sliceId)` using the existing `simulate` helper and `nativeToScVal(BigInt(sliceId), { type: 'u64' })`
    - Return type is the `QuorumSlice` plain object `{ id, creator, attestors, threshold }`
    - _Requirements: 4.1_
  - [x] 1.2 Add `getSlice(sliceId: bigint | number): Promise<QuorumSlice>` to `frontend/src/lib/contracts/quorumProof.ts`
    - Use the existing `simulate<QuorumSlice>` helper and `u64` encoder already in that file
    - _Requirements: 4.1_

- [x] 2. Create `WalletGate` component
  - [x] 2.1 Create `frontend/src/components/WalletGate.tsx`
    - Props: `{ hasFreighter: boolean; connect: () => Promise<void> }`
    - When `hasFreighter` is true: render a "Connect Wallet" button that calls `connect()`
    - When `hasFreighter` is false: render an install prompt with a link to `https://freighter.app`
    - _Requirements: 1.1, 1.2, 1.5_
  - [ ]* 2.2 Write unit tests for `WalletGate` in `frontend/src/__tests__/WalletGate.test.tsx`
    - Test: renders connect button when `hasFreighter: true`
    - Test: renders install prompt + freighter.app link when `hasFreighter: false`
    - _Requirements: 1.2, 1.5_

- [x] 3. Create `EmptyState` component
  - [x] 3.1 Create `frontend/src/components/EmptyState.tsx`
    - Props: `{ address: string }`
    - Render: icon, descriptive message, connected address display, "Request Credential Issuance" CTA button/link
    - Use `.empty-state` CSS class
    - _Requirements: 5.1, 5.2, 5.3, 5.4, 5.5_
  - [ ]* 3.2 Write property test P8 in `frontend/src/__tests__/EmptyState.test.tsx`
    - **Property 8: EmptyState displays connected address**
    - **Validates: Requirements 5.4**
    - Generate random address strings; assert the address appears in rendered output
    - `{ numRuns: 100 }`
  - [ ]* 3.3 Write unit tests for `EmptyState` in `frontend/src/__tests__/EmptyState.test.tsx`
    - Test: renders descriptive message (Req 5.2)
    - Test: renders "Request Credential Issuance" CTA (Req 5.3)
    - Test: visually distinct from loading/error states (Req 5.5)
    - _Requirements: 5.1, 5.2, 5.3_

- [x] 4. Implement pure utility functions and `deriveStatus`
  - [x] 4.1 Create `frontend/src/lib/credentialUtils.ts` with exported pure functions
    - `deriveStatus(revoked: boolean, expired: boolean, attested: boolean): AttestationStatus`
    - `formatAddress(addr: string): string` — `addr.slice(0,8) + '…' + addr.slice(-6)` for length ≥ 10
    - `attestorRole(index: number): string` — `ATTESTOR_ROLES[index] ?? \`Member ${index + 1}\``
    - `credTypeLabel(n: number | bigint): string`
    - `formatTimestamp(ts: number | bigint | null | undefined): string`
    - Export `AttestationStatus` type and `ATTESTOR_ROLES` constant
    - _Requirements: 3.1, 3.2, 3.3, 3.4, 4.2, 4.3_
  - [ ]* 4.2 Write property test P1 in `frontend/src/__tests__/deriveStatus.test.ts`
    - **Property 1: Status derivation correctness**
    - **Validates: Requirements 3.1, 3.2, 3.3, 3.4**
    - `fc.assert(fc.property(fc.boolean(), fc.boolean(), fc.boolean(), ...))`
    - Assert priority: revoked > expired > attested > pending
    - `{ numRuns: 100 }`
  - [ ]* 4.3 Write property test P5 (pure function part) in `frontend/src/__tests__/formatAddress.test.ts`
    - **Property 5: Attestor address truncation**
    - **Validates: Requirements 4.2**
    - Generate strings of length ≥ 10; assert result equals `s.slice(0,8) + '…' + s.slice(-6)`
    - `{ numRuns: 100 }`

- [x] 5. Create `CredentialCard` component
  - [x] 5.1 Create `frontend/src/components/CredentialCard.tsx`
    - Props: `{ data: CredCardData; sliceId: bigint | null }`
    - Import `CredCardData` type and utility functions from `credentialUtils.ts`
    - Header: credential type label + status badge with `aria-label` containing the status string
    - Body: truncated credential ID, truncated issuer with `title` attribute, metadata, optional expiry
    - Quorum slice section: if `data.sliceError` → "Slice unavailable"; if `data.slice` → attestor list with role labels, count, threshold; if no slice → "No slice data available"; if zero attestors → "No attestors assigned"
    - Each attestor element: truncated address + `title={fullAddress}` + role label
    - Footer: "View Public Page →" link to `/verify?credentialId=<id>`
    - If `data.credError` is set: render inline error panel instead of body
    - Use `.cred-card`, `.cred-card__header`, `.badge`, `.attestor-mini-list` CSS classes
    - _Requirements: 2.6, 2.7, 3.2, 3.3, 3.4, 3.5, 4.2, 4.3, 4.4, 4.5, 4.6_
  - [ ]* 5.2 Write property test P4 in `frontend/src/__tests__/CredentialCard.test.tsx`
    - **Property 4: Credential card required fields**
    - **Validates: Requirements 2.6, 2.7**
    - Generate random `Credential` objects; assert truncated ID, type label, truncated issuer present; when `expires_at` non-null assert expiry date present
    - `{ numRuns: 100 }`
  - [ ]* 5.3 Write property test P5 (DOM part) in `frontend/src/__tests__/CredentialCard.test.tsx`
    - **Property 5: Attestor address truncation with full address on hover**
    - **Validates: Requirements 4.2**
    - Generate random `QuorumSlice` objects; assert each attestor element has `title === fullAddress`
    - `{ numRuns: 100 }`
  - [ ]* 5.4 Write property test P6 in `frontend/src/__tests__/CredentialCard.test.tsx`
    - **Property 6: Slice section renders role labels, count, and threshold**
    - **Validates: Requirements 4.3, 4.4**
    - Generate random `QuorumSlice` objects; assert role labels, count, and threshold appear
    - `{ numRuns: 100 }`
  - [ ]* 5.5 Write property test P7 in `frontend/src/__tests__/CredentialCard.test.tsx`
    - **Property 7: Aria-label contains attestation status**
    - **Validates: Requirements 3.5**
    - Generate random `CredCardData` objects; assert status badge `aria-label` contains the status string
    - `{ numRuns: 100 }`
  - [ ]* 5.6 Write unit tests for `CredentialCard` in `frontend/src/__tests__/CredentialCard.test.tsx`
    - Test: renders "Slice unavailable" when `sliceError: true` (Req 4.5)
    - Test: renders "No attestors assigned" when slice has zero attestors (Req 4.6)
    - Test: renders inline error panel when `credError` is set (Req 2.5)
    - _Requirements: 2.5, 4.5, 4.6_

- [ ] 6. Checkpoint — Ensure all component tests pass
  - Ensure all tests pass, ask the user if questions arise.

- [x] 7. Rewrite `Dashboard.tsx`
  - [x] 7.1 Rewrite `frontend/src/pages/Dashboard.tsx`
    - Import `useFreighter`, `WalletGate`, `CredentialCard`, `EmptyState` components
    - Import `getCredentialsBySubject`, `getCredential`, `isAttested`, `getAttestors`, `getSlice`, `isExpired` from `stellar.ts`
    - Import `deriveStatus` and `CredCardData` from `credentialUtils.ts`
    - On mount: read `sliceId` from `localStorage.getItem('qp-slice-id')`
    - On `address` change: fetch all credential IDs via `getCredentialsBySubject(address)`, then fetch each card's data in parallel using `Promise.allSettled` or per-card try/catch
    - Per-card fetch: call `getCredential(id)`, `isExpired(id)`, and either `isAttested(id, sliceId)` + `getSlice(sliceId)` (when sliceId known) or `getAttestors(id)` fallback; catch `isAttested` failures → log + default `attested: false`; catch `getSlice` failures → `slice: null, sliceError: true`
    - Render states: `isInitializing` → loading spinner; `!address && !isInitializing` → `<WalletGate>`; `loading` → loading spinner; `error` → error card with retry button; `cards.length === 0` → `<EmptyState address={address} />`; otherwise → `dashboard-grid` of `<CredentialCard>`s
    - _Requirements: 1.1, 1.2, 1.3, 1.4, 2.1, 2.2, 2.3, 2.4, 2.5, 3.6, 4.1_
  - [ ]* 7.2 Write property test P2 in `frontend/src/__tests__/Dashboard.test.tsx`
    - **Property 2: Card count matches credential ID count**
    - **Validates: Requirements 2.2**
    - Generate random arrays of bigint IDs; mock `getCredential` to resolve for each; assert rendered card count equals ID array length
    - `{ numRuns: 100 }`
  - [ ]* 7.3 Write property test P3 in `frontend/src/__tests__/Dashboard.test.tsx`
    - **Property 3: Partial failure resilience**
    - **Validates: Requirements 2.5**
    - Generate random ID arrays where a random subset throws on `getCredential`; assert successfully fetched cards render normally and failed cards show inline errors
    - `{ numRuns: 100 }`
  - [ ]* 7.4 Write unit tests for `Dashboard` in `frontend/src/__tests__/Dashboard.test.tsx`
    - Test: shows loading indicator when `isInitializing: true` (Req 1.3)
    - Test: shows `WalletGate` when `address: null, isInitializing: false` (Req 1.1)
    - Test: triggers fetch on address change without page reload (Req 1.4)
    - Test: shows loading spinner while fetching credentials (Req 2.3)
    - Test: shows error card with retry when `getCredentialsBySubject` throws (Req 2.4)
    - Test: renders `EmptyState` when credential list is empty (Req 5.1)
    - Test: calls `getSlice` when sliceId present in localStorage (Req 4.1)
    - Test: calls `getCredentialsBySubject` with connected address (Req 2.1)
    - Test: `is_attested` failure defaults to Pending and logs to console (Req 3.6)
    - _Requirements: 1.1, 1.3, 1.4, 2.1, 2.3, 2.4, 4.1, 5.1_

- [x] 8. Update component exports
  - [x] 8.1 Update `frontend/src/components/index.ts` to export `WalletGate`, `CredentialCard`, and `EmptyState`
    - _Requirements: 1.1, 2.2, 5.1_

- [x] 9. Verify `App.tsx` routing
  - [x] 9.1 Confirm `/dashboard` route in `frontend/src/App.tsx` points to the rewritten `Dashboard` component
    - No changes expected; verify the import path and route definition are correct
    - _Requirements: 1.1_

- [ ] 10. Final checkpoint — Ensure all tests pass
  - Ensure all tests pass, ask the user if questions arise.

## Notes

- Tasks marked with `*` are optional and can be skipped for faster MVP
- `deriveStatus` priority: `revoked` > `expired` > `attested` > `pending`
- Role labels are UI-layer only: `['Lead Verifier', 'Co-Verifier', 'Auditor', 'Reviewer', 'Observer']`
- `isAttested` requires both `credentialId` and `sliceId` — fall back to `getAttestors(credId).length > 0` when no sliceId in localStorage
- Slice ID stored in localStorage under key `qp-slice-id`
- PBT library: fast-check (already in devDependencies); test runner: vitest (already configured)
- Each property test must include comment: `// Feature: credential-dashboard, Property N: <property_text>`
- Each property test must set `{ numRuns: 100 }` explicitly
