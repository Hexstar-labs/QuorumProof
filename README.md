# QuorumProof — Federated Engineering Credential Auditor

A decentralized professional credential verification platform built on Stellar Soroban smart contracts, using Federated Byzantine Agreement (FBA) trust slices to audit engineering licenses and degrees across borders.

Engineering certifications vary by country, making it difficult for international firms to verify credentials quickly and reliably. QuorumProof replaces fragmented government portals with a trustless, privacy-preserving audit layer — powered by the same consensus model that underlies Stellar itself.

## 🎯 What is QuorumProof?

QuorumProof lets engineers build a **Quorum Slice** — a personal trust network made up of:

- 🎓 Their **University** (degree attestation)
- 🏛️ A **National Engineering Society** (license validation)
- 🏢 **Previous Employers** (professional history)

Each node in the slice co-signs a **Soulbound Token (SBT)** on Stellar, creating a tamper-proof, portable credential that any firm can verify instantly — without contacting each institution individually.

This applies the Stellar whitepaper's "individual trust decisions" model to a high-stakes professional use case.

## ⚠️ ZK Verification — Non-Functional Stub

> **Do not use `verify_claim` in production.**
> The `zk_verifier` contract accepts **any non-empty byte string** as a valid proof.
> It performs **no cryptographic verification** and provides **no privacy guarantees**.
> It is admin-gated to limit exposure, but the gate is not a substitute for real ZK logic.
> Real proof verification (Groth16/PLONK) is tracked in [#ZK-IMPL](https://github.com/cryptonautt/QuorumProof/issues) and targeted for v1.1.

## 🚀 Features

- **Audit Slices**: Define your own quorum of trusted attestors (university, licensing body, employers)
- **Soulbound Tokens (SBTs)**: Non-transferable on-chain credentials tied to your Stellar identity
- **Conditional Verification (stub)**: API exists for claim-specific proofs (e.g. "has a Mechanical Engineering degree") but ZK verification is not yet implemented — see warning above
- **Cross-Border Ready**: Instant verification for international hiring, no embassy letters or notarizations
- **Privacy-First**: Credential holders control what is revealed and to whom
- **Trustless**: No central registry — verification is enforced by smart contract logic

## 🛠️ Quick Start

### Prerequisites

- Rust (1.70+)
- Soroban CLI
- Stellar CLI

### Build

```bash
./scripts/build.sh
```

### Test

```bash
./scripts/test.sh
```

### Setup Environment

Copy the example environment file:

```bash
cp .env.example .env
```

Configure your environment variables in `.env`:

```env
# Network configuration
STELLAR_NETWORK=testnet
STELLAR_RPC_URL=https://soroban-testnet.stellar.org

# Contract addresses (after deployment)
CONTRACT_QUORUM_PROOF=<your-contract-id>
CONTRACT_SBT_REGISTRY=<your-contract-id>
CONTRACT_ZK_VERIFIER=<your-contract-id>

# Frontend configuration
VITE_STELLAR_NETWORK=testnet
VITE_STELLAR_RPC_URL=https://soroban-testnet.stellar.org
```

Network configurations are defined in `environments.toml`:

- `testnet` — Stellar testnet
- `mainnet` — Stellar mainnet
- `futurenet` — Stellar futurenet
- `standalone` — Local development

### Deploy to Testnet

```bash
# Configure your testnet identity first
stellar keys generate deployer --network testnet

# Deploy
./scripts/deploy_testnet.sh
```

### Run Demo

Follow the step-by-step guide in `demo/demo-script.md`

## 📖 Documentation

- [Architecture Overview](docs/architecture.md)
- [Trust Slice Model](docs/trust-slices.md)
- [ZK Verification Design](docs/zk-verification.md)
- [Threat Model & Security](docs/threat-model.md)
- [Roadmap](docs/roadmap.md)

## 🎓 Smart Contract API

### Credential Management

```rust
issue_credential(subject, credential_type, metadata_hash) -> u64
get_credential(credential_id) -> Credential
revoke_credential(credential_id)
```

### Quorum Slices

```rust
create_slice(attestors: Vec<Address>, threshold: u32) -> u64
get_slice(slice_id) -> QuorumSlice
add_attestor(slice_id, attestor)
```

### Attestation

```rust
attest(credential_id, slice_id)
is_attested(credential_id) -> bool
get_attestors(credential_id) -> Vec<Address>
```

### Conditional Verification (ZK)

```rust
verify_claim(credential_id, claim_type, proof) -> bool
generate_proof_request(credential_id, claim_type) -> ProofRequest
```

## 🧪 Testing

Comprehensive test suite covering:

- ✅ Credential issuance and revocation
- ✅ Quorum slice creation and attestor management
- ✅ Multi-party attestation flow
- ✅ ZK conditional verification
- ✅ SBT non-transferability enforcement
- ✅ Error handling and edge cases

Run tests:

```bash
cargo test
```

## 🌍 Why This Matters

**The Problem**: A Mechanical Engineer licensed in Brazil applying for a role in Germany faces weeks of manual credential verification across institutions, embassies, and licensing bodies.

**The Solution**: QuorumProof collapses that process to a single on-chain query — verified in seconds, privacy-preserving by design.

**Blockchain Benefits**:

- No trusted central registry to corrupt or go offline
- Transparent attestation history, auditable by any party
- Programmable verification rules enforced by smart contracts
- Accessible to any engineer with a Stellar wallet

**Target Users**:

- International engineering firms hiring across borders
- Engineers seeking global mobility
- Universities and licensing bodies issuing verifiable credentials
- Governments modernizing professional certification infrastructure

## 🗺️ Roadmap

- **v1.0 (Current)**: Core SBT issuance, quorum slice model, multi-attestor signing
- **v1.1**: ZK conditional verification (claim-specific proofs)
- **v2.0**: Revocation registry, credential expiry, renewal flows
- **v3.0**: Frontend UI with Stellar wallet integration
- **v4.0**: Mobile app, integration with national licensing APIs

See [docs/roadmap.md](docs/roadmap.md) for details.

## 🤝 Contributing

We welcome contributions! Please:

1. Fork the repository
2. Create a feature branch (`git checkout -b feature/amazing-feature`)
3. Commit your changes (`git commit -m 'Add amazing feature'`)
4. Push to the branch (`git push origin feature/amazing-feature`)
5. Open a Pull Request

See our [Code of Conduct](CODE_OF_CONDUCT.md) and [Contributing Guidelines](CONTRIBUTING.md).

## 🌊 Drips Wave Contributors

This project participates in **Drips Wave** — a contributor funding program! Check out:

- [Wave Contributor Guide](docs/wave-guide.md) — How to earn funding for contributions
- [Wave-Ready Issues](https://github.com/issues?q=label%3Awave-ready) — Funded issues ready to tackle
- GitHub Issues labeled `wave-ready` — Earn 100–200 points per issue

Issues are categorized as:

- `trivial` (100 points) — Documentation, simple tests, minor fixes
- `medium` (150 points) — Helper functions, validation logic, moderate features
- `high` (200 points) — Core features, ZK integrations, security enhancements

## 📄 License

This project is licensed under the MIT License — see the [LICENSE](LICENSE) file for details.

## 🙏 Acknowledgments

- [Stellar Development Foundation](https://stellar.org) for Soroban
- The Stellar whitepaper for the FBA trust model that inspired this design
- Drips Wave for supporting public goods funding
