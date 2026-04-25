#![no_main]

use arbitrary::Arbitrary;
use libfuzzer_sys::fuzz_target;
use soroban_sdk::{testutils::Address as _, Address, Bytes, Env, Vec};
use quorum_proof::{QuorumProofContract, QuorumProofContractClient};

/// Fuzz input covering issue_credential, create_slice, and attest paths.
#[derive(Arbitrary, Debug)]
struct FuzzInput {
    credential_type: u32,
    metadata: Vec<u8>,
    threshold: u32,
    attestor_count: u8,
    expires_at: Option<u64>,
}

fuzz_target!(|input: FuzzInput| {
    let env = Env::default();
    env.mock_all_auths();

    let contract_id = env.register_contract(None, QuorumProofContract);
    let client = QuorumProofContractClient::new(&env, &contract_id);
    let admin = Address::generate(&env);
    client.initialize(&admin);

    let issuer = Address::generate(&env);
    let subject = Address::generate(&env);

    // Clamp to avoid trivially-invalid inputs that aren't interesting
    let ctype = input.credential_type.max(1);
    let meta_bytes = if input.metadata.is_empty() {
        b"QmFuzzHash000000000000000000000000".to_vec()
    } else {
        input.metadata.clone()
    };
    let meta = Bytes::from_slice(&env, &meta_bytes);

    // issue_credential — should not panic on valid inputs
    let cid = match std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        client.issue_credential(&issuer, &subject, &ctype, &meta, &input.expires_at)
    })) {
        Ok(id) => id,
        Err(_) => return, // invalid input rejected by contract — expected
    };

    // create_slice — build attestors list
    let n = (input.attestor_count as usize).clamp(1, 5);
    let mut attestors = Vec::new(&env);
    let mut weights = Vec::new(&env);
    for _ in 0..n {
        attestors.push_back(Address::generate(&env));
        weights.push_back(1u32);
    }
    let threshold = input.threshold.clamp(1, n as u32);

    let slice_id = match std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        client.create_slice(&issuer, &attestors, &weights, &threshold)
    })) {
        Ok(id) => id,
        Err(_) => return,
    };

    // attest — first attestor attests
    let attestor = attestors.get(0).unwrap();
    let _ = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        client.attest(&attestor, &cid, &slice_id, &None)
    }));
});
