#![no_std]
use soroban_sdk::{contract, contractimpl, contracttype, Address, Bytes, Env};

/// Supported claim types for ZK verification.
#[contracttype]
#[derive(Clone, PartialEq)]
pub enum ClaimType {
    HasDegree,
    HasLicense,
    HasEmploymentHistory,
    HasCertification,
    HasResearchPublication,
}

#[contracttype]
#[derive(Clone)]
pub struct ProofRequest {
    pub credential_id: u64,
    pub claim_type: ClaimType,
    pub nonce: u64,
}

#[contract]
pub struct ZkVerifierContract;

#[contractimpl]
impl ZkVerifierContract {
    /// Generate a proof request for a given credential and claim type.
    pub fn generate_proof_request(
        env: Env,
        credential_id: u64,
        claim_type: ClaimType,
    ) -> ProofRequest {
        let nonce = env.ledger().sequence() as u64;
        ProofRequest {
            credential_id,
            claim_type,
            nonce,
        }
    }

    /// Verify a ZK proof for a claim.
    ///
    /// # ⚠️ STUB IMPLEMENTATION — NOT PRODUCTION READY ⚠️
    ///
    /// This function is a placeholder. It accepts any non-empty `Bytes` value as a
    /// valid proof and provides **zero cryptographic security or privacy guarantees**.
    ///
    /// This function is gated behind admin authorization until real ZK verification
    /// is implemented in v1.1. Only the admin address stored at initialization may
    /// invoke this function.
    ///
    /// Tracking issue: implement real ZK proof verification (Groth16 / PLONK on Soroban).
    pub fn verify_claim(
        env: Env,
        admin: Address,
        _quorum_proof_id: Address,
        _credential_id: u64,
        _claim_type: ClaimType,
        proof: Bytes,
    ) -> bool {
        // Admin gate: real ZK is not implemented; restrict to authorized callers only.
        admin.require_auth();
        let stored_admin: Address = env.storage().instance()
            .get(&DataKey::Admin)
            .expect("not initialized");
        assert!(stored_admin == admin, "unauthorized");

        // STUB: not production-ready. Any non-empty proof passes.
        // No cryptographic verification is performed — no Groth16, no PLONK, no privacy guarantees.
        // Tracked for real implementation in v1.1: https://github.com/cryptonautt/QuorumProof/issues
        !proof.is_empty()
    }

    /// Set the admin address once after deployment.
    pub fn initialize(env: Env, admin: Address) {
        assert!(!env.storage().instance().has(&DataKey::Admin), "already initialized");
        env.storage().instance().set(&DataKey::Admin, &admin);
    }

    /// Admin-only contract upgrade to new WASM.
    pub fn upgrade(env: Env, admin: Address, new_wasm_hash: soroban_sdk::BytesN<32>) {
        admin.require_auth();
        env.deployer().update_current_contract_wasm(new_wasm_hash);
    }
}

#[contracttype]
#[derive(Clone)]
pub enum DataKey {
    Admin,
}

#[cfg(test)]
mod tests {
    use super::*;
    use soroban_sdk::testutils::Address as _;
    use soroban_sdk::{Bytes, Env};

    fn setup(env: &Env) -> (ZkVerifierContractClient, Address) {
        let contract_id = env.register_contract(None, ZkVerifierContract);
        let client = ZkVerifierContractClient::new(env, &contract_id);
        let admin = Address::generate(env);
        client.initialize(&admin);
        (client, admin)
    }

    #[test]
    fn test_verify_claim_degree_success() {
        let env = Env::default();
        env.mock_all_auths();
        let (client, admin) = setup(&env);
        let qp_id = Address::generate(&env);

        let proof = Bytes::from_slice(&env, b"valid-proof");
        assert!(client.verify_claim(&admin, &qp_id, &1u64, &ClaimType::HasDegree, &proof));
    }

    #[test]
    fn test_verify_claim_revoked_fails() {
        let env = Env::default();
        env.mock_all_auths();
        let (client, admin) = setup(&env);
        let qp_id = Address::generate(&env);

        let proof = Bytes::new(&env);
        assert!(!client.verify_claim(&admin, &qp_id, &1u64, &ClaimType::HasDegree, &proof));
    }

    #[test]
    fn test_verify_claim_wrong_type_fails() {
        let env = Env::default();
        env.mock_all_auths();
        let (client, admin) = setup(&env);
        let qp_id = Address::generate(&env);

        let proof = Bytes::new(&env);
        assert!(!client.verify_claim(&admin, &qp_id, &1u64, &ClaimType::HasLicense, &proof));
    }

    /// Documents stub behavior: any non-empty bytes passes, empty bytes fails.
    /// This test must be removed when real ZK is implemented.
    #[test]
    fn test_verify_claim_stub_behavior_any_nonempty_bytes_passes() {
        let env = Env::default();
        env.mock_all_auths();
        let (client, admin) = setup(&env);
        let qp_id = Address::generate(&env);

        // STUB: these are not real ZK proofs — they pass only because they are non-empty.
        let fake_proof = Bytes::from_slice(&env, b"not-a-real-zk-proof");
        assert!(client.verify_claim(&admin, &qp_id, &1u64, &ClaimType::HasDegree, &fake_proof),
            "STUB: non-empty bytes should pass until real ZK is implemented");

        let empty_proof = Bytes::new(&env);
        assert!(!client.verify_claim(&admin, &qp_id, &1u64, &ClaimType::HasDegree, &empty_proof),
            "STUB: empty bytes should fail");
    }

    /// Non-admin callers must be rejected.
    #[test]
    #[should_panic]
    fn test_verify_claim_non_admin_panics() {
        let env = Env::default();
        env.mock_all_auths_allowing_non_root_auth();
        let (client, _admin) = setup(&env);
        let non_admin = Address::generate(&env);
        let qp_id = Address::generate(&env);
        let proof = Bytes::from_slice(&env, b"proof");
        // non_admin is not the stored admin — should panic with "unauthorized"
        client.verify_claim(&non_admin, &qp_id, &1u64, &ClaimType::HasDegree, &proof);
    }

    #[test]
    #[should_panic]
    fn test_upgrade_success() {
        let env = Env::default();
        env.mock_all_auths();
        let contract_id = env.register_contract(None, ZkVerifierContract);
        let client = ZkVerifierContractClient::new(&env, &contract_id);

        let admin = Address::generate(&env);
        let wasm_hash = soroban_sdk::BytesN::from_array(&env, &[0u8; 32]);
        client.upgrade(&admin, &wasm_hash);
    }

    #[test]
    fn test_generate_proof_request() {
        let env = Env::default();
        let contract_id = env.register_contract(None, ZkVerifierContract);
        let client = ZkVerifierContractClient::new(&env, &contract_id);
        let req = client.generate_proof_request(&42u64, &ClaimType::HasEmploymentHistory);
        assert_eq!(req.credential_id, 42u64);
    }

    #[test]
    fn test_verify_claim_certification_success() {
        let env = Env::default();
        env.mock_all_auths();
        let (client, admin) = setup(&env);
        let qp_id = Address::generate(&env);

        let proof = Bytes::from_slice(&env, b"valid-proof");
        assert!(client.verify_claim(&admin, &qp_id, &1u64, &ClaimType::HasCertification, &proof));
    }

    #[test]
    fn test_verify_claim_research_publication_success() {
        let env = Env::default();
        env.mock_all_auths();
        let (client, admin) = setup(&env);
        let qp_id = Address::generate(&env);

        let proof = Bytes::from_slice(&env, b"valid-proof");
        assert!(client.verify_claim(&admin, &qp_id, &1u64, &ClaimType::HasResearchPublication, &proof));
    }
}
