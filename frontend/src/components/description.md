# IssueCredentialForm — Feature Description

## Overview

`IssueCredentialForm` is a React component that allows attestors (universities, licensing bodies, employers) to issue verifiable on-chain credentials to engineers via their Stellar address. It is rendered inside the `/issue` page, which is wallet-gated.

## Files Changed

| File | Change |
|------|--------|
| `src/components/IssueCredentialForm.tsx` | New component — the form itself |
| `src/pages/IssueCredential.tsx` | New page — wallet guard + form host |
| `src/App.tsx` | Added `/issue` route |
| `src/components/Navbar.tsx` | Added Issue nav link |
| `src/styles.css` | Added `.issue-form` CSS |
| `tsconfig.json` | Fixed config (verbatimModuleSyntax, moduleDetection) |
| `package.json` | Fixed duplicate blocks, aligned to React 19 deps |

## Component: IssueCredentialForm

### Props

| Prop | Type | Description |
|------|------|-------------|
| `issuerAddress` | `string` | The connected wallet's Stellar public key, used as the on-chain issuer |

### Form Fields

| Field | Type | Validation |
|-------|------|------------|
| Subject Stellar Address | Text input | Required, must match `/^G[A-Z2-7]{55}$/` |
| Credential Type | Dropdown | One of: Degree (1), License (2), Employment (3) |
| Metadata Hash | Text input | Required, min 4 chars — expects an IPFS CID or SHA-256 hash |

### States

- **idle** — form ready for input
- **submitting** — button shows spinner, disabled while awaiting on-chain call
- **error** — displays an error card with the contract/network error message
- **success** — replaces the form with a success banner showing the issued credential ID and two actions: "View Credential" (navigates to `/verify?credentialId=...`) and "Issue Another" (resets the form)

### On-Chain Call

Calls `issueCredential(issuer, subject, credentialType, metadataHash)` from `lib/contracts/quorumProof.ts`, which simulates the `issue_credential` Soroban contract method and returns the new `bigint` credential ID.

The metadata hash string is UTF-8 encoded to `Uint8Array` before being passed to the contract.

## Page: IssueCredential (`/issue`)

Wraps the form with:

- **Wallet guard** — if no Freighter wallet is connected, shows a prompt to connect (or install Freighter if the extension isn't detected)
- **Issuer pill** — displays the truncated connected address so the attestor knows which key will sign as issuer
- Uses the existing `useFreighter` hook for wallet state

## Routing

`/issue` is registered in `App.tsx` alongside `/dashboard` and `/verify`. The Navbar includes an "Issue" link that highlights when active.
