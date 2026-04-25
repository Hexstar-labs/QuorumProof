# Design Document: Public Credential Verification Page

## Overview

The Public Credential Verification Page (`/verify`) is a read-only, wallet-free React page that lets anyone verify engineering credentials stored on the Stellar Soroban blockchain. It is already substantially implemented in `frontend/src/pages/Verify.tsx`; this document describes the complete intended design, identifies what needs to be fixed, and defines correctness properties for property-based testing.

The page operates entirely through Soroban RPC *simulation* — no transaction signing, no wallet, no auth. All contract calls use a randomly-generated ephemeral keypair as the dummy source account, which is the standard pattern for read-only Soroban calls.

### Key Design Decisions

- **No WalletGate**: The route is registered directly in `App.tsx` without wrapping in `WalletGate`. This is intentional and must not change.
- **is_attested proxy**: `is_attested(credId, sliceId)` requires a `sliceId` that is not publicly discoverable from the credential alone. The proxy is `getAttestors(credId).length > 0`, which is semantically equivalent for the "has any attestation" check and is already the pattern used in the existing implementation.
- **Typed contract clients over legacy stellar.ts**: `Verify.tsx` currently imports from `../stellar` (untyped JS). The design calls for migrating to `../lib/contracts/quorumProof` and `../lib/contracts/zkVerifier` for type safety and correct ScVal encoding.
- **Exactly 3 ZK claim types**: The ZK Verifier contract's `ClaimType` enum has exactly three variants. The UI dropdown must match this exactly.

---

## Architecture

```
App.tsx
  └── <Route path="/verify" element={<Verify />} />   ← no WalletGate

Verify (page component)
  ├── Navbar                          (existing shared component)
  ├── Hero section                    (title + subtitle)
  ├── SearchCard                      (tab-based input)
  │     ├── CredentialIdTab           (numeric input + verify button)
  │     └── StellarAddressTab         (text input + look-up button)
  ├── ResultsArea
  │     ├── LoadingState              (spinner)
  │     ├── ErrorCard                 (on contract call failure)
  │     ├── EmptyState                (address lookup → 0 results)
  │     ├── CredentialList            (address lookup → N results)
  │     └── CredentialResult          (full result panel)
  │           ├── StatusBanner        (Verified / Pending / Revoked / Expired)
  │           ├── ShareBar            (URL display + copy button)
  │           ├── MetadataCard        (credential fields grid)
  │           ├── AttestorList        (attestor addresses + badges)
  │           └── ZkClaimForm         (claim type dropdown + proof textarea + result)
  └── Footer

Contract layer (frontend/src/lib/contracts/)
  ├── quorumProof.ts   → getCredential, getAttestors, getCredentialsBySubject, isExpired
  └── zkVerifier.ts    → verifyClaim(credId, ClaimType, proof)
```

The page does **not** use `useContractClient` hook — it calls the contract module functions directly, which is simpler for a page-level component with its own loading/error state.

---

## Components and Interfaces

### Verify (page)

The top-level page component. Owns all top-level state and orchestrates contract calls.

```typescript
// State
activeTab: 'id' | 'addr'
credInput: string          // raw text in the credential ID field
addrInput: string          // raw text in the address field
loading: boolean
error: string | null
result: CredLookupResult | null
addrResults: bigint[] | null

// Internal types
interface CredLookupResult {
  credential: Credential;   // from quorumProof.ts
  attestors: string[];
  expired: boolean;
  attestorsError: boolean;  // true if getAttestors threw
}
```

### CredentialResult

Receives a `CredLookupResult` and renders the full result panel. Owns ZK form state.

```typescript
interface CredentialResultProps {
  credential: Credential;
  attestors: string[];
  expired: boolean;
  attestorsError: boolean;
}

// ZK form state (local)
zkClaimType: ClaimType        // 'HasDegree' | 'HasLicense' | 'HasEmploymentHistory'
zkProof: string               // raw hex input
zkResult: ZkResultState | null
zkLoading: boolean

type ZkResultState =
  | { type: 'success' }
  | { type: 'fail' }
  | { type: 'error'; message: string }
```

### StatusBanner

Renders the attestation status with appropriate styling and `aria-label`.

```typescript
interface StatusBannerProps {
  status: 'valid' | 'revoked' | 'expired' | 'pending';
  title: string;
  subtitle: string;
}
// Renders: <div className={`status-banner status-banner--${status}`} aria-label={title}>
```

### AttestorList

Renders the list of attestor addresses or a fallback message.

```typescript
interface AttestorListProps {
  attestors: string[];
  error: boolean;   // if true, show "Attestor data unavailable"
}
```

### ZkClaimForm

The ZK verification sub-form. Calls `verifyClaim` from `zkVerifier.ts`.

```typescript
interface ZkClaimFormProps {
  credentialId: bigint;
}
```

### ShareBar

Displays the shareable URL and a clipboard copy button.

```typescript
interface ShareBarProps {
  credentialId: bigint;
}
// shareUrl = `${window.location.origin}/verify?credentialId=${credentialId}`
```

---

## Data Models

### Credential (from quorumProof.ts)

```typescript
interface Credential {
  id: bigint;
  subject: string;          // Stellar address (G...)
  issuer: string;           // Stellar address (G...)
  credential_type: number;  // 1–5
  metadata_hash: Uint8Array;
  revoked: boolean;
  expires_at: bigint | null; // Unix timestamp in seconds, or null
}
```

### ClaimType (from zkVerifier.ts)

```typescript
type ClaimType = 'HasDegree' | 'HasLicense' | 'HasEmploymentHistory';
```

### UI label mapping

| ClaimType              | Dropdown label              |
|------------------------|-----------------------------|
| `HasDegree`            | 🎓 Degree                   |
| `HasLicense`           | 🏛️ License                  |
| `HasEmploymentHistory` | 💼 Employment History       |

### Credential type label mapping (from credentialUtils.ts)

| credential_type | Label              |
|-----------------|--------------------|
| 1               | 🎓 Degree          |
| 2               | 🏛️ License         |
| 3               | 💼 Employment      |
| 4               | 📜 Certification   |
| 5               | 🔬 Research        |

### Attestation status derivation

Priority order (highest wins):

```
revoked=true                          → 'revoked'  ("Credential Revoked")
expired=true (and not revoked)        → 'expired'  ("Credential Expired")
attestors.length > 0 (and not above)  → 'valid'    ("Credential Verified")
otherwise                             → 'pending'  ("Awaiting Attestation")
```

This logic lives in `credentialUtils.ts` as `deriveStatus(revoked, expired, attested)` where `attested = attestors.length > 0`.

---

## Data Flow

### Credential ID lookup

```
User types ID → handleVerifyId()
  → validate: parseInt(credInput) > 0
  → if invalid: setError("Please enter a valid credential ID (positive integer)")
  → if valid:
      setLoading(true)
      setSearchParams({ credentialId: id.toString() })
      Promise.all([
        getCredential(id),                    // quorumProof.ts
        getAttestors(id).catch(e => null),    // quorumProof.ts — graceful degrade
        isExpired(id).catch(() => false),     // quorumProof.ts — graceful degrade
      ])
      → on success: setResult({ credential, attestors, expired, attestorsError })
      → on getCredential failure: setError(err.message), no Result_Panel
      setLoading(false)
```

### Stellar address lookup

```
User types address → handleVerifyAddr()
  → validate: addr.startsWith('G') && addr.length === 56
  → if invalid: setError("Please enter a valid Stellar address.")
  → if valid:
      setLoading(true)
      getCredentialsBySubject(addr)           // quorumProof.ts
      → on success: setAddrResults(ids)
      → on failure: setError(err.message)
      setLoading(false)
```

### Auto-trigger from query param

```
useEffect (on mount only):
  const preId = searchParams.get('credentialId')
  if (preId && parseInt(preId) > 0):
    fetchCred(BigInt(preId))
```

### ZK claim verification

```
User selects ClaimType, pastes proof hex → handleZkVerify()
  → validate: zkProof.trim() !== ''
  → if empty: setZkResult({ type: 'error', message: '⚠️ Please paste proof bytes.' })
  → if valid:
      setZkLoading(true)
      verifyClaim(credential.id, zkClaimType, zkProof)   // zkVerifier.ts
      → true:  setZkResult({ type: 'success' })
      → false: setZkResult({ type: 'fail' })
      → throws: setZkResult({ type: 'error', message: err.message })
      setZkLoading(false)
```

---

## Contract Integration

### QuorumProof contract (`frontend/src/lib/contracts/quorumProof.ts`)

| Method | Called when | Args | Returns |
|---|---|---|---|
| `getCredential(id)` | ID lookup | `bigint` | `Credential` |
| `getAttestors(id)` | ID lookup (parallel) | `bigint` | `string[]` |
| `isExpired(id)` | ID lookup (parallel) | `bigint` | `boolean` |
| `getCredentialsBySubject(addr)` | Address lookup | `string` | `bigint[]` |

`isAttested` is **not called directly** — the proxy `getAttestors(id).length > 0` is used instead, because `isAttested` requires a `sliceId` that is not publicly discoverable from the credential alone.

### ZK Verifier contract (`frontend/src/lib/contracts/zkVerifier.ts`)

| Method | Called when | Args | Returns |
|---|---|---|---|
| `verifyClaim(credId, claimType, proof)` | ZK form submit | `bigint`, `ClaimType`, `string` | `boolean` |

`zkVerifier.ts` handles hex decoding internally via its `hexToBytes` helper, so `Verify.tsx` can pass the raw hex string directly.

### Migration from stellar.ts

`Verify.tsx` currently imports from `../stellar` (legacy untyped JS). The required migration:

| Current import (stellar.ts) | Replace with |
|---|---|
| `getCredential` | `quorumProof.getCredential` from `../lib/contracts/quorumProof` |
| `getAttestors` | `quorumProof.getAttestors` |
| `isExpired` | `quorumProof.isExpired` |
| `getCredentialsBySubject` | `quorumProof.getCredentialsBySubject` |
| `verifyClaim` | `zkVerifier.verifyClaim` from `../lib/contracts/zkVerifier` |
| `decodeMetadataHash` | inline or move to `credentialUtils.ts` |
| `CONTRACT_ID`, `RPC_URL`, `NETWORK` | read from `import.meta.env` directly |

The critical fix is `verifyClaim`: `stellar.ts` encodes `claimType` as a plain string (`nativeToScVal(claimType, { type: 'string' })`), but the on-chain contract expects a `scvVec([scvSymbol(claimType)])`. `zkVerifier.ts` already encodes this correctly via `claimTypeToScVal()`.

---

## Shareable URL Pattern

Uses React Router's `useSearchParams`:

```typescript
const [searchParams, setSearchParams] = useSearchParams();

// Read on mount
const preId = searchParams.get('credentialId');

// Write after successful lookup
setSearchParams({ credentialId: id.toString() });

// Display in ShareBar
const shareUrl = `${window.location.origin}/verify?credentialId=${credential.id}`;
```

---

## Validation Rules

| Input | Valid condition | Error message |
|---|---|---|
| Credential ID | `parseInt(value) > 0` (positive integer) | "Please enter a valid credential ID (positive integer)." |
| Stellar Address | `value.startsWith('G') && value.length === 56` | "Please enter a valid Stellar address." |
| ZK Proof | `value.trim().length > 0` | "⚠️ Please paste proof bytes." |

Note: the current implementation uses `addr.length < 56` which is incorrect per R3.5. The fix is `addr.length !== 56`.

---

## CSS Class Inventory

All classes used in `Verify.tsx` must be defined in `frontend/src/styles.css` or `frontend/src/index.css`:

**Layout**: `verify-hero`, `verify-hero__eyebrow`, `verify-hero__title`, `verify-hero__subtitle`, `search-card`, `search-card__label`, `search-card__tabs`, `result-section`

**Inputs**: `tab-btn`, `tab-btn.active`, `input-wrap`, `input-icon`, `input-group`

**Buttons**: `btn`, `btn--primary`, `btn--ghost`, `btn--sm`

**Status**: `status-banner`, `status-banner--valid`, `status-banner--revoked`, `status-banner--expired`, `status-banner--pending`, `status-banner__icon`, `status-banner__title`, `status-banner__sub`

**Cards**: `detail-card`, `detail-card__header`, `detail-card__title`, `detail-card__body`, `meta-grid`, `meta-item`, `meta-item__label`, `meta-item__value`, `meta-item__value--mono`

**Attestors**: `attestor-list`, `attestor-item`, `attestor-item__avatar`, `attestor-item__addr`, `attestor-item__badge`

**ZK**: `zk-card`, `zk-card__header`, `zk-card__icon`, `zk-card__title`, `zk-card__sub`, `zk-card__body`, `zk-result`, `zk-result--success`, `zk-result--fail`, `zk-result--error`

**Badges**: `badge`, `badge--green`, `badge--red`, `badge--gray`, `badge--blue`

**States**: `loading-state`, `spinner`, `error-card`, `error-card__icon`, `error-card__title`, `error-card__msg`, `empty-state`, `empty-state__icon`, `empty-state__title`

**Lists**: `cred-list`, `cred-list-item`, `cred-list-item__id`

**Share**: `share-bar`, `share-bar__url`

**Forms**: `form-row`, `form-label`

---

## Error Handling Strategy

| Failure | Behavior |
|---|---|
| `getCredential` throws | `setError(err.message)`, no Result_Panel rendered |
| `getAttestors` throws | `attestorsError = true`, show "Attestor data unavailable" in attestor section; rest of panel renders normally |
| `isExpired` throws | Default `expired = false` (`.catch(() => false)`), log to console |
| `getCredentialsBySubject` throws | `setError(err.message)`, no list rendered |
| `verifyClaim` throws | `setZkResult({ type: 'error', message: err.message })` |
| Contract not configured | `getContractId()` throws "VITE_CONTRACT_* is not set" — surfaces as error card |

All contract calls are wrapped in try/catch. `getAttestors` and `isExpired` use `.catch()` in the `Promise.all` to allow graceful degradation without blocking the credential display.

---

## Correctness Properties

*A property is a characteristic or behavior that should hold true across all valid executions of a system — essentially, a formal statement about what the system should do. Properties serve as the bridge between human-readable specifications and machine-verifiable correctness guarantees.*

### Property 1: Valid credential ID triggers all contract calls

*For any* positive integer credential ID, submitting it should trigger `getCredential`, `getAttestors`, and `isExpired` calls and render the Result_Panel when all succeed.

**Validates: Requirements 2.2, 2.3, 2.4**

### Property 2: Invalid credential ID is rejected without contract calls

*For any* input that is not a positive integer (zero, negative, non-numeric, empty string), submitting it should display a validation error and make no contract calls.

**Validates: Requirements 2.5**

### Property 3: Valid Stellar address triggers subject lookup and displays results

*For any* string starting with `G` and exactly 56 characters long, submitting it should call `getCredentialsBySubject` and display one selectable item per returned credential ID.

**Validates: Requirements 3.2, 3.3**

### Property 4: Invalid Stellar address is rejected without contract calls

*For any* string that does not start with `G` or is not exactly 56 characters long, submitting it should display a validation error and make no contract calls.

**Validates: Requirements 3.5**

### Property 5: Result_Panel displays all required credential metadata fields

*For any* credential object, the rendered Result_Panel should contain the credential ID, credential type label, full subject address, full issuer address, and expiration date (or "Never").

**Validates: Requirements 4.1, 4.2, 4.3, 4.4, 4.5, 4.6**

### Property 6: Attestation status derivation is correct for all input combinations

*For any* combination of `(revoked: boolean, expired: boolean, attestorCount: number)`, the status banner should display exactly the correct status: revoked → "Credential Revoked", expired (not revoked) → "Credential Expired", attestorCount > 0 (not revoked, not expired) → "Credential Verified", otherwise → "Awaiting Attestation".

**Validates: Requirements 5.2, 5.3, 5.4, 5.5**

### Property 7: Status banner always has aria-label matching the status title

*For any* credential state, the rendered status banner element should have an `aria-label` attribute whose value equals the displayed status title string.

**Validates: Requirements 5.6**

### Property 8: Attestor list renders all addresses with correct count and badges

*For any* non-empty list of attestor addresses, the rendered attestor section should display each address in full, show a "✓ Signed" badge per address, and show a count equal to the list length.

**Validates: Requirements 6.1, 6.2, 6.3**

### Property 9: Shareable URL always encodes the current credential ID

*For any* credential ID, the shareable URL displayed in the ShareBar should be of the form `<origin>/verify?credentialId=<id>` where `<id>` matches the credential's ID exactly.

**Validates: Requirements 7.3**

### Property 10: Query param auto-triggers lookup for any valid credential ID

*For any* positive integer `credentialId` in the URL query string on mount, the page should automatically call `getCredential` without user interaction.

**Validates: Requirements 7.2**

### Property 11: ZK form calls verifyClaim with correct typed arguments

*For any* `(ClaimType, non-empty hex proof string)` pair, submitting the ZK form should call `zkVerifier.verifyClaim` with the current credential ID, the exact `ClaimType` value, and the proof string.

**Validates: Requirements 8.4**

### Property 12: Empty ZK proof is rejected without calling verifyClaim

*For any* string composed entirely of whitespace (including empty string), submitting the ZK form should display a validation error and not call `verifyClaim`.

**Validates: Requirements 8.5**

### Property 13: ZK result shows exactly one state at a time

*For any* `verifyClaim` outcome (true, false, or error), exactly one of the three result states (success, fail, error) should be visible and the other two should be absent.

**Validates: Requirements 9.3, 9.5**

### Property 14: get_credential failure shows error card and suppresses Result_Panel

*For any* error thrown by `getCredential`, the page should render an error card containing the error message and should not render the Result_Panel.

**Validates: Requirements 10.1, 10.2**

### Property 15: get_attestors failure degrades gracefully without blocking credential display

*For any* error thrown by `getAttestors`, the credential metadata and status banner should still render, and the attestor section should display "Attestor data unavailable".

**Validates: Requirements 10.4**

---

## Testing Strategy

### Unit tests

Focus on specific examples, edge cases, and integration points:

- Render `Verify` and assert no WalletGate is present (R1.2)
- Render `Verify` and assert search card is immediately visible (R1.3)
- Assert the ZK dropdown has exactly 3 options with values `HasDegree`, `HasLicense`, `HasEmploymentHistory` (R8.2)
- Assert `formatTimestamp(null)` returns `"Never"` (R4.6)
- Assert empty attestor list shows the "no attestors" message (R6.4)
- Assert address lookup returning `[]` shows the empty state (R3.4)
- Assert `verify_claim` returning `true` shows "✅ Claim Verified" (R9.1)
- Assert `verify_claim` returning `false` shows "❌ Claim Not Verified" (R9.2)
- Assert loading state disables submit button (R10.6)
- Assert clipboard copy writes the correct URL (R7.4)

### Property-based tests

Use a property-based testing library (e.g., `fast-check` for TypeScript/Jest). Each test runs a minimum of 100 iterations.

Tag format: `Feature: public-verify-page, Property {N}: {property_text}`

| Property | Generator | Assertion |
|---|---|---|
| P1 | `fc.bigInt({ min: 1n })` | Result_Panel rendered, 3 contract calls made |
| P2 | `fc.oneof(fc.constant(0), fc.integer({ max: 0 }), fc.string())` | Error shown, no contract calls |
| P3 | `fc.string({ minLength: 55, maxLength: 55 }).map(s => 'G' + s)` | `getCredentialsBySubject` called, N items rendered |
| P4 | invalid address generators | Error shown, no contract calls |
| P5 | `fc.record({ id, subject, issuer, credential_type, expires_at })` | All fields present in rendered output |
| P6 | `fc.record({ revoked: fc.boolean(), expired: fc.boolean(), attestorCount: fc.nat() })` | Correct status banner text |
| P7 | same as P6 | `aria-label` equals status title |
| P8 | `fc.array(fc.string({ minLength: 56, maxLength: 56 }), { minLength: 1 })` | Count correct, all addresses shown, all badges present |
| P9 | `fc.bigInt({ min: 1n })` | ShareBar URL contains `credentialId=<id>` |
| P10 | `fc.bigInt({ min: 1n })` | `getCredential` called on mount |
| P11 | `fc.tuple(fc.constantFrom('HasDegree','HasLicense','HasEmploymentHistory'), fc.hexaString({ minLength: 2 }))` | `verifyClaim` called with correct args |
| P12 | `fc.stringOf(fc.constant(' '))` | Error shown, `verifyClaim` not called |
| P13 | `fc.oneof(fc.constant(true), fc.constant(false), fc.constant(new Error('x')))` | Exactly one result state visible |
| P14 | `fc.string()` (error message) | Error card shown, no Result_Panel |
| P15 | `fc.string()` (error message) | Metadata renders, attestor section shows fallback |

Each property test must include a comment:
```typescript
// Feature: public-verify-page, Property N: <property_text>
```
