/// Performance benchmarks for QuorumProof contracts.
///
/// Uses soroban-sdk's `env.budget()` to capture CPU instructions consumed
/// and memory bytes consumed for each key operation.
///
/// Regression thresholds are defined as constants. A test failure means a
/// contract change has exceeded the budget baseline — investigate before merging.
///
/// Run with: `cargo test -p quorum-proof-benches -- --nocapture`
use soroban_sdk::{testutils::Address as _, Address, Bytes, Env, Vec};
use quorum_proof::{QuorumProofContract, QuorumProofContractClient};
use sbt_registry::{SbtRegistryContract, SbtRegistryContractClient};
use zk_verifier::{ClaimType, ZkVerifierContract, ZkVerifierContractClient};

// ── Regression thresholds (CPU instructions) ─────────────────────────────────
// Measured on soroban-sdk 21.x testutils. Raise only with justification.
const THRESHOLD_ISSUE_CREDENTIAL_CPU: u64    = 2_000_000;
const THRESHOLD_CREATE_SLICE_CPU: u64        = 2_000_000;
const THRESHOLD_ATTEST_CPU: u64              = 2_000_000;
const THRESHOLD_REVOKE_CREDENTIAL_CPU: u64   = 1_500_000;
const THRESHOLD_MINT_SBT_CPU: u64            = 3_000_000;
const THRESHOLD_BURN_SBT_CPU: u64            = 2_000_000;
const THRESHOLD_VERIFY_CLAIM_CPU: u64        = 1_500_000;

// ── Regression thresholds (memory bytes) ─────────────────────────────────────
const THRESHOLD_ISSUE_CREDENTIAL_MEM: u64    = 2_000_000;
const THRESHOLD_CREATE_SLICE_MEM: u64        = 2_000_000;
const THRESHOLD_ATTEST_MEM: u64              = 2_000_000;
const THRESHOLD_REVOKE_CREDENTIAL_MEM: u64   = 1_500_000;
const THRESHOLD_MINT_SBT_MEM: u64            = 3_000_000;
const THRESHOLD_BURN_SBT_MEM: u64            = 2_000_000;
const THRESHOLD_VERIFY_CLAIM_MEM: u64        = 1_500_000;

// ── Helpers ───────────────────────────────────────────────────────────────────

struct Metrics {
    cpu: u64,
    mem: u64,
}

/// Resets the budget, runs `f`, then returns consumed CPU + mem.
fn measure(env: &Env, f: impl FnOnce()) -> Metrics {
    env.budget().reset_default().unwrap();
    f();
    Metrics {
        cpu: env.budget().get_cpu_insns_consumed().unwrap(),
        mem: env.budget().get_mem_bytes_consumed().unwrap(),
    }
}

fn setup_qp(env: &Env) -> (QuorumProofContractClient, Address) {
    env.mock_all_auths();
    let id = env.register_contract(None, QuorumProofContract);
    let client = QuorumProofContractClient::new(env, &id);
    let admin = Address::generate(env);
    client.initialize(&admin);
    (client, admin)
}

fn setup_sbt(env: &Env, qp_id: &Address) -> (SbtRegistryContractClient, Address) {
    let id = env.register_contract(None, SbtRegistryContract);
    let client = SbtRegistryContractClient::new(env, &id);
    let admin = Address::generate(env);
    client.initialize(&admin, qp_id);
    (client, admin)
}

fn setup_zk(env: &Env) -> (ZkVerifierContractClient, Address) {
    let id = env.register_contract(None, ZkVerifierContract);
    let client = ZkVerifierContractClient::new(env, &id);
    let admin = Address::generate(env);
    client.initialize(&admin);
    (client, admin)
}

// ── quorum_proof benchmarks ───────────────────────────────────────────────────

#[test]
fn bench_issue_credential() {
    let env = Env::default();
    let (client, _) = setup_qp(&env);
    let issuer = Address::generate(&env);
    let subject = Address::generate(&env);
    let meta = Bytes::from_slice(&env, b"QmBenchHash000000000000000000000000");

    let m = measure(&env, || {
        client.issue_credential(&issuer, &subject, &1u32, &meta, &None);
    });

    println!("[bench_issue_credential] cpu={} mem={}", m.cpu, m.mem);
    assert!(m.cpu <= THRESHOLD_ISSUE_CREDENTIAL_CPU,
        "issue_credential CPU regression: {} > {}", m.cpu, THRESHOLD_ISSUE_CREDENTIAL_CPU);
    assert!(m.mem <= THRESHOLD_ISSUE_CREDENTIAL_MEM,
        "issue_credential MEM regression: {} > {}", m.mem, THRESHOLD_ISSUE_CREDENTIAL_MEM);
}

#[test]
fn bench_create_slice() {
    let env = Env::default();
    let (client, _) = setup_qp(&env);
    let creator = Address::generate(&env);
    let attestor = Address::generate(&env);
    let mut attestors = Vec::new(&env);
    attestors.push_back(attestor);
    let mut weights = Vec::new(&env);
    weights.push_back(1u32);

    let m = measure(&env, || {
        client.create_slice(&creator, &attestors, &weights, &1u32);
    });

    println!("[bench_create_slice] cpu={} mem={}", m.cpu, m.mem);
    assert!(m.cpu <= THRESHOLD_CREATE_SLICE_CPU,
        "create_slice CPU regression: {} > {}", m.cpu, THRESHOLD_CREATE_SLICE_CPU);
    assert!(m.mem <= THRESHOLD_CREATE_SLICE_MEM,
        "create_slice MEM regression: {} > {}", m.mem, THRESHOLD_CREATE_SLICE_MEM);
}

#[test]
fn bench_attest() {
    let env = Env::default();
    let (client, _) = setup_qp(&env);
    let issuer = Address::generate(&env);
    let subject = Address::generate(&env);
    let attestor = Address::generate(&env);
    let meta = Bytes::from_slice(&env, b"QmBenchHash000000000000000000000000");
    let cid = client.issue_credential(&issuer, &subject, &1u32, &meta, &None);
    let mut attestors = Vec::new(&env);
    attestors.push_back(attestor.clone());
    let mut weights = Vec::new(&env);
    weights.push_back(1u32);
    let slice_id = client.create_slice(&issuer, &attestors, &weights, &1u32);

    let m = measure(&env, || {
        client.attest(&attestor, &cid, &slice_id, &None);
    });

    println!("[bench_attest] cpu={} mem={}", m.cpu, m.mem);
    assert!(m.cpu <= THRESHOLD_ATTEST_CPU,
        "attest CPU regression: {} > {}", m.cpu, THRESHOLD_ATTEST_CPU);
    assert!(m.mem <= THRESHOLD_ATTEST_MEM,
        "attest MEM regression: {} > {}", m.mem, THRESHOLD_ATTEST_MEM);
}

#[test]
fn bench_revoke_credential() {
    let env = Env::default();
    let (client, _) = setup_qp(&env);
    let issuer = Address::generate(&env);
    let subject = Address::generate(&env);
    let meta = Bytes::from_slice(&env, b"QmBenchHash000000000000000000000000");
    let cid = client.issue_credential(&issuer, &subject, &1u32, &meta, &None);

    let m = measure(&env, || {
        client.revoke_credential(&issuer, &cid);
    });

    println!("[bench_revoke_credential] cpu={} mem={}", m.cpu, m.mem);
    assert!(m.cpu <= THRESHOLD_REVOKE_CREDENTIAL_CPU,
        "revoke_credential CPU regression: {} > {}", m.cpu, THRESHOLD_REVOKE_CREDENTIAL_CPU);
    assert!(m.mem <= THRESHOLD_REVOKE_CREDENTIAL_MEM,
        "revoke_credential MEM regression: {} > {}", m.mem, THRESHOLD_REVOKE_CREDENTIAL_MEM);
}

// ── sbt_registry benchmarks ───────────────────────────────────────────────────

#[test]
fn bench_mint_sbt() {
    let env = Env::default();
    env.mock_all_auths();
    let qp_id = env.register_contract(None, QuorumProofContract);
    let qp_client = QuorumProofContractClient::new(&env, &qp_id);
    let admin = Address::generate(&env);
    qp_client.initialize(&admin);

    let (sbt_client, _) = setup_sbt(&env, &qp_id);
    let issuer = Address::generate(&env);
    let owner = Address::generate(&env);
    let meta = Bytes::from_slice(&env, b"ipfs://bench");
    let cred_id = qp_client.issue_credential(&issuer, &owner, &1u32, &meta, &None);
    let uri = Bytes::from_slice(&env, b"ipfs://QmBench");

    let m = measure(&env, || {
        sbt_client.mint(&owner, &cred_id, &uri);
    });

    println!("[bench_mint_sbt] cpu={} mem={}", m.cpu, m.mem);
    assert!(m.cpu <= THRESHOLD_MINT_SBT_CPU,
        "mint_sbt CPU regression: {} > {}", m.cpu, THRESHOLD_MINT_SBT_CPU);
    assert!(m.mem <= THRESHOLD_MINT_SBT_MEM,
        "mint_sbt MEM regression: {} > {}", m.mem, THRESHOLD_MINT_SBT_MEM);
}

#[test]
fn bench_burn_sbt() {
    let env = Env::default();
    env.mock_all_auths();
    let qp_id = env.register_contract(None, QuorumProofContract);
    let qp_client = QuorumProofContractClient::new(&env, &qp_id);
    let admin = Address::generate(&env);
    qp_client.initialize(&admin);

    let (sbt_client, _) = setup_sbt(&env, &qp_id);
    let issuer = Address::generate(&env);
    let owner = Address::generate(&env);
    let meta = Bytes::from_slice(&env, b"ipfs://bench");
    let cred_id = qp_client.issue_credential(&issuer, &owner, &1u32, &meta, &None);
    let uri = Bytes::from_slice(&env, b"ipfs://QmBench");
    let token_id = sbt_client.mint(&owner, &cred_id, &uri);

    let m = measure(&env, || {
        sbt_client.burn(&owner, &token_id);
    });

    println!("[bench_burn_sbt] cpu={} mem={}", m.cpu, m.mem);
    assert!(m.cpu <= THRESHOLD_BURN_SBT_CPU,
        "burn_sbt CPU regression: {} > {}", m.cpu, THRESHOLD_BURN_SBT_CPU);
    assert!(m.mem <= THRESHOLD_BURN_SBT_MEM,
        "burn_sbt MEM regression: {} > {}", m.mem, THRESHOLD_BURN_SBT_MEM);
}

// ── zk_verifier benchmarks ────────────────────────────────────────────────────

#[test]
fn bench_verify_claim() {
    let env = Env::default();
    env.mock_all_auths();
    let (client, admin) = setup_zk(&env);
    let qp_id = Address::generate(&env);
    let proof = Bytes::from_slice(&env, b"bench-proof");

    let m = measure(&env, || {
        client.verify_claim(&admin, &qp_id, &1u64, &ClaimType::HasDegree, &proof);
    });

    println!("[bench_verify_claim] cpu={} mem={}", m.cpu, m.mem);
    assert!(m.cpu <= THRESHOLD_VERIFY_CLAIM_CPU,
        "verify_claim CPU regression: {} > {}", m.cpu, THRESHOLD_VERIFY_CLAIM_CPU);
    assert!(m.mem <= THRESHOLD_VERIFY_CLAIM_MEM,
        "verify_claim MEM regression: {} > {}", m.mem, THRESHOLD_VERIFY_CLAIM_MEM);
}

// ── Scaling benchmarks (regression detection for N-item operations) ───────────

/// Measures how attest cost scales with attestor count in a slice.
/// Detects O(n²) regressions in attestation logic.
#[test]
fn bench_attest_scaling() {
    for n in [1u32, 5, 10] {
        let env = Env::default();
        let (client, _) = setup_qp(&env);
        let issuer = Address::generate(&env);
        let subject = Address::generate(&env);
        let meta = Bytes::from_slice(&env, b"QmBenchHash000000000000000000000000");
        let cid = client.issue_credential(&issuer, &subject, &1u32, &meta, &None);

        let mut attestors = Vec::new(&env);
        let mut weights = Vec::new(&env);
        for _ in 0..n {
            attestors.push_back(Address::generate(&env));
            weights.push_back(1u32);
        }
        let slice_id = client.create_slice(&issuer, &attestors, &weights, &1u32);
        let first_attestor = attestors.get(0).unwrap();

        let m = measure(&env, || {
            client.attest(&first_attestor, &cid, &slice_id, &None);
        });

        println!("[bench_attest_scaling n={}] cpu={} mem={}", n, m.cpu, m.mem);
        // Each attest must stay within the single-attest threshold regardless of slice size
        assert!(m.cpu <= THRESHOLD_ATTEST_CPU,
            "attest scaling CPU regression at n={}: {} > {}", n, m.cpu, THRESHOLD_ATTEST_CPU);
    }
}
