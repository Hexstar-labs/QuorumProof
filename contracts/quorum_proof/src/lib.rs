#![no_std]
use soroban_sdk::{contract, contractimpl, contracttype, Address, Env, String, Vec};

/// Event topic for credential revocation
const TOPIC_REVOKE: &str = "RevokeCredential";

#[contracttype]
#[derive(Clone)]
pub struct RevokeEventData {
    pub credential_id: u64,
    pub subject: Address,
}

/// TTL Strategy: Extends instance storage TTL after every write operation.
/// - STANDARD_TTL: 16_384 ledgers (~3 hours at 5s/ledger)
/// - EXTENDED_TTL: 524_288 ledgers (~4 days)
/// This ensures data persistence across typical usage while managing rent costs.
/// TTL is automatically extended on subsequent reads/bumps if needed.
const STANDARD_TTL: u32 = 16_384;
const EXTENDED_TTL: u32 = 524_288;

#[contracttype]
#[derive(Clone)]
pub enum DataKey {
    Credential(u64),
    CredentialCount,
    Slice(u64),
    SliceCount,
    Attestors(u64),
    SubjectCredentials(Address),
}

#[contracttype]
#[derive(Clone)]
pub struct Credential {
    pub id: u64,
    pub subject: Address,
    pub issuer: Address,
    pub credential_type: u32,
    pub metadata_hash: soroban_sdk::Bytes,
    pub revoked: bool,
    /// Optional Unix timestamp (seconds) after which the credential is considered expired.
    pub expires_at: Option<u64>,
}

#[contracttype]
#[derive(Clone)]
pub struct QuorumSlice {
    pub id: u64,
    pub creator: Address,
    pub attestors: Vec<Address>,
    pub threshold: u32,
}

#[contract]
pub struct QuorumProofContract;

#[contractimpl]
impl QuorumProofContract {
    /// Issue a new credential. Returns the credential ID.
    pub fn issue_credential(
        env: Env,
        issuer: Address,
        subject: Address,
        credential_type: u32,
        metadata_hash: soroban_sdk::Bytes,
        expires_at: Option<u64>,
    ) -> u64 {
        issuer.require_auth();
        let id: u64 = env
            .storage()
            .instance()
            .get(&DataKey::CredentialCount)
            .unwrap_or(0u64)
            + 1;
        let credential = Credential {
            id,
            subject,
            issuer,
            credential_type,
            metadata_hash,
            revoked: false,
            expires_at,
        };
        env.storage()
            .instance()
            .set(&DataKey::Credential(id), &credential);
        env.storage().instance().extend_ttl(STANDARD_TTL, EXTENDED_TTL);
        env.storage()
            .instance()
            .set(&DataKey::CredentialCount, &id);
        env.storage().instance().extend_ttl(STANDARD_TTL, EXTENDED_TTL);
        // Track credential ID under the subject's address for reverse lookup
        let mut subject_creds: Vec<u64> = env
            .storage()
            .instance()
            .get(&DataKey::SubjectCredentials(credential.subject.clone()))
            .unwrap_or(Vec::new(&env));
        subject_creds.push_back(id);
        env.storage()
            .instance()
            .set(&DataKey::SubjectCredentials(credential.subject), &subject_creds);
        env.storage().instance().extend_ttl(STANDARD_TTL, EXTENDED_TTL);
        id
    }

    /// Retrieve a credential by ID. Panics if the credential has expired.
    pub fn get_credential(env: Env, credential_id: u64) -> Credential {
        let credential: Credential = env
            .storage()
            .instance()
            .get(&DataKey::Credential(credential_id))
            .expect("credential not found");
        if let Some(expires_at) = credential.expires_at {
            assert!(
                env.ledger().timestamp() < expires_at,
                "credential has expired"
            );
        }
        credential
    }

    /// Return all credential IDs issued to a given subject address.
    pub fn get_credentials_by_subject(env: Env, subject: Address) -> Vec<u64> {
        env.storage()
            .instance()
            .get(&DataKey::SubjectCredentials(subject))
            .unwrap_or(Vec::new(&env))
    }

    /// Revoke a credential. Can be called by either the subject or the issuer.
    pub fn revoke_credential(env: Env, caller: Address, credential_id: u64) {
        caller.require_auth();
        let mut credential: Credential = env
            .storage()
            .instance()
            .get(&DataKey::Credential(credential_id))
            .expect("credential not found");
        assert!(
            caller == credential.subject || caller == credential.issuer,
            "only subject or issuer can revoke"
        );
        credential.revoked = true;
        env.storage()
            .instance()
            .set(&DataKey::Credential(credential_id), &credential);
        env.storage().instance().extend_ttl(STANDARD_TTL, EXTENDED_TTL);

        // Emit RevokeCredential event
        let event_data = RevokeEventData {
            credential_id,
            subject: credential.subject.clone(),
        };
        let topic = String::from_str(&env, TOPIC_REVOKE);
        let mut topics: Vec<String> = Vec::new(&env);
        topics.push_back(topic);
        env.events().publish(topics, event_data);
    }

    /// Create a quorum slice. Returns the slice ID.
    pub fn create_slice(env: Env, creator: Address, attestors: Vec<Address>, threshold: u32) -> u64 {
        creator.require_auth();
        let id: u64 = env
            .storage()
            .instance()
            .get(&DataKey::SliceCount)
            .unwrap_or(0u64)
            + 1;
        let slice = QuorumSlice {
            id,
            creator,
            attestors,
            threshold,
        };
        env.storage()
            .instance()
            .set(&DataKey::Slice(id), &slice);
        env.storage().instance().extend_ttl(STANDARD_TTL, EXTENDED_TTL);
        env.storage()
            .instance()
            .set(&DataKey::SliceCount, &id);
        env.storage().instance().extend_ttl(STANDARD_TTL, EXTENDED_TTL);
        id
    }

    /// Retrieve a quorum slice by ID.
    pub fn get_slice(env: Env, slice_id: u64) -> QuorumSlice {
        env.storage()
            .instance()
            .get(&DataKey::Slice(slice_id))
            .expect("slice not found")
    }

    /// Add a new attestor to an existing quorum slice.
    /// Only the slice creator can call this. Panics if attestor is already in the slice.
    pub fn add_attestor(env: Env, creator: Address, slice_id: u64, attestor: Address) {
        creator.require_auth();
        let mut slice: QuorumSlice = env
            .storage()
            .instance()
            .get(&DataKey::Slice(slice_id))
            .expect("slice not found");
        assert!(slice.creator == creator, "only the slice creator can add attestors");
        for a in slice.attestors.iter() {
            assert!(a != attestor, "attestor already in slice");
        }
        slice.attestors.push_back(attestor);
        env.storage()
            .instance()
            .set(&DataKey::Slice(slice_id), &slice);
        env.storage().instance().extend_ttl(STANDARD_TTL, EXTENDED_TTL);
    }

    /// Attest a credential using a quorum slice.
    pub fn attest(env: Env, attestor: Address, credential_id: u64, slice_id: u64) {
        attestor.require_auth();
        let slice: QuorumSlice = env
            .storage()
            .instance()
            .get(&DataKey::Slice(slice_id))
            .expect("slice not found");
        // Verify attestor is in the slice
        let mut found = false;
        for a in slice.attestors.iter() {
            if a == attestor {
                found = true;
                break;
            }
        }
        assert!(found, "attestor not in slice");

        let mut attestors: Vec<Address> = env
            .storage()
            .instance()
            .get(&DataKey::Attestors(credential_id))
            .unwrap_or(Vec::new(&env));
        attestors.push_back(attestor);
        env.storage()
            .instance()
            .set(&DataKey::Attestors(credential_id), &attestors);
        env.storage().instance().extend_ttl(STANDARD_TTL, EXTENDED_TTL);
    }

    /// Check if a credential has met its quorum threshold.
    /// Returns false if the credential is expired.
    pub fn is_attested(env: Env, credential_id: u64, slice_id: u64) -> bool {
        let credential: Credential = env
            .storage()
            .instance()
            .get(&DataKey::Credential(credential_id))
            .expect("credential not found");
        if let Some(expires_at) = credential.expires_at {
            if env.ledger().timestamp() >= expires_at {
                return false;
            }
        }
        let slice: QuorumSlice = env
            .storage()
            .instance()
            .get(&DataKey::Slice(slice_id))
            .expect("slice not found");
        let attestors: Vec<Address> = env
            .storage()
            .instance()
            .get(&DataKey::Attestors(credential_id))
            .unwrap_or(Vec::new(&env));
        attestors.len() >= slice.threshold
    }

    /// Returns true if the credential exists and its expiry timestamp has passed.
    pub fn is_expired(env: Env, credential_id: u64) -> bool {
        let credential: Credential = env
            .storage()
            .instance()
            .get(&DataKey::Credential(credential_id))
            .expect("credential not found");
        match credential.expires_at {
            Some(expires_at) => env.ledger().timestamp() >= expires_at,
            None => false,
        }
    }

    /// Get all attestors for a credential.
    pub fn get_attestors(env: Env, credential_id: u64) -> Vec<Address> {
        env.storage()
            .instance()
            .get(&DataKey::Attestors(credential_id))
            .unwrap_or(Vec::new(&env))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use soroban_sdk::testutils::{Address as _, Ledger as _, LedgerInfo};
    use soroban_sdk::{Bytes, Env};

    fn set_ledger_timestamp(env: &Env, timestamp: u64) {
        env.ledger().set(LedgerInfo {
            timestamp,
            protocol_version: 20,
            sequence_number: 100,
            network_id: Default::default(),
            base_reserve: 10,
            min_persistent_entry_ttl: 4096,
            min_temp_entry_ttl: 16,
            max_entry_ttl: 6_312_000,
        });
    }

    #[test]
    fn test_storage_persists_across_ledgers() {
        let env = Env::default();
        env.mock_all_auths();
        let contract_id = env.register_contract(None, QuorumProofContract);
        let client = QuorumProofContractClient::new(&env, &contract_id);

        let issuer = Address::generate(&env);
        let subject = Address::generate(&env);
        let metadata = Bytes::from_slice(&env, b"ipfs://QmTest");
        let id = client.issue_credential(&issuer, &subject, &1u32, &metadata);

        let id = client.issue_credential(&issuer, &subject, &1u32, &metadata, &None);

        // Advance ledger sequence by 20_000 ledgers (beyond default eviction TTL)
        env.ledger().set(LedgerInfo {
            timestamp: 1_000_000,
            protocol_version: 20,
            sequence_number: 20_000,
            network_id: Default::default(),
            base_reserve: 10,
            min_persistent_entry_ttl: 4096,
            min_temp_entry_ttl: 16,
            max_entry_ttl: 6_312_000,
        });

        // Verify data still accessible
        let cred = client.get_credential(&id);
        assert_eq!(cred.id, id);
        assert_eq!(cred.subject, subject);
        assert!(!cred.revoked);
    }

    #[test]
    fn test_issue_and_get_credential() {
        let env = Env::default();
        env.mock_all_auths();
        let contract_id = env.register_contract(None, QuorumProofContract);
        let client = QuorumProofContractClient::new(&env, &contract_id);

        let issuer = Address::generate(&env);
        let subject = Address::generate(&env);
        let metadata = Bytes::from_slice(&env, b"ipfs://QmTest");
        let id = client.issue_credential(&issuer, &subject, &1u32, &metadata);
        let id = client.issue_credential(&issuer, &subject, &1u32, &metadata, &None);
        assert_eq!(id, 1);

        let cred = client.get_credential(&id);
        assert_eq!(cred.subject, subject);
        assert_eq!(cred.issuer, issuer);
        assert!(!cred.revoked);
        assert_eq!(cred.expires_at, None);
    }

    #[test]
    fn test_quorum_slice_and_attestation() {
        let env = Env::default();
        env.mock_all_auths();
        let contract_id = env.register_contract(None, QuorumProofContract);
        let client = QuorumProofContractClient::new(&env, &contract_id);

        let issuer = Address::generate(&env);
        let subject = Address::generate(&env);
        let attestor1 = Address::generate(&env);
        let attestor2 = Address::generate(&env);

        let metadata = Bytes::from_slice(&env, b"ipfs://QmTest");
        let cred_id = client.issue_credential(&issuer, &subject, &1u32, &metadata);
        let cred_id = client.issue_credential(&issuer, &subject, &1u32, &metadata, &None);

        let mut attestors = soroban_sdk::Vec::new(&env);
        attestors.push_back(attestor1.clone());
        attestors.push_back(attestor2.clone());
        let slice_id = client.create_slice(&attestors, &2u32);
        let slice_id = client.create_slice(&issuer, &attestors, &2u32);

        assert!(!client.is_attested(&cred_id, &slice_id));
        client.attest(&attestor1, &cred_id, &slice_id);
        assert!(!client.is_attested(&cred_id, &slice_id));
        client.attest(&attestor2, &cred_id, &slice_id);
        assert!(client.is_attested(&cred_id, &slice_id));
    }

    #[test]
    fn test_issuer_revoke_credential() {
        let env = Env::default();
        env.mock_all_auths();
        let contract_id = env.register_contract(None, QuorumProofContract);
        let client = QuorumProofContractClient::new(&env, &contract_id);

        let issuer = Address::generate(&env);
        let subject = Address::generate(&env);
        let metadata = Bytes::from_slice(&env, b"ipfs://QmTest");
        let id = client.issue_credential(&issuer, &subject, &1u32, &metadata);
        let id = client.issue_credential(&issuer, &subject, &1u32, &metadata, &None);

        client.revoke_credential(&issuer, &id);

        let cred = client.get_credential(&id);
        assert!(cred.revoked);
        assert_eq!(cred.issuer, issuer);
        assert_eq!(cred.subject, subject);
    }

    #[test]
    fn test_subject_revoke_credential() {
        let env = Env::default();
        env.mock_all_auths();
        let contract_id = env.register_contract(None, QuorumProofContract);
        let client = QuorumProofContractClient::new(&env, &contract_id);

        let issuer = Address::generate(&env);
        let subject = Address::generate(&env);
        let metadata = Bytes::from_slice(&env, b"ipfs://QmTest");
        let id = client.issue_credential(&issuer, &subject, &1u32, &metadata);
        let id = client.issue_credential(&issuer, &subject, &1u32, &metadata, &None);

        client.revoke_credential(&subject, &id);

        let cred = client.get_credential(&id);
        assert!(cred.revoked);
        assert_eq!(cred.issuer, issuer);
        assert_eq!(cred.subject, subject);
    }

    #[test]
    #[should_panic(expected = "only subject or issuer can revoke")]
    fn test_unauthorized_revoke_credential() {
        let env = Env::default();
        env.mock_all_auths();
        let contract_id = env.register_contract(None, QuorumProofContract);
        let client = QuorumProofContractClient::new(&env, &contract_id);

        let issuer = Address::generate(&env);
        let subject = Address::generate(&env);
        let unauthorized = Address::generate(&env);
        let metadata = Bytes::from_slice(&env, b"ipfs://QmTest");
        let id = client.issue_credential(&issuer, &subject, &1u32, &metadata);
        let id = client.issue_credential(&issuer, &subject, &1u32, &metadata, &None);

        client.revoke_credential(&unauthorized, &id);
    }

    #[test]
    fn test_get_credentials_by_subject_single() {
    fn test_credential_not_expired_before_expiry() {
        let env = Env::default();
        env.mock_all_auths();
        let contract_id = env.register_contract(None, QuorumProofContract);
        let client = QuorumProofContractClient::new(&env, &contract_id);

        let issuer = Address::generate(&env);
        let subject = Address::generate(&env);
        let metadata = Bytes::from_slice(&env, b"ipfs://QmTest");

        let id = client.issue_credential(&issuer, &subject, &1u32, &metadata);

        let ids = client.get_credentials_by_subject(&subject);
        assert_eq!(ids.len(), 1);
        assert_eq!(ids.get(0).unwrap(), id);
    }

    #[test]
    fn test_get_credentials_by_subject_multiple() {
        set_ledger_timestamp(&env, 1_000);

        let issuer = Address::generate(&env);
        let subject = Address::generate(&env);
        let metadata = Bytes::from_slice(&env, b"ipfs://QmTest");
        // expires at timestamp 2_000
        let id = client.issue_credential(&issuer, &subject, &1u32, &metadata, &Some(2_000u64));

        assert!(!client.is_expired(&id));
        // get_credential should succeed
        let cred = client.get_credential(&id);
        assert_eq!(cred.expires_at, Some(2_000u64));
    }

    #[test]
    fn test_is_expired_after_expiry() {
        let env = Env::default();
        env.mock_all_auths();
        let contract_id = env.register_contract(None, QuorumProofContract);
        let client = QuorumProofContractClient::new(&env, &contract_id);

        set_ledger_timestamp(&env, 1_000);

        let issuer = Address::generate(&env);
        let subject = Address::generate(&env);
        let metadata = Bytes::from_slice(&env, b"ipfs://QmTest");
        let id = client.issue_credential(&issuer, &subject, &1u32, &metadata, &Some(2_000u64));

        // Advance past expiry
        set_ledger_timestamp(&env, 3_000);

        assert!(client.is_expired(&id));
    }

    #[test]
    #[should_panic(expected = "credential has expired")]
    fn test_get_credential_panics_when_expired() {
        let env = Env::default();
        env.mock_all_auths();
        let contract_id = env.register_contract(None, QuorumProofContract);
        let client = QuorumProofContractClient::new(&env, &contract_id);

        set_ledger_timestamp(&env, 1_000);

        let issuer = Address::generate(&env);
        let subject = Address::generate(&env);
        let metadata = Bytes::from_slice(&env, b"ipfs://QmTest");
        let id = client.issue_credential(&issuer, &subject, &1u32, &metadata, &Some(2_000u64));

        set_ledger_timestamp(&env, 3_000);

        client.get_credential(&id); // should panic
    }

    #[test]
    fn test_is_attested_returns_false_when_expired() {
        let env = Env::default();
        env.mock_all_auths();
        let contract_id = env.register_contract(None, QuorumProofContract);
        let client = QuorumProofContractClient::new(&env, &contract_id);

        let issuer = Address::generate(&env);
        let subject = Address::generate(&env);
        let metadata = Bytes::from_slice(&env, b"ipfs://QmTest");

        let id1 = client.issue_credential(&issuer, &subject, &1u32, &metadata);
        let id2 = client.issue_credential(&issuer, &subject, &2u32, &metadata);
        let id3 = client.issue_credential(&issuer, &subject, &3u32, &metadata);

        let ids = client.get_credentials_by_subject(&subject);
        assert_eq!(ids.len(), 3);
        assert_eq!(ids.get(0).unwrap(), id1);
        assert_eq!(ids.get(1).unwrap(), id2);
        assert_eq!(ids.get(2).unwrap(), id3);
    }

    #[test]
    fn test_get_credentials_by_subject_empty() {
        set_ledger_timestamp(&env, 1_000);

        let issuer = Address::generate(&env);
        let subject = Address::generate(&env);
        let attestor = Address::generate(&env);
        let metadata = Bytes::from_slice(&env, b"ipfs://QmTest");
        let cred_id = client.issue_credential(&issuer, &subject, &1u32, &metadata, &Some(2_000u64));

        let mut attestors = soroban_sdk::Vec::new(&env);
        attestors.push_back(attestor.clone());
        let slice_id = client.create_slice(&issuer, &attestors, &1u32);

        client.attest(&attestor, &cred_id, &slice_id);
        // Before expiry: attested
        assert!(client.is_attested(&cred_id, &slice_id));

        // After expiry: not attested
        set_ledger_timestamp(&env, 3_000);
        assert!(!client.is_attested(&cred_id, &slice_id));
    }

    #[test]
    fn test_is_expired_no_expiry() {
        let env = Env::default();
        env.mock_all_auths();
        let contract_id = env.register_contract(None, QuorumProofContract);
        let client = QuorumProofContractClient::new(&env, &contract_id);

        let subject = Address::generate(&env);

        let ids = client.get_credentials_by_subject(&subject);
        assert_eq!(ids.len(), 0);
    }

    #[test]
    fn test_get_credentials_by_subject_isolated_per_subject() {
        let issuer = Address::generate(&env);
        let subject = Address::generate(&env);
        let metadata = Bytes::from_slice(&env, b"ipfs://QmTest");
        let id = client.issue_credential(&issuer, &subject, &1u32, &metadata, &None);

        // No expiry set — should never be expired
        set_ledger_timestamp(&env, 999_999_999);
        assert!(!client.is_expired(&id));
    }

    #[test]
    fn test_add_attestor_success() {
        let env = Env::default();
        env.mock_all_auths();
        let contract_id = env.register_contract(None, QuorumProofContract);
        let client = QuorumProofContractClient::new(&env, &contract_id);

        let issuer = Address::generate(&env);
        let subject_a = Address::generate(&env);
        let subject_b = Address::generate(&env);
        let metadata = Bytes::from_slice(&env, b"ipfs://QmTest");

        let id_a1 = client.issue_credential(&issuer, &subject_a, &1u32, &metadata);
        let id_a2 = client.issue_credential(&issuer, &subject_a, &2u32, &metadata);
        let id_b1 = client.issue_credential(&issuer, &subject_b, &1u32, &metadata);

        let ids_a = client.get_credentials_by_subject(&subject_a);
        assert_eq!(ids_a.len(), 2);
        assert_eq!(ids_a.get(0).unwrap(), id_a1);
        assert_eq!(ids_a.get(1).unwrap(), id_a2);

        let ids_b = client.get_credentials_by_subject(&subject_b);
        assert_eq!(ids_b.len(), 1);
        assert_eq!(ids_b.get(0).unwrap(), id_b1);
        let creator = Address::generate(&env);
        let attestor1 = Address::generate(&env);
        let attestor2 = Address::generate(&env);

        let mut initial = soroban_sdk::Vec::new(&env);
        initial.push_back(attestor1.clone());
        let slice_id = client.create_slice(&creator, &initial, &1u32);

        // Add a second attestor
        client.add_attestor(&creator, &slice_id, &attestor2);

        let slice = client.get_slice(&slice_id);
        assert_eq!(slice.attestors.len(), 2);
        assert_eq!(slice.attestors.get(1).unwrap(), attestor2);
    }

    #[test]
    #[should_panic(expected = "attestor already in slice")]
    fn test_add_attestor_duplicate_panics() {
        let env = Env::default();
        env.mock_all_auths();
        let contract_id = env.register_contract(None, QuorumProofContract);
        let client = QuorumProofContractClient::new(&env, &contract_id);

        let creator = Address::generate(&env);
        let attestor = Address::generate(&env);

        let mut initial = soroban_sdk::Vec::new(&env);
        initial.push_back(attestor.clone());
        let slice_id = client.create_slice(&creator, &initial, &1u32);

        // Adding the same attestor again should panic
        client.add_attestor(&creator, &slice_id, &attestor);
    }

    #[test]
    #[should_panic(expected = "only the slice creator can add attestors")]
    fn test_add_attestor_unauthorized_panics() {
        let env = Env::default();
        env.mock_all_auths();
        let contract_id = env.register_contract(None, QuorumProofContract);
        let client = QuorumProofContractClient::new(&env, &contract_id);

        let creator = Address::generate(&env);
        let non_creator = Address::generate(&env);
        let attestor = Address::generate(&env);

        let initial = soroban_sdk::Vec::new(&env);
        let slice_id = client.create_slice(&creator, &initial, &1u32);

        // Non-creator trying to add an attestor should panic
        client.add_attestor(&non_creator, &slice_id, &attestor);
    }

    #[test]
    fn test_add_attestor_enables_attestation() {
        let env = Env::default();
        env.mock_all_auths();
        let contract_id = env.register_contract(None, QuorumProofContract);
        let client = QuorumProofContractClient::new(&env, &contract_id);

        let creator = Address::generate(&env);
        let issuer = Address::generate(&env);
        let subject = Address::generate(&env);
        let attestor = Address::generate(&env);
        let metadata = Bytes::from_slice(&env, b"ipfs://QmTest");

        // Create slice with no attestors initially
        let initial = soroban_sdk::Vec::new(&env);
        let slice_id = client.create_slice(&creator, &initial, &1u32);

        let cred_id = client.issue_credential(&issuer, &subject, &1u32, &metadata, &None);

        // Add attestor after creation
        client.add_attestor(&creator, &slice_id, &attestor);

        // Attestor can now attest
        client.attest(&attestor, &cred_id, &slice_id);
        assert!(client.is_attested(&cred_id, &slice_id));
    }
}

