# QuorumProof Frontend

Public-facing credential verification web app for QuorumProof.

## Features

- `/verify` — Employer verification page (no wallet required)
  - Look up credentials by **Credential ID** or **Stellar Address**
  - View attestor list, metadata, revocation/expiry status
  - Shareable URL with `?credentialId=<id>` query param
  - Zero-Knowledge claim verification form

## Setup

```bash
cd frontend

# Copy environment config
cp .env.example .env

# Edit .env with your contract IDs
# VITE_CONTRACT_QUORUM_PROOF=C...
# VITE_CONTRACT_ZK_VERIFIER=C...

# Install dependencies
npm install

# Start dev server
npm run dev
```

Open [http://localhost:5173/verify](http://localhost:5173/verify).

## Environment Variables

| Variable | Description |
|---|---|
| `VITE_STELLAR_NETWORK` | `testnet`, `mainnet`, or `futurenet` |
| `VITE_STELLAR_RPC_URL` | Soroban RPC endpoint |
| `VITE_CONTRACT_QUORUM_PROOF` | Deployed QuorumProof contract ID |
| `VITE_CONTRACT_ZK_VERIFIER` | Deployed ZK Verifier contract ID |

## Architecture

```
frontend/
├── index.html          # HTML entry point
├── vite.config.js      # Vite config
├── .env.example        # Environment template
└── src/
    ├── main.js         # SPA router
    ├── verify.js       # /verify page logic
    ├── stellar.js      # Soroban RPC wrapper (read-only, no wallet)
    └── styles.css      # Design system
```

## Tech Stack

- **Vite** — build tool
- **@stellar/stellar-sdk** — Soroban contract simulation
- **Vanilla JS** — no framework
- **Google Fonts (Inter)** — typography
