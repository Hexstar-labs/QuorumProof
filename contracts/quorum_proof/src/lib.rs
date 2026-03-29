#![no_std]
use sbt_registry::SbtRegistryContractClient;
use soroban_sdk::{
    contract, contracterror, contractimpl, contracttype, panic_with_error, Address, Env, String,
    Vec,
};
use zk_verifier::{ClaimType, ZkVerifierContractClient};

const TOPIC_ISSUE: &str = "CredentialIssued";
const TOPIC_REVOKE: &str = "RevokeCredential";
const TOPIC_ATTESTATION: &str = "attestation";
const TOPIC_RENEWAL: &str = "CredentialRenewed";
const TOPIC_SBT_TRANSFER: &str = "SbtTransferred";
const TOPIC_PROOF_REQUEST: &str = "ProofRequested";
const STANDARD_TTL: u32 = 16_384;
const EXTENDED_TTL: u32 = 524_288;
const MAX_ATTESTORS_PER_SLICE: u32 = 20;

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

#[contracttype]
#[derive(Clone)]
pub struct AttestationEventData {
    pub attestor: Address,
    pub credential_id: u64,
    pub slice_id: u64,
}

#[contracttype]
#[derive(Clone)]
pub struct RenewalEventData {
    pub credential_id: u64,
    pub issuer: Address,
    pub new_expires_at: u64,
}

#[contracterror]
#[derive(Copy, Clone, Debug, Eq, PartialEq, PartialOrd, Ord)]
#[repr(u32)]
pub enum ContractError {
    CredentialNotFound = 1,
    SliceNotFound = 2,
    ContractPaused = 3,
    DuplicateCredential = 4,
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
    SubjectIssuerType(Address, Address, u32),
    /// Stores the Vec<ProofRequest> history for a credential
    ProofRequests(u64),
    /// Global monotonic counter for proof request IDs
    ProofRequestCount,
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

/// A single proof request record, capturing who requested proof of a credential and when.
#[contracttype]
#[derive(Clone)]
pub struct ProofRequest {
    /// Unique monotonic ID across all proof requests on this contract.
    pub id: u64,
    /// The credential for which proof was requested.
    pub credential_id: u64,
    /// The address of the verifier that initiated this request.
    pub verifier: Address,
    /// Ledger timestamp at the time this request was created.
    pub requested_at: u64,
    /// The ZK claim types the verifier wants proven.
    pub claim_types: Vec<zk_verifier::ClaimType>,
}

/// QuorumSlice represents a federated Byzantine agreement (FBA) trust slice.
/// Each attestor has an associated weight that contributes to the threshold check.
/// The threshold represents the minimum total weight of attestors required
/// for a credential to be considered attested, not just the count of attestors.
///
/// This implements a weighted FBA model where trust is proportional to the
/// stake/weight assigned to each attestor, as described in the Stellar whitepaper.
#[contracttype]
#[derive(Clone)]
pub struct QuorumSlice {
    pub id: u64,
    pub creator: Address,
    pub attestors: Vec<Address>,
    /// Weights corresponding to each attestor. Each weight represents the
    /// attestor's stake/contribution to the quorum. Higher weight = more trust.
    pub weights: Vec<u32>,
    /// Threshold is measured in weight units, not attestor count.
    /// The sum of weights from attesting parties must meet or exceed this value.
    pub threshold: u32,
}

#[contract]
pub struct QuorumProofContract;

#[contractimpl]
impl QuorumProofContract {
    /// Set the admin address once after deployment. Panics if already initialized.
    pub fn initialize(env: Env, admin: Address) {
        assert!(
            !env.storage().instance().has(&DataKey::Admin),
            "already initialized"
        );
        env.storage().instance().set(&DataKey::Admin, &admin);
        env.storage()
            .instance()
            .extend_ttl(STANDARD_TTL, EXTENDED_TTL);
    }

    /// Pause the contract. Only admin may call this.
    pub fn pause(env: Env, admin: Address) {
        admin.require_auth();
        let stored: Address = env
            .storage()
            .instance()
            .get(&DataKey::Admin)
            .expect("not initialized");
        assert!(stored == admin, "unauthorized");
        env.storage().instance().set(&DataKey::Paused, &true);
        env.storage()
            .instance()
            .extend_ttl(STANDARD_TTL, EXTENDED_TTL);
    }

    /// Unpause the contract. Only admin may call this.
    pub fn unpause(env: Env, admin: Address) {
        admin.require_auth();
        let stored: Address = env
            .storage()
            .instance()
            .get(&DataKey::Admin)
            .expect("not initialized");
        assert!(stored == admin, "unauthorized");
        env.storage().instance().set(&DataKey::Paused, &false);
        env.storage()
            .instance()
            .extend_ttl(STANDARD_TTL, EXTENDED_TTL);
    }

    /// Returns true if the contract is currently paused.
    pub fn is_paused(env: Env) -> bool {
        env.storage()
            .instance()
            .get(&DataKey::Paused)
            .unwrap_or(false)
    }

    fn require_not_paused(env: &Env) {
        if env
            .storage()
            .instance()
            .get(&DataKey::Paused)
            .unwrap_or(false)
        {
            panic_with_error!(env, ContractError::ContractPaused);
        }
    }

    /// Issue a new credential to a subject. Returns the new credential ID.
    ///
    /// # Parameters
    /// - `issuer`: The address issuing the credential; must authorize this call.
    /// - `subject`: The address receiving the credential.
    /// - `credential_type`: Numeric type identifier for the credential.
    /// - `metadata_hash`: Non-empty IPFS or content-addressed hash of credential metadata.
    /// - `expires_at`: Optional Unix timestamp after which the credential is considered expired.
    ///
    /// # Panics
    /// Panics if the contract is paused.
    /// Panics if `metadata_hash` is empty.
    /// Panics with `ContractError::DuplicateCredential` if the same issuer has already issued
    /// a credential of the same type to the same subject.
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
        
        // Check for duplicate credential of same type from same issuer to same subject
        let duplicate_key = DataKey::SubjectIssuerType(subject.clone(), issuer.clone(), credential_type);
        if env.storage().instance().has(&duplicate_key) {
            panic_with_error!(&env, ContractError::DuplicateCredential);
        }
        
        let id: u64 = env.storage().instance().get(&DataKey::CredentialCount).unwrap_or(0u64) + 1;
        let credential = Credential { id, subject: subject.clone(), issuer: issuer.clone(), credential_type, metadata_hash, revoked: false, expires_at };
        env.storage().instance().set(&DataKey::Credential(id), &credential);
        env.storage().instance().extend_ttl(STANDARD_TTL, EXTENDED_TTL);
        env.storage().instance().set(&DataKey::CredentialCount, &id);
        env.storage()
            .instance()
            .extend_ttl(STANDARD_TTL, EXTENDED_TTL);
        let mut subject_creds: Vec<u64> = env
            .storage()
            .instance()
            .get(&DataKey::SubjectCredentials(subject.clone()))
            .unwrap_or(Vec::new(&env));
        subject_creds.push_back(id);
        env.storage().instance().set(&DataKey::SubjectCredentials(subject), &subject_creds);
        env.storage().instance().extend_ttl(STANDARD_TTL, EXTENDED_TTL);
        
        // Store duplicate prevention mapping
        env.storage().instance().set(&duplicate_key, &id);
        env.storage().instance().extend_ttl(STANDARD_TTL, EXTENDED_TTL);
        
        let event_data = CredentialIssuedEventData { id, subject: credential.subject.clone(), credential_type };
        let topic = String::from_str(&env, TOPIC_ISSUE);
        let mut topics: Vec<String> = Vec::new(&env);
        topics.push_back(topic);
        env.events().publish(topics, event_data);
        id
    }

    /// Issue credentials to multiple subjects in one call. Returns a `Vec` of new credential IDs
    /// in the same order as the input subjects.
    ///
    /// # Parameters
    /// - `issuer`: The address issuing all credentials; must authorize this call.
    /// - `subjects`: Ordered list of recipient addresses.
    /// - `credential_types`: Ordered list of credential type IDs, one per subject.
    /// - `metadata_hashes`: Ordered list of metadata hashes, one per subject.
    /// - `expires_at`: Optional shared expiry timestamp applied to all issued credentials.
    ///
    /// # Panics
    /// Panics if the contract is paused.
    /// Panics if `subjects`, `credential_types`, and `metadata_hashes` have different lengths.
    /// Panics for any individual credential that would violate duplicate or empty-hash rules.
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
        assert!(
            credential_types.len() == n && metadata_hashes.len() == n,
            "input lengths must match"
        );
        let mut ids: Vec<u64> = Vec::new(&env);
        for i in 0..n {
            let id = Self::issue_credential(env.clone(), issuer.clone(), subjects.get(i).unwrap(), credential_types.get(i).unwrap(), metadata_hashes.get(i).unwrap(), expires_at.clone());
            ids.push_back(id);
        }
        ids
    }

    fn issue_inner(
        env: &Env,
        issuer: Address,
        subject: Address,
        credential_type: u32,
        metadata_hash: soroban_sdk::Bytes,
        expires_at: Option<u64>,
    ) -> u64 {
        assert!(!metadata_hash.is_empty(), "metadata_hash cannot be empty");
        let id: u64 = env
            .storage()
            .instance()
            .get(&DataKey::CredentialCount)
            .unwrap_or(0u64)
            + 1;
        let credential = Credential {
            id,
            subject: subject.clone(),
            issuer,
            credential_type,
            metadata_hash,
            revoked: false,
            expires_at,
        };
        env.storage()
            .instance()
            .set(&DataKey::Credential(id), &credential);
        env.storage()
            .instance()
            .extend_ttl(STANDARD_TTL, EXTENDED_TTL);
        env.storage().instance().set(&DataKey::CredentialCount, &id);
        env.storage()
            .instance()
            .extend_ttl(STANDARD_TTL, EXTENDED_TTL);
        let mut subject_creds: Vec<u64> = env
            .storage()
            .instance()
            .get(&DataKey::SubjectCredentials(subject.clone()))
            .unwrap_or(Vec::new(env));
        subject_creds.push_back(id);
        env.storage()
            .instance()
            .set(&DataKey::SubjectCredentials(subject), &subject_creds);
        env.storage()
            .instance()
            .extend_ttl(STANDARD_TTL, EXTENDED_TTL);
        let event_data = CredentialIssuedEventData {
            id,
            subject: credential.subject.clone(),
            credential_type,
        };
        let topic = String::from_str(env, TOPIC_ISSUE);
        let mut topics: Vec<String> = Vec::new(env);
        topics.push_back(topic);
        env.events().publish(topics, event_data);
        id
    }

    /// Retrieve a credential by ID.
    ///
    /// # Parameters
    /// - `credential_id`: The ID of the credential to retrieve.
    ///
    /// # Panics
    /// Panics with `ContractError::CredentialNotFound` if no credential exists with that ID.
    /// Panics with "credential has expired" if the credential's `expires_at` has passed.
    pub fn get_credential(env: Env, credential_id: u64) -> Credential {
        let credential: Credential = env.storage().instance()
            .get(&DataKey::Credential(credential_id))
            .unwrap_or_else(|| panic_with_error!(&env, ContractError::CredentialNotFound));
        if let Some(expires_at) = credential.expires_at {
            assert!(
                env.ledger().timestamp() < expires_at,
                "credential has expired"
            );
        }
        credential
    }

    /// Return all credential IDs issued to a subject.
    ///
    /// # Parameters
    /// - `subject`: The address whose credentials to look up.
    ///
    /// # Panics
    /// Does not panic; returns an empty `Vec` if the subject has no credentials.
    pub fn get_credentials_by_subject(env: Env, subject: Address) -> Vec<u64> {
        env.storage()
            .instance()
            .get(&DataKey::SubjectCredentials(subject))
            .unwrap_or(Vec::new(&env))
    }

    /// Revoke a credential. Only the original issuer can revoke.
    ///
    /// # Parameters
    /// - `issuer`: The address that originally issued the credential; must authorize this call.
    /// - `credential_id`: The ID of the credential to revoke.
    ///
    /// # Panics
    /// Panics if the contract is paused.
    /// Panics with `ContractError::CredentialNotFound` if no credential exists with that ID.
    /// Panics if the caller is not the original issuer.
    /// Panics if the credential is already revoked.
    /// Panics with "credential has expired" if the credential's `expires_at` has passed.
    pub fn revoke_credential(env: Env, issuer: Address, credential_id: u64) {
        issuer.require_auth();
        Self::require_not_paused(&env);
        let mut credential: Credential = env.storage().instance()
            .get(&DataKey::Credential(credential_id))
            .unwrap_or_else(|| panic_with_error!(&env, ContractError::CredentialNotFound));
        assert!(issuer == credential.issuer, "only the original issuer can revoke");
        assert!(!credential.revoked, "credential already revoked");
        if let Some(expires_at) = credential.expires_at {
            assert!(
                env.ledger().timestamp() < expires_at,
                "credential has expired"
            );
        }
        credential.revoked = true;
        env.storage()
            .instance()
            .set(&DataKey::Credential(credential_id), &credential);
        env.storage()
            .instance()
            .extend_ttl(STANDARD_TTL, EXTENDED_TTL);
        let event_data = RevokeEventData {
            credential_id,
            subject: credential.subject.clone(),
        };
        let topic = String::from_str(&env, TOPIC_REVOKE);
        let mut topics: Vec<String> = Vec::new(&env);
        topics.push_back(topic);
        env.events().publish(topics, event_data);
    }

    /// Renew a credential by extending its expiry. Only the original issuer may call this.
    /// Emits a renewal event.
    pub fn renew_credential(env: Env, issuer: Address, credential_id: u64, new_expires_at: u64) {
        issuer.require_auth();
        Self::require_not_paused(&env);
        let mut credential: Credential = env.storage().instance()
            .get(&DataKey::Credential(credential_id))
            .unwrap_or_else(|| panic_with_error!(&env, ContractError::CredentialNotFound));
        assert!(credential.issuer == issuer, "only the original issuer can renew");
        assert!(!credential.revoked, "cannot renew a revoked credential");
        assert!(new_expires_at > env.ledger().timestamp(), "new_expires_at must be in the future");
        credential.expires_at = Some(new_expires_at);
        env.storage().instance().set(&DataKey::Credential(credential_id), &credential);
        env.storage().instance().extend_ttl(STANDARD_TTL, EXTENDED_TTL);
        let event_data = RenewalEventData { credential_id, issuer, new_expires_at };
        let topic = String::from_str(&env, TOPIC_RENEWAL);
        let mut topics: Vec<String> = Vec::new(&env);
        topics.push_back(topic);
        env.events().publish(topics, event_data);
    }

    /// Create a quorum slice with weighted attestors. Returns the slice ID.
    ///
    /// # Threshold Semantics
    /// The threshold is measured in weight units, not attestor count.
    /// Each attestor's weight represents their stake/contribution to the quorum.
    /// The sum of weights from attesting parties must meet or exceed this value.
    ///
    /// For example, with attestors having weights [50, 30, 20] and threshold 50:
    /// - One attestor with weight 50 would satisfy the threshold
    /// - Two attestors with weights 30 and 20 would also satisfy (50 >= 50)
    /// - Only one attestor with weight 30 would NOT satisfy (30 < 50)
    pub fn create_slice(
        env: Env,
        creator: Address,
        attestors: Vec<Address>,
        weights: Vec<u32>,
        threshold: u32,
    ) -> u64 {
        creator.require_auth();
        assert!(!attestors.is_empty(), "attestors cannot be empty");
        assert!(
            attestors.len() as u32 <= MAX_ATTESTORS_PER_SLICE,
            "attestors exceed maximum allowed per slice"
        );
        assert!(
            weights.len() == attestors.len(),
            "weights length must match attestors length"
        );
        assert!(threshold > 0, "threshold must be greater than 0");
        // Calculate total weight sum
        let mut total_weight: u32 = 0;
        for w in weights.iter() {
            total_weight = total_weight.saturating_add(*w);
        }
        assert!(
            threshold <= total_weight,
            "threshold cannot exceed total weight sum"
        );
        assert!(
            total_weight > 0,
            "total weight must be greater than 0"
        );
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
            weights,
            threshold,
        };
        env.storage().instance().set(&DataKey::Slice(id), &slice);
        env.storage()
            .instance()
            .extend_ttl(STANDARD_TTL, EXTENDED_TTL);
        env.storage().instance().set(&DataKey::SliceCount, &id);
        env.storage()
            .instance()
            .extend_ttl(STANDARD_TTL, EXTENDED_TTL);
        id
    }

    /// Retrieve a quorum slice by ID.
    ///
    /// # Parameters
    /// - `slice_id`: The ID of the slice to retrieve.
    ///
    /// # Panics
    /// Panics with `ContractError::SliceNotFound` if no slice exists with that ID.
    pub fn get_slice(env: Env, slice_id: u64) -> QuorumSlice {
        env.storage()
            .instance()
            .get(&DataKey::Slice(slice_id))
            .unwrap_or_else(|| panic_with_error!(&env, ContractError::SliceNotFound))
    }

    /// Return the creator address of a slice.
    ///
    /// # Parameters
    /// - `slice_id`: The ID of the slice to inspect.
    ///
    /// # Panics
    /// Panics with `ContractError::SliceNotFound` if no slice exists with that ID.
    pub fn get_slice_creator(env: Env, slice_id: u64) -> Address {
        let slice: QuorumSlice = env
            .storage()
            .instance()
            .get(&DataKey::Slice(slice_id))
            .unwrap_or_else(|| panic_with_error!(&env, ContractError::SliceNotFound));
        slice.creator
    }

    /// Remove an attestor from an existing quorum slice. Only the slice creator may call this.
    /// If the removal would make the threshold unreachable, the threshold is clamped to the new total weight.
    pub fn remove_attestor(env: Env, creator: Address, slice_id: u64, attestor: Address) {
        creator.require_auth();
        let mut slice: QuorumSlice = env
            .storage()
            .instance()
            .get(&DataKey::Slice(slice_id))
            .unwrap_or_else(|| panic_with_error!(&env, ContractError::SliceNotFound));
        assert!(slice.creator == creator, "only the slice creator can remove attestors");
        let pos = slice
            .attestors
            .iter()
            .position(|a| a == attestor)
            .expect("attestor not in slice") as u32;
        slice.attestors.remove(pos);
        slice.weights.remove(pos);
        assert!(!slice.attestors.is_empty(), "cannot remove the last attestor");
        // Clamp threshold to new total weight if needed
        let mut total_weight: u32 = 0;
        for w in slice.weights.iter() {
            total_weight = total_weight.saturating_add(*w);
        }
        if slice.threshold > total_weight {
            slice.threshold = total_weight;
        }
        env.storage().instance().set(&DataKey::Slice(slice_id), &slice);
        env.storage().instance().extend_ttl(STANDARD_TTL, EXTENDED_TTL);
    }

    /// Add a new attestor with a given weight to an existing quorum slice.
    ///
    /// # Weight Semantics
    /// The weight represents the attestor's stake/contribution to the quorum.
    /// When updating threshold, ensure the new threshold doesn't exceed
    /// the total weight sum (existing + new attestor).
    pub fn add_attestor(env: Env, creator: Address, slice_id: u64, attestor: Address, weight: u32) {
        creator.require_auth();
        let mut slice: QuorumSlice = env
            .storage()
            .instance()
            .get(&DataKey::Slice(slice_id))
            .unwrap_or_else(|| panic_with_error!(&env, ContractError::SliceNotFound));
        assert!(
            slice.creator == creator,
            "only the slice creator can add attestors"
        );
        assert!(
            (slice.attestors.len() as u32) < MAX_ATTESTORS_PER_SLICE,
            "attestors exceed maximum allowed per slice"
        );
        assert!(weight > 0, "weight must be greater than 0");
        for a in slice.attestors.iter() {
            assert!(a != attestor, "attestor already in slice");
        }
        slice.attestors.push_back(attestor);
        slice.weights.push_back(weight);
        env.storage()
            .instance()
            .set(&DataKey::Slice(slice_id), &slice);
        env.storage()
            .instance()
            .extend_ttl(STANDARD_TTL, EXTENDED_TTL);
    }

    /// Update the threshold of an existing quorum slice.
    ///
    /// # Threshold Semantics
    /// The threshold is measured in weight units, not attestor count.
    /// Must be greater than 0 and cannot exceed the total weight sum of all attestors.
    pub fn update_threshold(env: Env, creator: Address, slice_id: u64, new_threshold: u32) {
        creator.require_auth();
        let mut slice: QuorumSlice = env
            .storage()
            .instance()
            .get(&DataKey::Slice(slice_id))
            .unwrap_or_else(|| panic_with_error!(&env, ContractError::SliceNotFound));
        assert!(
            slice.creator == creator,
            "only the slice creator can update threshold"
        );
        assert!(new_threshold > 0, "threshold must be greater than 0");
        // Calculate total weight sum
        let mut total_weight: u32 = 0;
        for w in slice.weights.iter() {
            total_weight = total_weight.saturating_add(*w);
        }
        assert!(
            new_threshold <= total_weight,
            "threshold cannot exceed total weight sum"
        );
        slice.threshold = new_threshold;
        env.storage()
            .instance()
            .set(&DataKey::Slice(slice_id), &slice);
        env.storage()
            .instance()
            .extend_ttl(STANDARD_TTL, EXTENDED_TTL);
    }

    /// Attest a credential using a quorum slice.
    ///
    /// Records the attestor's signature for the given credential. Once the total weight
    /// of all attestors meets or exceeds the slice threshold, `is_attested` returns `true`.
    ///
    /// # Parameters
    /// - `attestor`: The address attesting; must be a member of the slice and must authorize.
    /// - `credential_id`: The credential being attested.
    /// - `slice_id`: The quorum slice the attestor belongs to.
    ///
    /// # Panics
    /// Panics if the contract is paused.
    /// Panics with `ContractError::CredentialNotFound` if the credential does not exist.
    /// Panics if the credential is revoked.
    /// Panics if the attestor is not a member of the slice.
    /// Panics if the attestor has already attested for this credential.
    pub fn attest(env: Env, attestor: Address, credential_id: u64, slice_id: u64) {
        attestor.require_auth();
        Self::require_not_paused(&env);
        let credential: Credential = env
            .storage()
            .instance()
            .get(&DataKey::Credential(credential_id))
            .unwrap_or_else(|| panic_with_error!(&env, ContractError::CredentialNotFound));
        assert!(!credential.revoked, "credential is revoked");
        let slice: QuorumSlice = env
            .storage()
            .instance()
            .get(&DataKey::Slice(slice_id))
            .unwrap_or_else(|| panic_with_error!(&env, ContractError::SliceNotFound));
        let mut found = false;
        for a in slice.attestors.iter() {
            if a == attestor {
                found = true;
                break;
            }
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
        env.storage()
            .instance()
            .extend_ttl(STANDARD_TTL, EXTENDED_TTL);
        let event_data = AttestationEventData {
            attestor: attestor.clone(),
            credential_id,
            slice_id,
        };
        let topic = String::from_str(&env, TOPIC_ATTESTATION);
        let mut topics: Vec<String> = Vec::new(&env);
        topics.push_back(topic);
        env.events().publish(topics, event_data);
        let count: u64 = env
            .storage()
            .instance()
            .get(&DataKey::AttestorCount(attestor.clone()))
            .unwrap_or(0u64);
        env.storage()
            .instance()
            .set(&DataKey::AttestorCount(attestor), &(count + 1));
        env.storage()
            .instance()
            .extend_ttl(STANDARD_TTL, EXTENDED_TTL);
    }

    /// Batch attest multiple credentials in a single transaction.
    /// Each credential_id in the list is attested by the caller using the given slice.
    /// Caller must be a member of the slice for each credential.
    pub fn batch_attest(env: Env, attestor: Address, credential_ids: Vec<u64>, slice_id: u64) {
        attestor.require_auth();
        Self::require_not_paused(&env);
        let slice: QuorumSlice = env.storage().instance()
            .get(&DataKey::Slice(slice_id))
            .unwrap_or_else(|| panic_with_error!(&env, ContractError::SliceNotFound));
        let mut in_slice = false;
        for a in slice.attestors.iter() {
            if a == attestor {
                in_slice = true;
                break;
            }
        }
        assert!(in_slice, "attestor not in slice");
        for credential_id in credential_ids.iter() {
            let credential: Credential = env.storage().instance()
                .get(&DataKey::Credential(credential_id))
                .unwrap_or_else(|| panic_with_error!(&env, ContractError::CredentialNotFound));
            assert!(!credential.revoked, "credential is revoked");
            let mut attestors: Vec<Address> = env.storage().instance()
                .get(&DataKey::Attestors(credential_id))
                .unwrap_or(Vec::new(&env));
            for existing in attestors.iter() {
                if existing == attestor {
                    panic!("attestor has already attested for this credential");
                }
            }
            attestors.push_back(attestor.clone());
            env.storage().instance().set(&DataKey::Attestors(credential_id), &attestors);
            env.storage().instance().extend_ttl(STANDARD_TTL, EXTENDED_TTL);
            let event_data = AttestationEventData { attestor: attestor.clone(), credential_id, slice_id };
            let topic = String::from_str(&env, TOPIC_ATTESTATION);
            let mut topics: Vec<String> = Vec::new(&env);
            topics.push_back(topic);
            env.events().publish(topics, event_data);
        }
        let count: u64 = env.storage().instance()
            .get(&DataKey::AttestorCount(attestor.clone()))
            .unwrap_or(0u64);
        env.storage().instance().set(&DataKey::AttestorCount(attestor), &(count + credential_ids.len() as u64));
        env.storage().instance().extend_ttl(STANDARD_TTL, EXTENDED_TTL);
    }

    /// Check if a credential has met its quorum threshold using weighted trust.
    ///
    /// # FBA Weighted Trust Model
    /// This function implements the federated Byzantine agreement (FBA) weighted trust model.
    /// Instead of simply counting attestors, this sums the weights of attesting parties.
    ///
    /// The threshold represents the minimum total weight required, not the count.
    /// For example, with threshold 50 and two attestors with weights 30 and 20:
    /// - If only one attestor with weight 30 has signed: NOT attested (30 < 50)
    /// - If both attestors have signed: attested (30 + 20 = 50 >= 50)
    ///
    /// Returns false if the credential is revoked or expired.
    /// Check if a credential is attested by a quorum slice.
    /// Panics with ContractError::CredentialNotFound if missing.
    pub fn is_attested(env: Env, credential_id: u64, slice_id: u64) -> bool {
        let credential: Credential = env
            .storage()
            .instance()
            .get(&DataKey::Credential(credential_id))
            .unwrap_or_else(|| panic_with_error!(&env, ContractError::CredentialNotFound));
        if credential.revoked {
            return false;
        }
        if let Some(expires_at) = credential.expires_at {
            if env.ledger().timestamp() >= expires_at {
                return false;
            }
        }
        let slice: QuorumSlice = env
            .storage()
            .instance()
            .get(&DataKey::Slice(slice_id))
            .unwrap_or_else(|| panic_with_error!(&env, ContractError::SliceNotFound));
        let attested_addresses: Vec<Address> = env
            .storage()
            .instance()
            .get(&DataKey::Attestors(credential_id))
            .unwrap_or(Vec::new(&env));
        
        // Calculate total weight of attesting parties
        let mut total_attested_weight: u32 = 0;
        for attested in attested_addresses.iter() {
            // Find the index of this attestor in the slice and sum their weight
            for (i, attestor) in slice.attestors.iter().enumerate() {
                if *attestor == attested {
                    total_attested_weight = total_attested_weight.saturating_add(slice.weights.get(i).unwrap_or(&0));
                    break;
                }
            }
        }
        
        total_attested_weight >= slice.threshold
    }

    /// Returns true if the credential has been revoked.
    ///
    /// # Parameters
    /// - `credential_id`: The credential to check.
    ///
    /// # Panics
    /// Panics with `ContractError::CredentialNotFound` if the credential does not exist.
    pub fn is_revoked(env: Env, credential_id: u64) -> bool {
        let credential: Credential = env
            .storage()
            .instance()
            .get(&DataKey::Credential(credential_id))
            .unwrap_or_else(|| panic_with_error!(&env, ContractError::CredentialNotFound));
        credential.revoked
    }

    /// Returns true if the credential exists and its expiry timestamp has passed.
    ///
    /// # Parameters
    /// - `credential_id`: The credential to check.
    ///
    /// # Panics
    /// Panics with `ContractError::CredentialNotFound` if the credential does not exist.
    pub fn is_expired(env: Env, credential_id: u64) -> bool {
        let credential: Credential = env
            .storage()
            .instance()
            .get(&DataKey::Credential(credential_id))
            .unwrap_or_else(|| panic_with_error!(&env, ContractError::CredentialNotFound));
        match credential.expires_at {
            Some(expires_at) => env.ledger().timestamp() >= expires_at,
            None => false,
        }
    }

    /// Get all attestors that have signed a credential.
    ///
    /// # Parameters
    /// - `credential_id`: The credential to query.
    ///
    /// # Panics
    /// Does not panic; returns an empty `Vec` if no attestations exist.
    pub fn get_attestors(env: Env, credential_id: u64) -> Vec<Address> {
        env.storage()
            .instance()
            .get(&DataKey::Attestors(credential_id))
            .unwrap_or(Vec::new(&env))
    }

    /// Returns the number of attestations recorded for a credential.
    ///
    /// # Parameters
    /// - `credential_id`: The credential to count attestations for.
    ///
    /// # Panics
    /// Does not panic; returns `0` if no attestations exist.
    pub fn get_attestation_count(env: Env, credential_id: u64) -> u32 {
        let attestors: Vec<Address> = env
            .storage()
            .instance()
            .get(&DataKey::Attestors(credential_id))
            .unwrap_or(Vec::new(&env));
        attestors.len()
    }

    /// Returns the total number of credentials an attestor has signed across all credentials.
    ///
    /// # Parameters
    /// - `attestor`: The attestor address to query.
    ///
    /// # Panics
    /// Does not panic; returns `0` if the attestor has never attested.
    pub fn get_attestor_reputation(env: Env, attestor: Address) -> u64 {
        env.storage()
            .instance()
            .get(&DataKey::AttestorCount(attestor))
            .unwrap_or(0u64)
    }

    /// Returns the total number of credentials issued on this contract.
    ///
    /// # Panics
    /// Panics with "not initialized" if the contract has not been initialized.
    pub fn get_credential_count(env: Env) -> u64 {
        assert!(env.storage().instance().has(&DataKey::Admin), "not initialized");
        env.storage().instance().extend_ttl(STANDARD_TTL, EXTENDED_TTL);
        env.storage()
            .instance()
            .get(&DataKey::CredentialCount)
            .unwrap_or(0u64)
    }

    /// Returns the total number of quorum slices created on this contract.
    ///
    /// # Panics
    /// Panics with "not initialized" if the contract has not been initialized.
    pub fn get_slice_count(env: Env) -> u64 {
        assert!(env.storage().instance().has(&DataKey::Admin), "not initialized");
        env.storage().instance().extend_ttl(STANDARD_TTL, EXTENDED_TTL);
        env.storage()
            .instance()
            .get(&DataKey::SliceCount)
            .unwrap_or(0u64)
    }

    /// Verify multiple ZK claims for a credential in a single call.
    ///
    /// Iterates over `claim_types` and `proofs` in parallel, calling the ZK verifier
    /// for each pair. Returns a `Vec<bool>` where each element corresponds to whether
    /// the claim at that index was verified successfully.
    ///
    /// # Parameters
    /// - `zk_verifier_id`: Address of the deployed ZK verifier contract.
    /// - `quorum_proof_id`: Address of this quorum proof contract (passed to ZK verifier).
    /// - `credential_id`: The credential to verify claims against.
    /// - `claim_types`: Ordered list of claim types to verify.
    /// - `proofs`: Ordered list of ZK proofs corresponding to each claim type.
    ///
    /// # Panics
    /// Panics if `claim_types` and `proofs` have different lengths.
    pub fn verify_claim_batch(
        env: Env,
        zk_verifier_id: Address,
        zk_admin: Address,
        quorum_proof_id: Address,
        credential_id: u64,
        claim_types: Vec<zk_verifier::ClaimType>,
        proofs: Vec<soroban_sdk::Bytes>,
    ) -> Vec<bool> {
        assert!(
            claim_types.len() == proofs.len(),
            "claim_types and proofs lengths must match"
        );
        let zk_client = ZkVerifierContractClient::new(&env, &zk_verifier_id);
        let mut results: Vec<bool> = Vec::new(&env);
        for i in 0..claim_types.len() {
            let result = zk_client.verify_claim(
                &zk_admin,
                &quorum_proof_id,
                &credential_id,
                &claim_types.get(i).unwrap(),
                &proofs.get(i).unwrap(),
            );
            results.push_back(result);
        }
        results
    }

    /// Returns the attestation status of each attestor in a slice for a given credential.
    ///
    /// For each attestor in the slice, returns a tuple of `(Address, bool)` where the
    /// boolean indicates whether that attestor has signed the credential. Useful for
    /// UX progress tracking (e.g. "2 of 3 attestors have signed").
    ///
    /// # Parameters
    /// - `credential_id`: The credential to check attestation status for.
    /// - `slice_id`: The quorum slice whose attestors to inspect.
    ///
    /// # Panics
    /// Panics with `ContractError::CredentialNotFound` if the credential does not exist.
    /// Panics with `ContractError::SliceNotFound` if the slice does not exist.
    pub fn get_slice_attestation_status(
        env: Env,
        credential_id: u64,
        slice_id: u64,
    ) -> Vec<(Address, bool)> {
        if !env
            .storage()
            .instance()
            .has(&DataKey::Credential(credential_id))
        {
            panic_with_error!(&env, ContractError::CredentialNotFound);
        }
        let slice: QuorumSlice = env
            .storage()
            .instance()
            .get(&DataKey::Slice(slice_id))
            .unwrap_or_else(|| panic_with_error!(&env, ContractError::SliceNotFound));
        let attested: Vec<Address> = env
            .storage()
            .instance()
            .get(&DataKey::Attestors(credential_id))
            .unwrap_or(Vec::new(&env));
        let mut status: Vec<(Address, bool)> = Vec::new(&env);
        for attestor in slice.attestors.iter() {
            let signed = attested.iter().any(|a| a == attestor);
            status.push_back((attestor, signed));
        }
        status
    }

    /// Unified engineer verification entry point.
    ///
    /// Checks that the subject holds an SBT linked to the credential, then delegates
    /// ZK claim verification to the `zk_verifier` contract.
    ///
    /// # Parameters
    /// - `quorum_proof_id`: Address of this contract (forwarded to the ZK verifier).
    /// - `sbt_registry_id`: Address of the deployed SBT registry contract.
    /// - `zk_verifier_id`: Address of the deployed ZK verifier contract.
    /// - `subject`: The engineer whose credential is being verified.
    /// - `credential_id`: The credential to verify.
    /// - `claim_type`: The specific claim to verify (degree, license, employment).
    /// - `proof`: The ZK proof bytes for the claim.
    ///
    /// # Panics
    /// Does not panic; returns `false` if the subject has no matching SBT or the proof fails.
    pub fn verify_engineer(
        env: Env,
        sbt_registry_id: Address,
        zk_verifier_id: Address,
        zk_admin: Address,
        subject: Address,
        credential_id: u64,
        claim_type: ClaimType,
        proof: soroban_sdk::Bytes,
    ) -> bool {
        let quorum_proof_id = env.current_contract_address();
        let sbt_client = SbtRegistryContractClient::new(&env, &sbt_registry_id);
        let tokens = sbt_client.get_tokens_by_owner(&subject);
        let has_sbt = tokens.iter().any(|token_id| {
            let token = sbt_client.get_token(&token_id);
            token.credential_id == credential_id
        });
        if !has_sbt {
            return false;
        }
        let zk_client = ZkVerifierContractClient::new(&env, &zk_verifier_id);
        zk_client.verify_claim(&zk_admin, &quorum_proof_id, &credential_id, &claim_type, &proof)
    }

    /// Register a human-readable label for a credential type.
    ///
    /// # Parameters
    /// - `admin`: The admin address; must authorize this call.
    /// - `type_id`: Numeric identifier for the credential type.
    /// - `name`: Human-readable name (e.g. "Mechanical Engineering Degree").
    /// - `description`: Longer description of what the credential type represents.
    ///
    /// # Panics
    /// Does not panic on duplicate registration; overwrites the existing entry.
    pub fn register_credential_type(
        env: Env,
        admin: Address,
        type_id: u32,
        name: soroban_sdk::String,
        description: soroban_sdk::String,
    ) {
        admin.require_auth();
        let stored_admin: Address = env.storage().instance().get(&DataKey::Admin).expect("not initialized");
        assert!(admin == stored_admin, "unauthorized");
        let def = CredentialTypeDef {
            type_id,
            name,
            description,
        };
        env.storage()
            .instance()
            .set(&DataKey::CredentialType(type_id), &def);
        env.storage()
            .instance()
            .extend_ttl(STANDARD_TTL, EXTENDED_TTL);
        let mut topics: Vec<soroban_sdk::Val> = Vec::new(&env);
        topics.push_back(symbol_short!("reg_type").into());
        env.events().publish(topics, type_id);
    }

    /// Look up the registered name and description for a credential type.
    ///
    /// # Parameters
    /// - `type_id`: The numeric credential type ID to look up.
    ///
    /// # Panics
    /// Panics with "credential type not registered" if the type has not been registered.
    pub fn get_credential_type(env: Env, type_id: u32) -> CredentialTypeDef {
        env.storage()
            .instance()
            .get(&DataKey::CredentialType(type_id))
            .expect("credential type not registered")
    }

    /// Admin-only contract upgrade to new WASM. Uses deployer convention for auth.
    ///
    /// # Parameters
    /// - `admin`: The admin address; must authorize this call.
    /// - `new_wasm_hash`: The 32-byte hash of the new WASM to upgrade to.
    ///
    /// # Panics
    /// Panics if `admin` does not authorize the call.
    pub fn upgrade(env: Env, admin: Address, new_wasm_hash: soroban_sdk::BytesN<32>) {
        admin.require_auth();
        env.deployer().update_current_contract_wasm(new_wasm_hash);
    }

    // ── Proof Request History (Issue #38) ────────────────────────────────────

    /// Record a new proof request for a credential and return its unique request ID.
    ///
    /// Verifiers call this to create an auditable trail every time they request
    /// proof of a credential. The request is appended to the per-credential history
    /// retrievable via [`get_proof_requests`].
    ///
    /// # Parameters
    /// - `verifier`: The address initiating the proof request; must authorize this call.
    /// - `credential_id`: The credential for which proof is being requested.
    /// - `claim_types`: The ZK claim types the verifier wants proven.
    ///
    /// # Returns
    /// The unique ID assigned to this proof request.
    ///
    /// # Panics
    /// Panics if the contract is paused.
    /// Panics with `ContractError::CredentialNotFound` if no credential exists with that ID.
    pub fn generate_proof_request(
        env: Env,
        verifier: Address,
        credential_id: u64,
        claim_types: Vec<zk_verifier::ClaimType>,
    ) -> u64 {
        verifier.require_auth();
        Self::require_not_paused(&env);

        // Verify that the credential exists.
        if !env
            .storage()
            .instance()
            .has(&DataKey::Credential(credential_id))
        {
            panic_with_error!(&env, ContractError::CredentialNotFound);
        }

        // Assign a globally unique ID.
        let request_id: u64 = env
            .storage()
            .instance()
            .get(&DataKey::ProofRequestCount)
            .unwrap_or(0u64)
            + 1;

        let request = ProofRequest {
            id: request_id,
            credential_id,
            verifier: verifier.clone(),
            requested_at: env.ledger().timestamp(),
            claim_types,
        };

        // Append to the per-credential history.
        let mut history: Vec<ProofRequest> = env
            .storage()
            .instance()
            .get(&DataKey::ProofRequests(credential_id))
            .unwrap_or(Vec::new(&env));
        history.push_back(request.clone());
        env.storage()
            .instance()
            .set(&DataKey::ProofRequests(credential_id), &history);
        env.storage()
            .instance()
            .extend_ttl(STANDARD_TTL, EXTENDED_TTL);

        // Update global counter.
        env.storage()
            .instance()
            .set(&DataKey::ProofRequestCount, &request_id);
        env.storage()
            .instance()
            .extend_ttl(STANDARD_TTL, EXTENDED_TTL);

        // Emit event so off-chain indexers can track requests without polling storage.
        let topic = String::from_str(&env, TOPIC_PROOF_REQUEST);
        let mut topics: Vec<String> = Vec::new(&env);
        topics.push_back(topic);
        env.events().publish(topics, request);

        request_id
    }

    /// Return all proof requests ever generated for a credential, in insertion order.
    ///
    /// Verifiers and auditors use this to inspect the full verification history of
    /// a credential.
    ///
    /// # Parameters
    /// - `credential_id`: The credential whose proof-request history to retrieve.
    ///
    /// # Returns
    /// A `Vec<ProofRequest>` in the order requests were recorded. Returns an empty
    /// `Vec` if no requests have been made yet (does not panic).
    ///
    /// # Panics
    /// Does not panic even if the credential does not exist; returns empty in that case.
    pub fn get_proof_requests(env: Env, credential_id: u64) -> Vec<ProofRequest> {
        env.storage()
            .instance()
            .get(&DataKey::ProofRequests(credential_id))
            .unwrap_or(Vec::new(&env))
    }

    /// Check if a credential exists without panicking.
    ///
    /// # Parameters
    /// - `credential_id`: The ID of the credential to check.
    ///
    /// # Returns
    /// `true` if the credential exists in storage, `false` otherwise.
    ///
    /// # Panics
    /// Does not panic.
    pub fn credential_exists(env: Env, credential_id: u64) -> bool {
        env.storage()
            .instance()
            .has(&DataKey::Credential(credential_id))
    }

    /// Check if a quorum slice exists without panicking.
    ///
    /// # Parameters
    /// - `slice_id`: The ID of the slice to check.
    ///
    /// # Returns
    /// `true` if the slice exists in storage, `false` otherwise.
    ///
    /// # Panics
    /// Does not panic.
    pub fn slice_exists(env: Env, slice_id: u64) -> bool {
        env.storage()
            .instance()
            .has(&DataKey::Slice(slice_id))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use soroban_sdk::testutils::{Address as _, Events as _, Ledger as _, LedgerInfo};
    use soroban_sdk::{Bytes, Env, FromVal, IntoVal};

    fn setup(env: &Env) -> (QuorumProofContractClient<'_>, Address) {
        env.mock_all_auths();
        let contract_id = env.register_contract(None, QuorumProofContract);
        let client = QuorumProofContractClient::new(env, &contract_id);
        let admin = Address::generate(env);
        client.initialize(&admin);
        (client, admin)
    }

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
            timestamp: 1_000,
            protocol_version: 20,
            sequence_number: 100,
            network_id: Default::default(),
            base_reserve: 10,
            min_persistent_entry_ttl: 4096,
            min_temp_entry_ttl: 16,
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
    }

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
        assert_ne!(
            client.get_credential(&id_a).issuer,
            client.get_credential(&id_b).issuer
        );
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

        let issued = all_events.iter().find(
            |(_, topics, _): &(
                Address,
                soroban_sdk::Vec<soroban_sdk::Val>,
                soroban_sdk::Val,
            )| {
                if let Some(raw) = topics.get(0) {
                    let s = String::from_val(&env, &raw);
                    return s == expected_topic;
                }
                false
            },
        );

        assert!(issued.is_some(), "CredentialIssued event was not emitted");

        let (_, _, data) = issued.unwrap();
        let event_data: CredentialIssuedEventData = soroban_sdk::Val::into_val(&data, &env);

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
        client.issue_credential(&issuer, &subject, &1u32, &metadata, &None);
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
        let mut weights = Vec::new(&env);
        weights.push_back(1u32);
        weights.push_back(1u32);
        client.create_slice(&creator, &attestors, &weights, &2u32);
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
        let mut weights = Vec::new(&env);
        for _ in 0..=MAX_ATTESTORS_PER_SLICE {
            weights.push_back(1u32);
        }
        client.create_slice(&creator, &attestors, &weights, &1u32);
    }

    #[test]
    #[should_panic(expected = "CredentialNotFound")]
    fn test_get_credential_not_found() {
        let env = Env::default();
        let contract_id = env.register_contract(None, QuorumProofContract);
        let client = QuorumProofContractClient::new(&env, &contract_id);

        // Try to get a credential that doesn't exist
        client.get_credential(&999u64);
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
    #[should_panic]
    fn test_pause_blocks_attest() {
        let env = Env::default();
        env.mock_all_auths();
        let (client, admin) = setup(&env);

        let issuer = Address::generate(&env);
        let subject = Address::generate(&env);
        let attestor = Address::generate(&env);
        let metadata = Bytes::from_slice(&env, b"ipfs://QmTest");
        let cred_id = client.issue_credential(&issuer, &subject, &1u32, &metadata, &None);

        let mut attestors = Vec::new(&env);
        attestors.push_back(attestor.clone());
        let mut weights = Vec::new(&env);
        weights.push_back(1u32);
        let slice_id = client.create_slice(&issuer, &attestors, &weights, &1u32);

        client.pause(&admin);
        client.attest(&attestor, &cred_id, &slice_id);
    }

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
        attestors.push_back(attestor.clone());
        let mut weights = Vec::new(&env);
        weights.push_back(1u32);
        let creator = Address::generate(&env);
        let slice_id = client.create_slice(&creator, &attestors, &weights, &1u32);

        assert!(!client.is_attested(&cred_id, &slice_id));
        client.attest(&attestor, &cred_id, &slice_id);
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
        let mut weights = Vec::new(&env);
        weights.push_back(1u32);
        let slice_id = client.create_slice(&issuer, &attestors, &weights, &1u32);
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
        let mut attestors = Vec::new(&env);
        attestors.push_back(attestor.clone());
        let mut weights = Vec::new(&env);
        weights.push_back(1u32);
        let slice_id = client.create_slice(&issuer, &attestors, &weights, &1u32);
        client.revoke_credential(&issuer, &id);
        client.attest(&attestor, &id, &slice_id);
    }

    // --- slice management ---

    #[test]
    #[should_panic(expected = "attestors cannot be empty")]
    fn test_create_slice_empty_attestors_panics() {
        let env = Env::default();
        env.mock_all_auths();
        let (client, _) = setup(&env);
        client.create_slice(&Address::generate(&env), &Vec::new(&env), &Vec::new(&env), &1u32);
    }

    #[test]
    #[should_panic(expected = "threshold must be greater than 0")]
    fn test_zero_threshold_rejection() {
        let env = Env::default();
        env.mock_all_auths();
        let contract_id = env.register_contract(None, QuorumProofContract);
        let client = QuorumProofContractClient::new(&env, &contract_id);
        // Create slice with empty vectors to trigger zero threshold panic
        client.create_slice(&Address::generate(&env), &Vec::new(&env), &Vec::new(&env), &0u32);
    }

    #[test]
    #[should_panic(expected = "threshold cannot exceed total weight sum")]
    fn test_threshold_exceeds_attestors() {
        let env = Env::default();
        env.mock_all_auths();
        let contract_id = env.register_contract(None, QuorumProofContract);
        let client = QuorumProofContractClient::new(&env, &contract_id);

        let creator = Address::generate(&env);
        let mut attestors = soroban_sdk::Vec::new(&env);
        attestors.push_back(Address::generate(&env));
        attestors.push_back(Address::generate(&env));
        let mut weights = Vec::new(&env);
        weights.push_back(1u32);
        weights.push_back(1u32);

        // 2 attestors with total weight 2 but threshold of 3 — must panic
        client.create_slice(&creator, &attestors, &weights, &3u32);
    }

    #[test]
    #[should_panic(expected = "attestors exceed maximum allowed per slice")]
    fn test_create_slice_exceeds_max_attestors() {
        let env = Env::default();
        env.mock_all_auths();
        let (client, _) = setup(&env);
        let creator = Address::generate(&env);
        let mut attestors = Vec::new(&env);
        let mut weights = Vec::new(&env);
        for _ in 0..=MAX_ATTESTORS_PER_SLICE {
            attestors.push_back(Address::generate(&env));
            weights.push_back(1u32);
        }
        client.create_slice(&creator, &attestors, &weights, &1u32);
    }

    #[test]
    fn test_get_slice_creator_matches() {
        let env = Env::default();
        env.mock_all_auths();
        let (client, _) = setup(&env);
        let creator = Address::generate(&env);
        let mut attestors = Vec::new(&env);
        attestors.push_back(Address::generate(&env));
        let mut weights = Vec::new(&env);
        weights.push_back(1u32);
        let slice_id = client.create_slice(&creator, &attestors, &weights, &1u32);
        assert_eq!(client.get_slice_creator(&slice_id), creator);
    }

    #[test]
    #[should_panic(expected = "SliceNotFound")]
    fn test_get_slice_not_found() {
        let env = Env::default();
        env.mock_all_auths();
        let (client, _) = setup(&env);
        // Try to get a slice that doesn't exist
        client.get_slice(&999u64);
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
    fn test_get_credentials_by_subject_multiple() {
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
    #[should_panic(expected = "SliceNotFound")]
    fn test_add_attestor_slice_not_found_panics() {
        let env = Env::default();
        env.mock_all_auths();
        let (client, _) = setup(&env);
        let creator = Address::generate(&env);
        client.add_attestor(&creator, &999u64, &Address::generate(&env), &1u32);
    }

    #[test]
    #[should_panic(expected = "SliceNotFound")]
    fn test_update_threshold_slice_not_found_panics() {
        let env = Env::default();
        env.mock_all_auths();
        let (client, _) = setup(&env);
        let creator = Address::generate(&env);
        client.update_threshold(&creator, &999u64, &1u32);
    }

    #[test]
    fn test_single_attestation_produces_exactly_one_entry() {
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
        let mut weights = Vec::new(&env);
        weights.push_back(1u32);
        let slice_id = client.create_slice(&issuer, &attestors, &weights, &1u32);
        client.attest(&attestor, &cred_id, &slice_id);
        assert_eq!(client.get_attestors(&cred_id).len(), 1);
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
        let mut weights = Vec::new(&env);
        weights.push_back(1u32);
        let slice_id = client.create_slice(&issuer, &attestors, &weights, &1u32);
        client.attest(&attestor, &cred_id, &slice_id);
        assert!(client.is_attested(&cred_id, &slice_id));

        set_ledger_timestamp(&env, 3_000);
        assert!(!client.is_attested(&cred_id, &slice_id));
    }

    // --- batch issue ---

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
        let mut initial_weights = Vec::new(&env);
        initial_weights.push_back(1u32);
        let slice_id = client.create_slice(&creator, &initial, &initial_weights, &1u32);

        client.add_attestor(&creator, &slice_id, &attestor2, &1u32);

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
        let mut initial_weights = Vec::new(&env);
        initial_weights.push_back(1u32);
        let slice_id = client.create_slice(&creator, &initial, &initial_weights, &1u32);

        client.add_attestor(&creator, &slice_id, &attestor, &1u32);
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

        // Create slice with at least one attestor to avoid "attestors cannot be empty" panic
        let mut initial = Vec::new(&env);
        initial.push_back(Address::generate(&env));
        let mut initial_weights = Vec::new(&env);
        initial_weights.push_back(1u32);
        let slice_id = client.create_slice(&creator, &initial, &initial_weights, &1u32);

        // This should panic with "only the slice creator can add attestors"
        client.add_attestor(&non_creator, &slice_id, &attestor, &1u32);
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
        let mut initial_weights = Vec::new(&env);
        initial_weights.push_back(1u32);
        let slice_id = client.create_slice(&creator, &initial, &initial_weights, &1u32);

        let cred_id = client.issue_credential(&issuer, &subject, &1u32, &metadata, &None);

        client.attest(&attestor1, &cred_id, &slice_id);
        assert!(client.is_attested(&cred_id, &slice_id)); // threshold=1, met after attestor1

        client.add_attestor(&creator, &slice_id, &attestor2, &1u32);
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
        let weights = Vec::new(&env);
        client.create_slice(&creator, &attestors, &weights, &1u32);
    }

    #[test]
    fn test_verify_engineer_success() {
        use sbt_registry::{SbtRegistryContract, SbtRegistryContractClient};
        use zk_verifier::{ClaimType, ZkVerifierContract, ZkVerifierContractClient};

        let env = Env::default();
        env.mock_all_auths();

        let qp_id = env.register_contract(None, QuorumProofContract);
        let sbt_id = env.register_contract(None, SbtRegistryContract);
        let zk_id = env.register_contract(None, ZkVerifierContract);

        let qp = QuorumProofContractClient::new(&env, &qp_id);
        let sbt = SbtRegistryContractClient::new(&env, &sbt_id);
        let zk_admin = Address::generate(&env);
        ZkVerifierContractClient::new(&env, &zk_id).initialize(&zk_admin);
        sbt.initialize(&zk_admin, &qp_id);

        let issuer = Address::generate(&env);
        let subject = Address::generate(&env);
        let metadata = Bytes::from_slice(&env, b"ipfs://QmTest");

        let cred_id = qp.issue_credential(&issuer, &subject, &1u32, &metadata, &None);

        let sbt_uri = Bytes::from_slice(&env, b"ipfs://QmSbt");
        sbt.mint(&subject, &cred_id, &sbt_uri);

        let proof = Bytes::from_slice(&env, b"valid-proof");
        let result = qp.verify_engineer(
            &sbt_id,
            &zk_id,
            &zk_admin,
            &subject,
            &cred_id,
            &ClaimType::HasDegree,
            &proof,
        );
        assert!(result);
    }

    #[test]
    fn test_verify_engineer_fails_without_sbt() {
        use zk_verifier::{ClaimType, ZkVerifierContract, ZkVerifierContractClient};

        let env = Env::default();
        env.mock_all_auths();

        let qp_id = env.register_contract(None, QuorumProofContract);
        let sbt_id = env.register_contract(None, sbt_registry::SbtRegistryContract);
        let zk_id = env.register_contract(None, ZkVerifierContract);

        let qp = QuorumProofContractClient::new(&env, &qp_id);
        let zk_admin = Address::generate(&env);
        ZkVerifierContractClient::new(&env, &zk_id).initialize(&zk_admin);
        sbt_registry::SbtRegistryContractClient::new(&env, &sbt_id).initialize(&zk_admin, &qp_id);

        let issuer = Address::generate(&env);
        let subject = Address::generate(&env);
        let metadata = Bytes::from_slice(&env, b"ipfs://QmTest");

        let cred_id = qp.issue_credential(&issuer, &subject, &1u32, &metadata, &None);

        let proof = Bytes::from_slice(&env, b"valid-proof");
        let result = qp.verify_engineer(
            &sbt_id,
            &zk_id,
            &zk_admin,
            &subject,
            &cred_id,
            &ClaimType::HasDegree,
            &proof,
        );
        assert!(!result);
    }

    #[test]
    fn test_verify_engineer_fails_with_empty_proof() {
        use sbt_registry::{SbtRegistryContract, SbtRegistryContractClient};
        use zk_verifier::{ClaimType, ZkVerifierContract, ZkVerifierContractClient};

        let env = Env::default();
        env.mock_all_auths();

        let qp_id = env.register_contract(None, QuorumProofContract);
        let sbt_id = env.register_contract(None, SbtRegistryContract);
        let zk_id = env.register_contract(None, ZkVerifierContract);

        let qp = QuorumProofContractClient::new(&env, &qp_id);
        let sbt = SbtRegistryContractClient::new(&env, &sbt_id);
        let zk_admin = Address::generate(&env);
        ZkVerifierContractClient::new(&env, &zk_id).initialize(&zk_admin);
        sbt.initialize(&zk_admin, &qp_id);

        let issuer = Address::generate(&env);
        let subject = Address::generate(&env);
        let metadata = Bytes::from_slice(&env, b"ipfs://QmTest");

        let cred_id = qp.issue_credential(&issuer, &subject, &1u32, &metadata, &None);
        let sbt_uri = Bytes::from_slice(&env, b"ipfs://QmSbt");
        sbt.mint(&subject, &cred_id, &sbt_uri);

        let proof = Bytes::from_slice(&env, b"");
        let result = qp.verify_engineer(
            &sbt_id,
            &zk_id,
            &zk_admin,
            &subject,
            &cred_id,
            &ClaimType::HasLicense,
            &proof,
        );
        assert!(!result);
    }

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
        let mut weights = Vec::new(&env);
        weights.push_back(1u32);
        let slice_id = client.create_slice(&issuer, &attestors, &weights, &1u32);
        let cred_id1 = client.issue_credential(&issuer, &subject, &1u32, &metadata, &None);
        let cred_id2 = client.issue_credential(&issuer, &subject, &2u32, &metadata, &None);
        assert_eq!(client.get_attestor_reputation(&attestor), 0);
        client.attest(&attestor, &cred_id1, &slice_id);
        assert_eq!(client.get_attestor_reputation(&attestor), 1);
        client.attest(&attestor, &cred_id2, &slice_id);
        assert_eq!(client.get_attestor_reputation(&attestor), 2);
    }

    #[test]
    fn test_single_attestation_produces_exactly_one_entry() {
        let env = Env::default();
        env.mock_all_auths();
        let contract_id = env.register_contract(None, QuorumProofContract);
        let client = QuorumProofContractClient::new(&env, &contract_id);

        let creator = Address::generate(&env);
        let mut attestors = Vec::new(&env);
        attestors.push_back(Address::generate(&env));
        attestors.push_back(Address::generate(&env));
        let mut weights = Vec::new(&env);
        weights.push_back(1u32);
        weights.push_back(1u32);
        let slice_id = client.create_slice(&creator, &attestors, &weights, &2u32);

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
        let mut weights = Vec::new(&env);
        weights.push_back(1u32);
        let slice_id = client.create_slice(&creator, &attestors, &weights, &1u32);

        client.update_threshold(&non_creator, &slice_id, &1u32);
    }

    #[test]
    fn test_update_threshold_success() {
        let env = Env::default();
        env.mock_all_auths();
        let (client, _) = setup(&env);
        let creator = Address::generate(&env);
        let mut attestors = Vec::new(&env);
        attestors.push_back(Address::generate(&env));
        let mut weights = Vec::new(&env);
        weights.push_back(1u32);
        let slice_id = client.create_slice(&creator, &attestors, &weights, &1u32);

        client.update_threshold(&creator, &slice_id, &1u32);
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
        let (client, admin) = setup(&env);
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
        let (client, admin) = setup(&env);

        let name_v1 = String::from_str(&env, "Old Name");
        let name_v2 = String::from_str(&env, "New Name");
        let desc = String::from_str(&env, "desc");

        client.register_credential_type(&admin, &1u32, &name_v1, &desc);
        client.register_credential_type(&admin, &1u32, &name_v2, &desc);

        let def = client.get_credential_type(&1u32);
        assert_eq!(def.name, name_v2);
    }

    #[test]
    #[should_panic(expected = "unauthorized")]
    fn test_register_credential_type_non_admin_panics() {
        let env = Env::default();
        env.mock_all_auths();
        let (client, _admin) = setup(&env);
        let non_admin = Address::generate(&env);
        let name = String::from_str(&env, "Fake Type");
        let desc = String::from_str(&env, "desc");
        client.register_credential_type(&non_admin, &1u32, &name, &desc);
    }

    #[test]
    fn test_register_credential_type_emits_event() {
        let env = Env::default();
        env.mock_all_auths();
        let (client, admin) = setup(&env);
        let name = String::from_str(&env, "Civil Engineering");
        let desc = String::from_str(&env, "desc");

        client.register_credential_type(&admin, &5u32, &name, &desc);

        let events = env.events().all();
        let reg_event = events.iter().find(|(_, topics, _)| {
            if let Some(first) = topics.get(0) {
                soroban_sdk::Symbol::try_from_val(&env, &first)
                    .map(|s| s == symbol_short!("reg_type"))
                    .unwrap_or(false)
            } else {
                false
            }
        });
        assert!(reg_event.is_some(), "reg_type event not emitted");
        let (_, _, data) = reg_event.unwrap();
        let emitted_id = u32::try_from_val(&env, &data).expect("data should be type_id");
        assert_eq!(emitted_id, 5u32);
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
        let (client, _admin) = setup(&env);

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
        let (client, _admin) = setup(&env);

        let creator = Address::generate(&env);
        let mut attestors = Vec::new(&env);
        attestors.push_back(Address::generate(&env));
        let mut weights = Vec::new(&env);
        weights.push_back(1u32);

        assert_eq!(client.get_slice_count(), 0);

        client.create_slice(&creator, &attestors.clone(), &weights.clone(), &1u32);
        client.create_slice(&creator, &attestors, &weights, &1u32);

        assert_eq!(client.get_slice_count(), 2);
    }

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
        let mut weights = Vec::new(&env);
        weights.push_back(1u32);
        let slice_id = client.create_slice(&issuer, &attestors, &weights, &1u32);

        // Revoke the credential
        client.revoke_credential(&issuer, &cred_id);

        // Attempting to attest a revoked credential must panic
        client.attest(&attestor, &cred_id, &slice_id);
    }

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
        let mut weights = Vec::new(&env);
        weights.push_back(1u32);
        weights.push_back(1u32);
        let slice_id = client.create_slice(&issuer, &attestors, &weights, &1u32);

        assert_eq!(client.get_attestation_count(&cred_id), 0);
        client.attest(&attestor1, &cred_id, &slice_id);
        assert_eq!(client.get_attestation_count(&cred_id), 1);
        client.attest(&attestor2, &cred_id, &slice_id);
        assert_eq!(client.get_attestation_count(&cred_id), 2);
    }

    // --- duplicate credential tests ---

    #[test]
    #[should_panic(expected = "DuplicateCredential")]
    fn test_duplicate_credential_issuance_rejection() {
        let env = Env::default();
        env.mock_all_auths();
        let contract_id = env.register_contract(None, QuorumProofContract);
        let client = QuorumProofContractClient::new(&env, &contract_id);

        let issuer = Address::generate(&env);
        let subject = Address::generate(&env);
        let metadata = Bytes::from_slice(&env, b"ipfs://QmTest");
        let credential_type: u32 = 1;

        // Issue first credential
        client.issue_credential(&issuer, &subject, &credential_type, &metadata, &None);

        // Try to issue duplicate credential of same type from same issuer to same subject
        client.issue_credential(&issuer, &subject, &credential_type, &metadata, &None);
    }

    #[test]
    fn test_different_credential_types_allowed() {
        let env = Env::default();
        env.mock_all_auths();
        let contract_id = env.register_contract(None, QuorumProofContract);
        let client = QuorumProofContractClient::new(&env, &contract_id);

        let issuer = Address::generate(&env);
        let subject = Address::generate(&env);
        let metadata = Bytes::from_slice(&env, b"ipfs://QmTest");

        // Issue credentials of different types - should succeed
        let id1 = client.issue_credential(&issuer, &subject, &1u32, &metadata, &None);
        let id2 = client.issue_credential(&issuer, &subject, &2u32, &metadata, &None);
        let id3 = client.issue_credential(&issuer, &subject, &3u32, &metadata, &None);

        assert_eq!(id1, 1);
        assert_eq!(id2, 2);
        assert_eq!(id3, 3);
    }

    #[test]
    fn test_different_issuers_allowed() {
        let env = Env::default();
        env.mock_all_auths();
        let contract_id = env.register_contract(None, QuorumProofContract);
        let client = QuorumProofContractClient::new(&env, &contract_id);

        let issuer1 = Address::generate(&env);
        let issuer2 = Address::generate(&env);
        let subject = Address::generate(&env);
        let metadata = Bytes::from_slice(&env, b"ipfs://QmTest");
        let credential_type: u32 = 1;

        // Issue credentials of same type from different issuers - should succeed
        let id1 = client.issue_credential(&issuer1, &subject, &credential_type, &metadata, &None);
        let id2 = client.issue_credential(&issuer2, &subject, &credential_type, &metadata, &None);

        assert_eq!(id1, 1);
        assert_eq!(id2, 2);
    }

    #[test]
    fn test_different_subjects_allowed() {
        let env = Env::default();
        env.mock_all_auths();
        let contract_id = env.register_contract(None, QuorumProofContract);
        let client = QuorumProofContractClient::new(&env, &contract_id);

        let issuer = Address::generate(&env);
        let subject1 = Address::generate(&env);
        let subject2 = Address::generate(&env);
        let metadata = Bytes::from_slice(&env, b"ipfs://QmTest");
        let credential_type: u32 = 1;

        // Issue credentials of same type to different subjects - should succeed
        let id1 = client.issue_credential(&issuer, &subject1, &credential_type, &metadata, &None);
        let id2 = client.issue_credential(&issuer, &subject2, &credential_type, &metadata, &None);

        assert_eq!(id1, 1);
        assert_eq!(id2, 2);
    }

    // --- unauthorized revocation tests ---

    #[test]
    #[should_panic(expected = "only the original issuer can revoke")]
    fn test_subject_revoke_credential_fails() {
        let env = Env::default();
        env.mock_all_auths();
        let contract_id = env.register_contract(None, QuorumProofContract);
        let client = QuorumProofContractClient::new(&env, &contract_id);

        let issuer = Address::generate(&env);
        let subject = Address::generate(&env);
        let metadata = Bytes::from_slice(&env, b"ipfs://QmTest");
        let id = client.issue_credential(&issuer, &subject, &1u32, &metadata, &None);

        // Subject should not be able to revoke
        client.revoke_credential(&subject, &id);
    }

    #[test]
    #[should_panic(expected = "only the original issuer can revoke")]
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

        // Unauthorized address should not be able to revoke
        client.revoke_credential(&unauthorized, &id);
    }

    // Issue #48: Full Credential Lifecycle End-to-End
    #[test]
    fn test_full_credential_lifecycle_e2e() {
        use sbt_registry::{SbtRegistryContract, SbtRegistryContractClient};
        use zk_verifier::{ClaimType, ZkVerifierContract, ZkVerifierContractClient};

        let env = Env::default();
        env.mock_all_auths();

        // Step 1: Set up all three contracts
        let qp_id = env.register_contract(None, QuorumProofContract);
        let sbt_id = env.register_contract(None, SbtRegistryContract);
        let zk_id = env.register_contract(None, ZkVerifierContract);

        let qp = QuorumProofContractClient::new(&env, &qp_id);
        let sbt = SbtRegistryContractClient::new(&env, &sbt_id);
        let zk_admin = Address::generate(&env);
        ZkVerifierContractClient::new(&env, &zk_id).initialize(&zk_admin);
        sbt.initialize(&zk_admin, &qp_id);

        let issuer = Address::generate(&env);
        let subject = Address::generate(&env);
        let attestor1 = Address::generate(&env);
        let attestor2 = Address::generate(&env);
        let metadata = Bytes::from_slice(&env, b"ipfs://QmLifecycleTest");

        // Step 2: Issue credential
        let cred_id = qp.issue_credential(&issuer, &subject, &1u32, &metadata, &None);

        // Assert credential state after issuance
        let cred = qp.get_credential(&cred_id);
        assert_eq!(cred.issuer, issuer);
        assert_eq!(cred.subject, subject);
        assert!(!cred.revoked);
        assert_eq!(qp.get_credential_count(), 1);

        // Step 3: Create quorum slice with two attestors, threshold of 2
        let mut attestors = soroban_sdk::Vec::new(&env);
        attestors.push_back(attestor1.clone());
        attestors.push_back(attestor2.clone());
        let mut weights = Vec::new(&env);
        weights.push_back(1u32);
        weights.push_back(1u32);
        let slice_id = qp.create_slice(&issuer, &attestors, &weights, &2u32);

        // Assert slice state
        let slice = qp.get_slice(&slice_id);
        assert_eq!(slice.threshold, 2);
        assert_eq!(slice.attestors.len(), 2);

        // Step 4: Attest — quorum not yet met after first attestor
        qp.attest(&attestor1, &cred_id, &slice_id);
        assert!(!qp.is_attested(&cred_id, &slice_id));

        // Attest — quorum met after second attestor
        qp.attest(&attestor2, &cred_id, &slice_id);
        assert!(qp.is_attested(&cred_id, &slice_id));

        // Assert attestor reputations incremented
        assert_eq!(qp.get_attestor_reputation(&attestor1), 1);
        assert_eq!(qp.get_attestor_reputation(&attestor2), 1);

        // Step 5: Mint SBT for subject linked to the credential
        let sbt_uri = Bytes::from_slice(&env, b"ipfs://QmSbtLifecycle");
        let token_id = sbt.mint(&subject, &cred_id, &sbt_uri);

        // Assert SBT ownership
        assert_eq!(sbt.owner_of(&token_id), subject);
        let owned_tokens = sbt.get_tokens_by_owner(&subject);
        assert_eq!(owned_tokens.len(), 1);
        assert_eq!(owned_tokens.get(0).unwrap(), token_id);

        // Assert SBT is linked to the correct credential
        let token = sbt.get_token(&token_id);
        assert_eq!(token.credential_id, cred_id);
        assert_eq!(token.owner, subject);

        // Step 6: Verify ZK claim via verify_engineer
        let proof = Bytes::from_slice(&env, b"valid-proof");
        let verified = qp.verify_engineer(&sbt_id, &zk_id, &zk_admin, &subject, &cred_id, &ClaimType::HasDegree, &proof);
        assert!(verified);

        // Assert empty proof fails verification
        let empty_proof = Bytes::new(&env);
        let not_verified = qp.verify_engineer(&sbt_id, &zk_id, &zk_admin, &subject, &cred_id, &ClaimType::HasDegree, &empty_proof);
        assert!(!not_verified);
    }

    // Issue #45: attest by address not in slice should panic
    #[test]
    #[should_panic(expected = "attestor not in slice")]
    fn test_attest_by_non_member_panics() {
        let env = Env::default();
        env.mock_all_auths();
        let contract_id = env.register_contract(None, QuorumProofContract);
        let client = QuorumProofContractClient::new(&env, &contract_id);

        let issuer = Address::generate(&env);
        let subject = Address::generate(&env);
        let attestor1 = Address::generate(&env);
        let attestor2 = Address::generate(&env); // not in slice

        let metadata = Bytes::from_slice(&env, b"ipfs://QmTest");
        let cred_id = client.issue_credential(&issuer, &subject, &1u32, &metadata, &None);

        // Create slice with only attestor1
        let mut attestors = soroban_sdk::Vec::new(&env);
        attestors.push_back(attestor1.clone());
        let mut weights = Vec::new(&env);
        weights.push_back(1u32);
        let slice_id = client.create_slice(&issuer, &attestors, &weights, &1u32);

        // attestor2 is not in the slice — must panic
        client.attest(&attestor2, &cred_id, &slice_id);
    }

    // --- Issue #185: remove_attestor ---

    #[test]
    fn test_remove_attestor_success() {
        let env = Env::default();
        env.mock_all_auths();
        let (client, _) = setup(&env);
        let creator = Address::generate(&env);
        let attestor1 = Address::generate(&env);
        let attestor2 = Address::generate(&env);

        let mut attestors = Vec::new(&env);
        attestors.push_back(attestor1.clone());
        attestors.push_back(attestor2.clone());
        let mut weights = Vec::new(&env);
        weights.push_back(1u32);
        weights.push_back(1u32);
        let slice_id = client.create_slice(&creator, &attestors, &weights, &2u32);

        client.remove_attestor(&creator, &slice_id, &attestor2);

        let slice = client.get_slice(&slice_id);
        assert_eq!(slice.attestors.len(), 1);
        assert_eq!(slice.attestors.get(0).unwrap(), attestor1);
        // threshold clamped to new total weight (1)
        assert_eq!(slice.threshold, 1);
    }

    #[test]
    #[should_panic(expected = "only the slice creator can remove attestors")]
    fn test_remove_attestor_unauthorized_panics() {
        let env = Env::default();
        env.mock_all_auths();
        let (client, _) = setup(&env);
        let creator = Address::generate(&env);
        let non_creator = Address::generate(&env);
        let attestor = Address::generate(&env);

        let mut attestors = Vec::new(&env);
        attestors.push_back(attestor.clone());
        attestors.push_back(Address::generate(&env));
        let mut weights = Vec::new(&env);
        weights.push_back(1u32);
        weights.push_back(1u32);
        let slice_id = client.create_slice(&creator, &attestors, &weights, &1u32);

        client.remove_attestor(&non_creator, &slice_id, &attestor);
    }

    #[test]
    #[should_panic(expected = "attestor not in slice")]
    fn test_remove_attestor_not_in_slice_panics() {
        let env = Env::default();
        env.mock_all_auths();
        let (client, _) = setup(&env);
        let creator = Address::generate(&env);
        let attestor = Address::generate(&env);
        let stranger = Address::generate(&env);

        let mut attestors = Vec::new(&env);
        attestors.push_back(attestor.clone());
        attestors.push_back(Address::generate(&env));
        let mut weights = Vec::new(&env);
        weights.push_back(1u32);
        weights.push_back(1u32);
        let slice_id = client.create_slice(&creator, &attestors, &weights, &1u32);

        client.remove_attestor(&creator, &slice_id, &stranger);
    }

    #[test]
    #[should_panic(expected = "cannot remove the last attestor")]
    fn test_remove_last_attestor_panics() {
        let env = Env::default();
        env.mock_all_auths();
        let (client, _) = setup(&env);
        let creator = Address::generate(&env);
        let attestor = Address::generate(&env);

        let mut attestors = Vec::new(&env);
        attestors.push_back(attestor.clone());
        let mut weights = Vec::new(&env);
        weights.push_back(1u32);
        let slice_id = client.create_slice(&creator, &attestors, &weights, &1u32);

        client.remove_attestor(&creator, &slice_id, &attestor);
    }

    // --- Issue #189: get_attestors ---

    #[test]
    fn test_get_attestors_returns_attested_addresses() {
        let env = Env::default();
        env.mock_all_auths();
        let (client, _) = setup(&env);
        let issuer = Address::generate(&env);
        let subject = Address::generate(&env);
        let attestor1 = Address::generate(&env);
        let attestor2 = Address::generate(&env);
        let metadata = Bytes::from_slice(&env, b"ipfs://QmTest");

        let cred_id = client.issue_credential(&issuer, &subject, &1u32, &metadata, &None);

        let mut attestors = Vec::new(&env);
        attestors.push_back(attestor1.clone());
        attestors.push_back(attestor2.clone());
        let mut weights = Vec::new(&env);
        weights.push_back(1u32);
        weights.push_back(1u32);
        let slice_id = client.create_slice(&issuer, &attestors, &weights, &1u32);

        assert_eq!(client.get_attestors(&cred_id).len(), 0);

        client.attest(&attestor1, &cred_id, &slice_id);
        let result = client.get_attestors(&cred_id);
        assert_eq!(result.len(), 1);
        assert_eq!(result.get(0).unwrap(), attestor1);

        client.attest(&attestor2, &cred_id, &slice_id);
        assert_eq!(client.get_attestors(&cred_id).len(), 2);
    }

    // --- Issue #226: credential_exists ---

    #[test]
    fn test_credential_exists_returns_true_for_existing() {
        let env = Env::default();
        env.mock_all_auths();
        let (client, _) = setup(&env);
        let issuer = Address::generate(&env);
        let subject = Address::generate(&env);
        let metadata = Bytes::from_slice(&env, b"ipfs://QmTest");

        let cred_id = client.issue_credential(&issuer, &subject, &1u32, &metadata, &None);
        assert!(client.credential_exists(&cred_id));
    }

    #[test]
    fn test_credential_exists_returns_false_for_nonexisting() {
        let env = Env::default();
        env.mock_all_auths();
        let (client, _) = setup(&env);
        assert!(!client.credential_exists(&999u64));
    }

    // --- Issue #227: slice_exists ---

    #[test]
    fn test_slice_exists_returns_true_for_existing() {
        let env = Env::default();
        env.mock_all_auths();
        let (client, _) = setup(&env);
        let creator = Address::generate(&env);
        let mut attestors = Vec::new(&env);
        attestors.push_back(Address::generate(&env));
        let mut weights = Vec::new(&env);
        weights.push_back(1u32);

        let slice_id = client.create_slice(&creator, &attestors, &weights, &1u32);
        assert!(client.slice_exists(&slice_id));
    }

    #[test]
    fn test_slice_exists_returns_false_for_nonexisting() {
        let env = Env::default();
        env.mock_all_auths();
        let (client, _) = setup(&env);
        assert!(!client.slice_exists(&999u64));
    }
}
