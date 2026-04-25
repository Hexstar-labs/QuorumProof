# Requirements Document

## Introduction

This feature establishes the foundational frontend layer for QuorumProof: a Vite + React + TypeScript
application with Stellar wallet connectivity via @stellar/freighter-api. It covers project scaffolding,
environment configuration, wallet connect/disconnect UI, address display, and a network indicator
sourced from `environments.toml` values. All other frontend features depend on this bootstrap layer.

## Glossary

- **App**: The QuorumProof frontend React application built with Vite and TypeScript.
- **Freighter**: The Stellar browser wallet extension accessed via `@stellar/freighter-api`.
- **WalletConnector**: The React component responsible for connecting and disconnecting the Freighter wallet.
- **NetworkIndicator**: The React component that displays the active Stellar network name.
- **StellarNetwork**: One of `testnet`, `mainnet`, or `futurenet`, as defined in `environments.toml`.
- **EnvConfig**: The runtime configuration object derived from Vite environment variables (`VITE_STELLAR_NETWORK`, `VITE_STELLAR_RPC_URL`).
- **PublicKey**: A Stellar G-address (56-character base32 string) identifying a wallet account.

---

## Requirements

### Requirement 1: Project Scaffold

**User Story:** As a developer, I want a Vite + React + TypeScript project under `frontend/`, so that I have a consistent, typed build environment for all frontend work.

#### Acceptance Criteria

1. THE App SHALL be scaffolded using Vite with the React + TypeScript template under the `frontend/` directory.
2. THE App SHALL include `@vitejs/plugin-react` and `vite-plugin-node-polyfills` (with `Buffer` global) in `vite.config.ts`.
3. THE App SHALL compile without TypeScript errors when `tsc -b` is run.
4. THE App SHALL start a development server when `vite` is invoked.

---

### Requirement 2: Freighter API Integration

**User Story:** As a developer, I want `@stellar/freighter-api` integrated into the project, so that wallet connectivity is available to all frontend features.

#### Acceptance Criteria

1. THE App SHALL declare `@stellar/freighter-api` as a dependency in `package.json`.
2. WHEN `@stellar/freighter-api` is imported, THE App SHALL resolve the module without build errors.
3. IF Freighter is not installed in the browser, THEN THE WalletConnector SHALL display a message indicating that the Freighter extension is required.

---

### Requirement 3: Environment Variable Handling

**User Story:** As a developer, I want environment variables loaded from `.env` at build time, so that network configuration is decoupled from source code.

#### Acceptance Criteria

1. THE EnvConfig SHALL read `VITE_STELLAR_NETWORK` from the Vite environment, defaulting to `"testnet"` when the variable is absent.
2. THE EnvConfig SHALL read `VITE_STELLAR_RPC_URL` from the Vite environment, defaulting to `"https://soroban-testnet.stellar.org"` when the variable is absent.
3. THE App SHALL expose a `.env.example` file listing all required `VITE_` variables with placeholder values.
4. IF `VITE_STELLAR_NETWORK` is set to a value not present in `environments.toml` (`testnet`, `mainnet`, `futurenet`, `standalone`), THEN THE EnvConfig SHALL fall back to `"testnet"` and log a console warning.

---

### Requirement 4: Wallet Connect / Disconnect

**User Story:** As a user, I want a connect/disconnect button, so that I can link or unlink my Freighter wallet from the application.

#### Acceptance Criteria

1. WHEN the user clicks the connect button and Freighter is installed, THE WalletConnector SHALL request access to the user's public key via `@stellar/freighter-api`.
2. WHEN Freighter grants access, THE WalletConnector SHALL store the retrieved PublicKey in component state and display it.
3. WHEN the user clicks the disconnect button, THE WalletConnector SHALL clear the stored PublicKey from component state.
4. WHILE the wallet connection request is pending, THE WalletConnector SHALL display a loading indicator and disable the connect button.
5. IF the user rejects the Freighter permission prompt, THEN THE WalletConnector SHALL display an error message and return to the disconnected state.
6. IF Freighter returns an error during connection, THEN THE WalletConnector SHALL display the error message to the user.

---

### Requirement 5: Wallet Address Display

**User Story:** As a user, I want to see my connected wallet address, so that I can confirm which account is active.

#### Acceptance Criteria

1. WHEN a wallet is connected, THE WalletConnector SHALL display the full PublicKey string.
2. WHEN no wallet is connected, THE WalletConnector SHALL display no address.
3. THE WalletConnector SHALL render the PublicKey in a monospace font element to aid readability.

---

### Requirement 6: Network Indicator

**User Story:** As a user, I want to see which Stellar network the application is targeting, so that I can avoid submitting transactions to the wrong network.

#### Acceptance Criteria

1. THE NetworkIndicator SHALL read the active network name from `VITE_STELLAR_NETWORK` via EnvConfig.
2. WHEN `VITE_STELLAR_NETWORK` is `"testnet"`, THE NetworkIndicator SHALL display the label `"Testnet"`.
3. WHEN `VITE_STELLAR_NETWORK` is `"mainnet"`, THE NetworkIndicator SHALL display the label `"Mainnet"`.
4. WHEN `VITE_STELLAR_NETWORK` is `"futurenet"`, THE NetworkIndicator SHALL display the label `"Futurenet"`.
5. WHEN `VITE_STELLAR_NETWORK` is `"standalone"`, THE NetworkIndicator SHALL display the label `"Standalone"`.
6. THE NetworkIndicator SHALL apply a visually distinct style for `"mainnet"` to warn users they are on the production network.
7. THE NetworkIndicator SHALL be visible in the application header at all times, regardless of wallet connection state.
