# Implementation Plan: frontend-bootstrap

## Overview

Implement the foundational React + TypeScript shell for QuorumProof: environment config module,
`WalletConnector` component, `NetworkIndicator` component, updated `App.tsx`, and property-based
tests using `fast-check` with Vitest + React Testing Library.

## Tasks

- [x] 1. Install missing dependencies
  - Add `@stellar/freighter-api` to `dependencies` in `frontend/package.json`
  - Add `fast-check`, `vitest`, `@vitest/coverage-v8`, `@testing-library/react`, `@testing-library/jest-dom`, and `jsdom` to `devDependencies`
  - Add a `test` script (`"vitest --run"`) to `package.json`
  - Configure Vitest in `vite.config.ts`: add `test` block with `environment: 'jsdom'` and `setupFiles`
  - Create `frontend/src/setupTests.ts` that imports `@testing-library/jest-dom`
  - _Requirements: 2.1, 2.2, 1.2_

- [x] 2. Implement `envConfig.ts`
  - [x] 2.1 Create `frontend/src/envConfig.ts`
    - Export `StellarNetwork` type and `VALID_NETWORKS` array
    - Export `EnvConfig` interface with `network` and `rpcUrl` fields
    - Read `import.meta.env.VITE_STELLAR_NETWORK`; default to `"testnet"` when absent; fall back to `"testnet"` with `console.warn` when value is not in `VALID_NETWORKS`
    - Read `import.meta.env.VITE_STELLAR_RPC_URL`; default to `"https://soroban-testnet.stellar.org"` when absent
    - Export a single `envConfig` const of type `EnvConfig`
    - _Requirements: 3.1, 3.2, 3.4_

  - [ ]* 2.2 Write unit tests for `envConfig.ts`
    - Test: returns `"testnet"` when `VITE_STELLAR_NETWORK` is absent
    - Test: returns correct default RPC URL when `VITE_STELLAR_RPC_URL` is absent
    - Test: returns supplied RPC URL when `VITE_STELLAR_RPC_URL` is set
    - Test: `.env.example` file contains all required `VITE_` keys
    - Test: `package.json` declares `@stellar/freighter-api` as a dependency
    - _Requirements: 3.1, 3.2, 3.3, 2.1_

  - [ ]* 2.3 Write property test for `envConfig` network validity
    - **Property 1: EnvConfig always produces a valid network**
    - **Validates: Requirements 3.1, 3.4**
    - Use `fc.string()` (and `fc.constantFrom(...VALID_NETWORKS)` for valid cases) to generate arbitrary `VITE_STELLAR_NETWORK` values
    - Assert returned `network` is always one of the four valid `StellarNetwork` values
    - _Requirements: 3.1, 3.4_

- [x] 3. Implement `NetworkIndicator` component
  - [x] 3.1 Create `frontend/src/components/NetworkIndicator.tsx`
    - Import `envConfig` from `envConfig.ts`
    - Define `NETWORK_LABELS` map for all four `StellarNetwork` values
    - Render a `<span>` with class `network-badge network-badge--{network}`
    - Display the human-readable label from `NETWORK_LABELS`
    - _Requirements: 6.1, 6.2, 6.3, 6.4, 6.5, 6.6, 6.7_

  - [ ]* 3.2 Write unit tests for `NetworkIndicator`
    - Test: applies `network-badge--mainnet` CSS class when network is `"mainnet"`
    - _Requirements: 6.6_

  - [ ]* 3.3 Write property test for `NetworkIndicator` label mapping
    - **Property 2: NetworkIndicator label mapping**
    - **Validates: Requirements 6.2, 6.3, 6.4, 6.5**
    - Use `fc.constantFrom('testnet', 'mainnet', 'futurenet', 'standalone')` to drive network value
    - Mock `envConfig` to inject each generated value; assert rendered text matches expected label
    - _Requirements: 6.2, 6.3, 6.4, 6.5_

- [x] 4. Checkpoint — Ensure all tests pass
  - Ensure all tests pass, ask the user if questions arise.

- [x] 5. Implement `WalletConnector` component
  - [x] 5.1 Create `frontend/src/components/WalletConnector.tsx`
    - Define `WalletState` discriminated union (`disconnected | connecting | connected | error`)
    - Initialise state as `{ status: 'disconnected' }`
    - On connect click: set `connecting`, call `isConnected()` then `requestAccess()` from `@stellar/freighter-api`
    - On success: set `{ status: 'connected', publicKey }`
    - On user rejection or any Freighter error: set `{ status: 'error', message }`
    - If Freighter is not installed (extension absent): render static "Freighter extension is required." message
    - On disconnect click: set `{ status: 'disconnected' }`
    - Render `<code>` element (monospace) for the public key when connected
    - Show loading indicator and disabled button while `connecting`
    - _Requirements: 4.1, 4.2, 4.3, 4.4, 4.5, 4.6, 2.3, 5.1, 5.2, 5.3_

  - [ ]* 5.2 Write unit tests for `WalletConnector`
    - Test: renders "Connect Wallet" button in disconnected state
    - Test: calls `requestAccess` when connect button is clicked (mock Freighter)
    - Test: shows loading indicator and disabled button while connecting (pending promise mock)
    - Test: shows "Freighter extension required" when Freighter is absent
    - Test: shows error message and re-enabled button when user rejects permission
    - Test: displays no address in disconnected state
    - _Requirements: 4.1, 4.2, 4.3, 4.4, 4.5, 5.2_

  - [ ]* 5.3 Write property test for connected wallet renders full key in monospace
    - **Property 3: Connected wallet renders full key in monospace**
    - **Validates: Requirements 5.1, 5.3**
    - Use `fc.stringMatching(/^G[A-Z2-7]{55}$/)` to generate Stellar-like public keys
    - Mock Freighter `requestAccess` to resolve with the generated key
    - Assert the full key appears inside a `<code>` element
    - _Requirements: 5.1, 5.3_

  - [ ]* 5.4 Write property test for connect then disconnect round trip
    - **Property 4: Connect then disconnect returns to disconnected state**
    - **Validates: Requirements 4.3**
    - Generate arbitrary public keys; mock Freighter to return each key
    - Simulate connect then disconnect; assert component is in disconnected state with no key visible
    - _Requirements: 4.3_

  - [ ]* 5.5 Write property test for Freighter error message surfaced verbatim
    - **Property 5: Freighter error message is surfaced verbatim**
    - **Validates: Requirements 4.6**
    - Use `fc.string({ minLength: 1 })` to generate arbitrary error message strings
    - Mock `requestAccess` to reject with the generated message
    - Assert the rendered output contains the exact message string
    - _Requirements: 4.6_

- [x] 6. Update `App.tsx` and add `NetworkIndicator` always-present property test
  - [x] 6.1 Replace `frontend/src/App.tsx` with the application shell
    - Render `<header className="app-header">` containing `<span className="app-logo">⬡ QuorumProof</span>`, `<NetworkIndicator />`, and `<WalletConnector />`
    - Render `<main>` below the header for future route content
    - Remove all placeholder Vite scaffold content
    - _Requirements: 1.1, 6.7_

  - [ ]* 6.2 Write property test for NetworkIndicator always present
    - **Property 6: NetworkIndicator is always present regardless of wallet state**
    - **Validates: Requirements 6.7**
    - Use `fc.constantFrom('disconnected', 'connecting', 'connected', 'error')` to drive wallet state
    - Render `App` with each wallet state injected; assert `NetworkIndicator` element is present in the DOM
    - _Requirements: 6.7_

- [x] 7. Verify TypeScript compilation
  - Confirm `tsc -b` passes with zero errors across all new `.ts` / `.tsx` files
  - Fix any type errors surfaced by the compiler
  - _Requirements: 1.3_

- [x] 8. Final checkpoint — Ensure all tests pass
  - Ensure all tests pass, ask the user if questions arise.

## Notes

- Tasks marked with `*` are optional and can be skipped for faster MVP
- Each task references specific requirements for traceability
- Property tests use `fast-check`; each test has a comment `// Feature: frontend-bootstrap, Property N: <title>`
- Existing `.js` files (`stellar.js`, `dashboard.js`, `verify.js`, `main.js`) are out of scope and must not be modified
