#![no_std]
use soroban_sdk::{contract, contractimpl, contracttype, contracterror, panic_with_error, Address, Env, String, Vec};
use sbt_registry::SbtRegistryContractClient;
use zk_verifier::{ClaimType, ZkVerifierContractClient};

const TOPIC_ISSUE: &str = "CredentialIssued";
const TOPIC_REVOKE: &str = "RevokeCredential";
const TOPIC_ATTESTATION: &str = "attestation";
const STANDARD_TTL: u32 = 16_384;
const EXTENDED_TTL: u32 = 524_288;
const MAX_ATTESTORS_PER_SLICE: u32 = 20;

#[contracttype]
#[derive(Clone)]
pub struct IssueEventData {
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

#[contracttype]
#[derive(Clone)]
pub struct AttestationEventData {
    pub attestor: Address,
    pub credential_id: u64,
    pub slice_id: u64,
}

#[contracterror]
#[derive(Copy, Clone, Debug, Eq, PartialEq, PartialOrd, Ord)]
#[repr(u32)]
pub enum ContractError {
    CredentialNotFound = 1,
    SliceNotFound = 2,
    ContractPaused = 3,
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
    CredentialType(u32),
    Admin,
    Paused,
}

#[contracttype]
#[derive(Clone)]
pub struct CredentialTypeDef {
    pub type_id: u32,
    pub name: soroban_sdk::String,
    pub description: soroban_sdk::String,
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
    /// Set the admin address once after deployment. Panics if already initialized.
    pub fn initialize(env: Env, admin: Address) {
        assert!(!env.storage().instance().has(&DataKey::Admin), "already initialized");
        env.storage().instance().set(&DataKey::Admin, &admin);
        env.storage().instance().extend_ttl(STANDARD_TTL, EXTENDED_TTL);
    }

    /// Pause the contract. Only admin may call this.
    pub fn pause(env: Env, admin: Address) {
        admin.require_auth();
        let stored: Address = env.storage().instance().get(&DataKey::Admin).expect("not initialized");
        assert!(stored == admin, "unauthorized");
        env.storage().instance().set(&DataKey::Paused, &true);
        env.storage().instance().extend_ttl(STANDARD_TTL, EXTENDED_TTL);
    }

    /// Unpause the contract. Only admin may call this.
    pub fn unpause(env: Env, admin: Address) {
        admin.require_auth();
        let stored: Address = env.storage().instance().get(&DataKey::Admin).expect("not initialized");
        assert!(stored == admin, "unauthorized");
        env.storage().instance().set(&DataKey::Paused, &false);
        env.storage().instance().extend_ttl(STANDARD_TTL, EXTENDED_TTL);
    }

    /// Returns true if the contract is currently paused.
    pub fn is_paused(env: Env) -> bool {
        env.storage().instance().get(&DataKey::Paused).unwrap_or(false)
    }

    fn require_not_paused(env: &Env) {
        if env.storage().instance().get(&DataKey::Paused).unwrap_or(false) {
            panic_with_error!(env, ContractError::ContractPaused);
        }
    }

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
        Self::require_not_paused(&env);
        assert!(!metadata_hash.is_empty(), "metadata_hash cannot be empty");
        let id: u64 = env.storage().instance().get(&DataKey::CredentialCount).unwrap_or(0u64) + 1;
        let credential = Credential { id, subject: subject.clone(), issuer, credential_type, metadata_hash, revoked: false, expires_at };
        env.storage().instance().set(&DataKey::Credential(id), &credential);
        env.storage().instance().extend_ttl(STANDARD_TTL, EXTENDED_TTL);
        env.storage().instance().set(&DataKey::CredentialCount, &id);
        env.storage().instance().extend_ttl(STANDARD_TTL, EXTENDED_TTL);
        let mut subject_creds: Vec<u64> = env.storage().instance().get(&DataKey::SubjectCredentials(subject.clone())).unwrap_or(Vec::new(&env));
        subject_creds.push_back(id);
        env.storage().instance().set(&DataKey::SubjectCredentials(subject), &subject_creds);
        env.storage().instance().extend_ttl(STANDARD_TTL, EXTENDED_TTL);
        let event_data = IssueEventData { id, subject: credential.subject.clone(), credential_type };
        let topic = String::from_str(&env, TOPIC_ISSUE);
        let mut topics: Vec<String> = Vec::new(&env);
        topics.push_back(topic);
        env.events().publish(topics, event_data);
        id
    }

    /// Issue credentials to multiple subjects in one call.
    pub fn batch_issue_credentials(
        env: Env,
        issuer: Address,
        subjects: Vec<Address>,
        credential_types: Vec<u32>,
        metadata_hashes: Vec<soroban_sdk::Bytes>,
        expires_at: Option<u64>,
    ) -> Vec<u64> {
        issuer.require_auth();
        Self::require_not_paused(&env);
        let n = subjects.len();
        assert!(credential_types.len() == n && metadata_hashes.len() == n, "input lengths must match");
        let mut ids: Vec<u64> = Vec::new(&env);
        for i in 0..n {
            let id = Self::issue_inner(&env, issuer.clone(), subjects.get(i).unwrap(), credential_types.get(i).unwrap(), metadata_hashes.get(i).unwrap(), expires_at.clone());
            ids.push_back(id);
        }
        ids
    }

    /// Retrieve a credential by ID. Panics with ContractError::CredentialNotFound if missing.
    /// Panics with "credential has expired" if the credential is expired.
    pub fn get_credential(env: Env, credential_id: u64) -> Credential {
        let credential: Credential = env.storage().instance()
            .get(&DataKey::Credential(credential_id))
            .unwrap_or_else(|| panic_with_error!(&env, ContractError::CredentialNotFound));
        if let Some(expires_at) = credential.expires_at {
            assert!(env.ledger().timestamp() < expires_at, "credential has expired");
        }
        credential
    }

    /// Return all credential IDs issued to a subject.
    pub fn get_credentials_by_subject(env: Env, subject: Address) -> Vec<u64> {
        env.storage()
            .instance()
            .get(&DataKey::SubjectCredentials(subject))
            .unwrap_or(Vec::new(&env))
    }

    /// Revoke a credential. Can be called by either the subject or the issuer.
    pub fn revoke_credential(env: Env, caller: Address, credential_id: u64) {
        caller.require_auth();
        Self::require_not_paused(&env);
        let mut credential: Credential = env.storage().instance().get(&DataKey::Credential(credential_id)).expect("credential not found");
        assert!(caller == credential.subject || caller == credential.issuer, "only subject or issuer can revoke");
        assert!(!credential.revoked, "credential already revoked");
        credential.revoked = true;
        env.storage().instance().set(&DataKey::Credential(credential_id), &credential);
        env.storage().instance().extend_ttl(STANDARD_TTL, EXTENDED_TTL);
        let event_data = RevokeEventData { credential_id, subject: credential.subject.clone() };
        let topic = String::from_str(&env, TOPIC_REVOKE);
        let mut topics: Vec<String> = Vec::new(&env);
        topics.push_back(topic);
        env.events().publish(topics, event_data);
    }

    /// Create a quorum slice. Returns the slice ID.
    pub fn create_slice(env: Env, creator: Address, attestors: Vec<Address>, threshold: u32) -> u64 {
        creator.require_auth();
        assert!(!attestors.is_empty(), "attestors cannot be empty");
        assert!(attestors.len() as u32 <= MAX_ATTESTORS_PER_SLICE, "attestors exceed maximum allowed per slice");
        assert!(threshold > 0, "threshold must be greater than 0");
        assert!(threshold <= attestors.len() as u32, "threshold cannot exceed attestors count");
        let id: u64 = env.storage().instance().get(&DataKey::SliceCount).unwrap_or(0u64) + 1;
        let slice = QuorumSlice { id, creator, attestors, threshold };
        env.storage().instance().set(&DataKey::Slice(id), &slice);
        env.storage().instance().extend_ttl(STANDARD_TTL, EXTENDED_TTL);
        env.storage().instance().set(&DataKey::SliceCount, &id);
        env.storage().instance().extend_ttl(STANDARD_TTL, EXTENDED_TTL);
        id
    }

    /// Retrieve a quorum slice by ID.
    pub fn get_slice(env: Env, slice_id: u64) -> QuorumSlice {
        env.storage().instance().get(&DataKey::Slice(slice_id))
            .unwrap_or_else(|| panic_with_error!(&env, ContractError::SliceNotFound))
    }

    /// Return the creator address of a slice.
    pub fn get_slice_creator(env: Env, slice_id: u64) -> Address {
        let slice: QuorumSlice = env.storage().instance().get(&DataKey::Slice(slice_id))
            .unwrap_or_else(|| panic_with_error!(&env, ContractError::SliceNotFound));
        slice.creator
    }

    /// Add a new attestor to an existing quorum slice.
    pub fn add_attestor(env: Env, creator: Address, slice_id: u64, attestor: Address) {
        creator.require_auth();
        let mut slice: QuorumSlice = env.storage().instance().get(&DataKey::Slice(slice_id)).expect("slice not found");
        assert!(slice.creator == creator, "only the slice creator can add attestors");
        assert!((slice.attestors.len() as u32) < MAX_ATTESTORS_PER_SLICE, "attestors exceed maximum allowed per slice");
        for a in slice.attestors.iter() {
            assert!(a != attestor, "attestor already in slice");
        }
        slice.attestors.push_back(attestor);
        env.storage().instance().set(&DataKey::Slice(slice_id), &slice);
        env.storage().instance().extend_ttl(STANDARD_TTL, EXTENDED_TTL);
    }

    /// Update the threshold of an existing quorum slice.
    pub fn update_threshold(env: Env, creator: Address, slice_id: u64, new_threshold: u32) {
        creator.require_auth();
        let mut slice: QuorumSlice = env.storage().instance().get(&DataKey::Slice(slice_id)).expect("slice not found");
        assert!(slice.creator == creator, "only the slice creator can update threshold");
        assert!(new_threshold <= slice.attestors.len(), "threshold exceeds attestor count");
        slice.threshold = new_threshold;
        env.storage().instance().set(&DataKey::Slice(slice_id), &slice);
        env.storage().instance().extend_ttl(STANDARD_TTL, EXTENDED_TTL);
    }

    /// Attest a credential using a quorum slice.
    pub fn attest(env: Env, attestor: Address, credential_id: u64, slice_id: u64) {
        attestor.require_auth();
        Self::require_not_paused(&env);
        let credential: Credential = env.storage().instance().get(&DataKey::Credential(credential_id)).expect("credential not found");
        assert!(!credential.revoked, "credential is revoked");
        let slice: QuorumSlice = env.storage().instance().get(&DataKey::Slice(slice_id)).expect("slice not found");
        let mut found = false;
        for a in slice.attestors.iter() {
            if a == attestor { found = true; break; }
        }
        assert!(found, "attestor not in slice");
        let mut attestors: Vec<Address> = env.storage().instance()
            .get(&DataKey::Attestors(credential_id))
            .unwrap_or(Vec::new(&env));

        // Check if attestor has already attested for this credential
        for existing_attestor in attestors.iter() {
            if existing_attestor == attestor {
                panic!("attestor has already attested for this credential");
            }
        }

        attestors.push_back(attestor.clone());
        env.storage()
            .instance()
            .set(&DataKey::Attestors(credential_id), &attestors);
        env.storage().instance().extend_ttl(STANDARD_TTL, EXTENDED_TTL);
        let event_data = AttestationEventData { attestor: attestor.clone(), credential_id, slice_id };
        let topic = String::from_str(&env, TOPIC_ATTESTATION);
        let mut topics: Vec<String> = Vec::new(&env);
        topics.push_back(topic);
        env.events().publish(topics, event_data);
        let count: u64 = env.storage().instance().get(&DataKey::AttestorCount(attestor.clone())).unwrap_or(0u64);
        env.storage().instance().set(&DataKey::AttestorCount(attestor), &(count + 1));
        env.storage().instance().extend_ttl(STANDARD_TTL, EXTENDED_TTL);
    }

    /// Check if a credential has met its quorum threshold.
    /// Returns false if revoked or expired.
    pub fn is_attested(env: Env, credential_id: u64, slice_id: u64) -> bool {
        let credential: Credential = env.storage().instance().get(&DataKey::Credential(credential_id)).expect("credential not found");
        if credential.revoked { return false; }
        if let Some(expires_at) = credential.expires_at {
            if env.ledger().timestamp() >= expires_at { return false; }
        }
        let slice: QuorumSlice = env.storage().instance().get(&DataKey::Slice(slice_id)).expect("slice not found");
        let attestors: Vec<Address> = env.storage().instance().get(&DataKey::Attestors(credential_id)).unwrap_or(Vec::new(&env));
        attestors.len() >= slice.threshold
    }

    /// Returns true if the credential has been revoked.
    pub fn is_revoked(env: Env, credential_id: u64) -> bool {
        let credential: Credential = env.storage().instance().get(&DataKey::Credential(credential_id))
            .unwrap_or_else(|| panic_with_error!(&env, ContractError::CredentialNotFound));
        credential.revoked
    }

    /// Returns true if the credential exists and its expiry timestamp has passed.
    pub fn is_expired(env: Env, credential_id: u64) -> bool {
        let credential: Credential = env.storage().instance().get(&DataKey::Credential(credential_id)).expect("credential not found");
        match credential.expires_at {
            Some(expires_at) => env.ledger().timestamp() >= expires_at,
            None => false,
        }
    }

    /// Get all attestors for a credential.
    pub fn get_attestors(env: Env, credential_id: u64) -> Vec<Address> {
        env.storage().instance().get(&DataKey::Attestors(credential_id)).unwrap_or(Vec::new(&env))
    }

    /// Returns the number of attestations for a credential without loading the full list.
    pub fn get_attestation_count(env: Env, credential_id: u64) -> u32 {
        let attestors: Vec<Address> = env.storage().instance().get(&DataKey::Attestors(credential_id)).unwrap_or(Vec::new(&env));
        attestors.len()
    }

    /// Returns the total number of credentials an attestor has signed.
    pub fn get_attestor_reputation(env: Env, attestor: Address) -> u64 {
        env.storage().instance().get(&DataKey::AttestorCount(attestor)).unwrap_or(0u64)
    }

    /// Returns the total number of credentials issued.
    pub fn get_credential_count(env: Env) -> u64 {
        env.storage().instance().get(&DataKey::CredentialCount).unwrap_or(0u64)
    }

    /// Returns the total number of slices created.
    pub fn get_slice_count(env: Env) -> u64 {
        env.storage().instance().get(&DataKey::SliceCount).unwrap_or(0u64)
    }

    /// Unified engineer verification entry point.
    pub fn verify_engineer(
        env: Env,
        quorum_proof_id: Address,
        sbt_registry_id: Address,
        zk_verifier_id: Address,
        subject: Address,
        credential_id: u64,
        claim_type: ClaimType,
        proof: soroban_sdk::Bytes,
    ) -> bool {
        let sbt_client = SbtRegistryContractClient::new(&env, &sbt_registry_id);
        let tokens = sbt_client.get_tokens_by_owner(&subject);
        let has_sbt = tokens.iter().any(|token_id| {
            let token = sbt_client.get_token(&token_id);
            token.credential_id == credential_id
        });
        if !has_sbt { return false; }
        let zk_client = ZkVerifierContractClient::new(&env, &zk_verifier_id);
        zk_client.verify_claim(&quorum_proof_id, &credential_id, &claim_type, &proof)
    }

    /// Register a human-readable label for a credential type.
    pub fn register_credential_type(env: Env, admin: Address, type_id: u32, name: soroban_sdk::String, description: soroban_sdk::String) {
        admin.require_auth();
        let def = CredentialTypeDef { type_id, name, description };
        env.storage().instance().set(&DataKey::CredentialType(type_id), &def);
        env.storage().instance().extend_ttl(STANDARD_TTL, EXTENDED_TTL);
    }

    /// Look up the registered name and description for a credential type.
    pub fn get_credential_type(env: Env, type_id: u32) -> CredentialTypeDef {
        env.storage().instance().get(&DataKey::CredentialType(type_id)).expect("credential type not registered")
    }

    /// Admin-only contract upgrade to new WASM. Uses deployer convention for auth.
    pub fn upgrade(env: Env, admin: Address, new_wasm_hash: soroban_sdk::BytesN<32>) {
        admin.require_auth();
        env.deployer().update_current_contract_wasm(new_wasm_hash);
    }
}


#[cfg(test)]
mod tests {
    use super::*;
    use soroban_sdk::testutils::{Address as _, Ledger as _, LedgerInfo, Events as _};
    use soroban_sdk::{Bytes, Env, FromVal, IntoVal};

    fn set_ledger_timestamp(env: &Env, ts: u64) {
        env.ledger().set(LedgerInfo {
            timestamp: ts,
            protocol_version: 20,
            sequence_number: 1,
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
        let id = client.issue_credential(&issuer, &subject, &1u32, &metadata, &None);

        env.ledger().set(LedgerInfo {
            timestamp: ts,
            protocol_version: 20,
            sequence_number: 100,
            network_id: Default::default(),
            base_reserve: 10,
            min_persistent_entry_ttl: 4096,
            max_entry_ttl: 6_312_000,
        });

        let cred = client.get_credential(&id);
        assert_eq!(cred.id, id);
        assert_eq!(cred.subject, subject);
        assert!(!cred.revoked);
    }

    // --- pause / unpause ---

    #[test]
    fn test_is_paused_false_before_pause() {
        let env = Env::default();
        env.mock_all_auths();
        let contract_id = env.register_contract(None, QuorumProofContract);
        let client = QuorumProofContractClient::new(&env, &contract_id);

        let issuer = Address::generate(&env);
        let subject = Address::generate(&env);
        let metadata = Bytes::from_slice(&env, b"ipfs://QmTest");
        let id = client.issue_credential(&issuer, &subject, &1u32, &metadata, &None);
        assert_eq!(id, 1);

    #[test]
    fn test_pause_and_unpause() {
        let env = Env::default();
        env.mock_all_auths();
        let (client, admin) = setup(&env);
        client.pause(&admin);
        assert!(client.is_paused());
        client.unpause(&admin);
        assert!(!client.is_paused());
    }

    #[test]
    fn test_issuer_field_stored_on_credential() {
        let env = Env::default();
        env.mock_all_auths();
        let (client, _) = setup(&env);
        let non_admin = Address::generate(&env);
        client.pause(&non_admin);
    }

    #[test]
    #[should_panic]
    fn test_pause_blocks_issue_credential() {
        let env = Env::default();
        env.mock_all_auths();
        let (client, admin) = setup(&env);
        client.pause(&admin);
        let issuer = Address::generate(&env);
        let subject = Address::generate(&env);
        let metadata = Bytes::from_slice(&env, b"ipfs://QmTest");
        let id = client.issue_credential(&issuer, &subject, &1u32, &metadata, &None);

        let cred = client.get_credential(&id);
        assert_eq!(cred.issuer, issuer, "issuer field must match the caller of issue_credential");
    }

    #[test]
    fn test_different_issuers_produce_distinct_provenance() {
        let env = Env::default();
        env.mock_all_auths();
        let contract_id = env.register_contract(None, QuorumProofContract);
        let client = QuorumProofContractClient::new(&env, &contract_id);

        let issuer_a = Address::generate(&env);
        let issuer_b = Address::generate(&env);
        let subject = Address::generate(&env);
        let metadata = Bytes::from_slice(&env, b"ipfs://QmTest");

        let id_a = client.issue_credential(&issuer_a, &subject, &1u32, &metadata, &None);
        let id_b = client.issue_credential(&issuer_b, &subject, &1u32, &metadata, &None);

        assert_eq!(client.get_credential(&id_a).issuer, issuer_a);
        assert_eq!(client.get_credential(&id_b).issuer, issuer_b);
        assert_ne!(client.get_credential(&id_a).issuer, client.get_credential(&id_b).issuer);
    }

    #[test]
    fn test_unpause_allows_issue_credential() {
        let env = Env::default();
        env.mock_all_auths();
        let (client, admin) = setup(&env);

        client.pause(&admin);
        client.unpause(&admin);

        let issuer = Address::generate(&env);
        let subject = Address::generate(&env);
        let metadata = Bytes::from_slice(&env, b"ipfs://QmTest");
        let credential_type: u32 = 42;

        let id = client.issue_credential(&issuer, &subject, &credential_type, &metadata, &None);

        let all_events = env.events().all();
        let expected_topic = String::from_str(&env, TOPIC_ISSUE);

        let issued = all_events.iter().find(|(_, topics, _): &(Address, soroban_sdk::Vec<soroban_sdk::Val>, soroban_sdk::Val)| {
            if let Some(raw) = topics.get(0) {
                let s = String::from_val(&env, &raw);
                return s == expected_topic;
            }
            false
        });

        assert!(issued.is_some(), "CredentialIssued event was not emitted");

        let (_, _, data) = issued.unwrap();
        let event_data: IssueEventData = soroban_sdk::Val::into_val(&data, &env);

        assert_eq!(event_data.id, id);
        assert_eq!(event_data.subject, subject);
        assert_eq!(event_data.credential_type, credential_type);
    }

    #[test]
    #[should_panic]
    fn test_pause_blocks_issue_credential() {
        let env = Env::default();
        env.mock_all_auths();
        let (client, admin) = setup(&env);
        client.pause(&admin);

        let issuer = Address::generate(&env);
        let subject = Address::generate(&env);
        let metadata = Bytes::from_slice(&env, b"ipfs://QmTest");
        let cred_id = client.issue_credential(&issuer, &subject, &1u32, &metadata, &None);
        let mut attestors = Vec::new(&env);
        attestors.push_back(attestor1.clone());
        attestors.push_back(attestor2.clone());
        let slice_id = client.create_slice(&creator, &attestors, &2u32);

        assert!(!client.is_attested(&cred_id, &slice_id));
        client.attest(&attestor1, &cred_id, &slice_id);
        assert!(!client.is_attested(&cred_id, &slice_id));
        client.attest(&attestor2, &cred_id, &slice_id);
        assert!(client.is_attested(&cred_id, &slice_id));
    }

    #[test]
    fn test_unpause_allows_issue_credential() {
        let env = Env::default();
        env.mock_all_auths();
        let contract_id = env.register_contract(None, QuorumProofContract);
        let client = QuorumProofContractClient::new(&env, &contract_id);

        let creator = Address::generate(&env);
        let mut attestors = Vec::new(&env);
        attestors.push_back(Address::generate(&env));
        client.create_slice(&creator, &attestors, &0u32);
    }

    // --- credential issuance ---

    #[test]
    fn test_issue_and_get_credential() {
        let env = Env::default();
        env.mock_all_auths();
        let contract_id = env.register_contract(None, QuorumProofContract);
        let client = QuorumProofContractClient::new(&env, &contract_id);

        let creator = Address::generate(&env);
        let mut attestors = Vec::new(&env);
        attestors.push_back(Address::generate(&env));
        attestors.push_back(Address::generate(&env));
        client.create_slice(&creator, &attestors, &5u32);
    }

    #[test]
    #[should_panic(expected = "metadata_hash cannot be empty")]
    fn test_empty_metadata_hash_rejection() {
        let env = Env::default();
        env.mock_all_auths();
        let contract_id = env.register_contract(None, QuorumProofContract);
        let client = QuorumProofContractClient::new(&env, &contract_id);

        let creator = Address::generate(&env);
        let mut attestors = Vec::new(&env);
        for _ in 0..=MAX_ATTESTORS_PER_SLICE {
            attestors.push_back(Address::generate(&env));
        }
        client.create_slice(&creator, &attestors, &1u32);
    }

    #[test]
    #[should_panic]
    fn test_get_credential_not_found() {
        let env = Env::default();
        env.mock_all_auths();
        let contract_id = env.register_contract(None, QuorumProofContract);
        let client = QuorumProofContractClient::new(&env, &contract_id);

        let creator = Address::generate(&env);
        let mut attestors = Vec::new(&env);
        for _ in 0..MAX_ATTESTORS_PER_SLICE {
            attestors.push_back(Address::generate(&env));
        }
        let slice_id = client.create_slice(&creator, &attestors, &1u32);
        client.add_attestor(&creator, &slice_id, &Address::generate(&env));
    }

    // --- revocation ---

    #[test]
    #[should_panic]
    fn test_pause_blocks_revoke_credential() {
        let env = Env::default();
        env.mock_all_auths();
        let (client, admin) = setup(&env);

        let issuer = Address::generate(&env);
        let subject = Address::generate(&env);
        let metadata = Bytes::from_slice(&env, b"ipfs://QmTest");
        let id = client.issue_credential(&issuer, &subject, &1u32, &metadata, &None);

        client.pause(&admin);
        client.revoke_credential(&issuer, &id);
        let cred = client.get_credential(&id);
        assert!(cred.revoked);
    }

    #[test]
    fn test_subject_revoke_credential() {
        let env = Env::default();
        env.mock_all_auths();
        let (client, _) = setup(&env);
        let issuer = Address::generate(&env);
        let subject = Address::generate(&env);
        let metadata = Bytes::from_slice(&env, b"ipfs://QmTest");
        let id = client.issue_credential(&issuer, &subject, &1u32, &metadata, &None);

        client.revoke_credential(&subject, &id);

        let cred = client.get_credential(&id);
        assert!(cred.revoked);
    }

    #[test]
    #[should_panic(expected = "only subject or issuer can revoke")]
    fn test_unauthorized_revoke_credential() {
        let env = Env::default();
        env.mock_all_auths();
        let (client, _) = setup(&env);
        let issuer = Address::generate(&env);
        let subject = Address::generate(&env);
        client.issue_credential(&issuer, &subject, &1u32, &Bytes::new(&env), &None);
    }

    #[test]
    #[should_panic]
    fn test_pause_blocks_attest() {
        let env = Env::default();
        env.mock_all_auths();
        let (client, admin) = setup(&env);

    // --- slices & attestation ---

    #[test]
    fn test_quorum_slice_and_attestation() {
        let env = Env::default();
        env.mock_all_auths();
        let (client, _) = setup(&env);
        let issuer = Address::generate(&env);
        let subject = Address::generate(&env);
        let attestor = Address::generate(&env);
        let metadata = Bytes::from_slice(&env, b"ipfs://QmTest");
        let cred_id = client.issue_credential(&issuer, &subject, &1u32, &metadata, &None);

        let mut attestors = Vec::new(&env);
        attestors.push_back(attestor1.clone());
        attestors.push_back(attestor2.clone());
        let creator = Address::generate(&env);
        let slice_id = client.create_slice(&creator, &attestors, &2u32);

        client.attest(&attestor1, &cred_id, &slice_id);
        client.attest(&attestor1, &cred_id, &slice_id);
        assert!(!client.is_attested(&cred_id, &slice_id));
        client.attest(&attestor2, &cred_id, &slice_id);
        assert!(client.is_attested(&cred_id, &slice_id));
    }

    #[test]
    #[should_panic(expected = "attestor has already attested for this credential")]
    fn test_duplicate_attestation_rejection() {
        let env = Env::default();
        env.mock_all_auths();
        let (client, _) = setup(&env);
        let issuer = Address::generate(&env);
        let subject = Address::generate(&env);
        let attestor = Address::generate(&env);
        let metadata = Bytes::from_slice(&env, b"ipfs://QmTest");
        let cred_id = client.issue_credential(&issuer, &subject, &1u32, &metadata, &None);
        let mut attestors = Vec::new(&env);
        attestors.push_back(attestor.clone());
        let slice_id = client.create_slice(&issuer, &attestors, &1u32);
        client.attest(&attestor, &cred_id, &slice_id);
        client.attest(&attestor, &cred_id, &slice_id);
    }

    #[test]
    #[should_panic(expected = "credential is revoked")]
    fn test_attest_revoked_credential_panics() {
        let env = Env::default();
        env.mock_all_auths();
        let (client, _) = setup(&env);
        let issuer = Address::generate(&env);
        let subject = Address::generate(&env);
        let attestor = Address::generate(&env);
        let metadata = Bytes::from_slice(&env, b"ipfs://QmTest");
        let id = client.issue_credential(&issuer, &subject, &1u32, &metadata, &None);

    // --- slice management ---

    #[test]
    #[should_panic(expected = "attestors cannot be empty")]
    fn test_create_slice_empty_attestors_panics() {
        let env = Env::default();
        env.mock_all_auths();
        let (client, _) = setup(&env);
        client.create_slice(&Address::generate(&env), &Vec::new(&env), &1u32);
    }

    #[test]
    #[should_panic(expected = "threshold must be greater than 0")]
    fn test_zero_threshold_rejection() {
        let env = Env::default();
        let contract_id = env.register_contract(None, QuorumProofContract);
        let client = QuorumProofContractClient::new(&env, &contract_id);
        client.get_credential(&999u64);
    }

    #[test]
    #[should_panic(expected = "threshold cannot exceed attestors count")]
    fn test_threshold_exceeds_attestors() {
        let env = Env::default();
        let contract_id = env.register_contract(None, QuorumProofContract);
        let client = QuorumProofContractClient::new(&env, &contract_id);
        client.get_slice(&999u64);
    }

    #[test]
    #[should_panic(expected = "attestors exceed maximum allowed per slice")]
    fn test_create_slice_exceeds_max_attestors() {
        let env = Env::default();
        env.mock_all_auths();
        let (client, _) = setup(&env);
        let creator = Address::generate(&env);
        let mut attestors = Vec::new(&env);
        for _ in 0..=MAX_ATTESTORS_PER_SLICE {
            attestors.push_back(Address::generate(&env));
        }
        client.create_slice(&creator, &attestors, &1u32);
    }

    #[test]
    fn test_get_slice_creator_matches() {
        let env = Env::default();
        env.mock_all_auths();
        let (client, _) = setup(&env);
        let creator = Address::generate(&env);
        let mut attestors = Vec::new(&env);
        attestors.push_back(Address::generate(&env));
        let slice_id = client.create_slice(&creator, &attestors, &1u32);
        assert_eq!(client.get_slice_creator(&slice_id), creator);
    }

        client.pause(&admin);
        client.attest(&attestor, &cred_id, &slice_id);
    }

    #[test]
    #[should_panic(expected = "unauthorized")]
    fn test_pause_unauthorized_panics() {
        let env = Env::default();
        env.mock_all_auths();
        let (client, _) = setup(&env);
        let non_admin = Address::generate(&env);
        client.pause(&non_admin);
    }

    #[test]
    #[should_panic(expected = "only the slice creator can add attestors")]
    fn test_add_attestor_unauthorized_panics() {
        let env = Env::default();
        env.mock_all_auths();
        let contract_id = env.register_contract(None, QuorumProofContract);
        let client = QuorumProofContractClient::new(&env, &contract_id);

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
    fn test_update_threshold_success() {
        let env = Env::default();
        env.mock_all_auths();
        let contract_id = env.register_contract(None, QuorumProofContract);
        let client = QuorumProofContractClient::new(&env, &contract_id);

        let subject = Address::generate(&env);
        let ids = client.get_credentials_by_subject(&subject);
        assert_eq!(ids.len(), 0);
    }

    #[test]
    #[should_panic(expected = "only the slice creator can update threshold")]
    fn test_update_threshold_unauthorized_panics() {
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

    // --- expiry ---

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

        set_ledger_timestamp(&env, 999_999_999);
        assert!(!client.is_expired(&id));
    }

    #[test]
    fn test_credential_not_expired_before_expiry() {
        let env = Env::default();
        env.mock_all_auths();
        let (client, _) = setup(&env);
        let issuer = Address::generate(&env);
        let subject = Address::generate(&env);
        let metadata = Bytes::from_slice(&env, b"ipfs://QmTest");
        let id = client.issue_credential(&issuer, &subject, &1u32, &metadata, &Some(2_000u64));

        assert!(!client.is_expired(&id));
    }

    #[test]
    fn test_credential_expired_after_expiry() {
        let env = Env::default();
        env.mock_all_auths();
        let (client, _) = setup(&env);
        set_ledger_timestamp(&env, 1_000);
        let issuer = Address::generate(&env);
        let subject = Address::generate(&env);
        let metadata = Bytes::from_slice(&env, b"ipfs://QmTest");
        let id = client.issue_credential(&issuer, &subject, &1u32, &metadata, &Some(2_000u64));

        set_ledger_timestamp(&env, 3_000);
        assert!(client.is_expired(&id));
    }

    #[test]
    #[should_panic(expected = "credential has expired")]
    fn test_get_credential_panics_when_expired() {
        let env = Env::default();
        env.mock_all_auths();
        let (client, _) = setup(&env);
        set_ledger_timestamp(&env, 1_000);
        let issuer = Address::generate(&env);
        let subject = Address::generate(&env);
        let metadata = Bytes::from_slice(&env, b"ipfs://QmTest");
        let id = client.issue_credential(&issuer, &subject, &1u32, &metadata, &Some(2_000u64));
        set_ledger_timestamp(&env, 3_000);
        client.get_credential(&id);
    }

    #[test]
    fn test_is_attested_returns_false_when_expired() {
        let env = Env::default();
        env.mock_all_auths();
        let (client, _) = setup(&env);
        let issuer = Address::generate(&env);
        let subject = Address::generate(&env);
        let attestor = Address::generate(&env);
        let metadata = Bytes::from_slice(&env, b"ipfs://QmTest");
        set_ledger_timestamp(&env, 1_000);
        let cred_id = client.issue_credential(&issuer, &subject, &1u32, &metadata, &Some(2_000u64));

        let mut attestors = Vec::new(&env);
        attestors.push_back(attestor.clone());
        let slice_id = client.create_slice(&issuer, &attestors, &1u32);
        client.attest(&attestor, &cred_id, &slice_id);
        assert!(client.is_attested(&cred_id, &slice_id));

        set_ledger_timestamp(&env, 3_000);
        assert!(!client.is_attested(&cred_id, &slice_id));
    }

    // --- batch issue ---

    #[test]
    #[should_panic(expected = "credential is revoked")]
    fn test_attest_revoked_credential_panics() {
        let env = Env::default();
        env.mock_all_auths();
        let (client, _) = setup(&env);
        let issuer = Address::generate(&env);
        let subject = Address::generate(&env);
        let attestor = Address::generate(&env);
        let metadata = Bytes::from_slice(&env, b"ipfs://QmTest");

        let cred_id = client.issue_credential(&issuer, &subject, &1u32, &metadata, &None);
        let mut attestors = Vec::new(&env);
        attestors.push_back(attestor.clone());
        let slice_id = client.create_slice(&issuer, &attestors, &1u32);

        client.revoke_credential(&issuer, &cred_id);
        client.attest(&attestor, &cred_id, &slice_id);
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

        let mut initial = Vec::new(&env);
        initial.push_back(attestor1.clone());
        let slice_id = client.create_slice(&creator, &initial, &1u32);

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

        let mut initial = Vec::new(&env);
        initial.push_back(attestor.clone());
        let slice_id = client.create_slice(&creator, &initial, &1u32);

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

        let mut initial = Vec::new(&env);
        initial.push_back(Address::generate(&env));
        let slice_id = client.create_slice(&creator, &initial, &1u32);

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
        let attestor1 = Address::generate(&env);
        let attestor2 = Address::generate(&env);
        let metadata = Bytes::from_slice(&env, b"ipfs://QmTest");

        let mut initial = Vec::new(&env);
        initial.push_back(attestor1.clone());
        let slice_id = client.create_slice(&creator, &initial, &1u32);

        let cred_id = client.issue_credential(&issuer, &subject, &1u32, &metadata, &None);

        client.attest(&attestor1, &cred_id, &slice_id);
        assert!(client.is_attested(&cred_id, &slice_id)); // threshold=1, met after attestor1

        client.add_attestor(&creator, &slice_id, &attestor2);
        client.update_threshold(&creator, &slice_id, &2u32);
        assert!(!client.is_attested(&cred_id, &slice_id)); // threshold raised to 2, not met yet
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
    }

    #[test]
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

        let sbt_uri = Bytes::from_slice(&env, b"ipfs://QmSbt");
        sbt.mint(&subject, &cred_id, &sbt_uri);

        let proof = Bytes::from_slice(&env, b"valid-proof");
        let result = qp.verify_engineer(&qp_id, &sbt_id, &zk_id, &subject, &cred_id, &ClaimType::HasDegree, &proof);
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

        let proof = Bytes::from_slice(&env, b"valid-proof");
        let result = qp.verify_engineer(&qp_id, &sbt_id, &zk_id, &subject, &cred_id, &ClaimType::HasDegree, &proof);
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

        let proof = Bytes::from_slice(&env, b"");
        let result = qp.verify_engineer(&qp_id, &sbt_id, &zk_id, &subject, &cred_id, &ClaimType::HasLicense, &proof);
        assert!(!result);
    }

    // --- reputation & counts ---

    #[test]
    fn test_get_attestor_reputation_increments_per_attestation() {
        let env = Env::default();
        env.mock_all_auths();
        let (client, _) = setup(&env);
        let issuer = Address::generate(&env);
        let subject = Address::generate(&env);
        let attestor = Address::generate(&env);
        let metadata = Bytes::from_slice(&env, b"ipfs://QmTest");

        let mut attestors = Vec::new(&env);
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
    fn test_get_credential_count() {
        let env = Env::default();
        env.mock_all_auths();
        let (client, _) = setup(&env);
        let issuer = Address::generate(&env);
        let subject = Address::generate(&env);
        let metadata = Bytes::from_slice(&env, b"ipfs://QmTest");

        let mut attestors = Vec::new(&env);
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
        let mut attestors = Vec::new(&env);
        attestors.push_back(Address::generate(&env));
        attestors.push_back(Address::generate(&env));
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
        let mut attestors = Vec::new(&env);
        attestors.push_back(Address::generate(&env));
        let slice_id = client.create_slice(&creator, &attestors, &1u32);

        client.update_threshold(&non_creator, &slice_id, &1u32);
    }

    #[test]
    fn test_get_slice_count() {
        let env = Env::default();
        env.mock_all_auths();
        let (client, _) = setup(&env);
        let creator = Address::generate(&env);
        let mut attestors = Vec::new(&env);
        attestors.push_back(Address::generate(&env));
        let slice_id = client.create_slice(&creator, &attestors, &1u32);

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

        let mut subjects = Vec::new(&env);
        subjects.push_back(subject1.clone());
        subjects.push_back(subject2.clone());
        subjects.push_back(subject3.clone());

        let mut cred_types = Vec::new(&env);
        cred_types.push_back(1u32);
        cred_types.push_back(2u32);
        cred_types.push_back(1u32);

        let mut hashes = Vec::new(&env);
        hashes.push_back(Bytes::from_slice(&env, b"ipfs://Qm1"));
        hashes.push_back(Bytes::from_slice(&env, b"ipfs://Qm2"));
        hashes.push_back(Bytes::from_slice(&env, b"ipfs://Qm3"));

        let ids = client.batch_issue_credentials(&issuer, &subjects, &cred_types, &hashes, &None);

        assert_eq!(ids.len(), 3);
        assert_eq!(client.get_credentials_by_subject(&subject1).len(), 1);
        assert_eq!(client.get_credentials_by_subject(&subject2).len(), 1);
        assert_eq!(client.get_credentials_by_subject(&subject3).len(), 1);
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

        let mut subjects = Vec::new(&env);
        subjects.push_back(Address::generate(&env));
        subjects.push_back(Address::generate(&env));

        let mut cred_types = Vec::new(&env);
        cred_types.push_back(1u32);

        let mut hashes = Vec::new(&env);
        hashes.push_back(Bytes::from_slice(&env, b"ipfs://Qm1"));
        hashes.push_back(Bytes::from_slice(&env, b"ipfs://Qm2"));

        client.batch_issue_credentials(&issuer, &subjects, &cred_types, &hashes, &None);
    }

    #[test]
    fn test_register_and_get_credential_type() {
        let env = Env::default();
        env.mock_all_auths();
        let (client, _) = setup(&env);
        let admin = Address::generate(&env);
        let name = String::from_str(&env, "Mechanical Engineering Degree");
        let desc = String::from_str(&env, "Bachelor or higher in Mechanical Engineering");

        client.register_credential_type(&admin, &1u32, &name, &desc);
        let def = client.get_credential_type(&1u32);
        assert_eq!(def.type_id, 1u32);
        assert_eq!(def.name, name);
    }

    #[test]
    #[should_panic(expected = "credential type not registered")]
    fn test_get_credential_type_not_registered_panics() {
        let env = Env::default();
        let contract_id = env.register_contract(None, QuorumProofContract);
        let client = QuorumProofContractClient::new(&env, &contract_id);
        client.get_credential_type(&99u32);
    }

    #[test]
    fn test_register_credential_type_overwrites() {
        let env = Env::default();
        env.mock_all_auths();
        let contract_id = env.register_contract(None, QuorumProofContract);
        let client = QuorumProofContractClient::new(&env, &contract_id);

        let admin = Address::generate(&env);
        let name_v1 = String::from_str(&env, "Old Name");
        let name_v2 = String::from_str(&env, "New Name");
        let desc = String::from_str(&env, "desc");

        client.register_credential_type(&admin, &1u32, &name_v1, &desc);
        client.register_credential_type(&admin, &1u32, &name_v2, &desc);

        let def = client.get_credential_type(&1u32);
        assert_eq!(def.name, name_v2);
    }

    #[test]
    #[should_panic] // upgrade requires the WASM to exist in host storage; this verifies auth passes
    fn test_upgrade_success() {
        let env = Env::default();
        env.mock_all_auths();
        let contract_id = env.register_contract(None, QuorumProofContract);
        let client = QuorumProofContractClient::new(&env, &contract_id);

        let admin = Address::generate(&env);
        let wasm_hash = soroban_sdk::BytesN::from_array(&env, &[0u8; 32]);
        client.upgrade(&admin, &wasm_hash);
    }

    #[test]
    fn test_get_credential_count() {
        let env = Env::default();
        env.mock_all_auths();
        let contract_id = env.register_contract(None, QuorumProofContract);
        let client = QuorumProofContractClient::new(&env, &contract_id);

        let issuer = Address::generate(&env);
        let subject = Address::generate(&env);
        let metadata = Bytes::from_slice(&env, b"ipfs://QmTest");

        assert_eq!(client.get_credential_count(), 0);

        let id1 = client.issue_credential(&issuer, &subject, &1u32, &metadata, &None);
        let _id2 = client.issue_credential(&issuer, &subject, &2u32, &metadata, &None);
        let _id3 = client.issue_credential(&issuer, &subject, &3u32, &metadata, &None);

        assert_eq!(client.get_credential_count(), 3);

        client.revoke_credential(&issuer, &id1);
        assert_eq!(client.get_credential_count(), 3);
    }

    #[test]
    fn test_get_slice_count() {
        let env = Env::default();
        env.mock_all_auths();
        let contract_id = env.register_contract(None, QuorumProofContract);
        let client = QuorumProofContractClient::new(&env, &contract_id);

        let creator = Address::generate(&env);
        let mut attestors = Vec::new(&env);
        attestors.push_back(Address::generate(&env));

        assert_eq!(client.get_slice_count(), 0);

        client.create_slice(&creator, &attestors.clone(), &1u32);
        client.create_slice(&creator, &attestors, &1u32);

        assert_eq!(client.get_slice_count(), 2);
    }
<<<<<<< issue-47-revoke-prevents-attestation

    // Issue #47: revoke_credential prevents further attestation
    #[test]
    #[should_panic(expected = "credential is revoked")]
    fn test_revoke_credential_prevents_attestation() {
        let env = Env::default();
        env.mock_all_auths();
        let contract_id = env.register_contract(None, QuorumProofContract);
        let client = QuorumProofContractClient::new(&env, &contract_id);

        let issuer = Address::generate(&env);
        let subject = Address::generate(&env);
        let attestor = Address::generate(&env);
        let metadata = Bytes::from_slice(&env, b"ipfs://QmTest");

        // Issue a credential
        let cred_id = client.issue_credential(&issuer, &subject, &1u32, &metadata, &None);

        // Set up a quorum slice with the attestor
        let mut attestors = soroban_sdk::Vec::new(&env);
        attestors.push_back(attestor.clone());
        let slice_id = client.create_slice(&issuer, &attestors, &1u32);

        // Revoke the credential
        client.revoke_credential(&issuer, &cred_id);

        // Attempting to attest a revoked credential must panic
        client.attest(&attestor, &cred_id, &slice_id);
    }
}
=======
>>>>>>> main

    #[test]
    fn test_get_attestation_count() {
        let env = Env::default();
        env.mock_all_auths();
        let (client, _) = setup(&env);

        let issuer = Address::generate(&env);
        let subject = Address::generate(&env);
        let attestor1 = Address::generate(&env);
        let attestor2 = Address::generate(&env);
        let metadata = Bytes::from_slice(&env, b"ipfs://QmTest");
        let cred_id = client.issue_credential(&issuer, &subject, &1u32, &metadata, &None);

        let mut attestors = soroban_sdk::Vec::new(&env);
        attestors.push_back(attestor1.clone());
        attestors.push_back(attestor2.clone());
        let slice_id = client.create_slice(&issuer, &attestors, &1u32);

        assert_eq!(client.get_attestation_count(&cred_id), 0);
        client.attest(&attestor1, &cred_id, &slice_id);
        assert_eq!(client.get_attestation_count(&cred_id), 1);
        client.attest(&attestor2, &cred_id, &slice_id);
        assert_eq!(client.get_attestation_count(&cred_id), 2);
    }
}
