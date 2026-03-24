#[test]
#[should_panic(expected = "credential already revoked")]
fn test_double_revocation_rejection() {
    use soroban_sdk::testutils::{Address as _};
    use soroban_sdk::{Bytes, Env};
    
    let env = Env::default();
    env.mock_all_auths();
    let contract_id = env.register_contract(None, quorum_proof::QuorumProofContract);
    let client = quorum_proof::QuorumProofContractClient::new(&env, &contract_id);

    let issuer = Address::generate(&env);
    let subject = Address::generate(&env);
    let metadata = Bytes::from_slice(&env, b"ipfs://QmTest");
    let id = client.issue_credential(&issuer, &subject, &1u32, &metadata);

    // First revocation should succeed
    client.revoke_credential(&issuer, &id);

    // Second revocation should panic
    client.revoke_credential(&issuer, &id);
}
