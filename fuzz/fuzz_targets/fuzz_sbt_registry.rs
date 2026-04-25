#![no_main]

use arbitrary::Arbitrary;
use libfuzzer_sys::fuzz_target;
use soroban_sdk::{testutils::Address as _, Address, Bytes, Env};
use quorum_proof::{QuorumProofContract, QuorumProofContractClient};
use sbt_registry::{SbtRegistryContract, SbtRegistryContractClient};

/// Fuzz input covering mint and burn paths.
#[derive(Arbitrary, Debug)]
struct FuzzInput {
    metadata_uri: Vec<u8>,
    burn_after_mint: bool,
}

fuzz_target!(|input: FuzzInput| {
    let env = Env::default();
    env.mock_all_auths();

    // Set up quorum_proof dependency
    let qp_id = env.register_contract(None, QuorumProofContract);
    let qp_client = QuorumProofContractClient::new(&env, &qp_id);
    let admin = Address::generate(&env);
    qp_client.initialize(&admin);

    // Set up sbt_registry
    let sbt_id = env.register_contract(None, SbtRegistryContract);
    let sbt_client = SbtRegistryContractClient::new(&env, &sbt_id);
    sbt_client.initialize(&admin, &qp_id);

    let issuer = Address::generate(&env);
    let owner = Address::generate(&env);
    let meta = Bytes::from_slice(&env, b"ipfs://fuzz");

    // Issue a credential first
    let cred_id = match std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        qp_client.issue_credential(&issuer, &owner, &1u32, &meta, &None)
    })) {
        Ok(id) => id,
        Err(_) => return,
    };

    let uri_bytes = if input.metadata_uri.is_empty() {
        b"ipfs://QmFuzz".to_vec()
    } else {
        input.metadata_uri.clone()
    };
    let uri = Bytes::from_slice(&env, &uri_bytes);

    // mint — should succeed for valid credential
    let token_id = match std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        sbt_client.mint(&owner, &cred_id, &uri)
    })) {
        Ok(id) => id,
        Err(_) => return,
    };

    // burn — optionally burn the minted token
    if input.burn_after_mint {
        let _ = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            sbt_client.burn(&owner, &token_id)
        }));
    }
});
