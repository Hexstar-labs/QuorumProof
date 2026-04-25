# Requirements Document

## Introduction

The Credential Dashboard is the primary interface for engineers to view and manage their on-chain verifiable credentials issued via the QuorumProof Soroban smart contract system. The dashboard is wallet-gated (requires a connected Freighter wallet) and surfaces three key data domains: issued credentials, per-credential attestation status, and the quorum slice composition (attestor addresses and role labels) for each credential. Engineers with no credentials are shown an empty state with a call-to-action to request issuance.

The feature integrates with the existing `frontend/` React/TypeScript application, reusing the `useFreighter` wallet hook, the `quorumProof` contract client (`get_credentials_by_subject`, `is_attested`, `get_slice`), and the existing routing setup in `App.tsx`.

## Glossary

- **Dashboard**: The `/dashboard` route and page component that is the main entry point for authenticated engineers.
- **Wallet_Gate**: The access control mechanism that requires a connected Freighter wallet before credential data is fetched or displayed.
- **Credential**: An on-chain record issued to an engineer's Stellar address, represented by the `Credential` struct from the `quorumProof` contract.
- **Attestation_Status**: The computed state of a credential — one of `Attested`, `Pending`, or `Revoked` — derived from the `is_attested` and `revoked` fields.
- **Quorum_Slice**: A named group of attestor addresses and their role labels, retrieved via `get_slice` from the `quorumProof` contract.
- **Attestor**: A Stellar address that is a member of a Quorum_Slice and may sign credentials.
- **Empty_State**: The UI shown when a connected wallet has zero credentials on-chain.
- **CTA**: Call-to-action — a UI element prompting the engineer to request credential issuance.
- **Freighter**: The Stellar browser wallet extension used for wallet connection.
- **Dashboard_Page**: The React component rendered at the `/dashboard` route.
- **Credential_Card**: A UI component that displays a single credential's summary, status, and quorum slice.
- **Contract_Client**: The typed TypeScript wrappers in `frontend/src/lib/contracts/` that call Soroban RPC simulation endpoints.

---

## Requirements

### Requirement 1: Wallet-Gated Route

**User Story:** As an engineer, I want the dashboard to require a connected wallet, so that only my own credentials are displayed and no data is leaked to unauthenticated visitors.

#### Acceptance Criteria

1. WHEN a user navigates to `/dashboard` without a connected Freighter wallet, THE Dashboard_Page SHALL display a wallet connection prompt instead of credential data.
2. WHEN a user navigates to `/dashboard` without a connected Freighter wallet, THE Dashboard_Page SHALL render a "Connect Wallet" button that invokes the Freighter connection flow.
3. WHEN the Freighter wallet connection is initializing, THE Dashboard_Page SHALL display a loading indicator and SHALL NOT render credential data.
4. WHEN a user successfully connects their Freighter wallet, THE Dashboard_Page SHALL begin fetching credentials for the connected address without requiring a page reload.
5. IF the Freighter extension is not installed, THEN THE Dashboard_Page SHALL display a message directing the user to install Freighter and SHALL provide a link to `https://freighter.app`.

---

### Requirement 2: Fetch and Display Credentials

**User Story:** As an engineer, I want to see all credentials issued to my wallet address, so that I have a complete view of my on-chain identity.

#### Acceptance Criteria

1. WHEN a wallet address is connected, THE Dashboard_Page SHALL call `get_credentials_by_subject` with the connected address to retrieve all credential IDs.
2. WHEN credential IDs are retrieved, THE Dashboard_Page SHALL fetch the full `Credential` struct for each ID and render a Credential_Card per credential.
3. WHILE credential data is being fetched, THE Dashboard_Page SHALL display a loading indicator in place of the credential grid.
4. IF the `get_credentials_by_subject` call fails, THEN THE Dashboard_Page SHALL display an error message describing the failure and SHALL provide a retry action.
5. IF a single credential fetch fails, THEN THE Dashboard_Page SHALL display an inline error on that Credential_Card and SHALL continue rendering successfully fetched credentials.
6. THE Credential_Card SHALL display the credential ID (truncated to first 8 and last 6 characters), credential type label, issuer address (truncated), and issuance date.
7. WHEN a credential has an expiration date, THE Credential_Card SHALL display the expiration date.

---

### Requirement 3: Attestation Status per Credential

**User Story:** As an engineer, I want to see the attestation status of each credential, so that I know which credentials are verified, pending review, or revoked.

#### Acceptance Criteria

1. THE Dashboard_Page SHALL derive the Attestation_Status for each credential using the `revoked` field and the result of `is_attested`.
2. WHEN a credential's `revoked` field is `true`, THE Credential_Card SHALL display the status as `Revoked` with a visually distinct indicator (red/error color).
3. WHEN a credential is not revoked and `is_attested` returns `true`, THE Credential_Card SHALL display the status as `Attested` with a success indicator (green/success color).
4. WHEN a credential is not revoked and `is_attested` returns `false`, THE Credential_Card SHALL display the status as `Pending` with a neutral indicator (yellow/warning color).
5. THE Credential_Card SHALL expose the Attestation_Status via an `aria-label` attribute so screen readers can announce the status.
6. IF the `is_attested` call fails for a credential, THEN THE Credential_Card SHALL display the status as `Pending` and SHALL log the error to the browser console.

---

### Requirement 4: Quorum Slice Composition

**User Story:** As an engineer, I want to see the quorum slice members for each credential, so that I understand which institutions have attested or are expected to attest my credentials.

#### Acceptance Criteria

1. WHEN a Credential_Card is rendered, THE Dashboard_Page SHALL call `get_slice` to retrieve the Quorum_Slice associated with that credential.
2. THE Credential_Card SHALL display each Attestor address in the Quorum_Slice, truncated to first 8 and last 6 characters, with the full address available on hover via a `title` attribute.
3. WHERE a role label is available for an Attestor in the Quorum_Slice, THE Credential_Card SHALL display the role label alongside the attestor address.
4. THE Credential_Card SHALL display the total count of attestors in the Quorum_Slice and the threshold required for attestation.
5. IF the `get_slice` call fails for a credential, THEN THE Credential_Card SHALL display a "Slice unavailable" message in place of the attestor list and SHALL NOT block rendering of other credential data.
6. WHEN the Quorum_Slice has zero attestors, THE Credential_Card SHALL display "No attestors assigned" in the slice section.

---

### Requirement 5: Empty State with CTA

**User Story:** As an engineer with no issued credentials, I want to see a helpful empty state, so that I know my wallet is connected but I have no credentials yet and understand how to get one.

#### Acceptance Criteria

1. WHEN a connected wallet address has zero credentials returned by `get_credentials_by_subject`, THE Dashboard_Page SHALL display the Empty_State component instead of a credential grid.
2. THE Empty_State SHALL include a descriptive message indicating that no credentials have been issued to the connected address.
3. THE Empty_State SHALL include a CTA button or link labeled "Request Credential Issuance" that directs the engineer toward the issuance flow.
4. THE Empty_State SHALL display the connected wallet address so the engineer can confirm the correct wallet is connected.
5. THE Empty_State SHALL be visually distinct from the loading state and the error state so engineers can differentiate between "no credentials", "loading", and "error" conditions.
