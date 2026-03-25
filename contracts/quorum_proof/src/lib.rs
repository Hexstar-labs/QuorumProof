#![no_std]
use soroban_sdk::{contract, contractimpl, contracttype, Address, Env, String, Vec};
use sbt_registry::SbtRegistryContractClient;
use zk_verifier::{ClaimType, ZkVerifierContractClient};
use soroban_sdk::{contract, contractimpl, contracttype, contracterror, panic_with_error, Address, Env, Vec};

/// Event topic for credential issuance — consumed by off-chain indexers via
/// the Stellar RPC `getEvents` endpoint without polling contract storage.
const TOPIC_ISSUE: &str = "CredentialIssued";

/// Event topic for credential revocation
const TOPIC_REVOKE: &str = "RevokeCredential";

/// Data payload emitted when a new credential is issued.
/// Off-chain listeners can filter on the `CredentialIssued` topic and
/// decode this struct to learn the credential id, recipient, and type
/// without ever reading contract storage.
#[contracttype]
#[derive(Clone)]
pub struct CredentialIssuedEventData {
    pub id: u64,
    pub subject: Address,
    pub credential_type: u32,
}

#[contracttype]
#[derive(Clone)]
pub struct RevokeEventData {
    pub credential_id: u64,
    pub subject: Address,
}

/// Event topic for credential issuance
const TOPIC_ISSUE: &str = "CredentialIssued";

#[contracttype]
#[derive(Clone)]
pub struct IssueEventData {
    pub id: u64,
    pub subject: Address,
    pub credential_type: u32,
}

/// TTL Strategy: Extends instance storage TTL after every write operation.
/// - STANDARD_TTL: 16_384 ledgers (~3 hours at 5s/ledger)
/// - EXTENDED_TTL: 524_288 ledgers (~4 days)
/// This ensures data persistence across typical usage while managing rent costs.
/// TTL is automatically extended on subsequent reads/bumps if needed.
const STANDARD_TTL: u32 = 16_384;
const EXTENDED_TTL: u32 = 524_288;

#[contracterror]
#[derive(Copy, Clone, Debug, Eq, PartialEq, PartialOrd, Ord)]
#[repr(u32)]
pub enum ContractError {
    CredentialNotFound = 1,
    SliceNotFound = 2,
}

#[contracttype]
#[derive(Clone)]
pub enum DataKey {
    Credential(u64),
    CredentialCount,
    Slice(u64),
    SliceCount,
    Attestors(u64),
    SubjectCredentials(Address),
    AttestorCount(Address),
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

#[contracttype]
#[derive(Clone)]
pub struct AttestationEventData {
    pub attestor: Address,
    pub credential_id: u64,
    pub slice_id: u64,
}

const TOPIC_ATTESTATION: &amp;str = "attestation";

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
        assert!(!metadata_hash.is_empty(), "metadata_hash cannot be empty");
        let id: u64 = env
            .storage()
            .instance()
            .get(&DataKey::CredentialCount)
            .unwrap_or(0u64)
            + 1;
        let subject_key = subject.clone();
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
            .set(&DataKey::SubjectCredentials(credential.subject.clone()), &subject_creds);
        env.storage().instance().extend_ttl(STANDARD_TTL, EXTENDED_TTL);

        // Emit CredentialIssued event so off-chain listeners (e.g. Stellar RPC
        // `getEvents`) can react immediately without polling contract storage.
        // The topic acts as a filter key; the data payload carries the fields
        // most useful for indexing: credential id, recipient, and type.
        let event_data = CredentialIssuedEventData {
            id,
            subject: credential.subject,
        // Emit CredentialIssued event
        let event_data = IssueEventData {
            id,
            subject: credential.subject.clone(),
            credential_type,
        };
        let topic = String::from_str(&env, TOPIC_ISSUE);
        let mut topics: Vec<String> = Vec::new(&env);
        topics.push_back(topic);
        env.events().publish(topics, event_data);

        id
    }

    /// Issue credentials to multiple subjects in one call. Returns a Vec of credential IDs.
    /// Panics if subjects, credential_types, and metadata_hashes lengths differ.
    pub fn batch_issue_credentials(
        env: Env,
        issuer: Address,
        subjects: Vec<Address>,
        credential_types: Vec<u32>,
        metadata_hashes: Vec<soroban_sdk::Bytes>,
        expires_at: Option<u64>,
    ) -> Vec<u64> {
        issuer.require_auth();
        let n = subjects.len();
        assert!(
            credential_types.len() == n && metadata_hashes.len() == n,
            "input lengths must match"
        );
        let mut ids: Vec<u64> = Vec::new(&env);
        for i in 0..n {
            let id = Self::issue_credential(
                env.clone(),
                issuer.clone(),
                subjects.get(i).unwrap(),
                credential_types.get(i).unwrap(),
                metadata_hashes.get(i).unwrap(),
                expires_at.clone(),
            );
            ids.push_back(id);
        }
        ids
    }

    /// Retrieve a credential by ID. Panics if the credential has expired.
    /// Retrieve a credential by ID. Panics with ContractError::CredentialNotFound if missing.
    pub fn get_credential(env: Env, credential_id: u64) -> Credential {
        let credential: Credential = env
            .storage()
            .instance()
            .get(&DataKey::Credential(credential_id))
            .unwrap_or_else(|| panic_with_error!(&env, ContractError::CredentialNotFound))
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
        assert!(!credential.revoked, "credential already revoked");
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
        assert!(!attestors.is_empty(), "attestors cannot be empty");
        assert!(threshold > 0, "threshold must be greater than 0");
        assert!(threshold <= attestors.len() as u32, "threshold cannot exceed attestors count");
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
        env.storage().instance().set(&DataKey::Slice(id), &slice);
        env.storage().instance().extend_ttl(STANDARD_TTL, EXTENDED_TTL);
        env.storage().instance().set(&DataKey::SliceCount, &id);
        env.storage().instance().extend_ttl(STANDARD_TTL, EXTENDED_TTL);
        id
    }

    /// Retrieve a quorum slice by ID. Panics with ContractError::SliceNotFound if missing.
    pub fn get_slice(env: Env, slice_id: u64) -> QuorumSlice {
        env.storage()
            .instance()
            .get(&DataKey::Slice(slice_id))
            .unwrap_or_else(|| panic_with_error!(&env, ContractError::SliceNotFound))
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
        env.storage().instance().set(&DataKey::Slice(slice_id), &slice);
        env.storage().instance().extend_ttl(STANDARD_TTL, EXTENDED_TTL);
    }

    /// Update the threshold of an existing quorum slice.
    /// Only the slice creator can call this.
    /// Panics if new_threshold exceeds the current attestor count.
    pub fn update_threshold(env: Env, creator: Address, slice_id: u64, new_threshold: u32) {
        creator.require_auth();
        let mut slice: QuorumSlice = env
            .storage()
            .instance()
            .get(&DataKey::Slice(slice_id))
            .expect("slice not found");
        assert!(slice.creator == creator, "only the slice creator can update threshold");
        assert!(
            new_threshold <= slice.attestors.len(),
            "threshold exceeds attestor count"
        );
        slice.threshold = new_threshold;
        env.storage()
            .instance()
            .set(&DataKey::Slice(slice_id), &slice);
        env.storage().instance().extend_ttl(STANDARD_TTL, EXTENDED_TTL);
    }

    /// Attest a credential using a quorum slice.
    pub fn attest(env: Env, attestor: Address, credential_id: u64, slice_id: u64) {
        attestor.require_auth();

        // Issue #8: load credential and panic if revoked
        let credential: Credential = env
            .storage()
            .instance()
            .get(&DataKey::Credential(credential_id))
            .expect("credential not found");
        assert!(!credential.revoked, "credential is revoked");

        let slice: QuorumSlice = env
            .storage()
            .instance()
            .get(&DataKey::Slice(slice_id))
            .expect("slice not found");
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
        attestors.push_back(attestor.clone());
        
        // Check if attestor has already attested for this credential
        for existing_attestor in attestors.iter() {
            if existing_attestor == attestor {
                panic!("attestor has already attested for this credential");
            }
        }
        
        attestors.push_back(attestor);
        env.storage()
            .instance()
            .set(&DataKey::Attestors(credential_id), &attestors);
        env.storage().instance().extend_ttl(STANDARD_TTL, EXTENDED_TTL);

        // Emit Attestation event
        let event_data = AttestationEventData {
            attestor: attestor.clone(),
            credential_id,
            slice_id,
        };
        let topic = String::from_str(&env, TOPIC_ATTESTATION);
        let mut topics: Vec<String> = Vec::new(&env);
        topics.push_back(topic);
        env.events().publish(topics, event_data);
        // Increment attestor reputation count
        let count: u64 = env
            .storage()
            .instance()
            .get(&DataKey::AttestorCount(attestor.clone()))
            .unwrap_or(0u64);
        env.storage()
            .instance()
            .set(&DataKey::AttestorCount(attestor), &(count + 1));
        env.storage().instance().extend_ttl(STANDARD_TTL, EXTENDED_TTL);
    }

    /// Check if a credential has met its quorum threshold.
    /// Returns false if revoked or expired.
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

    /// Returns the total number of credentials an attestor has signed.
    pub fn get_attestor_reputation(env: Env, attestor: Address) -> u64 {
        env.storage()
            .instance()
            .get(&DataKey::AttestorCount(attestor))
            .unwrap_or(0u64)
    }

    /// Unified engineer verification entry point.
    /// 1. Confirms the subject owns at least one SBT linked to the credential.
    /// 2. Verifies the ZK claim proof.
    /// Returns true only when both checks pass.
    pub fn verify_engineer(
        env: Env,
        sbt_registry_id: Address,
        zk_verifier_id: Address,
        subject: Address,
        credential_id: u64,
        claim_type: ClaimType,
        proof: soroban_sdk::Bytes,
    ) -> bool {
        // 1. Confirm SBT ownership: subject must own a token linked to this credential
        let sbt_client = SbtRegistryContractClient::new(&env, &sbt_registry_id);
        let tokens = sbt_client.get_tokens_by_owner(&subject);
        let has_sbt = tokens.iter().any(|token_id| {
            let token = sbt_client.get_token(&token_id);
            token.credential_id == credential_id
        });
        if !has_sbt {
            return false;
        }

        // 2. Verify the ZK claim proof
        let zk_client = ZkVerifierContractClient::new(&env, &zk_verifier_id);
        zk_client.verify_claim(&credential_id, &claim_type, &proof)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use soroban_sdk::testutils::{Address as _, Events, Ledger as _, LedgerInfo};
    use soroban_sdk::{Bytes, Env, FromVal, IntoVal};

    #[test]
    fn test_storage_persists_across_ledgers() {
        let env = Env::default();
        env.mock_all_auths();
        let contract_id = env.register_contract(None, QuorumProofContract);
        let client = QuorumProofContractClient::new(&env, &contract_id);

        let issuer = Address::generate(&env);
        let subject = Address::generate(&env);
        let metadata = Bytes::from_slice(&env, b"ipfs://QmTest");
        let id = client.issue_credential(&issuer, &subject, &1u32, &metadata, &None);
        let id = client.issue_credential(&issuer, &subject, &1u32, &metadata);

        // Advance ledger sequence by 20_000 ledgers (beyond default eviction TTL)
        env.ledger().set(LedgerInfo {
            timestamp: 1_000_000,
            protocol_version: 20,
            sequence_number: 20_000,
            network_id: Default::default(),
            base_reserve: 10,
            max_entry_ttl: 311_040,
            min_persistent_entry_ttl: 10_000,
            min_temp_entry_ttl: 10,
            min_persistent_entry_ttl: 4_320,
            min_temp_entry_ttl: 16,
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
        let id = client.issue_credential(&issuer, &subject, &1u32, &metadata, &None);
        let id = client.issue_credential(&issuer, &subject, &1u32, &metadata, &None);
        let id = client.issue_credential(&issuer, &subject, &1u32, &metadata);
        assert_eq!(id, 1);

        let cred = client.get_credential(&id);
        assert_eq!(cred.subject, subject);
        assert_eq!(cred.issuer, issuer);
        assert!(!cred.revoked);
    }

    /// Verifies that issuing a credential emits a `CredentialIssued` contract event
    /// with the correct id, subject, and credential_type.
    ///
    /// Off-chain services (e.g. Stellar RPC `getEvents`, horizon event stream)
    /// can subscribe to the `CredentialIssued` topic and decode `CredentialIssuedEventData`
    /// without ever polling contract storage — satisfying issue #16.
    #[test]
    fn test_issue_credential_emits_event() {
        let env = Env::default();
        env.mock_all_auths();
        let contract_id = env.register_contract(None, QuorumProofContract);
        let client = QuorumProofContractClient::new(&env, &contract_id);

        let issuer = Address::generate(&env);
        let subject = Address::generate(&env);
        let metadata = Bytes::from_slice(&env, b"ipfs://QmTest");
        let credential_type: u32 = 42;

        let id = client.issue_credential(&issuer, &subject, &credential_type, &metadata, &None);

        // Collect all events emitted during this invocation.
        // Contract events (type_ = "contract") are the ones observable off-chain
        // via Stellar RPC getEvents; diagnostic events are filtered out here.
        let all_events = env.events().all();

        // Find the CredentialIssued event by matching the first topic string.
        let expected_topic = String::from_str(&env, TOPIC_ISSUE);

        let issued = all_events.iter().find(|(_, topics, _)| {
            if let Some(raw) = topics.get(0) {
                // Convert the raw Val back to a soroban String for comparison
                let s = String::from_val(&env, &raw);
                return s == expected_topic;
            }
            false
        });

        assert!(issued.is_some(), "CredentialIssued event was not emitted");

        // Decode the event data and assert each field matches what was issued.
        let (_, _, data) = issued.unwrap();
        let event_data: CredentialIssuedEventData = data.into_val(&env);

        assert_eq!(event_data.id, id, "event id should match returned credential id");
        assert_eq!(event_data.subject, subject, "event subject should match the recipient");
        assert_eq!(
            event_data.credential_type, credential_type,
            "event credential_type should match the issued type"
        );
    }

    #[test]
    fn test_issue_credential_emits_event() {
        let env = Env::default();
        env.mock_all_auths();
        let contract_id = env.register_contract(None, QuorumProofContract);
        let client = QuorumProofContractClient::new(&env, &contract_id);

        let issuer = Address::generate(&env);
        let subject = Address::generate(&env);
        let metadata = Bytes::from_slice(&env, b"ipfs://QmTest");

        let id = client.issue_credential(&issuer, &subject, &1u32, &metadata, &None);

        let events = env.events().all();
        let (_addr, topics, data) = events.last().unwrap();
        
        let expected_topic = String::from_str(&env, TOPIC_ISSUE);
        let stored_topic: String = topics.get(0).unwrap().try_into_val(&env).unwrap();
        assert_eq!(stored_topic, expected_topic);

        let event_data: IssueEventData = data.try_into_val(&env).unwrap();
        assert_eq!(event_data.id, id);
        assert_eq!(event_data.subject, subject);
        assert_eq!(event_data.credential_type, 1u32);
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
        let creator = Address::generate(&env);

        let metadata = Bytes::from_slice(&env, b"ipfs://QmTest");
        let cred_id = client.issue_credential(&issuer, &subject, &1u32, &metadata);
        let cred_id = client.issue_credential(&issuer, &subject, &1u32, &metadata, &None);

        let mut attestors = Vec::new(&env);
        attestors.push_back(attestor1.clone());
        attestors.push_back(attestor2.clone());
        let slice_id = client.create_slice(&issuer, &attestors, &2u32);
        let slice_id = client.create_slice(&attestors, &2u32);
        let slice_id = client.create_slice(&creator, &attestors, &2u32);

        assert!(!client.is_attested(&cred_id, &slice_id));
        client.attest(&attestor1, &cred_id, &slice_id);
        assert!(!client.is_attested(&cred_id, &slice_id));
        client.attest(&attestor2, &cred_id, &slice_id);
        assert!(client.is_attested(&cred_id, &slice_id));
    }

    #[test]
    #[should_panic(expected = "threshold must be greater than 0")]
    fn test_zero_threshold_rejection() {
        let env = Env::default();
        env.mock_all_auths();
        let contract_id = env.register_contract(None, QuorumProofContract);
        let client = QuorumProofContractClient::new(&env, &contract_id);

        let creator = Address::generate(&env);
        let attestor = Address::generate(&env);

        let mut attestors = Vec::new(&env);
        attestors.push_back(attestor);
        // This should panic with "threshold must be greater than 0"
        let _slice_id = client.create_slice(&creator, &attestors, &0u32);
    }

    #[test]
    #[should_panic(expected = "threshold cannot exceed attestors count")]
    fn test_threshold_exceeds_attestors() {
        let env = Env::default();
        env.mock_all_auths();
        let contract_id = env.register_contract(None, QuorumProofContract);
        let client = QuorumProofContractClient::new(&env, &contract_id);

        let creator = Address::generate(&env);
        let attestor1 = Address::generate(&env);
        let attestor2 = Address::generate(&env);

        let mut attestors = Vec::new(&env);
        attestors.push_back(attestor1);
        attestors.push_back(attestor2);
        // threshold=5 with only 2 attestors - impossible to reach quorum
        let _slice_id = client.create_slice(&creator, &attestors, &5u32);
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
        let id = client.issue_credential(&issuer, &subject, &1u32, &metadata, &None);
        let id = client.issue_credential(&issuer, &subject, &1u32, &metadata, &None);
        let id = client.issue_credential(&issuer, &subject, &1u32, &metadata);

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
        let id = client.issue_credential(&issuer, &subject, &1u32, &metadata, &None);
        let id = client.issue_credential(&issuer, &subject, &1u32, &metadata, &None);
        let id = client.issue_credential(&issuer, &subject, &1u32, &metadata);

        client.revoke_credential(&subject, &id);

        let cred = client.get_credential(&id);
        assert!(cred.revoked);
        assert_eq!(cred.issuer, issuer);
        assert_eq!(cred.subject, subject);
    }

    #[test]
    #[should_panic(expected = "metadata_hash cannot be empty")]
    fn test_empty_metadata_hash_rejection() {
        let env = Env::default();
        env.mock_all_auths();
        let contract_id = env.register_contract(None, QuorumProofContract);
        let client = QuorumProofContractClient::new(&env, &contract_id);

        let issuer = Address::generate(&env);
        let subject = Address::generate(&env);
        let empty_metadata = Bytes::new(&env);
        
        client.issue_credential(&issuer, &subject, &1u32, &empty_metadata, &None);
    }

    #[test]
    #[should_panic(expected = "attestor has already attested for this credential")]
    fn test_duplicate_attestation_rejection() {
        let env = Env::default();
        env.mock_all_auths();
        let contract_id = env.register_contract(None, QuorumProofContract);
        let client = QuorumProofContractClient::new(&env, &contract_id);

        let issuer = Address::generate(&env);
        let subject = Address::generate(&env);
        let attestor1 = Address::generate(&env);
        let attestor2 = Address::generate(&env);

        let metadata = Bytes::from_slice(&env, b"ipfs://QmTest");
        let cred_id = client.issue_credential(&issuer, &subject, &1u32, &metadata, &None);

        let mut attestors = soroban_sdk::Vec::new(&env);
        attestors.push_back(attestor1.clone());
        attestors.push_back(attestor2.clone());
        let creator = Address::generate(&env);
        let slice_id = client.create_slice(&creator, &attestors, &2u32);

        // First attestation should succeed
        client.attest(&attestor1, &cred_id, &slice_id);
        
        // Second attestation by same attestor should panic
        client.attest(&attestor1, &cred_id, &slice_id);
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
        let id = client.issue_credential(&issuer, &subject, &1u32, &metadata, &None);
        let id = client.issue_credential(&issuer, &subject, &1u32, &metadata, &None);
        let id = client.issue_credential(&issuer, &subject, &1u32, &metadata);

        client.revoke_credential(&unauthorized, &id);
    }

    #[test]
    #[should_panic]
    fn test_get_credential_not_found() {
        let env = Env::default();
        let contract_id = env.register_contract(None, QuorumProofContract);
        let client = QuorumProofContractClient::new(&env, &contract_id);
        // credential ID 999 was never issued — should panic with ContractError::CredentialNotFound
        client.get_credential(&999u64);
    }

    #[test]
    #[should_panic]
    fn test_get_slice_not_found() {
        let env = Env::default();
        let contract_id = env.register_contract(None, QuorumProofContract);
        let client = QuorumProofContractClient::new(&env, &contract_id);
        // slice ID 999 was never issued — should panic with ContractError::SliceNotFound
        client.get_slice(&999u64);
    }

    #[test]
    fn test_get_credentials_by_subject_single() {
        let env = Env::default();
        env.mock_all_auths();
        let contract_id = env.register_contract(None, QuorumProofContract);
        let client = QuorumProofContractClient::new(&env, &contract_id);

        let issuer = Address::generate(&env);
        let subject = Address::generate(&env);
        let metadata = Bytes::from_slice(&env, b"ipfs://QmTest");
        let id = client.issue_credential(&issuer, &subject, &1u32, &metadata, &None);

        let ids = client.get_credentials_by_subject(&subject);
        assert_eq!(ids.len(), 1);
        assert_eq!(ids.get(0).unwrap(), id);
    }

    #[test]
    fn test_credential_not_expired_before_expiry() {
    #[should_panic(expected = "credential is revoked")]
    fn test_attest_revoked_credential_panics() {
    fn test_get_credentials_by_subject_single() {
        let env = Env::default();
        env.mock_all_auths();
        let contract_id = env.register_contract(None, QuorumProofContract);
        let client = QuorumProofContractClient::new(&env, &contract_id);

        let issuer = Address::generate(&env);
        let subject = Address::generate(&env);
        let metadata = Bytes::from_slice(&env, b"ipfs://QmTest");
        let id = client.issue_credential(&issuer, &subject, &1u32, &metadata, &None);

        let ids = client.get_credentials_by_subject(&subject);
        assert_eq!(ids.len(), 1);
        assert_eq!(ids.get(0).unwrap(), id);
    }

    #[test]
    fn test_get_credentials_by_subject_multiple() {
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
    }

    #[test]
    fn test_credential_not_expired_before_expiry() {
        let env = Env::default();
        env.mock_all_auths();
        let contract_id = env.register_contract(None, QuorumProofContract);
        let client = QuorumProofContractClient::new(&env, &contract_id);

        set_ledger_timestamp(&env, 1_000);

        let issuer = Address::generate(&env);
        let subject = Address::generate(&env);
        let metadata = Bytes::from_slice(&env, b"ipfs://QmTest");

        let id1 = client.issue_credential(&issuer, &subject, &1u32, &metadata, &None);
        let id2 = client.issue_credential(&issuer, &subject, &2u32, &metadata, &None);
        let id3 = client.issue_credential(&issuer, &subject, &3u32, &metadata, &None);

        let ids = client.get_credentials_by_subject(&subject);
        assert_eq!(ids.len(), 3);
        assert_eq!(ids.get(0).unwrap(), id1);
        assert_eq!(ids.get(1).unwrap(), id2);
        assert_eq!(ids.get(2).unwrap(), id3);
    }

    #[test]
    fn test_credential_not_expired_before_expiry() {
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
        let attestor = Address::generate(&env);
        let metadata = Bytes::from_slice(&env, b"ipfs://QmTest");

        set_ledger_timestamp(&env, 1_000);
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
    fn test_get_credentials_by_subject_empty() {
        let env = Env::default();
        env.mock_all_auths();
        let contract_id = env.register_contract(None, QuorumProofContract);
        let client = QuorumProofContractClient::new(&env, &contract_id);

        let issuer = Address::generate(&env);
        let subject = Address::generate(&env);
        let unauthorized = Address::generate(&env);
        let metadata = Bytes::from_slice(&env, b"ipfs://QmTest");
        let id = client.issue_credential(&issuer, &subject, &1u32, &metadata);

        client.revoke_credential(&unauthorized, &id);
    }
}
        let subject = Address::generate(&env);
        let ids = client.get_credentials_by_subject(&subject);
        assert_eq!(ids.len(), 0);
    }

    #[test]
    fn test_is_expired_no_expiry() {
        let env = Env::default();
        env.mock_all_auths();
        let contract_id = env.register_contract(None, QuorumProofContract);
        let client = QuorumProofContractClient::new(&env, &contract_id);

        let issuer = Address::generate(&env);
        let subject = Address::generate(&env);
        let metadata = Bytes::from_slice(&env, b"ipfs://QmTest");
        let id = client.issue_credential(&issuer, &subject, &1u32, &metadata, &None);

        // No expiry set — should never be expired
        set_ledger_timestamp(&env, 999_999_999);
        assert!(!client.is_expired(&id));
    }

    #[test]
    fn test_get_credentials_by_subject_isolated_per_subject() {
        let env = Env::default();
        env.mock_all_auths();
        let contract_id = env.register_contract(None, QuorumProofContract);
        let client = QuorumProofContractClient::new(&env, &contract_id);

        let issuer = Address::generate(&env);
        let subject_a = Address::generate(&env);
        let subject_b = Address::generate(&env);
        let metadata = Bytes::from_slice(&env, b"ipfs://QmTest");

        let id_a1 = client.issue_credential(&issuer, &subject_a, &1u32, &metadata, &None);
        let id_a2 = client.issue_credential(&issuer, &subject_a, &2u32, &metadata, &None);
        let id_b1 = client.issue_credential(&issuer, &subject_b, &1u32, &metadata, &None);

        let ids_a = client.get_credentials_by_subject(&subject_a);
        assert_eq!(ids_a.len(), 2);
        assert_eq!(ids_a.get(0).unwrap(), id_a1);
        assert_eq!(ids_a.get(1).unwrap(), id_a2);

        let ids_b = client.get_credentials_by_subject(&subject_b);
        assert_eq!(ids_b.len(), 1);
        assert_eq!(ids_b.get(0).unwrap(), id_b1);
    }

    #[test]
    fn test_add_attestor_success() {
        let env = Env::default();
        env.mock_all_auths();
        let contract_id = env.register_contract(None, QuorumProofContract);
        let client = QuorumProofContractClient::new(&env, &contract_id);

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
    #[should_panic(expected = "attestors cannot be empty")]
    fn test_create_slice_empty_attestors_panics() {
    fn test_add_attestor_enables_attestation() {
        let env = Env::default();
        env.mock_all_auths();
        let contract_id = env.register_contract(None, QuorumProofContract);
        let client = QuorumProofContractClient::new(&env, &contract_id);

        let creator = Address::generate(&env);
        let attestors = Vec::new(&env);

        client.create_slice(&creator, &attestors, &1u32);
        let issuer = Address::generate(&env);
        let subject = Address::generate(&env);
        let attestor1 = Address::generate(&env);
        let attestor2 = Address::generate(&env);
        let metadata = Bytes::from_slice(&env, b"ipfs://QmTest");

        // Create slice with one attestor initially, threshold 2
        let mut initial = soroban_sdk::Vec::new(&env);
        initial.push_back(attestor1.clone());
        let slice_id = client.create_slice(&creator, &initial, &2u32);

        let cred_id = client.issue_credential(&issuer, &subject, &1u32, &metadata, &None);

        // With only 1 attestor, not attested
        client.attest(&attestor1, &cred_id, &slice_id);
        assert!(!client.is_attested(&cred_id, &slice_id));

        // Add second attestor
        client.add_attestor(&creator, &slice_id, &attestor2);

        // Now with 2 attestors, attested
        client.attest(&attestor2, &cred_id, &slice_id);
        assert!(client.is_attested(&cred_id, &slice_id));
    }

    #[test]
    #[should_panic(expected = "attestors cannot be empty")]
    fn test_create_slice_empty_attestors_panics() {
        let env = Env::default();
        env.mock_all_auths();
        let contract_id = env.register_contract(None, QuorumProofContract);
        let client = QuorumProofContractClient::new(&env, &contract_id);

        let creator = Address::generate(&env);
        let attestors = Vec::new(&env);

        client.create_slice(&creator, &attestors, &1u32);
    fn test_verify_engineer_success() {
        use sbt_registry::SbtRegistryContract;
        use zk_verifier::{ClaimType, ZkVerifierContract};

        let env = Env::default();
        env.mock_all_auths();

        let qp_id = env.register_contract(None, QuorumProofContract);
        let sbt_id = env.register_contract(None, SbtRegistryContract);
        let zk_id = env.register_contract(None, ZkVerifierContract);

        let qp = QuorumProofContractClient::new(&env, &qp_id);
        let sbt = sbt_registry::SbtRegistryContractClient::new(&env, &sbt_id);

        let issuer = Address::generate(&env);
        let subject = Address::generate(&env);
        let metadata = Bytes::from_slice(&env, b"ipfs://QmTest");

        let cred_id = qp.issue_credential(&issuer, &subject, &1u32, &metadata, &None);

        // Mint an SBT for the subject linked to the credential
        let sbt_uri = Bytes::from_slice(&env, b"ipfs://QmSbt");
        sbt.mint(&subject, &cred_id, &sbt_uri);

        let proof = Bytes::from_slice(&env, b"valid-proof");
        let result = qp.verify_engineer(&sbt_id, &zk_id, &subject, &cred_id, &ClaimType::HasDegree, &proof);
        assert!(result);
    }

    #[test]
    fn test_verify_engineer_fails_without_sbt() {
        use zk_verifier::ClaimType;

        let env = Env::default();
        env.mock_all_auths();

        let qp_id = env.register_contract(None, QuorumProofContract);
        let sbt_id = env.register_contract(None, sbt_registry::SbtRegistryContract);
        let zk_id = env.register_contract(None, zk_verifier::ZkVerifierContract);

        let qp = QuorumProofContractClient::new(&env, &qp_id);
        let issuer = Address::generate(&env);
        let subject = Address::generate(&env);
        let metadata = Bytes::from_slice(&env, b"ipfs://QmTest");

        let cred_id = qp.issue_credential(&issuer, &subject, &1u32, &metadata, &None);

        // No SBT minted — should return false
        let proof = Bytes::from_slice(&env, b"valid-proof");
        let result = qp.verify_engineer(&sbt_id, &zk_id, &subject, &cred_id, &ClaimType::HasDegree, &proof);
        assert!(!result);
    }

    #[test]
    fn test_verify_engineer_fails_with_empty_proof() {
        use sbt_registry::SbtRegistryContract;
        use zk_verifier::ClaimType;

        let env = Env::default();
        env.mock_all_auths();

        let qp_id = env.register_contract(None, QuorumProofContract);
        let sbt_id = env.register_contract(None, SbtRegistryContract);
        let zk_id = env.register_contract(None, zk_verifier::ZkVerifierContract);

        let qp = QuorumProofContractClient::new(&env, &qp_id);
        let sbt = sbt_registry::SbtRegistryContractClient::new(&env, &sbt_id);

        let issuer = Address::generate(&env);
        let subject = Address::generate(&env);
        let metadata = Bytes::from_slice(&env, b"ipfs://QmTest");

        let cred_id = qp.issue_credential(&issuer, &subject, &1u32, &metadata, &None);
        let sbt_uri = Bytes::from_slice(&env, b"ipfs://QmSbt");
        sbt.mint(&subject, &cred_id, &sbt_uri);

        // Empty proof — ZK verifier stub returns false
        let proof = Bytes::from_slice(&env, b"");
        let result = qp.verify_engineer(&sbt_id, &zk_id, &subject, &cred_id, &ClaimType::HasLicense, &proof);
        assert!(!result);
    }

    #[test]
    fn test_get_attestor_reputation_zero_before_any_attestation() {
        let env = Env::default();
        env.mock_all_auths();
        let contract_id = env.register_contract(None, QuorumProofContract);
        let client = QuorumProofContractClient::new(&env, &contract_id);

        let attestor = Address::generate(&env);
        assert_eq!(client.get_attestor_reputation(&attestor), 0);
    }

    #[test]
    fn test_get_attestor_reputation_increments_per_attestation() {
        let env = Env::default();
        env.mock_all_auths();
        let contract_id = env.register_contract(None, QuorumProofContract);
        let client = QuorumProofContractClient::new(&env, &contract_id);

        let issuer = Address::generate(&env);
        let subject = Address::generate(&env);
        let attestor = Address::generate(&env);
        let metadata = Bytes::from_slice(&env, b"ipfs://QmTest");

        let mut attestors = soroban_sdk::Vec::new(&env);
        attestors.push_back(attestor.clone());
        let slice_id = client.create_slice(&issuer, &attestors, &1u32);

        let cred_id1 = client.issue_credential(&issuer, &subject, &1u32, &metadata, &None);
        let cred_id2 = client.issue_credential(&issuer, &subject, &2u32, &metadata, &None);

        assert_eq!(client.get_attestor_reputation(&attestor), 0);
        client.attest(&attestor, &cred_id1, &slice_id);
        assert_eq!(client.get_attestor_reputation(&attestor), 1);
        client.attest(&attestor, &cred_id2, &slice_id);
        assert_eq!(client.get_attestor_reputation(&attestor), 2);
    }

    #[test]
    fn test_get_attestor_reputation_independent_per_attestor() {
        let env = Env::default();
        env.mock_all_auths();
        let contract_id = env.register_contract(None, QuorumProofContract);
        let client = QuorumProofContractClient::new(&env, &contract_id);

        let issuer = Address::generate(&env);
        let subject = Address::generate(&env);
        let attestor_a = Address::generate(&env);
        let attestor_b = Address::generate(&env);
        let metadata = Bytes::from_slice(&env, b"ipfs://QmTest");

        let mut attestors = soroban_sdk::Vec::new(&env);
        attestors.push_back(attestor_a.clone());
        attestors.push_back(attestor_b.clone());
        let slice_id = client.create_slice(&issuer, &attestors, &1u32);

        let cred_id = client.issue_credential(&issuer, &subject, &1u32, &metadata, &None);

        client.attest(&attestor_a, &cred_id, &slice_id);

        assert_eq!(client.get_attestor_reputation(&attestor_a), 1);
        assert_eq!(client.get_attestor_reputation(&attestor_b), 0);
    }

    #[test]
    fn test_update_threshold_success() {
        let env = Env::default();
        env.mock_all_auths();
        let contract_id = env.register_contract(None, QuorumProofContract);
        let client = QuorumProofContractClient::new(&env, &contract_id);

        let creator = Address::generate(&env);
        let attestor1 = Address::generate(&env);
        let attestor2 = Address::generate(&env);

        let mut attestors = soroban_sdk::Vec::new(&env);
        attestors.push_back(attestor1.clone());
        attestors.push_back(attestor2.clone());
        let slice_id = client.create_slice(&creator, &attestors, &2u32);

        client.update_threshold(&creator, &slice_id, &1u32);

        let slice = client.get_slice(&slice_id);
        assert_eq!(slice.threshold, 1);
    }

    #[test]
    #[should_panic(expected = "only the slice creator can update threshold")]
    fn test_update_threshold_unauthorized_panics() {
        let env = Env::default();
        env.mock_all_auths();
        let contract_id = env.register_contract(None, QuorumProofContract);
        let client = QuorumProofContractClient::new(&env, &contract_id);

        let creator = Address::generate(&env);
        let non_creator = Address::generate(&env);
        let attestor = Address::generate(&env);

        let mut attestors = soroban_sdk::Vec::new(&env);
        attestors.push_back(attestor.clone());
        let slice_id = client.create_slice(&creator, &attestors, &1u32);

        client.update_threshold(&non_creator, &slice_id, &1u32);
    }

    #[test]
    #[should_panic(expected = "threshold exceeds attestor count")]
    fn test_update_threshold_exceeds_attestor_count_panics() {
        let env = Env::default();
        env.mock_all_auths();
        let contract_id = env.register_contract(None, QuorumProofContract);
        let client = QuorumProofContractClient::new(&env, &contract_id);

        let creator = Address::generate(&env);
        let attestor = Address::generate(&env);

        let mut attestors = soroban_sdk::Vec::new(&env);
        attestors.push_back(attestor.clone());
        let slice_id = client.create_slice(&creator, &attestors, &1u32);

        // 1 attestor, threshold of 2 should panic
        client.update_threshold(&creator, &slice_id, &2u32);
    }

    #[test]
    fn test_batch_issue_credentials_success() {
        let env = Env::default();
        env.mock_all_auths();
        let contract_id = env.register_contract(None, QuorumProofContract);
        let client = QuorumProofContractClient::new(&env, &contract_id);

        let issuer = Address::generate(&env);
        let subject1 = Address::generate(&env);
        let subject2 = Address::generate(&env);
        let subject3 = Address::generate(&env);

        let mut subjects = soroban_sdk::Vec::new(&env);
        subjects.push_back(subject1.clone());
        subjects.push_back(subject2.clone());
        subjects.push_back(subject3.clone());

        let mut cred_types = soroban_sdk::Vec::new(&env);
        cred_types.push_back(1u32);
        cred_types.push_back(2u32);
        cred_types.push_back(1u32);

        let mut hashes = soroban_sdk::Vec::new(&env);
        hashes.push_back(Bytes::from_slice(&env, b"ipfs://Qm1"));
        hashes.push_back(Bytes::from_slice(&env, b"ipfs://Qm2"));
        hashes.push_back(Bytes::from_slice(&env, b"ipfs://Qm3"));

        let ids = client.batch_issue_credentials(&issuer, &subjects, &cred_types, &hashes, &None);

        assert_eq!(ids.len(), 3);
        // Each subject should have exactly one credential
        assert_eq!(client.get_credentials_by_subject(&subject1).len(), 1);
        assert_eq!(client.get_credentials_by_subject(&subject2).len(), 1);
        assert_eq!(client.get_credentials_by_subject(&subject3).len(), 1);
        // IDs are sequential
        assert_eq!(ids.get(1).unwrap(), ids.get(0).unwrap() + 1);
        assert_eq!(ids.get(2).unwrap(), ids.get(0).unwrap() + 2);
    }

    #[test]
    #[should_panic(expected = "input lengths must match")]
    fn test_batch_issue_credentials_mismatched_lengths_panics() {
        let env = Env::default();
        env.mock_all_auths();
        let contract_id = env.register_contract(None, QuorumProofContract);
        let client = QuorumProofContractClient::new(&env, &contract_id);

        let issuer = Address::generate(&env);

        let mut subjects = soroban_sdk::Vec::new(&env);
        subjects.push_back(Address::generate(&env));
        subjects.push_back(Address::generate(&env));

        let mut cred_types = soroban_sdk::Vec::new(&env);
        cred_types.push_back(1u32); // only 1, mismatched

        let mut hashes = soroban_sdk::Vec::new(&env);
        hashes.push_back(Bytes::from_slice(&env, b"ipfs://Qm1"));
        hashes.push_back(Bytes::from_slice(&env, b"ipfs://Qm2"));

        client.batch_issue_credentials(&issuer, &subjects, &cred_types, &hashes, &None);
    }
}

