# Requirements Document

## Introduction

The Public Credential Verification Page is a public-facing route at `/verify` that allows anyone — employers, institutions, or the general public — to verify engineering credentials on the Stellar blockchain without connecting a wallet. Users can look up credentials by credential ID or by an engineer's Stellar address, view full credential metadata and attestor lists, and run Zero-Knowledge (ZK) claim verification for specific claim types (Degree, License, Employment). The page supports shareable URLs with a credential ID query parameter so verification links can be embedded in resumes, emails, or job applications.

The feature is built inside the existing `frontend/` React + TypeScript application. It reuses the existing `Verify.tsx` page stub, the `quorumProof` and `zkVerifier` contract clients in `frontend/src/lib/contracts/`, and the existing routing in `App.tsx`. No wallet connection is required at any point on this page.

## Glossary

- **Verify_Page**: The React component rendered at the `/verify` route; the primary subject of this document.
- **Credential_ID**: A positive integer (`u64`) that uniquely identifies a credential on-chain in the QuorumProof contract.
- **Stellar_Address**: A Stellar public key string beginning with `G`, exactly 56 characters long, used as the subject address for credential lookup.
- **Credential**: An on-chain record in the QuorumProof contract representing a verifiable credential issued to an engineer's Stellar address.
- **Attestor**: A Stellar address that has signed a credential as part of a quorum, confirming its authenticity.
- **Attestation_Status**: The computed validity state of a credential — one of `Verified`, `Pending`, `Revoked`, or `Expired` — derived from on-chain data.
- **ZK_Claim**: A Zero-Knowledge proof assertion about a specific property of a credential (e.g., "has a degree") that can be verified without revealing the full credential contents.
- **Claim_Type**: The category of a ZK_Claim; one of `HasDegree`, `HasLicense`, or `HasEmploymentHistory`, corresponding to the on-chain `ClaimType` enum in the ZK Verifier contract.
- **ZK_Verifier**: The Soroban smart contract that exposes the `verify_claim` method for ZK proof verification.
- **QuorumProof**: The Soroban smart contract that exposes `get_credential`, `is_attested`, and `get_attestors` methods.
- **Shareable_URL**: A URL of the form `/verify?credentialId=<id>` that encodes a Credential_ID as a query parameter, enabling direct deep-linking to a verification result.
- **Result_Panel**: The UI section rendered after a successful lookup, displaying credential metadata, attestor list, and the ZK claim form.
- **Contract_Client**: The typed TypeScript wrappers in `frontend/src/lib/contracts/` that call Soroban RPC simulation endpoints without requiring a wallet.

---

## Requirements

### Requirement 1: Public Route — No Wallet Required

**User Story:** As an employer, I want to access the verification page without connecting a wallet, so that I can verify an engineer's credentials instantly without any blockchain setup.

#### Acceptance Criteria

1. THE Verify_Page SHALL be accessible at the `/verify` route without requiring a connected Freighter wallet.
2. THE Verify_Page SHALL NOT render the WalletGate component or any wallet connection prompt.
3. THE Verify_Page SHALL display a search interface immediately upon navigation, without any authentication step.
4. WHEN a user navigates to `/verify`, THE Verify_Page SHALL display a page title and subtitle describing the public verification purpose.

---

### Requirement 2: Credential ID Lookup

**User Story:** As an employer, I want to enter a credential ID and instantly see the credential details, so that I can confirm an engineer's specific credential is authentic.

#### Acceptance Criteria

1. THE Verify_Page SHALL provide a numeric input field for entering a Credential_ID.
2. WHEN a user submits a valid Credential_ID (a positive integer), THE Verify_Page SHALL call `get_credential` on the QuorumProof contract with that ID and display the Result_Panel.
3. WHEN a user submits a valid Credential_ID, THE Verify_Page SHALL call `is_attested` on the QuorumProof contract with that ID and display the Attestation_Status.
4. WHEN a user submits a valid Credential_ID, THE Verify_Page SHALL call `get_attestors` on the QuorumProof contract with that ID and display the full list of Attestor addresses.
5. IF a user submits a non-positive integer or non-numeric value as a Credential_ID, THEN THE Verify_Page SHALL display a validation error message and SHALL NOT call any contract method.
6. WHEN the user presses the Enter key while the Credential_ID input is focused, THE Verify_Page SHALL trigger the same lookup as clicking the verify button.

---

### Requirement 3: Stellar Address Lookup

**User Story:** As an employer, I want to enter an engineer's Stellar address and see all their credentials, so that I can get a complete picture of their verified qualifications.

#### Acceptance Criteria

1. THE Verify_Page SHALL provide a text input field for entering a Stellar_Address.
2. WHEN a user submits a valid Stellar_Address (starts with `G`, exactly 56 characters), THE Verify_Page SHALL call `get_credentials_by_subject` on the QuorumProof contract and display a list of all returned Credential_IDs.
3. WHEN the address lookup returns one or more Credential_IDs, THE Verify_Page SHALL display each ID as a selectable item that, when activated, fetches and displays the full Result_Panel for that credential.
4. WHEN the address lookup returns zero Credential_IDs, THE Verify_Page SHALL display an empty state message indicating no credentials are associated with that address.
5. IF a user submits a string that does not start with `G` or is not 56 characters long, THEN THE Verify_Page SHALL display a validation error and SHALL NOT call any contract method.
6. WHEN the user presses the Enter key while the Stellar_Address input is focused, THE Verify_Page SHALL trigger the same lookup as clicking the look-up button.

---

### Requirement 4: Credential Metadata Display

**User Story:** As an employer, I want to see the full details of a credential, so that I can confirm the credential type, issuer, subject, and validity period.

#### Acceptance Criteria

1. THE Result_Panel SHALL display the Credential_ID.
2. THE Result_Panel SHALL display the credential type label corresponding to the on-chain `credential_type` integer (1 = Degree, 2 = License, 3 = Employment, 4 = Certification, 5 = Research).
3. THE Result_Panel SHALL display the full subject Stellar_Address.
4. THE Result_Panel SHALL display the full issuer Stellar_Address.
5. WHEN a credential has a non-null `expires_at` value, THE Result_Panel SHALL display the expiration date formatted as a human-readable date string.
6. WHEN a credential has a null `expires_at` value, THE Result_Panel SHALL display "Never" for the expiration field.
7. THE Result_Panel SHALL display the network name (testnet or mainnet) on which the credential was verified.

---

### Requirement 5: Attestation Status Banner

**User Story:** As an employer, I want a clear visual indicator of whether a credential is valid, so that I can make a quick trust decision at a glance.

#### Acceptance Criteria

1. THE Result_Panel SHALL display an Attestation_Status banner as the first visible element of the result.
2. WHEN a credential's `revoked` field is `true`, THE Result_Panel SHALL display the status as "Credential Revoked" with a visually distinct error indicator.
3. WHEN a credential is not revoked and its `expires_at` timestamp is in the past, THE Result_Panel SHALL display the status as "Credential Expired" with a neutral indicator.
4. WHEN a credential is not revoked, not expired, and has one or more Attestors, THE Result_Panel SHALL display the status as "Credential Verified" with a success indicator and the count of attesting nodes.
5. WHEN a credential is not revoked, not expired, and has zero Attestors, THE Result_Panel SHALL display the status as "Awaiting Attestation" with a pending indicator.
6. THE Attestation_Status banner SHALL include an `aria-label` attribute containing the status text so screen readers can announce the credential status.

---

### Requirement 6: Attestor List Display

**User Story:** As an employer, I want to see which institutions have attested a credential, so that I can assess the trustworthiness of the verification.

#### Acceptance Criteria

1. THE Result_Panel SHALL display a dedicated attestor section showing all Attestor addresses returned by `get_attestors`.
2. THE Result_Panel SHALL display the total count of Attestors in the attestor section header.
3. WHEN the attestor list is non-empty, THE Result_Panel SHALL display each Attestor address in full (not truncated) with a "✓ Signed" badge.
4. WHEN the attestor list is empty, THE Result_Panel SHALL display a message indicating no attestors have signed the credential.

---

### Requirement 7: Shareable URL

**User Story:** As an engineer, I want to share a direct link to my credential verification, so that employers can verify my credentials by clicking a single URL.

#### Acceptance Criteria

1. THE Verify_Page SHALL accept a `credentialId` query parameter in the URL (e.g., `/verify?credentialId=42`).
2. WHEN the page loads with a valid `credentialId` query parameter, THE Verify_Page SHALL automatically trigger the credential lookup without requiring any user interaction.
3. WHEN a credential is displayed, THE Result_Panel SHALL show a shareable URL of the form `/verify?credentialId=<id>` that the user can copy.
4. WHEN the user clicks the copy button next to the shareable URL, THE Verify_Page SHALL write the full shareable URL to the system clipboard.
5. WHEN the page loads with a `credentialId` query parameter and the lookup succeeds, THE Verify_Page SHALL update the browser URL to include the `credentialId` parameter so the URL remains bookmarkable.

---

### Requirement 8: ZK Claim Verification Form

**User Story:** As an employer, I want to verify a specific ZK claim about a credential without seeing the full credential contents, so that I can confirm a property (e.g., "has a degree") while respecting the engineer's privacy.

#### Acceptance Criteria

1. THE Result_Panel SHALL display a ZK claim verification form below the credential metadata section.
2. THE ZK_Claim form SHALL include a dropdown for selecting the Claim_Type with exactly three options: "Degree" (maps to `HasDegree`), "License" (maps to `HasLicense`), and "Employment" (maps to `HasEmploymentHistory`).
3. THE ZK_Claim form SHALL include a text area for pasting hex-encoded ZK proof bytes.
4. WHEN a user submits the ZK_Claim form with a selected Claim_Type and non-empty proof bytes, THE Verify_Page SHALL call `verify_claim` on the ZK_Verifier contract with the current Credential_ID, the selected Claim_Type, and the decoded proof bytes.
5. IF the proof bytes field is empty when the user submits the ZK_Claim form, THEN THE Verify_Page SHALL display a validation error and SHALL NOT call `verify_claim`.
6. THE ZK_Claim form SHALL display a privacy explanation tooltip or note stating that ZK verification confirms a claim without revealing the underlying credential data.
7. THE ZK_Claim form SHALL display a "No wallet required" indicator to reassure users that verification is fully public.

---

### Requirement 9: ZK Claim Result Display

**User Story:** As an employer, I want a clear binary result from ZK claim verification, so that I know definitively whether the claim is valid.

#### Acceptance Criteria

1. WHEN `verify_claim` returns `true`, THE Verify_Page SHALL display "✅ Claim Verified" as the ZK result.
2. WHEN `verify_claim` returns `false`, THE Verify_Page SHALL display "❌ Claim Not Verified" as the ZK result.
3. THE ZK result display SHALL show exactly one of the two states at a time and SHALL NOT display both simultaneously.
4. WHILE `verify_claim` is executing, THE Verify_Page SHALL display a loading indicator in place of the ZK result and SHALL disable the submit button.
5. IF `verify_claim` throws an error, THEN THE Verify_Page SHALL display a warning message containing the error description and SHALL NOT display either the verified or not-verified state.
6. THE ZK result SHALL include a tooltip or inline note explaining that ZK proofs confirm a property of the credential without revealing the credential's private contents.

---

### Requirement 10: Error Handling

**User Story:** As an employer, I want clear error messages when verification fails, so that I understand what went wrong and can take corrective action.

#### Acceptance Criteria

1. IF the `get_credential` call fails for a given Credential_ID, THEN THE Verify_Page SHALL display an error card with a human-readable description of the failure.
2. IF the `get_credential` call fails, THEN THE Verify_Page SHALL NOT render the Result_Panel.
3. IF the `is_attested` call fails, THEN THE Verify_Page SHALL default the Attestation_Status to "Awaiting Attestation" and SHALL log the error to the browser console.
4. IF the `get_attestors` call fails, THEN THE Verify_Page SHALL display "Attestor data unavailable" in the attestor section and SHALL NOT block rendering of other credential data.
5. IF the `get_credentials_by_subject` call fails, THEN THE Verify_Page SHALL display an error card with a human-readable description of the failure.
6. WHILE any contract call is in progress, THE Verify_Page SHALL display a loading indicator and SHALL disable the relevant submit button to prevent duplicate requests.
