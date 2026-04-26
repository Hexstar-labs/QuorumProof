#[cfg(test)]
mod tests {
    use super::*;
    use soroban_sdk::testutils::{Address as _, Ledger as _, LedgerInfo};
    use soroban_sdk::{vec, Env};

    // ── Feature #355: Proof Expiry Tests ─────────────────────────────────────

    #[test]
    fn test_is_proof_expired() {
        let env = Env::default();
        env.mock_all_auths();
        
        let contract_id = env.register_contract(None, QuorumProofContract);
        let client = QuorumProofContractClient::new(&env, &contract_id);
        
        let admin = Address::generate(&env);
        let issuer = Address::generate(&env);
        let subject = Address::generate(&env);
        
        client.initialize(&admin);
        
        let metadata_hash = soroban_sdk::Bytes::from_array(&env, &[1u8; 32]);
        let cred_id = client.issue_credential(&issuer, &subject, &1u32, &metadata_hash, &None);
        
        // Set current time to 1000
        env.ledger().with_mut(|li| {
            li.timestamp = 1000;
        });
        
        // Proof expires at 2000 - should not be expired
        assert_eq!(client.is_proof_expired(&cred_id, &2000u64), false);
        
        // Move time forward to 2000
        env.ledger().with_mut(|li| {
            li.timestamp = 2000;
        });
        
        // Proof should now be expired
        assert_eq!(client.is_proof_expired(&cred_id, &2000u64), true);
    }

    #[test]
    fn test_renew_proof() {
        let env = Env::default();
        env.mock_all_auths();
        
        let contract_id = env.register_contract(None, QuorumProofContract);
        let client = QuorumProofContractClient::new(&env, &contract_id);
        
        let admin = Address::generate(&env);
        let issuer = Address::generate(&env);
        let subject = Address::generate(&env);
        
        client.initialize(&admin);
        
        env.ledger().with_mut(|li| {
            li.timestamp = 1000;
        });
        
        let metadata_hash = soroban_sdk::Bytes::from_array(&env, &[1u8; 32]);
        let cred_id = client.issue_credential(&issuer, &subject, &1u32, &metadata_hash, &None);
        
        // Renew proof to expire at 5000
        let new_expiry = client.renew_proof(&issuer, &cred_id, &5000u64);
        assert_eq!(new_expiry, 5000u64);
        
        // Verify proof is not expired at current time
        assert_eq!(client.is_proof_expired(&cred_id, &new_expiry), false);
    }

    #[test]
    #[should_panic(expected = "only the issuer can renew proofs")]
    fn test_renew_proof_unauthorized() {
        let env = Env::default();
        env.mock_all_auths();
        
        let contract_id = env.register_contract(None, QuorumProofContract);
        let client = QuorumProofContractClient::new(&env, &contract_id);
        
        let admin = Address::generate(&env);
        let issuer = Address::generate(&env);
        let subject = Address::generate(&env);
        let stranger = Address::generate(&env);
        
        client.initialize(&admin);
        
        env.ledger().with_mut(|li| {
            li.timestamp = 1000;
        });
        
        let metadata_hash = soroban_sdk::Bytes::from_array(&env, &[1u8; 32]);
        let cred_id = client.issue_credential(&issuer, &subject, &1u32, &metadata_hash, &None);
        
        // Stranger tries to renew proof - should panic
        client.renew_proof(&stranger, &cred_id, &5000u64);
    }

    // ── Feature #356: Batch Proof Verification Tests ─────────────────────────

    #[test]
    fn test_batch_verify_proofs() {
        let env = Env::default();
        env.mock_all_auths();
        
        let contract_id = env.register_contract(None, QuorumProofContract);
        let client = QuorumProofContractClient::new(&env, &contract_id);
        
        let admin = Address::generate(&env);
        let issuer = Address::generate(&env);
        let subject = Address::generate(&env);
        let attestor1 = Address::generate(&env);
        let attestor2 = Address::generate(&env);
        
        client.initialize(&admin);
        
        env.ledger().with_mut(|li| {
            li.timestamp = 1000;
        });
        
        // Create credentials
        let metadata_hash = soroban_sdk::Bytes::from_array(&env, &[1u8; 32]);
        let cred_id1 = client.issue_credential(&issuer, &subject, &1u32, &metadata_hash, &None);
        let cred_id2 = client.issue_credential(&issuer, &subject, &2u32, &metadata_hash, &None);
        
        // Create slice
        let attestors = vec![&env, attestor1.clone(), attestor2.clone()];
        let weights = vec![&env, 50u32, 50u32];
        let slice_id = client.create_slice(&issuer, &attestors, &weights, &50u32);
        
        // Attest credentials
        client.attest(&attestor1, &cred_id1, &slice_id, &true, &None);
        client.attest(&attestor2, &cred_id2, &slice_id, &true, &None);
        
        // Batch verify
        let credential_ids = vec![&env, cred_id1, cred_id2];
        let slice_ids = vec![&env, slice_id, slice_id];
        let proof_expires_at_list = vec![&env, 2000u64, 3000u64];
        
        let results = client.batch_verify_proofs(&credential_ids, &slice_ids, &proof_expires_at_list);
        
        assert_eq!(results.len(), 2);
        
        // First credential should be valid and not expired
        let (id1, valid1, expired1) = results.get(0).unwrap();
        assert_eq!(id1, cred_id1);
        assert_eq!(valid1, true);
        assert_eq!(expired1, false);
        
        // Second credential should be valid and not expired
        let (id2, valid2, expired2) = results.get(1).unwrap();
        assert_eq!(id2, cred_id2);
        assert_eq!(valid2, true);
        assert_eq!(expired2, false);
    }

    #[test]
    fn test_batch_verify_proofs_with_expiry() {
        let env = Env::default();
        env.mock_all_auths();
        
        let contract_id = env.register_contract(None, QuorumProofContract);
        let client = QuorumProofContractClient::new(&env, &contract_id);
        
        let admin = Address::generate(&env);
        let issuer = Address::generate(&env);
        let subject = Address::generate(&env);
        
        client.initialize(&admin);
        
        env.ledger().with_mut(|li| {
            li.timestamp = 2500;
        });
        
        let metadata_hash = soroban_sdk::Bytes::from_array(&env, &[1u8; 32]);
        let cred_id1 = client.issue_credential(&issuer, &subject, &1u32, &metadata_hash, &None);
        let cred_id2 = client.issue_credential(&issuer, &subject, &2u32, &metadata_hash, &None);
        
        let credential_ids = vec![&env, cred_id1, cred_id2];
        let slice_ids = vec![&env, 1u64, 1u64];
        let proof_expires_at_list = vec![&env, 2000u64, 3000u64]; // First expired, second not
        
        let results = client.batch_verify_proofs(&credential_ids, &slice_ids, &proof_expires_at_list);
        
        // First proof should be expired
        let (_, _, expired1) = results.get(0).unwrap();
        assert_eq!(expired1, true);
        
        // Second proof should not be expired
        let (_, _, expired2) = results.get(1).unwrap();
        assert_eq!(expired2, false);
    }

    // ── Feature #357: Claim Type Validation Tests ────────────────────────────

    #[test]
    fn test_is_claim_type_supported() {
        let env = Env::default();
        let contract_id = env.register_contract(None, QuorumProofContract);
        let client = QuorumProofContractClient::new(&env, &contract_id);
        
        // All standard claim types should be supported
        assert_eq!(client.is_claim_type_supported(&ClaimType::Degree), true);
        assert_eq!(client.is_claim_type_supported(&ClaimType::License), true);
        assert_eq!(client.is_claim_type_supported(&ClaimType::Employment), true);
        assert_eq!(client.is_claim_type_supported(&ClaimType::Age), true);
        assert_eq!(client.is_claim_type_supported(&ClaimType::Citizenship), true);
        assert_eq!(client.is_claim_type_supported(&ClaimType::Custom), true);
    }

    #[test]
    fn test_get_supported_claim_types() {
        let env = Env::default();
        let contract_id = env.register_contract(None, QuorumProofContract);
        let client = QuorumProofContractClient::new(&env, &contract_id);
        
        let types = client.get_supported_claim_types();
        assert_eq!(types.len(), 6);
        
        // Verify all expected types are present
        assert!(types.iter().any(|t| t == ClaimType::Degree));
        assert!(types.iter().any(|t| t == ClaimType::License));
        assert!(types.iter().any(|t| t == ClaimType::Employment));
        assert!(types.iter().any(|t| t == ClaimType::Age));
        assert!(types.iter().any(|t| t == ClaimType::Citizenship));
        assert!(types.iter().any(|t| t == ClaimType::Custom));
    }

    #[test]
    fn test_validate_claim_types() {
        let env = Env::default();
        let contract_id = env.register_contract(None, QuorumProofContract);
        let client = QuorumProofContractClient::new(&env, &contract_id);
        
        // Valid claim types
        let valid_types = vec![&env, ClaimType::Degree, ClaimType::License];
        assert_eq!(client.validate_claim_types(&valid_types), true);
        
        // All supported types
        let all_types = vec![
            &env,
            ClaimType::Degree,
            ClaimType::License,
            ClaimType::Employment,
            ClaimType::Age,
            ClaimType::Citizenship,
            ClaimType::Custom,
        ];
        assert_eq!(client.validate_claim_types(&all_types), true);
    }

    // ── Feature #359: Credential Search Tests ────────────────────────────────

    #[test]
    fn test_search_credentials_by_subject() {
        let env = Env::default();
        env.mock_all_auths();
        
        let contract_id = env.register_contract(None, QuorumProofContract);
        let client = QuorumProofContractClient::new(&env, &contract_id);
        
        let admin = Address::generate(&env);
        let issuer = Address::generate(&env);
        let subject1 = Address::generate(&env);
        let subject2 = Address::generate(&env);
        
        client.initialize(&admin);
        
        let metadata_hash = soroban_sdk::Bytes::from_array(&env, &[1u8; 32]);
        
        // Issue credentials to different subjects
        let cred_id1 = client.issue_credential(&issuer, &subject1, &1u32, &metadata_hash, &None);
        let cred_id2 = client.issue_credential(&issuer, &subject1, &2u32, &metadata_hash, &None);
        let cred_id3 = client.issue_credential(&issuer, &subject2, &1u32, &metadata_hash, &None);
        
        // Search for subject1's credentials
        let results = client.search_credentials(
            &Some(subject1.clone()),
            &None,
            &None,
            &None,
            &None,
            &1u32,
            &10u32,
        );
        
        assert_eq!(results.len(), 2);
        assert!(results.iter().any(|id| id == cred_id1));
        assert!(results.iter().any(|id| id == cred_id2));
    }

    #[test]
    fn test_search_credentials_by_issuer() {
        let env = Env::default();
        env.mock_all_auths();
        
        let contract_id = env.register_contract(None, QuorumProofContract);
        let client = QuorumProofContractClient::new(&env, &contract_id);
        
        let admin = Address::generate(&env);
        let issuer1 = Address::generate(&env);
        let issuer2 = Address::generate(&env);
        let subject = Address::generate(&env);
        
        client.initialize(&admin);
        
        let metadata_hash = soroban_sdk::Bytes::from_array(&env, &[1u8; 32]);
        
        // Issue credentials from different issuers
        let cred_id1 = client.issue_credential(&issuer1, &subject, &1u32, &metadata_hash, &None);
        let cred_id2 = client.issue_credential(&issuer2, &subject, &1u32, &metadata_hash, &None);
        
        // Search for issuer1's credentials
        let results = client.search_credentials(
            &None,
            &Some(issuer1.clone()),
            &None,
            &None,
            &None,
            &1u32,
            &10u32,
        );
        
        assert_eq!(results.len(), 1);
        assert_eq!(results.get(0).unwrap(), cred_id1);
    }

    #[test]
    fn test_search_credentials_by_type() {
        let env = Env::default();
        env.mock_all_auths();
        
        let contract_id = env.register_contract(None, QuorumProofContract);
        let client = QuorumProofContractClient::new(&env, &contract_id);
        
        let admin = Address::generate(&env);
        let issuer = Address::generate(&env);
        let subject = Address::generate(&env);
        
        client.initialize(&admin);
        
        let metadata_hash = soroban_sdk::Bytes::from_array(&env, &[1u8; 32]);
        
        // Issue credentials of different types
        let cred_id1 = client.issue_credential(&issuer, &subject, &1u32, &metadata_hash, &None);
        let cred_id2 = client.issue_credential(&issuer, &subject, &2u32, &metadata_hash, &None);
        let cred_id3 = client.issue_credential(&issuer, &subject, &1u32, &metadata_hash, &None);
        
        // Search for type 1 credentials
        let results = client.search_credentials(
            &None,
            &None,
            &Some(1u32),
            &None,
            &None,
            &1u32,
            &10u32,
        );
        
        assert_eq!(results.len(), 2);
        assert!(results.iter().any(|id| id == cred_id1));
        assert!(results.iter().any(|id| id == cred_id3));
    }

    #[test]
    fn test_search_credentials_with_pagination() {
        let env = Env::default();
        env.mock_all_auths();
        
        let contract_id = env.register_contract(None, QuorumProofContract);
        let client = QuorumProofContractClient::new(&env, &contract_id);
        
        let admin = Address::generate(&env);
        let issuer = Address::generate(&env);
        let subject = Address::generate(&env);
        
        client.initialize(&admin);
        
        let metadata_hash = soroban_sdk::Bytes::from_array(&env, &[1u8; 32]);
        
        // Issue 5 credentials
        for i in 1..=5 {
            client.issue_credential(&issuer, &subject, &i, &metadata_hash, &None);
        }
        
        // Get first page (2 items)
        let page1 = client.search_credentials(
            &Some(subject.clone()),
            &None,
            &None,
            &None,
            &None,
            &1u32,
            &2u32,
        );
        assert_eq!(page1.len(), 2);
        
        // Get second page (2 items)
        let page2 = client.search_credentials(
            &Some(subject.clone()),
            &None,
            &None,
            &None,
            &None,
            &2u32,
            &2u32,
        );
        assert_eq!(page2.len(), 2);
        
        // Get third page (1 item)
        let page3 = client.search_credentials(
            &Some(subject.clone()),
            &None,
            &None,
            &None,
            &None,
            &3u32,
            &2u32,
        );
        assert_eq!(page3.len(), 1);
    }

    #[test]
    fn test_count_credentials() {
        let env = Env::default();
        env.mock_all_auths();
        
        let contract_id = env.register_contract(None, QuorumProofContract);
        let client = QuorumProofContractClient::new(&env, &contract_id);
        
        let admin = Address::generate(&env);
        let issuer = Address::generate(&env);
        let subject1 = Address::generate(&env);
        let subject2 = Address::generate(&env);
        
        client.initialize(&admin);
        
        let metadata_hash = soroban_sdk::Bytes::from_array(&env, &[1u8; 32]);
        
        // Issue credentials
        client.issue_credential(&issuer, &subject1, &1u32, &metadata_hash, &None);
        client.issue_credential(&issuer, &subject1, &2u32, &metadata_hash, &None);
        client.issue_credential(&issuer, &subject2, &1u32, &metadata_hash, &None);
        
        // Count all credentials
        let total = client.count_credentials(&None, &None, &None);
        assert_eq!(total, 3);
        
        // Count subject1's credentials
        let subject1_count = client.count_credentials(&Some(subject1), &None, &None);
        assert_eq!(subject1_count, 2);
        
        // Count type 1 credentials
        let type1_count = client.count_credentials(&None, &None, &Some(1u32));
        assert_eq!(type1_count, 2);
    }

    #[test]
    fn test_search_credentials_combined_filters() {
        let env = Env::default();
        env.mock_all_auths();
        
        let contract_id = env.register_contract(None, QuorumProofContract);
        let client = QuorumProofContractClient::new(&env, &contract_id);
        
        let admin = Address::generate(&env);
        let issuer1 = Address::generate(&env);
        let issuer2 = Address::generate(&env);
        let subject = Address::generate(&env);
        
        client.initialize(&admin);
        
        let metadata_hash = soroban_sdk::Bytes::from_array(&env, &[1u8; 32]);
        
        // Issue various credentials
        client.issue_credential(&issuer1, &subject, &1u32, &metadata_hash, &None);
        client.issue_credential(&issuer1, &subject, &2u32, &metadata_hash, &None);
        client.issue_credential(&issuer2, &subject, &1u32, &metadata_hash, &None);
        
        // Search for subject's type 1 credentials from issuer1
        let results = client.search_credentials(
            &Some(subject.clone()),
            &Some(issuer1.clone()),
            &Some(1u32),
            &None,
            &None,
            &1u32,
            &10u32,
        );
        
        assert_eq!(results.len(), 1);
    }
}
