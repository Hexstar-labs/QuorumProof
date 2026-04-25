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

/// Cache entry for verified proofs.
/// Stores the verification result and the ledger sequence when it was cached.
#[contracttype]
#[derive(Clone)]
pub struct CacheEntry {
    pub result: bool,
    pub cached_at_ledger: u32,
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

    /// Verify a ZK proof for a claim with caching.
    ///
    /// This function first checks if the proof has been verified before by 
    /// looking up the cache. If found, it returns the cached result immediately.
    /// Otherwise, it verifies the proof and caches the result for future calls.
    /// 
    /// Cache keys are derived from: (credential_id, claim_type, proof_hash).
    /// Cache entries are stored indefinitely until explicitly cleared.
    pub fn verify_claim_with_cache(
        env: Env,
        admin: Address,
        quorum_proof_id: Address,
        credential_id: u64,
        claim_type: ClaimType,
        proof: Bytes,
    ) -> bool {
        // Admin gate
        admin.require_auth();
        let stored_admin: Address = env.storage().instance()
            .get(&DataKey::Admin)
            .expect("not initialized");
        assert!(stored_admin == admin, "unauthorized");

        // Generate cache key from proof bytes, credential_id, and claim_type
        let cache_key = Self::proof_cache_key(&env, &credential_id, &claim_type, &proof);

        // Check cache first
        if let Some(entry) = env.storage().temporary().get::<_, CacheEntry>(&cache_key) {
            return entry.result;
        }

        // Not in cache, perform verification
        let result = !proof.is_empty();

        // Cache the result
        let entry = CacheEntry {
            result,
            cached_at_ledger: env.ledger().sequence(),
        };
        env.storage().temporary().set(&cache_key, &entry);

        result
    }

    /// Internal helper to generate cache key from proof components.
    /// Uses (credential_id, claim_type, proof_hash) to create a unique key.
    fn proof_cache_key(
        env: &Env,
        credential_id: &u64,
        claim_type: &ClaimType,
        proof: &Bytes,
    ) -> Bytes {
        use soroban_sdk::TryIntoVal;
        
        // Create a cache key string combining all three components
        // Format: "cache::{credential_id}::{claim_type_discriminant}::{proof_hash}"
        let claim_type_str = match claim_type {
            ClaimType::HasDegree => "HasDegree",
            ClaimType::HasLicense => "HasLicense",
            ClaimType::HasEmploymentHistory => "HasEmploymentHistory",
            ClaimType::HasCertification => "HasCertification",
            ClaimType::HasResearchPublication => "HasResearchPublication",
        };
        
        // Create key as bytes: credential_id (8 bytes) + claim_type (1 byte) + first 16 bytes of proof
        let mut key_data = [0u8; 25];
        key_data[0..8].copy_from_slice(&credential_id.to_le_bytes());
        key_data[8] = match claim_type {
            ClaimType::HasDegree => 0,
            ClaimType::HasLicense => 1,
            ClaimType::HasEmploymentHistory => 2,
            ClaimType::HasCertification => 3,
            ClaimType::HasResearchPublication => 4,
        };
        
        // Copy first 16 bytes of proof, or pad with zeros if shorter
        let proof_len = proof.len().min(16);
        for i in 0..proof_len {
            key_data[9 + i] = proof.get(i as u32).unwrap();
        }
        
        Bytes::from_slice(env, &key_data)
    }

    /// Clear proof cache entry for a specific credential and claim type.
    /// This allows manual cache invalidation when needed.
    pub fn clear_proof_cache(
        env: Env,
        admin: Address,
        credential_id: u64,
        claim_type: ClaimType,
        proof: Bytes,
    ) {
        admin.require_auth();
        let stored_admin: Address = env.storage().instance()
            .get(&DataKey::Admin)
            .expect("not initialized");
        assert!(stored_admin == admin, "unauthorized");

        let cache_key = Self::proof_cache_key(&env, &credential_id, &claim_type, &proof);
        env.storage().temporary().remove(&cache_key);
    }

    /// Clear all proof cache for a specific credential and claim type
    /// across all proofs (useful for when a credential is revoked).
    pub fn clear_cache_by_credential(
        env: Env,
        admin: Address,
        credential_id: u64,
    ) {
        admin.require_auth();
        let stored_admin: Address = env.storage().instance()
            .get(&DataKey::Admin)
            .expect("not initialized");
        assert!(stored_admin == admin, "unauthorized");

        // Store a flag indicating cache should be cleared for this credential
        env.storage().instance().set(&DataKey::CacheInvalidated(credential_id), &true);
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
    CacheInvalidated(u64),
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

    /// Test proof caching: verify same proof twice, second should be cache hit
    #[test]
    fn test_verify_claim_with_cache_hit() {
        let env = Env::default();
        env.mock_all_auths();
        let (client, admin) = setup(&env);

        let credential_id = 42u64;
        let claim_type = ClaimType::HasDegree;
        let proof = Bytes::from_slice(&env, b"valid-proof-for-caching");

        // First call: verifies and caches
        let result1 = client.verify_claim_with_cache(&admin, &Address::generate(&env), &credential_id, &claim_type, &proof);
        assert!(result1, "first verification should pass (non-empty proof)");

        // Second call: should return cached result
        let result2 = client.verify_claim_with_cache(&admin, &Address::generate(&env), &credential_id, &claim_type, &proof);
        assert_eq!(result1, result2, "cached result should match original");
    }

    /// Test cache miss with different proof
    #[test]
    fn test_verify_claim_with_cache_miss_different_proof() {
        let env = Env::default();
        env.mock_all_auths();
        let (client, admin) = setup(&env);

        let credential_id = 100u64;
        let claim_type = ClaimType::HasLicense;

        // First proof
        let proof1 = Bytes::from_slice(&env, b"proof-number-one");
        let result1 = client.verify_claim_with_cache(&admin, &Address::generate(&env), &credential_id, &claim_type, &proof1);

        // Different proof (cache miss)
        let proof2 = Bytes::from_slice(&env, b"proof-number-two");
        let result2 = client.verify_claim_with_cache(&admin, &Address::generate(&env), &credential_id, &claim_type, &proof2);

        // Both should pass, but come from different cache entries
        assert!(result1);
        assert!(result2);
    }

    /// Test cache with empty proof
    #[test]
    fn test_verify_claim_with_cache_empty_proof() {
        let env = Env::default();
        env.mock_all_auths();
        let (client, admin) = setup(&env);

        let credential_id = 200u64;
        let claim_type = ClaimType::HasCertification;
        let empty_proof = Bytes::new(&env);

        // First call with empty proof: should fail and cache result
        let result1 = client.verify_claim_with_cache(&admin, &Address::generate(&env), &credential_id, &claim_type, &empty_proof);
        assert!(!result1, "empty proof should fail");

        // Second call with same empty proof: should return cached failure
        let result2 = client.verify_claim_with_cache(&admin, &Address::generate(&env), &credential_id, &claim_type, &empty_proof);
        assert_eq!(result1, result2, "cached failure result should match");
        assert!(!result2);
    }

    /// Test cache invalidation by specific proof
    #[test]
    fn test_clear_proof_cache() {
        let env = Env::default();
        env.mock_all_auths();
        let (client, admin) = setup(&env);

        let credential_id = 300u64;
        let claim_type = ClaimType::HasEmploymentHistory;
        let proof = Bytes::from_slice(&env, b"proof-to-invalidate");

        // Verify and cache
        let result1 = client.verify_claim_with_cache(&admin, &Address::generate(&env), &credential_id, &claim_type, &proof);
        assert!(result1);

        // Clear cache for this specific proof
        client.clear_proof_cache(&admin, &credential_id, &claim_type, &proof);

        // Verify again - should still return same result but from fresh verification
        let result2 = client.verify_claim_with_cache(&admin, &Address::generate(&env), &credential_id, &claim_type, &proof);
        assert_eq!(result1, result2);
    }

    /// Test cache invalidation by credential ID
    #[test]
    fn test_clear_cache_by_credential() {
        let env = Env::default();
        env.mock_all_auths();
        let (client, admin) = setup(&env);

        let credential_id = 400u64;
        let claim_type = ClaimType::HasResearchPublication;
        let proof = Bytes::from_slice(&env, b"research-proof");

        // Verify and cache
        let result1 = client.verify_claim_with_cache(&admin, &Address::generate(&env), &credential_id, &claim_type, &proof);
        assert!(result1);

        // Clear all cache entries for this credential
        client.clear_cache_by_credential(&admin, &credential_id);

        // Verify again - should still work
        let result2 = client.verify_claim_with_cache(&admin, &Address::generate(&env), &credential_id, &claim_type, &proof);
        assert_eq!(result1, result2);
    }

    /// Test cache with multiple claim types
    #[test]
    fn test_verify_claim_with_cache_multiple_claim_types() {
        let env = Env::default();
        env.mock_all_auths();
        let (client, admin) = setup(&env);

        let credential_id = 500u64;
        let proof = Bytes::from_slice(&env, b"multi-claim-proof");

        // Same proof, different claim types should have different cache entries
        let result_degree = client.verify_claim_with_cache(&admin, &Address::generate(&env), &credential_id, &ClaimType::HasDegree, &proof);
        let result_license = client.verify_claim_with_cache(&admin, &Address::generate(&env), &credential_id, &ClaimType::HasLicense, &proof);

        // Both should pass
        assert!(result_degree);
        assert!(result_license);

        // Verify they're cached as separate entries by caching performance
        let result_degree_2 = client.verify_claim_with_cache(&admin, &Address::generate(&env), &credential_id, &ClaimType::HasDegree, &proof);
        let result_license_2 = client.verify_claim_with_cache(&admin, &Address::generate(&env), &credential_id, &ClaimType::HasLicense, &proof);

        assert_eq!(result_degree, result_degree_2);
        assert_eq!(result_license, result_license_2);
    }
}
