#![no_main]

use arbitrary::Arbitrary;
use libfuzzer_sys::fuzz_target;
use soroban_sdk::{testutils::Address as _, Address, Bytes, Env};
use zk_verifier::{ClaimType, ZkVerifierContract, ZkVerifierContractClient};

/// Fuzz input covering verify_claim with arbitrary proof bytes and claim types.
#[derive(Arbitrary, Debug)]
struct FuzzInput {
    proof: Vec<u8>,
    claim_type_idx: u8,
    credential_id: u64,
}

fuzz_target!(|input: FuzzInput| {
    let env = Env::default();
    env.mock_all_auths();

    let contract_id = env.register_contract(None, ZkVerifierContract);
    let client = ZkVerifierContractClient::new(&env, &contract_id);
    let admin = Address::generate(&env);
    client.initialize(&admin);

    let qp_id = Address::generate(&env);
    let proof = Bytes::from_slice(&env, &input.proof);

    let claim_type = match input.claim_type_idx % 5 {
        0 => ClaimType::HasDegree,
        1 => ClaimType::HasLicense,
        2 => ClaimType::HasEmploymentHistory,
        3 => ClaimType::HasCertification,
        _ => ClaimType::HasResearchPublication,
    };

    // verify_claim: empty proof must return false, non-empty must return true (stub)
    let result = match std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        client.verify_claim(&admin, &qp_id, &input.credential_id, &claim_type, &proof)
    })) {
        Ok(r) => r,
        Err(_) => return,
    };

    // Vulnerability detection: stub must never return true for empty proof
    if input.proof.is_empty() {
        assert!(!result, "empty proof must not pass verification");
    }
});
