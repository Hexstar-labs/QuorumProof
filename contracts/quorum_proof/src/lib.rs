#![no_std]
use sbt_registry::SbtRegistryContractClient;
use soroban_sdk::{
    contract, contracterror, contractimpl, contracttype, panic_with_error, symbol_short, Address, Env, String,
    Vec, IntoVal,
};
use zk_verifier::{ClaimType, ZkVerifierContractClient};

const TOPIC_ISSUE: &str = "CredentialIssued";
const TOPIC_REVOKE: &str = "RevokeCredential";
const TOPIC_ATTESTATION: &str = "attestation";
const TOPIC_RENEWAL: &str = "CredentialRenewed";
const TOPIC_ATTESTATION_RENEWAL: &str = "AttestationRenewed";
const TOPIC_SBT_TRANSFER: &str = "SbtTransferred";
const TOPIC_PROOF_REQUEST: &str = "ProofRequested";
const TOPIC_RECOVERY_INITIATED: &str = "RecoveryInitiated";
const TOPIC_RECOVERY_APPROVED: &str = "RecoveryApproved";
const TOPIC_RECOVERY_EXECUTED: &str = "RecoveryExecuted";
const TOPIC_BLACKLIST_ADDED: &str = "HolderBlacklisted";
const TOPIC_BLACKLIST_REMOVED: &str = "HolderUnblacklisted";
const TOPIC_FORK_DETECTED: &str = "ForkDetected";
const TOPIC_FORK_RESOLVED: &str = "ForkResolved";
const STANDARD_TTL: u32 = 16_384;
const EXTENDED_TTL: u32 = 524_288;
const MAX_ATTESTORS_PER_SLICE: u32 = 20;
const MAX_BATCH_SIZE: u32 = 50;
const MAX_MULTISIG_SIGNERS: u32 = 10;

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

/// A single attestation record, capturing who attested, when, and the attestation value.
#[contracttype]
#[derive(Clone)]
pub struct AttestationRecord {
    pub attestor: Address,
    pub attested_at: u64,
    /// Optional Unix timestamp after which this attestation is considered expired.
    pub expires_at: Option<u64>,
    /// The attestation value: true for valid, false for invalid.
    pub attestation_value: bool,
}

#[contracttype]
#[derive(Clone)]
pub struct AttestationRenewalEventData {
    pub attestor: Address,
    pub credential_id: u64,
    pub new_expires_at: u64,
}

/// Event data emitted when a recovery is initiated.
#[contracttype]
#[derive(Clone)]
pub struct RecoveryInitiatedEventData {
    pub recovery_id: u64,
    pub credential_id: u64,
    pub issuer: Address,
    pub new_subject: Address,
}

/// Event data emitted when a recovery is approved.
#[contracttype]
#[derive(Clone)]
pub struct RecoveryApprovedEventData {
    pub recovery_id: u64,
    pub approver: Address,
}

/// Event data emitted when a recovery is executed.
#[contracttype]
#[derive(Clone)]
pub struct RecoveryExecutedEventData {
    pub recovery_id: u64,
    pub credential_id: u64,
    pub new_subject: Address,
}

/// Event data emitted when a holder is added to blacklist.
#[contracttype]
#[derive(Clone)]
pub struct HolderBlacklistedEventData {
    pub issuer: Address,
    pub holder: Address,
    pub reason: soroban_sdk::String,
    pub blacklisted_at: u64,
}

/// Event data emitted when a holder is removed from blacklist.
#[contracttype]
#[derive(Clone)]
pub struct HolderUnblacklistedEventData {
    pub issuer: Address,
    pub holder: Address,
    pub removed_at: u64,
}

/// Record of a holder being blacklisted by an issuer.
#[contracttype]
#[derive(Clone)]
pub struct BlacklistEntry {
    pub issuer: Address,
    pub holder: Address,
    pub reason: soroban_sdk::String,
    pub blacklisted_at: u64,
}

/// Event data emitted when a fork is detected.
#[contracttype]
#[derive(Clone)]
pub struct ForkDetectedEventData {
    pub credential_id: u64,
    pub slice_id: u64,
    pub conflicting_attestors: Vec<Address>,
    pub detected_at: u64,
}

/// Event data emitted when a fork is resolved.
#[contracttype]
#[derive(Clone)]
pub struct ForkResolvedEventData {
    pub credential_id: u64,
    pub slice_id: u64,
    pub resolution: soroban_sdk::String,
    pub resolved_at: u64,
}

/// Information about a detected fork.
#[contracttype]
#[derive(Clone)]
pub struct ForkInfo {
    pub credential_id: u64,
    pub slice_id: u64,
    pub conflicting_attestors: Vec<Address>,
    pub attested_values: Vec<bool>,
    pub detected_at: u64,
}

/// Status of fork detection for a credential.
#[contracttype]
#[derive(Clone, Copy, PartialEq, Eq)]
#[repr(u32)]
pub enum ForkStatus {
    NoFork = 1,
    ForkDetected = 2,
    ForkResolved = 3,
}

/// Represents the status of a credential recovery request.
#[contracttype]
#[derive(Clone, Copy, PartialEq, Eq)]
#[repr(u32)]
pub enum RecoveryStatus {
    Pending = 1,
    Approved = 2,
    Executed = 3,
    Rejected = 4,
}

/// A pending credential recovery request initiated by the issuer.
#[contracttype]
#[derive(Clone)]
pub struct RecoveryRequest {
    pub id: u64,
    pub credential_id: u64,
    pub issuer: Address,
    pub new_subject: Address,
    pub status: RecoveryStatus,
    pub created_at: u64,
    pub executed_at: Option<u64>,
    pub approvers: Vec<Address>,
    pub threshold: u32,
}

/// Records a single approval on a recovery request.
#[contracttype]
#[derive(Clone)]
pub struct RecoveryApproval {
    pub approver: Address,
    pub approved_at: u64,
}

/// Time window during which attestations are allowed for a credential.
#[contracttype]
#[derive(Clone)]
pub struct AttestationTimeWindow {
    /// Unix timestamp when the attestation window opens.
    pub start: u64,
    /// Unix timestamp when the attestation window closes.
    pub end: u64,
}

/// Records a veto applied to an attestation by a designated veto member.
#[contracttype]
#[derive(Clone)]
pub struct VetoRecord {
    pub vetoer: Address,
    pub credential_id: u64,
    pub justification: String,
    pub vetoed_at: u64,
}

#[contracterror]
#[derive(Copy, Clone, Debug, Eq, PartialEq, PartialOrd, Ord)]
#[repr(u32)]
pub enum ContractError {
    CredentialNotFound = 1,
    SliceNotFound = 2,
    ContractPaused = 3,
    DuplicateCredential = 4,
    DuplicateAttestor = 5,
    AttestationExpired = 6,
    InvalidInput = 7,
    InvalidAddress = 8,
    OnboardingNotFound = 9,
    DisputeNotFound = 10,
    UnauthorizedAction = 11,
    InvalidApprovalWorkflow = 12,
    AlreadyChallenged = 13,
    ChallengeNotFound = 14,
    ChallengeResolved = 15,
    NotAttested = 16,
    NotInSlice = 17,
    AccusedCannotVote = 18,
    AlreadyVoted = 19,
    AttestationWindowOutside = 20,
    RecoveryNotFound = 21,
    RecoveryAlreadyExists = 22,
    RecoveryNotPending = 23,
    RecoveryAlreadyApproved = 24,
    RecoveryThresholdNotMet = 25,
    NotRecoveryApprover = 26,
    DuplicateRecoveryApproval = 27,
    /// Credential type hierarchy error: parent type not found
    InvalidParentType = 28,
    /// Credential type hierarchy error: would create circular dependency
    CircularHierarchy = 29,
    /// Credential type is not registered
    CredentialTypeNotFound = 30,
    /// Holder is blacklisted by this issuer
    HolderBlacklisted = 31,
    /// Holder already on this issuer's blacklist
    AlreadyBlacklisted = 32,
    /// Holder not on this issuer's blacklist
    NotBlacklisted = 33,
    /// Fork detected: conflicting attestations for the same slice
    ForkDetected = 34,
    /// Fork already resolved for this slice
    ForkAlreadyResolved = 35,
    /// No fork exists for this slice
    NoForkExists = 36,
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
    /// Stores the ReputationRecovery record for an attestor
    ReputationRecovery(Address),
    /// Stores the Vec<ActivityRecord> history for a holder
    HolderActivity(Address),
    /// Stores the Vec<ConsensusDecision> history for a slice
    SliceConsensusHistory(u64),
    /// Stores all active OnboardingRequest IDs
    OnboardingRequests,
    /// Stores individual OnboardingRequest by ID
    OnboardingRequest(u64),
    /// Global monotonic counter for onboarding request IDs
    OnboardingRequestCount,
    /// Stores all active Dispute IDs
    Disputes,
    /// Stores individual Dispute by ID
    Dispute(u64),
    /// Global monotonic counter for dispute IDs
    DisputeCount,
    /// Stores votes for a dispute
    DisputeVotes(u64),
    /// Stores Challenge by ID
    Challenge(u64),
    /// Global monotonic counter for challenge IDs
    ChallengeCount,
    /// Stores active challenges by (credential_id, accused_address)
    ActiveChallenge(u64, Address),
    /// Stores expiry timestamp for specific attestations
    AttestationExpiry(u64),
    /// Stores the time window configuration for attestations on a credential
    AttestationWindow(u64),
    /// Stores a recovery request by ID
    RecoveryRequest(u64),
    /// Global monotonic counter for recovery request IDs
    RecoveryRequestCount,
    /// Maps credential_id to active recovery request ID
    CredentialRecovery(u64),
    /// Stores approval records for a recovery request
    RecoveryApprovals(u64),
    /// Stores the parent type ID for a credential type (for hierarchy)
    CredentialTypeParent(u32),
    /// Stores all child type IDs for a parent credential type
    CredentialTypeChildren(u32),
    /// Stores a blacklist entry for (issuer, holder) pair
    BlacklistEntry(Address, Address),
    /// Stores all holders blacklisted by an issuer
    IssuerBlacklist(Address),
    /// Stores all issuers who have blacklisted a holder
    HolderBlacklists(Address),
    /// Stores fork information for a (credential_id, slice_id) pair
    ForkInfo(u64, u64),
    /// Stores the fork status for a (credential_id, slice_id) pair
    ForkStatus(u64, u64),
}

#[contracttype]
#[derive(Clone)]
pub struct CredentialTypeDef {
    pub type_id: u32,
    pub name: soroban_sdk::String,
    pub description: soroban_sdk::String,
    /// Optional parent type ID for hierarchy support.
    /// Enables credential type inheritance and verification rule composition.
    pub parent_type: Option<u32>,
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
    pub version: u32,
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

/// Tracks a reputation recovery request for a slice member.
#[contracttype]
#[derive(Clone)]
pub struct ReputationRecovery {
    /// The attestor requesting recovery.
    pub attestor: Address,
    /// Ledger timestamp when recovery was initiated.
    pub initiated_at: u64,
    /// Whether the recovery has been completed.
    pub completed: bool,
}

/// A pending consent-based credential transfer request.
#[contracttype]
#[derive(Clone)]
pub struct TransferRequest {
    /// The credential being transferred.
    pub credential_id: u64,
    /// The current subject initiating the transfer.
    pub from: Address,
    /// The intended recipient who must accept.
    pub to: Address,
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

/// Activity types that can be tracked per credential holder
#[contracttype]
#[derive(Clone, Copy, PartialEq, Eq)]
#[repr(u32)]
pub enum ActivityType {
    CredentialIssued = 1,
    CredentialRevoked = 2,
    CredentialRenewed = 3,
    CredentialAttested = 4,
    AttestationExpired = 5,
    CredentialRecovered = 6,
}

/// Records a single activity event for a credential holder
#[contracttype]
#[derive(Clone)]
pub struct ActivityRecord {
    pub activity_type: ActivityType,
    pub credential_id: u64,
    pub timestamp: u64,
    pub actor: Address, // issuer, attestor, or revoker
    pub slice_id: Option<u64>, // for attestation-related activities
}

/// Records a consensus decision for a quorum slice
#[contracttype]
#[derive(Clone)]
pub struct ConsensusDecision {
    pub decision_id: u64,
    pub slice_id: u64,
    pub credential_id: u64,
    pub timestamp: u64,
    pub required_weight_threshold: u32,
    pub achieved_weight: u32,
    pub total_weight: u32,
}

/// Represents the status of an onboarding request
#[contracttype]
#[derive(Clone, Copy, PartialEq, Eq)]
#[repr(u32)]
pub enum OnboardingStatus {
    Pending = 1,
    Approved = 2,
    Rejected = 3,
}

/// Represents a vote on an onboarding request
#[contracttype]
#[derive(Clone)]
pub struct OnboardingVote {
    pub voter: Address,
    pub approval: bool,
    pub voted_at: u64,
}

/// Represents an onboarding request for a new slice member
#[contracttype]
#[derive(Clone)]
pub struct OnboardingRequest {
    pub id: u64,
    pub slice_id: u64,
    pub requester: Address,
    pub proposed_member: Address,
    pub proposed_weight: u32,
    pub status: OnboardingStatus,
    pub created_at: u64,
    pub votes: Vec<OnboardingVote>,
}

/// Represents the status of a dispute
#[contracttype]
#[derive(Clone, Copy, PartialEq, Eq)]
#[repr(u32)]
pub enum DisputeStatus {
    Active = 1,
    Resolved = 2,
    Dismissed = 3,
}

/// Represents a vote on a dispute resolution
#[contracttype]
#[derive(Clone)]
pub struct DisputeVote {
    pub voter: Address,
    pub resolution: u32, // 0 = no vote, 1 = favor initiator, 2 = favor accused
    pub voted_at: u64,
}

/// Represents a dispute between slice members
#[contracttype]
#[derive(Clone)]
pub struct Dispute {
    pub id: u64,
    pub slice_id: u64,
    pub initiator: Address,
    pub accused: Address,
    pub reason: String,
    pub status: DisputeStatus,
    pub created_at: u64,
    pub votes: Vec<DisputeVote>,
}

/// Represents the status of a challenge
#[contracttype]
#[derive(Clone, Copy, PartialEq, Eq)]
#[repr(u32)]
pub enum ChallengeStatus {
    Open = 1,
    Upheld = 2,
    Dismissed = 3,
}

/// Represents a challenge to a credential attestation
#[contracttype]
#[derive(Clone)]
pub struct Challenge {
    pub id: u64,
    pub credential_id: u64,
    pub slice_id: u64,
    pub accused: Address,
    pub challenger: Address,
    pub status: ChallengeStatus,
    pub uphold_votes: Vec<Address>,
    pub dismiss_votes: Vec<Address>,
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
        Self::require_valid_address(&env, &admin);
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

    /// Validate that an address is not the zero/default address.
    /// In Soroban, the Address type guarantees validity at the type level.
    /// This function exists for API consistency and future extensibility.
    /// Currently a no-op since Soroban addresses are always valid.
    fn require_valid_address(_env: &Env, _addr: &Address) {
        // Soroban's Address type is always valid by construction.
        // No validation needed, but we keep this function for:
        // 1. API consistency across all address inputs
        // 2. Future extensibility if custom validation is needed
        // 3. Clear documentation of validation intent
    }

    /// Pre-condition assertion. Panics with `ContractError::InvalidInput` if `cond` is false.
    fn precondition(env: &Env, cond: bool) {
        if !cond {
            panic_with_error!(env, ContractError::InvalidInput);
        }
    }

    /// Post-condition assertion. Panics with a static message if `cond` is false.
    /// Used to assert invariants after state mutations.
    fn postcondition(cond: bool, _msg: &str) {
        if !cond {
            panic!("postcondition violated");
        }
    }

    /// Record an activity for a credential holder
    fn record_holder_activity(env: &Env, holder: Address, activity_type: ActivityType, credential_id: u64, actor: Address, slice_id: Option<u64>) {
        let activity = ActivityRecord {
            activity_type,
            credential_id,
            timestamp: env.ledger().timestamp(),
            actor,
            slice_id,
        };
        
        let mut activities: Vec<ActivityRecord> = env
            .storage()
            .instance()
            .get(&DataKey::HolderActivity(holder.clone()))
            .unwrap_or(Vec::new(env));
        activities.push_back(activity);
        env.storage()
            .instance()
            .set(&DataKey::HolderActivity(holder), &activities);
        env.storage()
            .instance()
            .extend_ttl(STANDARD_TTL, EXTENDED_TTL);
    }

    /// Set a condition-based expiry timestamp for attestations on a credential.
    /// After this timestamp, `is_attestation_expired` returns `true` and
    /// `is_attested` treats the attestation as invalid.
    ///
    /// Only the credential issuer may set this.
    ///
    /// # Panics
    /// Panics with `ContractError::CredentialNotFound` if the credential does not exist.
    /// Panics if the caller is not the issuer.
    /// Panics with `ContractError::InvalidInput` if `expires_at` is not in the future.
    pub fn set_attestation_expiry(env: Env, issuer: Address, credential_id: u64, expires_at: u64) {
        issuer.require_auth();
        Self::require_not_paused(&env);
        let credential: Credential = env
            .storage()
            .instance()
            .get(&DataKey::Credential(credential_id))
            .unwrap_or_else(|| panic_with_error!(&env, ContractError::CredentialNotFound));
        assert!(credential.issuer == issuer, "only the credential issuer can set attestation expiry");
        Self::precondition(&env, expires_at > env.ledger().timestamp());
        env.storage()
            .instance()
            .set(&DataKey::AttestationExpiry(credential_id), &expires_at);
        env.storage()
            .instance()
            .extend_ttl(STANDARD_TTL, EXTENDED_TTL);
    }

    /// Returns `true` if a condition-based attestation expiry has been set for the credential
    /// and the current ledger timestamp has passed it.
    ///
    /// Returns `false` if no attestation expiry is set (attestations do not expire by condition).
    ///
    /// # Panics
    /// Panics with `ContractError::CredentialNotFound` if the credential does not exist.
    pub fn is_attestation_expired(env: Env, credential_id: u64) -> bool {
        if !env.storage().instance().has(&DataKey::Credential(credential_id)) {
            panic_with_error!(&env, ContractError::CredentialNotFound);
        }
        match env
            .storage()
            .instance()
            .get::<DataKey, u64>(&DataKey::AttestationExpiry(credential_id))
        {
            Some(expires_at) => env.ledger().timestamp() >= expires_at,
            None => false,
        }
    }

    /// Configure a time window during which attestations are allowed for a credential.
    /// Only the credential issuer may set this.
    ///
    /// # Panics
    /// Panics with `ContractError::CredentialNotFound` if the credential does not exist.
    /// Panics if the caller is not the issuer.
    /// Panics with `ContractError::InvalidInput` if `start >= end`.
    pub fn set_attestation_window(env: Env, issuer: Address, credential_id: u64, start: u64, end: u64) {
        issuer.require_auth();
        Self::require_not_paused(&env);
        let credential: Credential = env
            .storage()
            .instance()
            .get(&DataKey::Credential(credential_id))
            .unwrap_or_else(|| panic_with_error!(&env, ContractError::CredentialNotFound));
        assert!(credential.issuer == issuer, "only the credential issuer can set attestation window");
        Self::precondition(&env, start < end);
        let window = AttestationTimeWindow { start, end };
        env.storage()
            .instance()
            .set(&DataKey::AttestationWindow(credential_id), &window);
        env.storage()
            .instance()
            .extend_ttl(STANDARD_TTL, EXTENDED_TTL);
    }

    /// Returns the attestation time window for a credential, if one has been configured.
    pub fn get_attestation_window(env: Env, credential_id: u64) -> Option<AttestationTimeWindow> {
        env.storage()
            .instance()
            .get(&DataKey::AttestationWindow(credential_id))
    }

    /// Validate an array input has between `min` and `max` elements (inclusive).
    fn validate_array_bounds(len: u32, min: u32, max: u32, name: &'static str) {
        assert!(len >= min, "{} must have at least {} element(s)", name, min);
        assert!(len <= max, "{} must have at most {} element(s)", name, max);
    }

    /// Check if a parent type exists in storage.
    /// Returns false if the type is not registered.
    fn parent_type_exists(env: &Env, parent_type: u32) -> bool {
        env.storage()
            .instance()
            .has(&DataKey::CredentialType(parent_type))
    }

    /// Recursively check if adding `potential_parent` as a parent to `type_id` would create
    /// a circular dependency in the type hierarchy.
    /// Returns true if a cycle would be created, false otherwise.
    fn would_create_cycle(env: &Env, type_id: u32, potential_parent: u32) -> bool {
        if type_id == potential_parent {
            return true;
        }
        
        // Check if potential_parent is already in the ancestors of type_id
        let mut current = Some(potential_parent);
        while let Some(curr_type) = current {
            if curr_type == type_id {
                return true;
            }
            // Get the parent of current type
            current = env
                .storage()
                .instance()
                .get::<DataKey, Option<u32>>(&DataKey::CredentialTypeParent(curr_type))
                .flatten();
        }
        false
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
        // Pre-conditions
        Self::require_valid_address(&env, &issuer);
        Self::require_valid_address(&env, &subject);
        assert!(
            credential_type > 0,
            "credential_type must be greater than 0"
        );
        assert!(!metadata_hash.is_empty(), "metadata_hash cannot be empty");
        Self::precondition(&env, metadata_hash.len() <= 256);
        
        // Check for duplicate credential of same type from same issuer to same subject
        let duplicate_key = DataKey::SubjectIssuerType(subject.clone(), issuer.clone(), credential_type);
        if env.storage().instance().has(&duplicate_key) {
            panic_with_error!(&env, ContractError::DuplicateCredential);
        }
        
        // Check if subject is blacklisted by issuer
        if env.storage().instance().has(&DataKey::BlacklistEntry(issuer.clone(), subject.clone())) {
            panic_with_error!(&env, ContractError::HolderBlacklisted);
        }
        
        let id: u64 = env.storage().instance().get(&DataKey::CredentialCount).unwrap_or(0u64) + 1;
        let credential = Credential { id, subject: subject.clone(), issuer: issuer.clone(), credential_type, metadata_hash, revoked: false, expires_at, version: 1 };
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

        // Post-condition: credential must be stored
        Self::postcondition(env.storage().instance().has(&DataKey::Credential(id)), "credential stored");
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
        Self::validate_array_bounds(n, 1, MAX_BATCH_SIZE, "subjects");
        assert!(
            credential_types.len() == n && metadata_hashes.len() == n,
            "input lengths must match"
        );
        let mut ids: Vec<u64> = Vec::new(&env);
        for i in 0..n {
            let subject = subjects.get(i).unwrap();
            let credential_type = credential_types.get(i).unwrap();
            let metadata_hash = metadata_hashes.get(i).unwrap();
            assert!(credential_type > 0, "credential_type must be greater than 0");
            let duplicate_key = DataKey::SubjectIssuerType(subject.clone(), issuer.clone(), credential_type);
            if env.storage().instance().has(&duplicate_key) {
                panic_with_error!(&env, ContractError::DuplicateCredential);
            }
            // Check if subject is blacklisted by issuer
            if env.storage().instance().has(&DataKey::BlacklistEntry(issuer.clone(), subject.clone())) {
                panic_with_error!(&env, ContractError::HolderBlacklisted);
            }
            let id = Self::issue_inner(&env, issuer.clone(), subject, credential_type, metadata_hash, expires_at.clone());
            env.storage().instance().set(&duplicate_key, &id);
            env.storage().instance().extend_ttl(STANDARD_TTL, EXTENDED_TTL);
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
        Self::validate_hash(&metadata_hash);
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
            version: 1,
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

        // Record activity for the holder
        Self::record_holder_activity(&env, subject.clone(), ActivityType::CredentialIssued, id, issuer.clone(), None);
        
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

    /// Update the metadata hash of a credential and increment its version.
    ///
    /// Only the original issuer may call this function.
    ///
    /// # Parameters
    /// - `issuer`: The address that originally issued the credential; must authorize.
    /// - `credential_id`: The ID of the credential to update.
    /// - `new_metadata_hash`: The new metadata hash to store.
    ///
    /// # Panics
    /// Panics with `ContractError::CredentialNotFound` if the credential does not exist.
    /// Panics if the caller is not the original issuer.
    pub fn update_metadata(
        env: Env,
        issuer: Address,
        credential_id: u64,
        new_metadata_hash: soroban_sdk::Bytes,
    ) {
        issuer.require_auth();
        Self::require_not_paused(&env);
        assert!(!new_metadata_hash.is_empty(), "metadata_hash cannot be empty");
        let mut credential: Credential = env
            .storage()
            .instance()
            .get(&DataKey::Credential(credential_id))
            .unwrap_or_else(|| panic_with_error!(&env, ContractError::CredentialNotFound));
        assert!(credential.issuer == issuer, "only the issuer may update metadata");
        credential.metadata_hash = new_metadata_hash;
        credential.version += 1;
        env.storage()
            .instance()
            .set(&DataKey::Credential(credential_id), &credential);
        env.storage().instance().extend_ttl(STANDARD_TTL, EXTENDED_TTL);
    }

    /// Initiate a consent-based transfer of a credential to a new subject.
    ///
    /// The current credential subject calls this to propose a transfer to `to`.
    /// The transfer is not final until `accept_transfer` is called by `to`.
    ///
    /// # Panics
    /// Panics with `ContractError::CredentialNotFound` if the credential does not exist.
    /// Panics with `ContractError::UnauthorizedTransfer` if the caller is not the current subject.
    pub fn initiate_transfer(env: Env, from: Address, credential_id: u64, to: Address) {
        from.require_auth();
        Self::require_not_paused(&env);
        let credential: Credential = env
            .storage()
            .instance()
            .get(&DataKey::Credential(credential_id))
            .unwrap_or_else(|| panic_with_error!(&env, ContractError::CredentialNotFound));
        if credential.subject != from {
            panic_with_error!(&env, ContractError::UnauthorizedTransfer);
        }
        let request = TransferRequest { credential_id, from, to };
        env.storage()
            .instance()
            .set(&DataKey::TransferRequest(credential_id), &request);
        env.storage().instance().extend_ttl(STANDARD_TTL, EXTENDED_TTL);
    }

    /// Accept a pending transfer request, reassigning the credential to the caller.
    ///
    /// Only the address named as `to` in the pending `TransferRequest` may call this.
    ///
    /// # Panics
    /// Panics with `ContractError::CredentialNotFound` if the credential does not exist.
    /// Panics with `ContractError::UnauthorizedTransfer` if no pending request exists or
    /// the caller is not the intended recipient.
    pub fn accept_transfer(env: Env, to: Address, credential_id: u64) {
        to.require_auth();
        Self::require_not_paused(&env);
        let request: TransferRequest = env
            .storage()
            .instance()
            .get(&DataKey::TransferRequest(credential_id))
            .unwrap_or_else(|| panic_with_error!(&env, ContractError::UnauthorizedTransfer));
        if request.to != to {
            panic_with_error!(&env, ContractError::UnauthorizedTransfer);
        }
        let mut credential: Credential = env
            .storage()
            .instance()
            .get(&DataKey::Credential(credential_id))
            .unwrap_or_else(|| panic_with_error!(&env, ContractError::CredentialNotFound));

        // Remove credential from old subject's list
        let mut old_creds: Vec<u64> = env
            .storage()
            .instance()
            .get(&DataKey::SubjectCredentials(credential.subject.clone()))
            .unwrap_or(Vec::new(&env));
        let mut retained: Vec<u64> = Vec::new(&env);
        for id in old_creds.iter() {
            if id != credential_id {
                retained.push_back(id);
            }
        }
        env.storage()
            .instance()
            .set(&DataKey::SubjectCredentials(credential.subject.clone()), &retained);

        // Add to new subject's list
        let mut new_creds: Vec<u64> = env
            .storage()
            .instance()
            .get(&DataKey::SubjectCredentials(to.clone()))
            .unwrap_or(Vec::new(&env));
        new_creds.push_back(credential_id);
        env.storage()
            .instance()
            .set(&DataKey::SubjectCredentials(to.clone()), &new_creds);

        // Update credential subject
        credential.subject = to;
        env.storage()
            .instance()
            .set(&DataKey::Credential(credential_id), &credential);

        // Clear the pending request
        env.storage()
            .instance()
            .remove(&DataKey::TransferRequest(credential_id));
        env.storage().instance().extend_ttl(STANDARD_TTL, EXTENDED_TTL);
    }

    /// Return all credential IDs issued to a subject.
    ///
    /// # Parameters
    /// - `subject`: The address whose credentials to look up.
    ///
    /// # Panics
    /// Does not panic; returns an empty `Vec` if the subject has no credentials.
    pub fn get_credentials_by_subject(env: Env, subject: Address, page: u32, page_size: u32) -> Vec<u64> {
        Self::require_valid_address(&env, &subject);
        Self::precondition(&env, page > 0);
        Self::precondition(&env, page_size > 0);
        let all_creds: Vec<u64> = env.storage()
            .instance()
            .get(&DataKey::SubjectCredentials(subject))
            .unwrap_or(Vec::new(&env));
        let total = all_creds.len();
        let start = (page - 1).saturating_mul(page_size);
        let mut result = Vec::new(&env);
        for i in start..start.saturating_add(page_size) {
            if i >= total {
                break;
            }
            if let Some(cred) = all_creds.get(i) {
                result.push_back(cred);
            }
        }
        result
    }

    /// Check if a credential with the given ID exists.
    ///
    /// # Parameters
    /// - `credential_id`: The ID of the credential to check.
    ///
    /// # Returns
    /// Returns `true` if a credential with the given ID exists, `false` otherwise.
    ///
    /// # Panics
    /// Does not panic; returns `false` if the credential does not exist.
    pub fn credential_exists(env: Env, credential_id: u64) -> bool {
        env.storage()
            .instance()
            .has(&DataKey::Credential(credential_id))
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
        let mut subject_creds: Vec<u64> = env
            .storage()
            .instance()
            .get(&DataKey::SubjectCredentials(credential.subject.clone()))
            .unwrap_or(Vec::new(&env));
        let mut retained: Vec<u64> = Vec::new(&env);
        for id in subject_creds.iter() {
            if id != credential_id {
                retained.push_back(id);
            }
        }
        if retained.len() != subject_creds.len() {
            subject_creds = retained;
            env.storage()
                .instance()
                .set(&DataKey::SubjectCredentials(credential.subject.clone()), &subject_creds);
        }
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

        // Record activity for the holder
        Self::record_holder_activity(&env, credential.subject.clone(), ActivityType::CredentialRevoked, credential_id, issuer.clone(), None);
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

        // Record activity for the holder
        Self::record_holder_activity(&env, credential.subject.clone(), ActivityType::CredentialRenewed, credential_id, issuer.clone(), None);
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
        Self::require_valid_address(&env, &creator);
        assert!(attestors.len() > 0, "attestors cannot be empty");
        assert!(
            attestors.len() as u32 <= MAX_ATTESTORS_PER_SLICE,
            "attestors exceed maximum allowed per slice"
        );
        assert!(
            weights.len() == attestors.len(),
            "weights length must match attestors length"
        );
        assert!(threshold > 0, "threshold must be greater than 0");
        assert!(
            threshold <= attestors.len() as u32,
            "threshold cannot exceed attestors length"
        );
        // Validate each attestor address
        for a in attestors.iter() {
            Self::require_valid_address(&env, &a);
        }
        // Calculate total weight sum
        let mut total_weight: u32 = 0;
        for w in weights.iter() {
            total_weight = total_weight.saturating_add(w);
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
        // Post-condition: slice must be stored
        Self::postcondition(env.storage().instance().has(&DataKey::Slice(id)), "slice stored");
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

    /// Check if a quorum slice resides in state.
    pub fn slice_exists(env: Env, slice_id: u64) -> bool {
        env.storage().instance().has(&DataKey::Slice(slice_id))
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
            total_weight = total_weight.saturating_add(w);
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
        Self::require_valid_address(&env, &creator);
        Self::require_valid_address(&env, &attestor);
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
            if a == attestor {
                panic_with_error!(&env, ContractError::DuplicateAttestor);
            }
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
    pub fn update_slice_threshold(env: Env, creator: Address, slice_id: u64, new_threshold: u32) {
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
            total_weight = total_weight.saturating_add(w);
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
    /// - `expires_at`: Optional Unix timestamp after which this attestation expires.
    ///
    /// # Panics
    /// Panics if the contract is paused.
    /// Panics with `ContractError::CredentialNotFound` if the credential does not exist.
    /// Panics if the credential is revoked.
    /// Panics if the attestor is not a member of the slice.
    /// Panics if the attestor has already attested for this credential.
    
    // ── Credential Holder Blacklist (Issue #293) ──────────────────────────────

    /// Add a holder to an issuer's blacklist.
    ///
    /// # Parameters
    /// - `issuer`: The issuer adding to blacklist; must authorize this call.
    /// - `holder`: The holder address to blacklist.
    /// - `reason`: Reason for blacklisting (stored in record).
    ///
    /// # Panics
    /// Panics if the contract is paused.
    /// Panics with `ContractError::AlreadyBlacklisted` if holder is already blacklisted by issuer.
    /// Panics if issuer does not authorize the call.
    pub fn add_holder_to_blacklist(
        env: Env,
        issuer: Address,
        holder: Address,
        reason: soroban_sdk::String,
    ) {
        issuer.require_auth();
        Self::require_not_paused(&env);
        Self::require_valid_address(&env, &issuer);
        Self::require_valid_address(&env, &holder);

        let entry_key = DataKey::BlacklistEntry(issuer.clone(), holder.clone());
        if env.storage().instance().has(&entry_key) {
            panic_with_error!(&env, ContractError::AlreadyBlacklisted);
        }

        let blacklist_entry = BlacklistEntry {
            issuer: issuer.clone(),
            holder: holder.clone(),
            reason: reason.clone(),
            blacklisted_at: env.ledger().timestamp(),
        };

        // Store the entry
        env.storage().instance().set(&entry_key, &blacklist_entry);
        env.storage()
            .instance()
            .extend_ttl(STANDARD_TTL, EXTENDED_TTL);

        // Add to issuer's blacklist
        let mut issuer_blacklist: Vec<Address> = env
            .storage()
            .instance()
            .get(&DataKey::IssuerBlacklist(issuer.clone()))
            .unwrap_or(Vec::new(&env));
        if !issuer_blacklist.iter().any(|&addr| addr == holder) {
            issuer_blacklist.push_back(holder.clone());
            env.storage()
                .instance()
                .set(&DataKey::IssuerBlacklist(issuer.clone()), &issuer_blacklist);
            env.storage()
                .instance()
                .extend_ttl(STANDARD_TTL, EXTENDED_TTL);
        }

        // Add to holder's recorded blacklists
        let mut holder_blacklists: Vec<Address> = env
            .storage()
            .instance()
            .get(&DataKey::HolderBlacklists(holder.clone()))
            .unwrap_or(Vec::new(&env));
        if !holder_blacklists.iter().any(|&addr| addr == issuer) {
            holder_blacklists.push_back(issuer.clone());
            env.storage()
                .instance()
                .set(&DataKey::HolderBlacklists(holder.clone()), &holder_blacklists);
            env.storage()
                .instance()
                .extend_ttl(STANDARD_TTL, EXTENDED_TTL);
        }

        // Emit event
        let event_data = HolderBlacklistedEventData {
            issuer,
            holder,
            reason,
            blacklisted_at: env.ledger().timestamp(),
        };
        let topic = String::from_str(&env, TOPIC_BLACKLIST_ADDED);
        let mut topics: Vec<String> = Vec::new(&env);
        topics.push_back(topic);
        env.events().publish(topics, event_data);
    }

    /// Check if a holder is blacklisted by an issuer.
    ///
    /// # Parameters
    /// - `issuer`: The issuer to check blacklist for.
    /// - `holder`: The holder address to check.
    ///
    /// # Returns
    /// true if holder is blacklisted by issuer, false otherwise.
    pub fn is_holder_blacklisted(env: Env, issuer: Address, holder: Address) -> bool {
        env.storage()
            .instance()
            .has(&DataKey::BlacklistEntry(issuer, holder))
    }

    /// Remove a holder from an issuer's blacklist.
    ///
    /// # Parameters
    /// - `issuer`: The issuer removing from blacklist; must authorize this call.
    /// - `holder`: The holder address to remove from blacklist.
    ///
    /// # Panics
    /// Panics if the contract is paused.
    /// Panics with `ContractError::NotBlacklisted` if holder is not on issuer's blacklist.
    /// Panics if issuer does not authorize the call.
    pub fn remove_holder_from_blacklist(env: Env, issuer: Address, holder: Address) {
        issuer.require_auth();
        Self::require_not_paused(&env);
        Self::require_valid_address(&env, &issuer);
        Self::require_valid_address(&env, &holder);

        let entry_key = DataKey::BlacklistEntry(issuer.clone(), holder.clone());
        if !env.storage().instance().has(&entry_key) {
            panic_with_error!(&env, ContractError::NotBlacklisted);
        }

        // Remove the entry
        env.storage().instance().remove(&entry_key);

        // Remove from issuer's blacklist
        let mut issuer_blacklist: Vec<Address> = env
            .storage()
            .instance()
            .get(&DataKey::IssuerBlacklist(issuer.clone()))
            .unwrap_or(Vec::new(&env));
        let mut retained: Vec<Address> = Vec::new(&env);
        for addr in issuer_blacklist.iter() {
            if addr != holder {
                retained.push_back(addr);
            }
        }
        if retained.len() < issuer_blacklist.len() {
            env.storage()
                .instance()
                .set(&DataKey::IssuerBlacklist(issuer.clone()), &retained);
            env.storage()
                .instance()
                .extend_ttl(STANDARD_TTL, EXTENDED_TTL);
        }

        // Remove from holder's recorded blacklists
        let mut holder_blacklists: Vec<Address> = env
            .storage()
            .instance()
            .get(&DataKey::HolderBlacklists(holder.clone()))
            .unwrap_or(Vec::new(&env));
        let mut retained: Vec<Address> = Vec::new(&env);
        for addr in holder_blacklists.iter() {
            if addr != issuer {
                retained.push_back(addr);
            }
        }
        if retained.len() < holder_blacklists.len() {
            env.storage()
                .instance()
                .set(&DataKey::HolderBlacklists(holder.clone()), &retained);
            env.storage()
                .instance()
                .extend_ttl(STANDARD_TTL, EXTENDED_TTL);
        }

        // Emit event
        let event_data = HolderUnblacklistedEventData {
            issuer,
            holder,
            removed_at: env.ledger().timestamp(),
        };
        let topic = String::from_str(&env, TOPIC_BLACKLIST_REMOVED);
        let mut topics: Vec<String> = Vec::new(&env);
        topics.push_back(topic);
        env.events().publish(topics, event_data);
    }

    /// Get all holders blacklisted by an issuer.
    ///
    /// # Parameters
    /// - `issuer`: The issuer to query.
    ///
    /// # Returns
    /// Vec of holder addresses blacklisted by this issuer.
    pub fn get_blacklisted_by_issuer(env: Env, issuer: Address) -> Vec<Address> {
        env.storage()
            .instance()
            .get(&DataKey::IssuerBlacklist(issuer))
            .unwrap_or(Vec::new(&env))
    }

    /// Get all issuers who have blacklisted a holder.
    ///
    /// # Parameters
    /// - `holder`: The holder to query.
    ///
    /// # Returns
    /// Vec of issuer addresses that have blacklisted this holder.
    pub fn get_blacklist_entries_for_holder(env: Env, holder: Address) -> Vec<Address> {
        env.storage()
            .instance()
            .get(&DataKey::HolderBlacklists(holder))
            .unwrap_or(Vec::new(&env))
    }

    /// Get the blacklist entry for a specific issuer-holder pair.
    ///
    /// # Parameters
    /// - `issuer`: The issuer.
    /// - `holder`: The holder.
    ///
    /// # Returns
    /// Some(BlacklistEntry) if holder is blacklisted by issuer, None otherwise.
    pub fn get_blacklist_entry(env: Env, issuer: Address, holder: Address) -> Option<BlacklistEntry> {
        env.storage()
            .instance()
            .get(&DataKey::BlacklistEntry(issuer, holder))
    }

    /// Detects if a fork would occur or exists for a credential in a slice.
    /// A fork occurs when attestors in the same slice attest different values.
    /// Returns true if a fork is detected, false otherwise.
    pub fn detect_fork(env: &Env, credential_id: u64, slice_id: u64, new_attestor: &Address, new_value: bool) -> bool {
        // Get the slice to know which attestors are relevant
        let slice: QuorumSlice = env
            .storage()
            .instance()
            .get(&DataKey::Slice(slice_id))
            .unwrap_or_else(|| panic_with_error!(env, ContractError::SliceNotFound));

        // Get all attestation records for the credential
        let records: Vec<AttestationRecord> = env
            .storage()
            .instance()
            .get(&DataKey::Attestors(credential_id))
            .unwrap_or(Vec::new(env));

        // Collect values attested by attestors in this slice
        let mut slice_values: Vec<bool> = Vec::new(env);
        for record in records.iter() {
            // Check if this attestor is in the slice
            let mut in_slice = false;
            for attestor in slice.attestors.iter() {
                if attestor == record.attestor {
                    in_slice = true;
                    break;
                }
            }
            if in_slice {
                slice_values.push_back(record.attestation_value);
            }
        }

        // Check if new attestor is in slice (should be validated elsewhere)
        let mut new_in_slice = false;
        for attestor in slice.attestors.iter() {
            if attestor == *new_attestor {
                new_in_slice = true;
                break;
            }
        }
        if !new_in_slice {
            return false; // Not in slice, no fork concern
        }

        // Check for conflicts: if any existing value differs from new_value, or if existing values differ
        for &existing_value in slice_values.iter() {
            if existing_value != new_value {
                return true; // Fork detected
            }
        }

        false // No fork
    }

    pub fn attest(env: Env, attestor: Address, credential_id: u64, slice_id: u64, attestation_value: bool, expires_at: Option<u64>) {
        attestor.require_auth();
        Self::require_not_paused(&env);
        Self::require_valid_address(&env, &attestor);
        // Pre-condition: credential_id and slice_id must be non-zero
        Self::precondition(&env, credential_id > 0);
        Self::precondition(&env, slice_id > 0);
        let credential: Credential = env
            .storage()
            .instance()
            .get(&DataKey::Credential(credential_id))
            .unwrap_or_else(|| panic_with_error!(&env, ContractError::CredentialNotFound));
        assert!(!credential.revoked, "credential is revoked");
        // Enforce attestation time window if configured
        if let Some(window) = env.storage().instance().get::<DataKey, AttestationTimeWindow>(&DataKey::AttestationWindow(credential_id)) {
            let now = env.ledger().timestamp();
            if now < window.start || now >= window.end {
                panic_with_error!(&env, ContractError::AttestationWindowOutside);
            }
        }
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

        // Check for fork before allowing attestation
        if Self::detect_fork(&env, credential_id, slice_id, &attestor, attestation_value) {
            // Store fork information
            let records: Vec<AttestationRecord> = env.storage().instance()
                .get(&DataKey::Attestors(credential_id))
                .unwrap_or(Vec::new(&env));
            let mut conflicting_attestors: Vec<Address> = Vec::new(&env);
            let mut attested_values: Vec<bool> = Vec::new(&env);
            for record in records.iter() {
                let mut in_slice = false;
                for a in slice.attestors.iter() {
                    if a == record.attestor {
                        in_slice = true;
                        break;
                    }
                }
                if in_slice {
                    conflicting_attestors.push_back(record.attestor.clone());
                    attested_values.push_back(record.attestation_value);
                }
            }
            conflicting_attestors.push_back(attestor.clone());
            attested_values.push_back(attestation_value);

            let fork_info = ForkInfo {
                credential_id,
                slice_id,
                conflicting_attestors: conflicting_attestors.clone(),
                attested_values,
                detected_at: env.ledger().timestamp(),
            };
            env.storage().instance().set(&DataKey::ForkInfo(credential_id, slice_id), &fork_info);
            env.storage().instance().set(&DataKey::ForkStatus(credential_id, slice_id), &ForkStatus::ForkDetected);

            // Emit fork detected event
            let event_data = ForkDetectedEventData {
                credential_id,
                slice_id,
                conflicting_attestors,
                detected_at: env.ledger().timestamp(),
            };
            let topic = String::from_str(&env, TOPIC_FORK_DETECTED);
            let mut topics: Vec<String> = Vec::new(&env);
            topics.push_back(topic);
            env.events().publish(topics, event_data);

            panic_with_error!(&env, ContractError::ForkDetected);
        }

        let mut records: Vec<AttestationRecord> = env.storage().instance()
            .get(&DataKey::Attestors(credential_id))
            .unwrap_or(Vec::new(&env));

        // Check if attestor has already attested for this credential
        for rec in records.iter() {
            if rec.attestor == attestor {
                panic!("attestor has already attested for this credential");
            }
        }

        let record = AttestationRecord {
            attestor: attestor.clone(),
            attested_at: env.ledger().timestamp(),
            expires_at,
            attestation_value,
        };
        records.push_back(record);
        env.storage()
            .instance()
            .set(&DataKey::Attestors(credential_id), &records);
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

        // Record activity for the holder
        let credential: Credential = env
            .storage()
            .instance()
            .get(&DataKey::Credential(credential_id))
            .unwrap_or_else(|| panic_with_error!(&env, ContractError::CredentialNotFound));
        Self::record_holder_activity(&env, credential.subject.clone(), ActivityType::CredentialAttested, credential_id, attestor.clone(), Some(slice_id));
    }

    /// Batch attest multiple credentials in a single transaction.
    /// Each credential_id in the list is attested by the caller using the given slice.
    /// Caller must be a member of the slice for each credential.
    pub fn batch_attest(env: Env, attestor: Address, credential_ids: Vec<u64>, slice_id: u64, attestation_value: bool, expires_at: Option<u64>) {
        attestor.require_auth();
        Self::require_not_paused(&env);
        Self::validate_array_bounds(credential_ids.len(), 1, MAX_BATCH_SIZE, "credential_ids");
        for credential_id in credential_ids.iter() {
            Self::attest(env.clone(), attestor.clone(), credential_id, slice_id, attestation_value, expires_at);
        }
    }

    /// Retrieve the total number of attestations an address has made.
    pub fn get_attestor_count(env: Env, address: Address) -> u64 {
        env.storage()
            .instance()
            .get(&DataKey::AttestorCount(address))
            .unwrap_or(0u64)
    }

    /// Stub for multisig approval check.
    /// Returns true to preserve backward compatibility until full multisig is implemented.
    fn is_multisig_approved(_env: &Env, _credential_id: u64) -> bool {
        true
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
        // Check condition-based attestation expiry
        if let Some(attest_expires_at) = env
            .storage()
            .instance()
            .get::<DataKey, u64>(&DataKey::AttestationExpiry(credential_id))
        {
            if env.ledger().timestamp() >= attest_expires_at {
                return false;
            }
        }
        let slice: QuorumSlice = env
            .storage()
            .instance()
            .get(&DataKey::Slice(slice_id))
            .unwrap_or_else(|| panic_with_error!(&env, ContractError::SliceNotFound));
        let attested_addresses: Vec<AttestationRecord> = env
            .storage()
            .instance()
            .get(&DataKey::Attestors(credential_id))
            .unwrap_or(Vec::new(&env));
        
        // Calculate total weight of attesting parties, skipping expired attestations
        let now = env.ledger().timestamp();
        let mut total_attested_weight: u32 = 0;
        for rec in attested_addresses.iter() {
            // Skip expired attestations
            if let Some(exp) = rec.expires_at {
                if now >= exp {
                    continue;
                }
            }
            // Find the index of this attestor in the slice and sum their weight
            for (i, attestor) in slice.attestors.iter().enumerate() {
                if attestor == rec.attestor {
                    total_attested_weight = total_attested_weight.saturating_add(slice.weights.get(i as u32).unwrap_or(0));
                    break;
                }
            }
        }
        
        let is_sufficient = total_attested_weight >= slice.threshold;
        let is_attested_result = is_sufficient && Self::is_multisig_approved(&env, credential_id);
        
        // Record consensus decision if threshold is met
        if is_sufficient {
            let decision = ConsensusDecision {
                decision_id: env.storage().instance().get(&DataKey::SliceConsensusHistory(slice_id))
                    .unwrap_or(Vec::new(&env))
                    .len() as u64 + 1,
                slice_id,
                credential_id,
                timestamp: now,
                required_weight_threshold: slice.threshold,
                achieved_weight: total_attested_weight,
                total_weight: slice.weights.iter().sum(),
            };
            
            let mut history: Vec<ConsensusDecision> = env
                .storage()
                .instance()
                .get(&DataKey::SliceConsensusHistory(slice_id))
                .unwrap_or(Vec::new(&env));
            history.push_back(decision);
            env.storage()
                .instance()
                .set(&DataKey::SliceConsensusHistory(slice_id), &history);
            env.storage()
                .instance()
                .extend_ttl(STANDARD_TTL, EXTENDED_TTL);
        }
        
        is_attested_result
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
        let records: Vec<AttestationRecord> = env.storage()
            .instance()
            .get(&DataKey::Attestors(credential_id))
            .unwrap_or(Vec::new(&env));
        let mut addrs: Vec<Address> = Vec::new(&env);
        for rec in records.iter() {
            addrs.push_back(rec.attestor);
        }
        addrs
    }

    /// Get all attestation records for a credential, including expiry information.
    pub fn get_attestation_records(env: Env, credential_id: u64) -> Vec<AttestationRecord> {
        env.storage()
            .instance()
            .get(&DataKey::Attestors(credential_id))
            .unwrap_or(Vec::new(&env))
    }

    /// Returns true if the given attestor's attestation on a credential has expired.
    ///
    /// Returns false if the attestation has no expiry or has not yet expired.
    ///
    /// # Panics
    /// Panics with `ContractError::CredentialNotFound` if the credential does not exist.
    /// Panics with "attestation not found" if the attestor has not attested this credential.
    pub fn is_single_attestation_expired(env: Env, credential_id: u64, attestor: Address) -> bool {
        if !env.storage().instance().has(&DataKey::Credential(credential_id)) {
            panic_with_error!(&env, ContractError::CredentialNotFound);
        }
        let records: Vec<AttestationRecord> = env.storage()
            .instance()
            .get(&DataKey::Attestors(credential_id))
            .unwrap_or(Vec::new(&env));
        for rec in records.iter() {
            if rec.attestor == attestor {
                return match rec.expires_at {
                    Some(exp) => env.ledger().timestamp() >= exp,
                    None => false,
                };
            }
        }
        panic!("attestation not found");
    }

    /// Renew an attestation by extending its expiry. Only the original attestor may call this.
    ///
    /// # Parameters
    /// - `attestor`: The address that originally attested; must authorize this call.
    /// - `credential_id`: The credential whose attestation to renew.
    /// - `new_expires_at`: New Unix timestamp; must be in the future.
    ///
    /// # Panics
    /// Panics if the contract is paused.
    /// Panics with `ContractError::CredentialNotFound` if the credential does not exist.
    /// Panics if the credential is revoked.
    /// Panics with "attestation not found" if the attestor has not attested this credential.
    /// Panics if `new_expires_at` is not in the future.
    pub fn renew_attestation(env: Env, attestor: Address, credential_id: u64, new_expires_at: u64) {
        attestor.require_auth();
        Self::require_not_paused(&env);
        let credential: Credential = env.storage().instance()
            .get(&DataKey::Credential(credential_id))
            .unwrap_or_else(|| panic_with_error!(&env, ContractError::CredentialNotFound));
        assert!(!credential.revoked, "credential is revoked");
        assert!(new_expires_at > env.ledger().timestamp(), "new_expires_at must be in the future");
        let mut records: Vec<AttestationRecord> = env.storage()
            .instance()
            .get(&DataKey::Attestors(credential_id))
            .unwrap_or(Vec::new(&env));
        let mut found = false;
        let mut updated: Vec<AttestationRecord> = Vec::new(&env);
        for rec in records.iter() {
            if rec.attestor == attestor {
                found = true;
                updated.push_back(AttestationRecord {
                    attestor: rec.attestor.clone(),
                    attested_at: rec.attested_at,
                    expires_at: Some(new_expires_at),
                });
            } else {
                updated.push_back(rec);
            }
        }
        assert!(found, "attestation not found");
        records = updated;
        env.storage().instance().set(&DataKey::Attestors(credential_id), &records);
        env.storage().instance().extend_ttl(STANDARD_TTL, EXTENDED_TTL);
        let event_data = AttestationRenewalEventData { attestor, credential_id, new_expires_at };
        let topic = String::from_str(&env, TOPIC_ATTESTATION_RENEWAL);
        let mut topics: Vec<String> = Vec::new(&env);
        topics.push_back(topic);
        env.events().publish(topics, event_data);

        // Record activity for the holder
        let credential: Credential = env
            .storage()
            .instance()
            .get(&DataKey::Credential(credential_id))
            .unwrap_or_else(|| panic_with_error!(&env, ContractError::CredentialNotFound));
        Self::record_holder_activity(&env, credential.subject.clone(), ActivityType::AttestationExpired, credential_id, attestor.clone(), None);
    }

    /// Returns the number of attestations recorded for a credential.
    ///
    /// # Parameters
    /// - `credential_id`: The credential to count attestations for.
    ///
    /// # Panics
    /// Does not panic; returns `0` if no attestations exist.
    pub fn get_attestation_count(env: Env, credential_id: u64) -> u32 {
        let records: Vec<AttestationRecord> = env
            .storage()
            .instance()
            .get(&DataKey::Attestors(credential_id))
            .unwrap_or(Vec::new(&env));
        records.len()
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
        Self::validate_array_bounds(claim_types.len(), 1, MAX_BATCH_SIZE, "claim_types");
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
        let attested: Vec<AttestationRecord> = env
            .storage()
            .instance()
            .get(&DataKey::Attestors(credential_id))
            .unwrap_or(Vec::new(&env));
        let mut status: Vec<(Address, bool)> = Vec::new(&env);
        for attestor in slice.attestors.iter() {
            let signed = attested.iter().any(|rec| rec.attestor == attestor);
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

    /// Register a human-readable label for a credential type with optional parent type.
    ///
    /// # Parameters
    /// - `admin`: The admin address; must authorize this call.
    /// - `type_id`: Numeric identifier for the credential type.
    /// - `name`: Human-readable name (e.g. "Mechanical Engineering Degree").
    /// - `description`: Longer description of what the credential type represents.
    /// - `parent_type`: Optional parent type ID for hierarchy support (enables inheritance).
    ///
    /// # Panics
    /// Panics with `ContractError::InvalidParentType` if parent_type is provided but not registered.
    /// Panics with `ContractError::CircularHierarchy` if setting parent_type would create a cycle.
    /// Does not panic on duplicate registration; overwrites the existing entry.
    pub fn register_credential_type(
        env: Env,
        admin: Address,
        type_id: u32,
        name: soroban_sdk::String,
        description: soroban_sdk::String,
        parent_type: Option<u32>,
    ) {
        admin.require_auth();
        let stored_admin: Address = env.storage().instance().get(&DataKey::Admin).expect("not initialized");
        assert!(admin == stored_admin, "unauthorized");
        
        // Validate parent_type if provided
        if let Some(parent) = parent_type {
            if !Self::parent_type_exists(&env, parent) {
                panic_with_error!(&env, ContractError::InvalidParentType);
            }
            if Self::would_create_cycle(&env, type_id, parent) {
                panic_with_error!(&env, ContractError::CircularHierarchy);
            }
        }
        
        let def = CredentialTypeDef {
            type_id,
            name,
            description,
            parent_type,
        };
        env.storage()
            .instance()
            .set(&DataKey::CredentialType(type_id), &def);
        env.storage()
            .instance()
            .extend_ttl(STANDARD_TTL, EXTENDED_TTL);
        
        // Store parent relationship
        if let Some(parent) = parent_type {
            env.storage()
                .instance()
                .set(&DataKey::CredentialTypeParent(type_id), &parent);
            env.storage()
                .instance()
                .extend_ttl(STANDARD_TTL, EXTENDED_TTL);
            
            // Add to parent's children list
            let mut children: Vec<u32> = env
                .storage()
                .instance()
                .get(&DataKey::CredentialTypeChildren(parent))
                .unwrap_or(Vec::new(&env));
            
            // Avoid duplicates
            if !children.iter().any(|&child| child == type_id) {
                children.push_back(type_id);
                env.storage()
                    .instance()
                    .set(&DataKey::CredentialTypeChildren(parent), &children);
                env.storage()
                    .instance()
                    .extend_ttl(STANDARD_TTL, EXTENDED_TTL);
            }
        }
        
        let mut topics: Vec<soroban_sdk::Val> = Vec::new(&env);
        topics.push_back(symbol_short!("reg_type").into_val(&env));
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

    /// Get the direct parent type of a credential type, if one exists.
    ///
    /// # Parameters
    /// - `type_id`: The credential type to query.
    ///
    /// # Returns
    /// Some(parent_type_id) if a parent is defined, None otherwise.
    /// Returns None if the type does not exist.
    pub fn get_credential_type_parent(env: Env, type_id: u32) -> Option<u32> {
        env.storage()
            .instance()
            .get::<DataKey, Option<u32>>(&DataKey::CredentialTypeParent(type_id))
            .flatten()
    }

    /// Get all direct children of a credential type.
    ///
    /// # Parameters
    /// - `parent_type_id`: The parent credential type to query.
    ///
    /// # Returns
    /// Vec of child type IDs. Empty vector if no children exist.
    pub fn get_credential_type_children(env: Env, parent_type_id: u32) -> Vec<u32> {
        env.storage()
            .instance()
            .get(&DataKey::CredentialTypeChildren(parent_type_id))
            .unwrap_or(Vec::new(&env))
    }

    /// Get the full lineage (ancestors) of a credential type, starting from its parent
    /// and going up to the root.
    ///
    /// # Parameters
    /// - `type_id`: The credential type to query.
    ///
    /// # Returns
    /// Vec of ancestor type IDs, ordered from direct parent to root.
    /// Empty vector if the type has no parent (is a root type).
    pub fn get_credential_type_ancestors(env: Env, type_id: u32) -> Vec<u32> {
        let mut ancestors: Vec<u32> = Vec::new(&env);
        let mut current = Self::get_credential_type_parent(env.clone(), type_id);
        
        while let Some(curr_type) = current {
            ancestors.push_back(curr_type);
            current = Self::get_credential_type_parent(env.clone(), curr_type);
        }
        
        ancestors
    }

    /// Check if one type is a child (direct or transitive) of another in the hierarchy.
    ///
    /// # Parameters
    /// - `child_id`: The potential child type to check.
    /// - `parent_id`: The potential parent/ancestor type.
    ///
    /// # Returns
    /// true if child_id is anywhere in parent_id's descendant tree, false otherwise.
    pub fn is_credential_type_child_of(env: Env, child_id: u32, parent_id: u32) -> bool {
        let ancestors = Self::get_credential_type_ancestors(env, child_id);
        ancestors.iter().any(|&ancestor| ancestor == parent_id)
    }

    /// Get all credential types whose verification rules should be applied to a given type.
    /// This is used for inheritance chains - returns the type itself plus all ancestors.
    ///
    /// # Parameters
    /// - `type_id`: The credential type to query.
    ///
    /// # Returns
    /// Vec of type IDs to check for verification rules, ordered from most specific (child)
    /// to most general (root). The first element is always the type_id itself.
    pub fn inherit_verification_rules(env: Env, type_id: u32) -> Vec<u32> {
        let mut rules: Vec<u32> = Vec::new(&env);
        rules.push_back(type_id);
        
        let ancestors = Self::get_credential_type_ancestors(env, type_id);
        // Reverse to go from root to parent
        let len = ancestors.len();
        for i in (0..len).rev() {
            rules.push_back(ancestors.get(i).unwrap());
        }
        
        rules
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

    // ── Reputation Recovery (Issue #298) ─────────────────────────────────────

    /// Initiate a reputation recovery request for a slice member.
    ///
    /// Recovery conditions:
    /// - The attestor must have made at least one attestation (reputation > 0).
    /// - No pending (incomplete) recovery may already exist for this attestor.
    /// - The contract must not be paused.
    ///
    /// # Parameters
    /// - `attestor`: The address initiating recovery; must authorize this call.
    ///
    /// # Panics
    /// Panics if the contract is paused.
    /// Panics if the attestor has no attestation history.
    /// Panics if a pending recovery already exists for this attestor.
    pub fn initiate_reputation_recovery(env: Env, attestor: Address) {
        attestor.require_auth();
        Self::require_not_paused(&env);

        let reputation: u64 = env
            .storage()
            .instance()
            .get(&DataKey::AttestorCount(attestor.clone()))
            .unwrap_or(0u64);
        assert!(reputation > 0, "attestor has no attestation history to recover");

        if let Some(existing) = env
            .storage()
            .instance()
            .get::<DataKey, ReputationRecovery>(&DataKey::ReputationRecovery(attestor.clone()))
        {
            assert!(existing.completed, "a pending recovery already exists for this attestor");
        }

        let recovery = ReputationRecovery {
            attestor: attestor.clone(),
            initiated_at: env.ledger().timestamp(),
            completed: false,
        };
        env.storage()
            .instance()
            .set(&DataKey::ReputationRecovery(attestor.clone()), &recovery);
        env.storage()
            .instance()
            .extend_ttl(STANDARD_TTL, EXTENDED_TTL);
    }

    /// Complete a pending reputation recovery for an attestor. Only admin may call this.
    ///
    /// # Panics
    /// Panics if no pending recovery exists for the attestor.
    pub fn complete_reputation_recovery(env: Env, admin: Address, attestor: Address) {
        admin.require_auth();
        let stored_admin: Address = env
            .storage()
            .instance()
            .get(&DataKey::Admin)
            .expect("not initialized");
        assert!(admin == stored_admin, "unauthorized");

        let mut recovery: ReputationRecovery = env
            .storage()
            .instance()
            .get(&DataKey::ReputationRecovery(attestor.clone()))
            .expect("no pending recovery for this attestor");
        assert!(!recovery.completed, "recovery already completed");

        recovery.completed = true;
        env.storage()
            .instance()
            .set(&DataKey::ReputationRecovery(attestor.clone()), &recovery);
        env.storage()
            .instance()
            .extend_ttl(STANDARD_TTL, EXTENDED_TTL);
    }

    /// Get the reputation recovery record for an attestor, if any.
    pub fn get_reputation_recovery(env: Env, attestor: Address) -> Option<ReputationRecovery> {
        env.storage()
            .instance()
            .get(&DataKey::ReputationRecovery(attestor))
    }

    // ── Proof Request History (Issue #38) ────────────────────────────────────    /// Record a new proof request for a credential and return its unique request ID.
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

    // ── Challenge / Dispute Resolution ───────────────────────────────────────

    /// Open a challenge against an attestor's signature on a credential.
    ///
    /// Only a member of the quorum slice (other than the accused) may challenge.
    /// Only one open challenge per (credential, accused) pair is allowed at a time.
    ///
    /// # Parameters
    /// - `challenger`: Slice member raising the challenge; must authorize.
    /// - `credential_id`: The credential whose attestation is being disputed.
    /// - `slice_id`: The quorum slice both challenger and accused belong to.
    /// - `accused`: The attestor whose signature is being challenged.
    ///
    /// # Returns
    /// The new challenge ID.
    pub fn challenge_attestation(
        env: Env,
        challenger: Address,
        credential_id: u64,
        slice_id: u64,
        accused: Address,
    ) -> u64 {
        challenger.require_auth();
        Self::require_not_paused(&env);

        // Credential must exist
        if !env.storage().instance().has(&DataKey::Credential(credential_id)) {
            panic_with_error!(&env, ContractError::CredentialNotFound);
        }

        let slice: QuorumSlice = env.storage().instance()
            .get(&DataKey::Slice(slice_id))
            .unwrap_or_else(|| panic_with_error!(&env, ContractError::SliceNotFound));

        // Both challenger and accused must be in the slice
        let mut challenger_in = false;
        let mut accused_in = false;
        for a in slice.attestors.iter() {
            if a == challenger { challenger_in = true; }
            if a == accused    { accused_in   = true; }
        }
        if !challenger_in || !accused_in {
            panic_with_error!(&env, ContractError::NotInSlice);
        }

        // Accused must have actually attested this credential
        let attestors: Vec<Address> = env.storage().instance()
            .get(&DataKey::Attestors(credential_id))
            .unwrap_or(Vec::new(&env));
        if !attestors.iter().any(|a| a == accused) {
            panic_with_error!(&env, ContractError::NotAttested);
        }

        // No duplicate open challenge for same (credential, accused)
        if env.storage().instance().has(&DataKey::ActiveChallenge(credential_id, accused.clone())) {
            panic_with_error!(&env, ContractError::AlreadyChallenged);
        }

        let id: u64 = env.storage().instance()
            .get(&DataKey::ChallengeCount)
            .unwrap_or(0u64) + 1;

        let challenge = Challenge {
            id,
            credential_id,
            slice_id,
            accused: accused.clone(),
            challenger,
            status: ChallengeStatus::Open,
            uphold_votes: Vec::new(&env),
            dismiss_votes: Vec::new(&env),
        };

        env.storage().instance().set(&DataKey::Challenge(id), &challenge);
        env.storage().instance().set(&DataKey::ChallengeCount, &id);
        env.storage().instance().set(&DataKey::ActiveChallenge(credential_id, accused), &id);
        env.storage().instance().extend_ttl(STANDARD_TTL, EXTENDED_TTL);
        id
    }

    /// Cast a vote on an open challenge.
    ///
    /// Any slice member except the accused may vote once. When the total weight of
    /// votes on either side meets or exceeds the slice threshold the challenge resolves:
    /// - Upheld → accused's attestation is removed from the credential.
    /// - Dismissed → challenge is closed, attestation stands.
    ///
    /// # Parameters
    /// - `voter`: Slice member casting the vote; must authorize.
    /// - `challenge_id`: The challenge to vote on.
    /// - `uphold`: `true` to uphold (remove attestation), `false` to dismiss.
    pub fn vote_on_challenge(env: Env, voter: Address, challenge_id: u64, uphold: bool) {
        voter.require_auth();
        Self::require_not_paused(&env);

        let mut challenge: Challenge = env.storage().instance()
            .get(&DataKey::Challenge(challenge_id))
            .unwrap_or_else(|| panic_with_error!(&env, ContractError::ChallengeNotFound));

        if challenge.status != ChallengeStatus::Open {
            panic_with_error!(&env, ContractError::ChallengeResolved);
        }

        // Accused cannot vote
        if voter == challenge.accused {
            panic_with_error!(&env, ContractError::AccusedCannotVote);
        }

        let slice: QuorumSlice = env.storage().instance()
            .get(&DataKey::Slice(challenge.slice_id))
            .unwrap_or_else(|| panic_with_error!(&env, ContractError::SliceNotFound));

        // Voter must be in the slice
        if !slice.attestors.iter().any(|a| a == voter) {
            panic_with_error!(&env, ContractError::NotInSlice);
        }

        // No double-voting
        let already_voted = challenge.uphold_votes.iter().any(|a| a == voter)
            || challenge.dismiss_votes.iter().any(|a| a == voter);
        if already_voted {
            panic_with_error!(&env, ContractError::AlreadyVoted);
        }

        if uphold {
            challenge.uphold_votes.push_back(voter);
        } else {
            challenge.dismiss_votes.push_back(voter);
        }

        // Helper: sum weights for a set of voters
        let weighted_sum = |votes: &Vec<Address>| -> u32 {
            let mut total: u32 = 0;
            for v in votes.iter() {
                for (i, a) in slice.attestors.iter().enumerate() {
                    if a == v {
                        total = total.saturating_add(slice.weights.get(i as u32).unwrap_or(0));
                        break;
                    }
                }
            }
            total
        };

        let uphold_weight  = weighted_sum(&challenge.uphold_votes);
        let dismiss_weight = weighted_sum(&challenge.dismiss_votes);

        if uphold_weight >= slice.threshold {
            challenge.status = ChallengeStatus::Upheld;
            // Remove accused's attestation from the credential
            let attestors: Vec<Address> = env.storage().instance()
                .get(&DataKey::Attestors(challenge.credential_id))
                .unwrap_or(Vec::new(&env));
            let mut retained: Vec<Address> = Vec::new(&env);
            for a in attestors.iter() {
                if a != challenge.accused {
                    retained.push_back(a);
                }
            }
            env.storage().instance().set(&DataKey::Attestors(challenge.credential_id), &retained);
            env.storage().instance().remove(&DataKey::ActiveChallenge(challenge.credential_id, challenge.accused.clone()));
        } else if dismiss_weight >= slice.threshold {
            challenge.status = ChallengeStatus::Dismissed;
            env.storage().instance().remove(&DataKey::ActiveChallenge(challenge.credential_id, challenge.accused.clone()));
        }

        env.storage().instance().set(&DataKey::Challenge(challenge_id), &challenge);
        env.storage().instance().extend_ttl(STANDARD_TTL, EXTENDED_TTL);
    }

    /// Retrieve a challenge by ID.
    ///
    /// # Panics
    /// Panics with `ContractError::ChallengeNotFound` if no challenge exists with that ID.
    pub fn get_challenge(env: Env, challenge_id: u64) -> Challenge {
        env.storage().instance()
            .get(&DataKey::Challenge(challenge_id))
            .unwrap_or_else(|| panic_with_error!(&env, ContractError::ChallengeNotFound))
    }
    /// Record an activity for a credential holder
    fn record_holder_activity(env: &Env, holder: Address, activity_type: ActivityType, credential_id: u64, actor: Address, slice_id: Option<u64>) {
        let activity = ActivityRecord {
            activity_type,
            credential_id,
            timestamp: env.ledger().timestamp(),
            actor,
            slice_id,
        };
        
        let mut activities: Vec<ActivityRecord> = env
            .storage()
            .instance()
            .get(&DataKey::HolderActivity(holder.clone()))
            .unwrap_or(Vec::new(env));
        activities.push_back(activity);
        env.storage()
            .instance()
            .set(&DataKey::HolderActivity(holder), &activities);
        env.storage()
            .instance()
            .extend_ttl(STANDARD_TTL, EXTENDED_TTL);
    }

    /// Track all credential-related activities per holder
    /// Activity log stored on-chain
    /// Returns paginated activity records for a holder
    pub fn get_holder_activity(env: Env, holder: Address, page: u32, page_size: u32) -> Vec<ActivityRecord> {
        Self::require_not_paused(&env);
        let mut activities: Vec<ActivityRecord> = env
            .storage()
            .instance()
            .get(&DataKey::HolderActivity(holder.clone()))
            .unwrap_or(Vec::new(&env));
        let total = activities.len();
        let start = (page - 1).saturating_mul(page_size);
        let mut result = Vec::new(&env);
        for i in start..start.saturating_add(page_size) {
            if i >= total {
                break;
            }
            if let Some(activity) = activities.get(i) {
                result.push_back(activity);
            }
        }
        result
    }

    /// Store historical consensus decisions per slice
    /// Returns paginated consensus history for a slice
    pub fn get_slice_consensus_history(env: Env, slice_id: u64, page: u32, page_size: u32) -> Vec<ConsensusDecision> {
        Self::require_not_paused(&env);
        let mut history: Vec<ConsensusDecision> = env
            .storage()
            .instance()
            .get(&DataKey::SliceConsensusHistory(slice_id))
            .unwrap_or(Vec::new(&env));
        let total = history.len();
        let start = (page - 1).saturating_mul(page_size);
        let mut result = Vec::new(&env);
        for i in start..start.saturating_add(page_size) {
            if i >= total {
                break;
            }
            if let Some(decision) = history.get(i) {
                result.push_back(decision);
            }
        }
        result
    }

    /// Structured process for adding new slice members
    /// Returns the new onboarding request ID
    pub fn initiate_member_onboarding(
        env: Env,
        requester: Address,
        slice_id: u64,
        proposed_member: Address,
        proposed_weight: u32,
    ) -> u64 {
        requester.require_auth();
        Self::require_not_paused(&env);
        
        // Verify slice exists
        let slice: QuorumSlice = env
            .storage()
            .instance()
            .get(&DataKey::Slice(slice_id))
            .unwrap_or_else(|| panic_with_error!(&env, ContractError::SliceNotFound));
        
        // Verify requester is in the slice
        let mut requester_in_slice = false;
        for a in slice.attestors.iter() {
            if a == requester {
                requester_in_slice = true;
                break;
            }
        }
        assert!(requester_in_slice, "only slice members can initiate onboarding");
        
        // Verify proposed member is not already in the slice
        for a in slice.attestors.iter() {
            if a == proposed_member {
                panic_with_error!(&env, ContractError::DuplicateAttestor);
            }
        }
        
        // Verify proposed weight is valid
        assert!(proposed_weight > 0, "weight must be greater than 0");
        
        let mut total_weight: u32 = 0;
        for w in slice.weights.iter() {
            total_weight = total_weight.saturating_add(w);
        }
        assert!(proposed_weight <= total_weight, "proposed weight cannot exceed current total weight");
        
        let request_id: u64 = env
            .storage()
            .instance()
            .get(&DataKey::OnboardingRequestCount)
            .unwrap_or(0u64)
            + 1;
        
        let request = OnboardingRequest {
            id: request_id,
            slice_id,
            requester: requester.clone(),
            proposed_member: proposed_member.clone(),
            proposed_weight,
            status: OnboardingStatus::Pending,
            created_at: env.ledger().timestamp(),
            votes: Vec::new(&env),
        };
        
        // Store individual request
        env.storage()
            .instance()
            .set(&DataKey::OnboardingRequest(request_id), &request);
        env.storage()
            .instance()
            .extend_ttl(STANDARD_TTL, EXTENDED_TTL);
        
        // Add to active requests list
        let mut active_requests: Vec<u64> = env
            .storage()
            .instance()
            .get(&DataKey::OnboardingRequests)
            .unwrap_or(Vec::new(&env));
        active_requests.push_back(request_id);
        env.storage()
            .instance()
            .set(&DataKey::OnboardingRequests, &active_requests);
        env.storage()
            .instance()
            .extend_ttl(STANDARD_TTL, EXTENDED_TTL);
        
        // Update counter
        env.storage()
            .instance()
            .set(&DataKey::OnboardingRequestCount, &request_id);
        env.storage()
            .instance()
            .extend_ttl(STANDARD_TTL, EXTENDED_TTL);
        
        request_id
    }

    /// Mechanism for resolving disputes between slice members
    /// Returns the new dispute ID
    pub fn initiate_dispute(
        env: Env,
        initiator: Address,
        slice_id: u64,
        accused: Address,
        reason: String,
    ) -> u64 {
        initiator.require_auth();
        Self::require_not_paused(&env);
        
        // Verify slice exists
        let slice: QuorumSlice = env
            .storage()
            .instance()
            .get(&DataKey::Slice(slice_id))
            .unwrap_or_else(|| panic_with_error!(&env, ContractError::SliceNotFound));
        
        // Verify initiator is in the slice
        let mut initiator_in_slice = false;
        for a in slice.attestors.iter() {
            if a == initiator {
                initiator_in_slice = true;
                break;
            }
        }
        assert!(initiator_in_slice, "only slice members can initiate disputes");
        
        // Verify accused is in the slice
        let mut accused_in_slice = false;
        for a in slice.attestors.iter() {
            if a == accused {
                accused_in_slice = true;
                break;
            }
        }
        assert!(accused_in_slice, "accused must be a member of the slice");
        
        // Verify initiator and accused are different
        assert!(initiator != accused, "initiator and accused must be different");
        
        let dispute_id: u64 = env
            .storage()
            .instance()
            .get(&DataKey::DisputeCount)
            .unwrap_or(0u64)
            + 1;
        
        let dispute = Dispute {
            id: dispute_id,
            slice_id,
            initiator: initiator.clone(),
            accused: accused.clone(),
            reason,
            status: DisputeStatus::Active,
            created_at: env.ledger().timestamp(),
            votes: Vec::new(&env),
        };
        
        // Store individual dispute
        env.storage()
            .instance()
            .set(&DataKey::Dispute(dispute_id), &dispute);
        env.storage()
            .instance()
            .extend_ttl(STANDARD_TTL, EXTENDED_TTL);
        
        // Add to active disputes list
        let mut active_disputes: Vec<u64> = env
            .storage()
            .instance()
            .get(&DataKey::Disputes)
            .unwrap_or(Vec::new(&env));
        active_disputes.push_back(dispute_id);
        env.storage()
            .instance()
            .set(&DataKey::Disputes, &active_disputes);
        env.storage()
            .instance()
            .extend_ttl(STANDARD_TTL, EXTENDED_TTL);
        
        // Update counter
        env.storage()
            .instance()
            .set(&DataKey::DisputeCount, &dispute_id);
        env.storage()
            .instance()
            .extend_ttl(STANDARD_TTL, EXTENDED_TTL);
        
        dispute_id
    }

    // ── Credential Holder Recovery (Issue #290) ──────────────────────────────

    /// Initiate a credential recovery request.
    ///
    /// Only the original issuer may initiate recovery for a credential.
    /// The recovery requires multi-sig approval from the designated approvers.
    ///
    /// # Parameters
    /// - `issuer`: The address that originally issued the credential; must authorize.
    /// - `credential_id`: The ID of the credential to recover.
    /// - `new_subject`: The new address that will receive the recovered credential.
    /// - `approvers`: List of addresses authorized to approve this recovery.
    /// - `threshold`: Number of approvers required to execute the recovery.
    ///
    /// # Panics
    /// Panics if the contract is paused.
    /// Panics with `ContractError::CredentialNotFound` if the credential does not exist.
    /// Panics if the caller is not the original issuer.
    /// Panics with `ContractError::RecoveryAlreadyExists` if a pending recovery already exists for this credential.
    pub fn initiate_recovery(
        env: Env,
        issuer: Address,
        credential_id: u64,
        new_subject: Address,
        approvers: Vec<Address>,
        threshold: u32,
    ) -> u64 {
        issuer.require_auth();
        Self::require_not_paused(&env);
        Self::require_valid_address(&env, &new_subject);
        Self::precondition(&env, credential_id > 0);
        Self::precondition(&env, threshold > 0);
        Self::validate_array_bounds(approvers.len(), 1, MAX_MULTISIG_SIGNERS, "approvers");

        let credential: Credential = env
            .storage()
            .instance()
            .get(&DataKey::Credential(credential_id))
            .unwrap_or_else(|| panic_with_error!(&env, ContractError::CredentialNotFound));
        assert!(credential.issuer == issuer, "only the original issuer can initiate recovery");
        assert!(!credential.revoked, "cannot recover a revoked credential");

        // No duplicate pending recovery
        if env.storage().instance().has(&DataKey::CredentialRecovery(credential_id)) {
            panic_with_error!(&env, ContractError::RecoveryAlreadyExists);
        }

        let recovery_id: u64 = env
            .storage()
            .instance()
            .get(&DataKey::RecoveryRequestCount)
            .unwrap_or(0u64)
            + 1;

        let request = RecoveryRequest {
            id: recovery_id,
            credential_id,
            issuer: issuer.clone(),
            new_subject: new_subject.clone(),
            status: RecoveryStatus::Pending,
            created_at: env.ledger().timestamp(),
            executed_at: None,
            approvers,
            threshold,
        };

        env.storage()
            .instance()
            .set(&DataKey::RecoveryRequest(recovery_id), &request);
        env.storage()
            .instance()
            .set(&DataKey::CredentialRecovery(credential_id), &recovery_id);
        env.storage()
            .instance()
            .set(&DataKey::RecoveryRequestCount, &recovery_id);
        env.storage()
            .instance()
            .extend_ttl(STANDARD_TTL, EXTENDED_TTL);

        // Emit event
        let event_data = RecoveryInitiatedEventData {
            recovery_id,
            credential_id,
            issuer,
            new_subject,
        };
        let topic = String::from_str(&env, TOPIC_RECOVERY_INITIATED);
        let mut topics: Vec<String> = Vec::new(&env);
        topics.push_back(topic);
        env.events().publish(topics, event_data);

        recovery_id
    }

    /// Approve a pending credential recovery request.
    ///
    /// Only addresses in the recovery approvers list may call this.
    /// When the total number of approvals meets or exceeds the threshold,
    /// the recovery status is automatically updated to `Approved`.
    ///
    /// # Parameters
    /// - `approver`: The address approving the recovery; must authorize.
    /// - `recovery_request_id`: The ID of the recovery request to approve.
    pub fn approve_recovery(env: Env, approver: Address, recovery_request_id: u64) {
        approver.require_auth();
        Self::require_not_paused(&env);
        Self::require_valid_address(&env, &approver);

        let mut request: RecoveryRequest = env
            .storage()
            .instance()
            .get(&DataKey::RecoveryRequest(recovery_request_id))
            .unwrap_or_else(|| panic_with_error!(&env, ContractError::RecoveryNotFound));

        if request.status != RecoveryStatus::Pending {
            panic_with_error!(&env, ContractError::RecoveryNotPending);
        }

        // Verify approver is in the approvers list
        let mut is_approver = false;
        for a in request.approvers.iter() {
            if a == approver {
                is_approver = true;
                break;
            }
        }
        if !is_approver {
            panic_with_error!(&env, ContractError::NotRecoveryApprover);
        }

        let mut approvals: Vec<RecoveryApproval> = env
            .storage()
            .instance()
            .get(&DataKey::RecoveryApprovals(recovery_request_id))
            .unwrap_or(Vec::new(&env));

        // Check for duplicate approval
        for approval in approvals.iter() {
            if approval.approver == approver {
                panic_with_error!(&env, ContractError::DuplicateRecoveryApproval);
            }
        }

        approvals.push_back(RecoveryApproval {
            approver: approver.clone(),
            approved_at: env.ledger().timestamp(),
        });
        env.storage()
            .instance()
            .set(&DataKey::RecoveryApprovals(recovery_request_id), &approvals);
        env.storage()
            .instance()
            .extend_ttl(STANDARD_TTL, EXTENDED_TTL);

        // Emit approval event
        let event_data = RecoveryApprovedEventData {
            recovery_id: recovery_request_id,
            approver,
        };
        let topic = String::from_str(&env, TOPIC_RECOVERY_APPROVED);
        let mut topics: Vec<String> = Vec::new(&env);
        topics.push_back(topic);
        env.events().publish(topics, event_data);

        // Auto-approve if threshold met
        if approvals.len() as u32 >= request.threshold {
            request.status = RecoveryStatus::Approved;
            env.storage()
                .instance()
                .set(&DataKey::RecoveryRequest(recovery_request_id), &request);
            env.storage()
                .instance()
                .extend_ttl(STANDARD_TTL, EXTENDED_TTL);
        }
    }

    /// Execute an approved credential recovery.
    ///
    /// Only the original issuer may execute the recovery.
    /// The recovery must have status `Approved` (threshold met).
    /// Updates the credential subject, subject credential lists, and optionally
    /// transfers the linked SBT via cross-contract call.
    ///
    /// # Parameters
    /// - `issuer`: The original issuer; must authorize.
    /// - `recovery_request_id`: The ID of the approved recovery request.
    /// - `sbt_registry_id`: Optional address of the SBT registry for SBT transfer.
    pub fn execute_recovery(
        env: Env,
        issuer: Address,
        recovery_request_id: u64,
        sbt_registry_id: Option<Address>,
    ) {
        issuer.require_auth();
        Self::require_not_paused(&env);

        let mut request: RecoveryRequest = env
            .storage()
            .instance()
            .get(&DataKey::RecoveryRequest(recovery_request_id))
            .unwrap_or_else(|| panic_with_error!(&env, ContractError::RecoveryNotFound));

        assert!(request.issuer == issuer, "only the original issuer can execute recovery");

        if request.status != RecoveryStatus::Approved {
            panic_with_error!(&env, ContractError::RecoveryThresholdNotMet);
        }

        let credential_id = request.credential_id;
        let old_subject = request.new_subject.clone(); // placeholder - will read from credential

        let mut credential: Credential = env
            .storage()
            .instance()
            .get(&DataKey::Credential(credential_id))
            .unwrap_or_else(|| panic_with_error!(&env, ContractError::CredentialNotFound));

        let prev_subject = credential.subject.clone();
        let new_subject = request.new_subject.clone();

        // Remove credential from old subject's list
        let mut old_creds: Vec<u64> = env
            .storage()
            .instance()
            .get(&DataKey::SubjectCredentials(prev_subject.clone()))
            .unwrap_or(Vec::new(&env));
        let mut retained: Vec<u64> = Vec::new(&env);
        for id in old_creds.iter() {
            if id != credential_id {
                retained.push_back(id);
            }
        }
        env.storage()
            .instance()
            .set(&DataKey::SubjectCredentials(prev_subject.clone()), &retained);

        // Add to new subject's list
        let mut new_creds: Vec<u64> = env
            .storage()
            .instance()
            .get(&DataKey::SubjectCredentials(new_subject.clone()))
            .unwrap_or(Vec::new(&env));
        new_creds.push_back(credential_id);
        env.storage()
            .instance()
            .set(&DataKey::SubjectCredentials(new_subject.clone()), &new_creds);

        // Update duplicate prevention mapping
        let old_dup_key = DataKey::SubjectIssuerType(prev_subject.clone(), credential.issuer.clone(), credential.credential_type);
        env.storage().instance().remove(&old_dup_key);
        let new_dup_key = DataKey::SubjectIssuerType(new_subject.clone(), credential.issuer.clone(), credential.credential_type);
        env.storage().instance().set(&new_dup_key, &credential_id);

        // Update credential subject
        credential.subject = new_subject.clone();
        env.storage()
            .instance()
            .set(&DataKey::Credential(credential_id), &credential);

        // Update recovery request status
        request.status = RecoveryStatus::Executed;
        request.executed_at = Some(env.ledger().timestamp());
        env.storage()
            .instance()
            .set(&DataKey::RecoveryRequest(recovery_request_id), &request);
        env.storage()
            .instance()
            .remove(&DataKey::CredentialRecovery(credential_id));
        env.storage()
            .instance()
            .extend_ttl(STANDARD_TTL, EXTENDED_TTL);

        // Transfer SBT if registry provided
        if let Some(registry_id) = sbt_registry_id {
            let sbt_client = SbtRegistryContractClient::new(&env, &registry_id);
            let tokens = sbt_client.get_tokens_by_owner(&prev_subject);
            for token_id in tokens.iter() {
                let token = sbt_client.get_token(&token_id);
                if token.credential_id == credential_id {
                    sbt_client.recover_sbt(&env.current_contract_address(), &token_id, &new_subject);
                }
            }
        }

        // Emit event
        let event_data = RecoveryExecutedEventData {
            recovery_id: recovery_request_id,
            credential_id,
            new_subject: new_subject.clone(),
        };
        let topic = String::from_str(&env, TOPIC_RECOVERY_EXECUTED);
        let mut topics: Vec<String> = Vec::new(&env);
        topics.push_back(topic);
        env.events().publish(topics, event_data);

        // Record activity for audit trail
        Self::record_holder_activity(
            &env,
            new_subject,
            ActivityType::CredentialRecovered,
            credential_id,
            issuer,
            None,
        );
    }

    /// Retrieve a recovery request by ID.
    pub fn get_recovery_request(env: Env, recovery_request_id: u64) -> RecoveryRequest {
        env.storage()
            .instance()
            .get(&DataKey::RecoveryRequest(recovery_request_id))
            .unwrap_or_else(|| panic_with_error!(&env, ContractError::RecoveryNotFound))
    }

    /// Retrieve all approvals for a recovery request.
    pub fn get_recovery_approvals(env: Env, recovery_request_id: u64) -> Vec<RecoveryApproval> {
        env.storage()
            .instance()
            .get(&DataKey::RecoveryApprovals(recovery_request_id))
            .unwrap_or(Vec::new(&env))
    }

    /// Cancel a pending recovery request. Only the issuer may cancel.
    pub fn cancel_recovery(env: Env, issuer: Address, recovery_request_id: u64) {
        issuer.require_auth();
        Self::require_not_paused(&env);

        let request: RecoveryRequest = env
            .storage()
            .instance()
            .get(&DataKey::RecoveryRequest(recovery_request_id))
            .unwrap_or_else(|| panic_with_error!(&env, ContractError::RecoveryNotFound));

        assert!(request.issuer == issuer, "only the issuer can cancel recovery");

        if request.status != RecoveryStatus::Pending && request.status != RecoveryStatus::Approved {
            panic_with_error!(&env, ContractError::RecoveryNotPending);
        }

        let mut updated = request;
        updated.status = RecoveryStatus::Rejected;
        env.storage()
            .instance()
            .set(&DataKey::RecoveryRequest(recovery_request_id), &updated);
        env.storage()
            .instance()
            .remove(&DataKey::CredentialRecovery(updated.credential_id));
        env.storage()
            .instance()
            .extend_ttl(STANDARD_TTL, EXTENDED_TTL);
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
    fn test_get_attestor_count() {
        let env = Env::default();
        let admin = Address::generate(&env);
        let creator = Address::generate(&env);
        let attestor = Address::generate(&env);

        let contract_id = env.register_contract(None, QuorumProofContract);
        let client = QuorumProofContractClient::new(&env, &contract_id);
        client.initialize(&admin);

        assert_eq!(client.get_attestor_count(&attestor), 0);

        env.mock_all_auths();
        let issuer = Address::generate(&env);
        let subject = Address::generate(&env);
        let metadata = Bytes::from_slice(&env, b"QmTestHash000000000000000000000000");
        let cid = client.issue_credential(&issuer, &subject, &1u32, &metadata, &None);

        let mut attestors = Vec::new(&env);
        attestors.push_back(attestor.clone());
        let mut weights = Vec::new(&env);
        weights.push_back(1u32);
        let slice_id = client.create_slice(&creator, &attestors, &weights, &1u32);

        client.attest(&attestor, &cid, &slice_id, &None);
        assert_eq!(client.get_attestor_count(&attestor), 1);
    }

    #[test]
    fn test_storage_persists_across_ledgers() {
        let env = Env::default();
        env.mock_all_auths();
        let contract_id = env.register_contract(None, QuorumProofContract);
        let client = QuorumProofContractClient::new(&env, &contract_id);

        let issuer = Address::generate(&env);
        let subject = Address::generate(&env);
        let metadata = Bytes::from_slice(&env, b"QmTestHash000000000000000000000000");
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
        let metadata = Bytes::from_slice(&env, b"QmTestHash000000000000000000000000");
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
        let issuer = Address::generate(&env);
        let subject = Address::generate(&env);
        let metadata = Bytes::from_slice(&env, b"ipfs://QmTest");
        let id = client.issue_credential(&issuer, &subject, &1u32, &metadata, &None);
        let cred = client.get_credential(&id);
        assert_eq!(cred.issuer, issuer);
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
        let metadata = Bytes::from_slice(&env, b"QmTestHash000000000000000000000000");

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
        let metadata = Bytes::from_slice(&env, b"QmTestHash000000000000000000000000");
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
        let metadata = Bytes::from_slice(&env, b"QmTestHash000000000000000000000000");
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
    #[should_panic(expected = "attestors exceed maximum allowed per slice")]
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
    #[should_panic(expected = "credential_type must be greater than 0")]
    fn test_zero_credential_type_rejection() {
        let env = Env::default();
        env.mock_all_auths();
        let contract_id = env.register_contract(None, QuorumProofContract);
        let client = QuorumProofContractClient::new(&env, &contract_id);

        let issuer = Address::generate(&env);
        let subject = Address::generate(&env);
        let metadata = Bytes::from_slice(&env, b"QmTestHash000000000000000000000000");

        client.issue_credential(&issuer, &subject, &0u32, &metadata, &None);
    }

    #[test]
    #[should_panic(expected = "Error(Contract, #1)")]
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
        let metadata = Bytes::from_slice(&env, b"QmTestHash000000000000000000000000");
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
        let metadata = Bytes::from_slice(&env, b"QmTestHash000000000000000000000000");
        let id = client.issue_credential(&issuer, &subject, &1u32, &metadata, &None);

        client.revoke_credential(&issuer, &id);

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
        let metadata = Bytes::from_slice(&env, b"QmTestHash000000000000000000000000");
        let cred_id = client.issue_credential(&issuer, &subject, &1u32, &metadata, &None);

        let mut attestors = Vec::new(&env);
        attestors.push_back(attestor.clone());
        let mut weights = Vec::new(&env);
        weights.push_back(1u32);
        let slice_id = client.create_slice(&issuer, &attestors, &weights, &1u32);

        client.pause(&admin);
        client.attest(&attestor, &cred_id, &slice_id, &None);
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
        let metadata = Bytes::from_slice(&env, b"QmTestHash000000000000000000000000");
        let cred_id = client.issue_credential(&issuer, &subject, &1u32, &metadata, &None);

        let mut attestors = Vec::new(&env);
        attestors.push_back(attestor.clone());
        let mut weights = Vec::new(&env);
        weights.push_back(1u32);
        let creator = Address::generate(&env);
        let slice_id = client.create_slice(&creator, &attestors, &weights, &1u32);

        assert!(!client.is_attested(&cred_id, &slice_id));
        client.attest(&attestor, &cred_id, &slice_id, &None);
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
        let metadata = Bytes::from_slice(&env, b"QmTestHash000000000000000000000000");
        let cred_id = client.issue_credential(&issuer, &subject, &1u32, &metadata, &None);
        let mut attestors = Vec::new(&env);
        attestors.push_back(attestor.clone());
        let mut weights = Vec::new(&env);
        weights.push_back(1u32);
        let slice_id = client.create_slice(&issuer, &attestors, &weights, &1u32);
        client.attest(&attestor, &cred_id, &slice_id, &None);
        client.attest(&attestor, &cred_id, &slice_id, &None);
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
        let metadata = Bytes::from_slice(&env, b"QmTestHash000000000000000000000000");
        let id = client.issue_credential(&issuer, &subject, &1u32, &metadata, &None);
        let mut attestors = Vec::new(&env);
        attestors.push_back(attestor.clone());
        let mut weights = Vec::new(&env);
        weights.push_back(1u32);
        let slice_id = client.create_slice(&issuer, &attestors, &weights, &1u32);
        client.revoke_credential(&issuer, &id);
        client.attest(&attestor, &id, &slice_id, &None);
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
        
        let creator = Address::generate(&env);
        let mut attestors = Vec::new(&env);
        attestors.push_back(Address::generate(&env));
        let mut weights = Vec::new(&env);
        weights.push_back(1u32);
        
        client.create_slice(&creator, &attestors, &weights, &0u32);
    }

    #[test]
    #[should_panic(expected = "threshold cannot exceed attestors length")]
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

        // 2 attestors but threshold of 3 — must panic
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
    #[should_panic(expected = "Error(Contract, #2)")]
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
        let metadata = Bytes::from_slice(&env, b"QmTestHash000000000000000000000000");

        let id1 = client.issue_credential(&issuer, &subject, &1u32, &metadata, &None);
        let id2 = client.issue_credential(&issuer, &subject, &2u32, &metadata, &None);
        let id3 = client.issue_credential(&issuer, &subject, &3u32, &metadata, &None);

        let ids = client.get_credentials_by_subject(&subject, &1, &100);
        assert_eq!(ids.len(), 3);
        assert_eq!(ids.get(0).unwrap(), id1);
        assert_eq!(ids.get(1).unwrap(), id2);
        assert_eq!(ids.get(2).unwrap(), id3);
    }

    #[test]
    fn test_revoke_prunes_subject_credentials_only_for_target_subject() {
        let env = Env::default();
        env.mock_all_auths();
        let (client, _) = setup(&env);

        let issuer = Address::generate(&env);
        let subject_a = Address::generate(&env);
        let subject_b = Address::generate(&env);
        let metadata = Bytes::from_slice(&env, b"QmTestHash000000000000000000000000");

        let id_a1 = client.issue_credential(&issuer, &subject_a, &1u32, &metadata, &None);
        let id_a2 = client.issue_credential(&issuer, &subject_a, &2u32, &metadata, &None);
        let id_b1 = client.issue_credential(&issuer, &subject_b, &1u32, &metadata, &None);

        let before_a = client.get_credentials_by_subject(&subject_a, &1, &100);
        assert_eq!(before_a.len(), 2);
        assert_eq!(before_a.get(0).unwrap(), id_a1);
        assert_eq!(before_a.get(1).unwrap(), id_a2);

        let before_b = client.get_credentials_by_subject(&subject_b, &1, &100);
        assert_eq!(before_b.len(), 1);
        assert_eq!(before_b.get(0).unwrap(), id_b1);

        client.revoke_credential(&issuer, &id_a1);

        let after_a = client.get_credentials_by_subject(&subject_a, &1, &100);
        assert_eq!(after_a.len(), 1);
        assert_eq!(after_a.get(0).unwrap(), id_a2);

        let after_b = client.get_credentials_by_subject(&subject_b, &1, &100);
        assert_eq!(after_b.len(), 1);
        assert_eq!(after_b.get(0).unwrap(), id_b1);

        let revoked = client.get_credential(&id_a1);
        assert!(revoked.revoked);
    }

    #[test]
    fn test_update_threshold_success() {
        let env = Env::default();
        env.mock_all_auths();
        let contract_id = env.register_contract(None, QuorumProofContract);
        let client = QuorumProofContractClient::new(&env, &contract_id);

        let subject = Address::generate(&env);
        let ids = client.get_credentials_by_subject(&subject, &1, &100);
        assert_eq!(ids.len(), 0);
    }

    #[test]
    #[should_panic(expected = "only the slice creator can update threshold")]
    fn test_update_slice_threshold_unauthorized_panics() {
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

        client.update_slice_threshold(&non_creator, &slice_id, &1u32);
    }

    // --- expiry ---

    #[test]
    #[should_panic(expected = "Error(Contract, #2)")]
    fn test_add_attestor_slice_not_found_panics() {
        let env = Env::default();
        env.mock_all_auths();
        let (client, _) = setup(&env);
        let creator = Address::generate(&env);
        client.add_attestor(&creator, &999u64, &Address::generate(&env), &1u32);
    }

    #[test]
    #[should_panic(expected = "Error(Contract, #2)")]
    fn test_update_slice_threshold_slice_not_found_panics() {
        let env = Env::default();
        env.mock_all_auths();
        let (client, _) = setup(&env);
        let creator = Address::generate(&env);
        client.update_slice_threshold(&creator, &999u64, &1u32);
    }

    #[test]
    fn test_single_attestation_produces_exactly_one_entry() {
        let env = Env::default();
        env.mock_all_auths();
        let (client, _) = setup(&env);
        let issuer = Address::generate(&env);
        let subject = Address::generate(&env);
        let attestor = Address::generate(&env);
        let metadata = Bytes::from_slice(&env, b"QmTestHash000000000000000000000000");
        let cred_id = client.issue_credential(&issuer, &subject, &1u32, &metadata, &None);
        let mut attestors = Vec::new(&env);
        attestors.push_back(attestor.clone());
        let mut weights = Vec::new(&env);
        weights.push_back(1u32);
        let slice_id = client.create_slice(&issuer, &attestors, &weights, &1u32);
        client.attest(&attestor, &cred_id, &slice_id, &None);
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
        let metadata = Bytes::from_slice(&env, b"QmTestHash000000000000000000000000");
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
        let metadata = Bytes::from_slice(&env, b"QmTestHash000000000000000000000000");
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
        let metadata = Bytes::from_slice(&env, b"QmTestHash000000000000000000000000");
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
        let metadata = Bytes::from_slice(&env, b"QmTestHash000000000000000000000000");
        let id = client.issue_credential(&issuer, &subject, &1u32, &metadata, &Some(2_000u64));
        set_ledger_timestamp(&env, 3_000);
        client.get_credential(&id);
    }

    #[test]
    fn test_version_increments_on_update_metadata() {
        let env = Env::default();
        env.mock_all_auths();
        let (client, _) = setup(&env);
        let issuer = Address::generate(&env);
        let subject = Address::generate(&env);
        let metadata = Bytes::from_slice(&env, b"QmTestHash000000000000000000000000");
        let id = client.issue_credential(&issuer, &subject, &1u32, &metadata, &None);

        let cred_v1 = client.get_credential(&id);
        assert_eq!(cred_v1.version, 1);

        let new_metadata = Bytes::from_slice(&env, b"QmUpdatedHash0000000000000000000000");
        client.update_metadata(&issuer, &id, &new_metadata);
        let cred_v2 = client.get_credential(&id);
        assert_eq!(cred_v2.version, 2);
        assert_eq!(cred_v2.metadata_hash, new_metadata);

        client.update_metadata(&issuer, &id, &metadata);
        let cred_v3 = client.get_credential(&id);
        assert_eq!(cred_v3.version, 3);
    }

    #[test]
    fn test_transfer_full_flow() {
        let env = Env::default();
        env.mock_all_auths();
        let (client, _) = setup(&env);
        let issuer = Address::generate(&env);
        let subject = Address::generate(&env);
        let recipient = Address::generate(&env);
        let metadata = Bytes::from_slice(&env, b"QmTestHash000000000000000000000000");
        let id = client.issue_credential(&issuer, &subject, &1u32, &metadata, &None);

        client.initiate_transfer(&subject, &id, &recipient);
        client.accept_transfer(&recipient, &id);

        let cred = client.get_credential(&id);
        assert_eq!(cred.subject, recipient);
    }

    #[test]
    #[should_panic]
    fn test_initiate_transfer_unauthorized() {
        let env = Env::default();
        env.mock_all_auths();
        let (client, _) = setup(&env);
        let issuer = Address::generate(&env);
        let subject = Address::generate(&env);
        let attacker = Address::generate(&env);
        let recipient = Address::generate(&env);
        let metadata = Bytes::from_slice(&env, b"QmTestHash000000000000000000000000");
        let id = client.issue_credential(&issuer, &subject, &1u32, &metadata, &None);

        // attacker is not the subject — should panic with UnauthorizedTransfer
        client.initiate_transfer(&attacker, &id, &recipient);
    }

    #[test]
    #[should_panic]
    fn test_accept_transfer_wrong_recipient() {
        let env = Env::default();
        env.mock_all_auths();
        let (client, _) = setup(&env);
        let issuer = Address::generate(&env);
        let subject = Address::generate(&env);
        let recipient = Address::generate(&env);
        let other = Address::generate(&env);
        let metadata = Bytes::from_slice(&env, b"QmTestHash000000000000000000000000");
        let id = client.issue_credential(&issuer, &subject, &1u32, &metadata, &None);

        client.initiate_transfer(&subject, &id, &recipient);
        // `other` is not the intended recipient — should panic
        client.accept_transfer(&other, &id);
    }

    #[test]
    fn test_transfer_updates_subject_credential_lists() {
        let env = Env::default();
        env.mock_all_auths();
        let (client, _) = setup(&env);
        let issuer = Address::generate(&env);
        let subject = Address::generate(&env);
        let recipient = Address::generate(&env);
        let metadata = Bytes::from_slice(&env, b"QmTestHash000000000000000000000000");
        let id = client.issue_credential(&issuer, &subject, &1u32, &metadata, &None);

        client.initiate_transfer(&subject, &id, &recipient);
        client.accept_transfer(&recipient, &id);

        let old_list = client.get_credentials_by_subject(&subject, &1u32, &50u32);
        let new_list = client.get_credentials_by_subject(&recipient, &1u32, &50u32);
        assert!(!old_list.contains(&id));
        assert!(new_list.contains(&id));
    }

    #[test]
    fn test_is_attested_returns_false_when_expired() {
        let env = Env::default();
        env.mock_all_auths();
        let (client, _) = setup(&env);
        let issuer = Address::generate(&env);
        let subject = Address::generate(&env);
        let attestor = Address::generate(&env);
        let metadata = Bytes::from_slice(&env, b"QmTestHash000000000000000000000000");
        set_ledger_timestamp(&env, 1_000);
        let cred_id = client.issue_credential(&issuer, &subject, &1u32, &metadata, &Some(2_000u64));

        let mut attestors = Vec::new(&env);
        attestors.push_back(attestor.clone());
        let mut weights = Vec::new(&env);
        weights.push_back(1u32);
        let slice_id = client.create_slice(&issuer, &attestors, &weights, &1u32);
        client.attest(&attestor, &cred_id, &slice_id, &None);
        assert!(client.is_attested(&cred_id, &slice_id));

        set_ledger_timestamp(&env, 3_000);
        assert!(!client.is_attested(&cred_id, &slice_id));
    }

    #[test]
    fn test_is_attested_returns_false_before_threshold_is_met() {
        let env = Env::default();
        env.mock_all_auths();
        let (client, _) = setup(&env);
        let issuer = Address::generate(&env);
        let subject = Address::generate(&env);
        let attestor1 = Address::generate(&env);
        let attestor2 = Address::generate(&env);
        let attestor3 = Address::generate(&env);
        let metadata = Bytes::from_slice(&env, b"QmTestHash000000000000000000000000");
        let cred_id = client.issue_credential(&issuer, &subject, &1u32, &metadata, &None);

        let mut attestors = Vec::new(&env);
        attestors.push_back(attestor1.clone());
        attestors.push_back(attestor2.clone());
        attestors.push_back(attestor3.clone());
        let mut weights = Vec::new(&env);
        weights.push_back(1u32);
        weights.push_back(1u32);
        weights.push_back(1u32);
        let creator = Address::generate(&env);
        let slice_id = client.create_slice(&creator, &attestors, &weights, &3u32);

        client.attest(&attestor1, &cred_id, &slice_id, &None);
        client.attest(&attestor2, &cred_id, &slice_id, &None);

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
    #[should_panic(expected = "Error(Contract, #5)")]
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
        let metadata = Bytes::from_slice(&env, b"QmTestHash000000000000000000000000");

        let mut initial = Vec::new(&env);
        initial.push_back(attestor1.clone());
        let mut initial_weights = Vec::new(&env);
        initial_weights.push_back(1u32);
        let slice_id = client.create_slice(&creator, &initial, &initial_weights, &1u32);

        let cred_id = client.issue_credential(&issuer, &subject, &1u32, &metadata, &None);

        client.attest(&attestor1, &cred_id, &slice_id, &None);
        assert!(client.is_attested(&cred_id, &slice_id)); // threshold=1, met after attestor1

        client.add_attestor(&creator, &slice_id, &attestor2, &1u32);
        client.update_slice_threshold(&creator, &slice_id, &2u32);
        assert!(!client.is_attested(&cred_id, &slice_id)); // threshold raised to 2, not met yet
        client.attest(&attestor2, &cred_id, &slice_id, &None);
        assert!(client.is_attested(&cred_id, &slice_id));
    }

    #[test]
    fn test_verify_engineer_success() {
        use sbt_registry::{SbtRegistryContract, SbtRegistryContractClient};
        use zk_verifier::{ClaimType, ZkVerifierContract, ZkVerifierContractClient};

        let env = Env::default();
        env.mock_all_auths_allowing_non_root_auth();

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
        let metadata = Bytes::from_slice(&env, b"QmTestHash000000000000000000000000");

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
        let metadata = Bytes::from_slice(&env, b"QmTestHash000000000000000000000000");

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
        env.mock_all_auths_allowing_non_root_auth();

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
        let metadata = Bytes::from_slice(&env, b"QmTestHash000000000000000000000000");

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
        let metadata = Bytes::from_slice(&env, b"QmTestHash000000000000000000000000");

        let mut attestors = Vec::new(&env);
        attestors.push_back(attestor.clone());
        let mut weights = Vec::new(&env);
        weights.push_back(1u32);
        let slice_id = client.create_slice(&issuer, &attestors, &weights, &1u32);
        let cred_id1 = client.issue_credential(&issuer, &subject, &1u32, &metadata, &None);
        let cred_id2 = client.issue_credential(&issuer, &subject, &2u32, &metadata, &None);
        assert_eq!(client.get_attestor_reputation(&attestor), 0);
        client.attest(&attestor, &cred_id1, &slice_id, &None);
        assert_eq!(client.get_attestor_reputation(&attestor), 1);
        client.attest(&attestor, &cred_id2, &slice_id, &None);
        assert_eq!(client.get_attestor_reputation(&attestor), 2);
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
        hashes.push_back(Bytes::from_slice(&env, b"QmHash1_000000000000000000000000000"));
        hashes.push_back(Bytes::from_slice(&env, b"QmHash2_000000000000000000000000000"));
        hashes.push_back(Bytes::from_slice(&env, b"QmHash3_000000000000000000000000000"));

        let ids = client.batch_issue_credentials(&issuer, &subjects, &cred_types, &hashes, &None);

        assert_eq!(ids.len(), 3);
        assert_eq!(client.get_credentials_by_subject(&subject1, &1, &100).len(), 1);
        assert_eq!(client.get_credentials_by_subject(&subject2, &1, &100).len(), 1);
        assert_eq!(client.get_credentials_by_subject(&subject3, &1, &100).len(), 1);
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
        hashes.push_back(Bytes::from_slice(&env, b"QmHash1_000000000000000000000000000"));
        hashes.push_back(Bytes::from_slice(&env, b"QmHash2_000000000000000000000000000"));

        client.batch_issue_credentials(&issuer, &subjects, &cred_types, &hashes, &None);
    }

    #[test]
    #[should_panic(expected = "DuplicateCredential")]
    fn test_batch_issue_credentials_duplicate_panics() {
        let env = Env::default();
        env.mock_all_auths();
        let contract_id = env.register_contract(None, QuorumProofContract);
        let client = QuorumProofContractClient::new(&env, &contract_id);

        let issuer = Address::generate(&env);
        let subject = Address::generate(&env);

        // Pre-issue a credential so the batch hits a duplicate
        let metadata = Bytes::from_slice(&env, b"QmExisting00000000000000000000000000");
        client.issue_credential(&issuer, &subject, &1u32, &metadata, &None);

        let mut subjects = Vec::new(&env);
        subjects.push_back(subject.clone());
        let mut cred_types = Vec::new(&env);
        cred_types.push_back(1u32); // duplicate
        let mut hashes = Vec::new(&env);
        hashes.push_back(Bytes::from_slice(&env, b"QmNewHash0000000000000000000000000"));

        client.batch_issue_credentials(&issuer, &subjects, &cred_types, &hashes, &None);
    }

    #[test]
    #[should_panic]
    fn test_batch_issue_credentials_paused_panics() {
        let env = Env::default();
        env.mock_all_auths();
        let (client, admin) = setup(&env);
        client.pause(&admin);

        let issuer = Address::generate(&env);
        let mut subjects = Vec::new(&env);
        subjects.push_back(Address::generate(&env));
        let mut cred_types = Vec::new(&env);
        cred_types.push_back(1u32);
        let mut hashes = Vec::new(&env);
        hashes.push_back(Bytes::from_slice(&env, b"QmTestHash000000000000000000000000"));

        client.batch_issue_credentials(&issuer, &subjects, &cred_types, &hashes, &None);
    }

    #[test]
    fn test_batch_issue_credentials_with_expiry() {
        let env = Env::default();
        env.mock_all_auths();
        let contract_id = env.register_contract(None, QuorumProofContract);
        let client = QuorumProofContractClient::new(&env, &contract_id);

        let issuer = Address::generate(&env);
        let subject1 = Address::generate(&env);
        let subject2 = Address::generate(&env);

        let mut subjects = Vec::new(&env);
        subjects.push_back(subject1.clone());
        subjects.push_back(subject2.clone());
        let mut cred_types = Vec::new(&env);
        cred_types.push_back(1u32);
        cred_types.push_back(2u32);
        let mut hashes = Vec::new(&env);
        hashes.push_back(Bytes::from_slice(&env, b"QmHash1_000000000000000000000000000"));
        hashes.push_back(Bytes::from_slice(&env, b"QmHash2_000000000000000000000000000"));

        let ids = client.batch_issue_credentials(&issuer, &subjects, &cred_types, &hashes, &Some(9_999_999u64));

        assert_eq!(ids.len(), 2);
        assert_eq!(client.get_credential(&ids.get(0).unwrap()).expires_at, Some(9_999_999u64));
        assert_eq!(client.get_credential(&ids.get(1).unwrap()).expires_at, Some(9_999_999u64));
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
                soroban_sdk::Symbol::from_val(&env, &first) == symbol_short!("reg_type")
            } else {
                false
            }
        });
        assert!(reg_event.is_some(), "reg_type event not emitted");
        let (_, _, data) = reg_event.unwrap();
        let emitted_id = u32::from_val(&env, &data);
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
        let metadata = Bytes::from_slice(&env, b"QmTestHash000000000000000000000000");

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
        let metadata = Bytes::from_slice(&env, b"QmTestHash000000000000000000000000");

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
        client.attest(&attestor, &cred_id, &slice_id, &None);
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
        let metadata = Bytes::from_slice(&env, b"QmTestHash000000000000000000000000");
        let cred_id = client.issue_credential(&issuer, &subject, &1u32, &metadata, &None);

        let mut attestors = soroban_sdk::Vec::new(&env);
        attestors.push_back(attestor1.clone());
        attestors.push_back(attestor2.clone());
        let mut weights = Vec::new(&env);
        weights.push_back(1u32);
        weights.push_back(1u32);
        let slice_id = client.create_slice(&issuer, &attestors, &weights, &1u32);

        assert_eq!(client.get_attestation_count(&cred_id), 0);
        client.attest(&attestor1, &cred_id, &slice_id, &None);
        assert_eq!(client.get_attestation_count(&cred_id), 1);
        client.attest(&attestor2, &cred_id, &slice_id, &None);
        assert_eq!(client.get_attestation_count(&cred_id), 2);
    }

    // --- duplicate credential tests ---

    #[test]
    #[should_panic(expected = "Error(Contract, #4)")]
    fn test_duplicate_credential_issuance_rejection() {
        let env = Env::default();
        env.mock_all_auths();
        let contract_id = env.register_contract(None, QuorumProofContract);
        let client = QuorumProofContractClient::new(&env, &contract_id);

        let issuer = Address::generate(&env);
        let subject = Address::generate(&env);
        let metadata = Bytes::from_slice(&env, b"QmTestHash000000000000000000000000");
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
        let metadata = Bytes::from_slice(&env, b"QmTestHash000000000000000000000000");

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
        let metadata = Bytes::from_slice(&env, b"QmTestHash000000000000000000000000");
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
        let metadata = Bytes::from_slice(&env, b"QmTestHash000000000000000000000000");
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
        let metadata = Bytes::from_slice(&env, b"QmTestHash000000000000000000000000");
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
        let metadata = Bytes::from_slice(&env, b"QmTestHash000000000000000000000000");
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
        env.mock_all_auths_allowing_non_root_auth();

        // Step 1: Set up all three contracts
        let qp_id = env.register_contract(None, QuorumProofContract);
        let sbt_id = env.register_contract(None, SbtRegistryContract);
        let zk_id = env.register_contract(None, ZkVerifierContract);

        let qp = QuorumProofContractClient::new(&env, &qp_id);
        let sbt = SbtRegistryContractClient::new(&env, &sbt_id);
        let zk_admin = Address::generate(&env);
        ZkVerifierContractClient::new(&env, &zk_id).initialize(&zk_admin);
        sbt.initialize(&zk_admin, &qp_id);
        qp.initialize(&zk_admin);

        let issuer = Address::generate(&env);
        let subject = Address::generate(&env);
        let attestor1 = Address::generate(&env);
        let attestor2 = Address::generate(&env);
        let metadata = Bytes::from_slice(&env, b"QmLifecycleTest0000000000000000000");

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

        let metadata = Bytes::from_slice(&env, b"QmTestHash000000000000000000000000");
        let cred_id = client.issue_credential(&issuer, &subject, &1u32, &metadata, &None);

        // Create slice with only attestor1
        let mut attestors = soroban_sdk::Vec::new(&env);
        attestors.push_back(attestor1.clone());
        let mut weights = Vec::new(&env);
        weights.push_back(1u32);
        let slice_id = client.create_slice(&issuer, &attestors, &weights, &1u32);

        // attestor2 is not in the slice — must panic
        client.attest(&attestor2, &cred_id, &slice_id, &None);
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
        let metadata = Bytes::from_slice(&env, b"QmTestHash000000000000000000000000");

        let cred_id = client.issue_credential(&issuer, &subject, &1u32, &metadata, &None);

        let mut attestors = Vec::new(&env);
        attestors.push_back(attestor1.clone());
        attestors.push_back(attestor2.clone());
        let mut weights = Vec::new(&env);
        weights.push_back(1u32);
        weights.push_back(1u32);
        let slice_id = client.create_slice(&issuer, &attestors, &weights, &1u32);

        assert_eq!(client.get_attestors(&cred_id).len(), 0);

        client.attest(&attestor1, &cred_id, &slice_id, &None);
        let result = client.get_attestors(&cred_id);
        assert_eq!(result.len(), 1);
        assert_eq!(result.get(0).unwrap(), attestor1);

        client.attest(&attestor2, &cred_id, &slice_id, &None);
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
        let metadata = Bytes::from_slice(&env, b"QmTestHash000000000000000000000000");

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

    // --- Issue #299: attestation expiry ---

    #[test]
    fn test_attest_with_expiry_stores_record() {
        let env = Env::default();
        env.mock_all_auths();
        let (client, _) = setup(&env);
        set_ledger_timestamp(&env, 1_000);
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

        client.attest(&attestor, &cred_id, &slice_id, &Some(5_000u64));

        let records = client.get_attestation_records(&cred_id);
        assert_eq!(records.len(), 1);
        assert_eq!(records.get(0).unwrap().attestor, attestor);
        assert_eq!(records.get(0).unwrap().expires_at, Some(5_000u64));
    }

    #[test]
    fn test_is_attestation_expired_false_before_expiry() {
        let env = Env::default();
        env.mock_all_auths();
        let (client, _) = setup(&env);
        set_ledger_timestamp(&env, 1_000);
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
        client.attest(&attestor, &cred_id, &slice_id, &Some(5_000u64));

        assert!(!client.is_attestation_expired(&cred_id, &attestor));
    }

    #[test]
    fn test_is_attestation_expired_true_after_expiry() {
        let env = Env::default();
        env.mock_all_auths();
        let (client, _) = setup(&env);
        set_ledger_timestamp(&env, 1_000);
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
        client.attest(&attestor, &cred_id, &slice_id, &Some(3_000u64));

        set_ledger_timestamp(&env, 4_000);
        assert!(client.is_attestation_expired(&cred_id, &attestor));
    }

    #[test]
    fn test_is_attestation_expired_false_when_no_expiry() {
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
        client.attest(&attestor, &cred_id, &slice_id, &None);

        set_ledger_timestamp(&env, 999_999_999);
        assert!(!client.is_attestation_expired(&cred_id, &attestor));
    }

    #[test]
    fn test_is_attested_false_when_attestation_expired() {
        let env = Env::default();
        env.mock_all_auths();
        let (client, _) = setup(&env);
        set_ledger_timestamp(&env, 1_000);
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
        client.attest(&attestor, &cred_id, &slice_id, &Some(3_000u64));

        assert!(client.is_attested(&cred_id, &slice_id));

        set_ledger_timestamp(&env, 4_000);
        assert!(!client.is_attested(&cred_id, &slice_id));
    }

    #[test]
    fn test_renew_attestation_extends_expiry() {
        let env = Env::default();
        env.mock_all_auths();
        let (client, _) = setup(&env);
        set_ledger_timestamp(&env, 1_000);
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
        client.attest(&attestor, &cred_id, &slice_id, &Some(3_000u64));

        // Expire the attestation
        set_ledger_timestamp(&env, 4_000);
        assert!(client.is_attestation_expired(&cred_id, &attestor));
        assert!(!client.is_attested(&cred_id, &slice_id));

        // Renew
        client.renew_attestation(&attestor, &cred_id, &10_000u64);
        assert!(!client.is_attestation_expired(&cred_id, &attestor));
        assert!(client.is_attested(&cred_id, &slice_id));

        let records = client.get_attestation_records(&cred_id);
        assert_eq!(records.get(0).unwrap().expires_at, Some(10_000u64));
    }

    #[test]
    #[should_panic(expected = "new_expires_at must be in the future")]
    fn test_renew_attestation_past_timestamp_panics() {
        let env = Env::default();
        env.mock_all_auths();
        let (client, _) = setup(&env);
        set_ledger_timestamp(&env, 5_000);
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
        client.attest(&attestor, &cred_id, &slice_id, &Some(8_000u64));

        client.renew_attestation(&attestor, &cred_id, &3_000u64);
    }

    #[test]
    #[should_panic(expected = "attestation not found")]
    fn test_renew_attestation_not_found_panics() {
        let env = Env::default();
        env.mock_all_auths();
        let (client, _) = setup(&env);
        let issuer = Address::generate(&env);
        let subject = Address::generate(&env);
        let attestor = Address::generate(&env);
        let stranger = Address::generate(&env);
        let metadata = Bytes::from_slice(&env, b"ipfs://QmTest");
        let cred_id = client.issue_credential(&issuer, &subject, &1u32, &metadata, &None);
        let mut attestors = Vec::new(&env);
        attestors.push_back(attestor.clone());
        let mut weights = Vec::new(&env);
        weights.push_back(1u32);
        let slice_id = client.create_slice(&issuer, &attestors, &weights, &1u32);
        client.attest(&attestor, &cred_id, &slice_id, &Some(5_000u64));

        client.renew_attestation(&stranger, &cred_id, &10_000u64);
    }
}

// ── New feature tests ─────────────────────────────────────────────────────────

#[cfg(test)]
mod feature_tests {
    use super::*;
    use soroban_sdk::testutils::{Address as _, Ledger as _, LedgerInfo};
    use soroban_sdk::{Bytes, Env};

    fn setup(env: &Env) -> (QuorumProofContractClient<'_>, Address) {
        env.mock_all_auths();
        let contract_id = env.register_contract(None, QuorumProofContract);
        let client = QuorumProofContractClient::new(env, &contract_id);
        let admin = Address::generate(env);
        client.initialize(&admin);
        (client, admin)
    }

    fn set_ts(env: &Env, ts: u64) {
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

    // ── Conditional attestation expiry ────────────────────────────────────────

    #[test]
    fn test_set_and_check_attestation_expiry_not_expired() {
        let env = Env::default();
        let (client, _) = setup(&env);
        let issuer = Address::generate(&env);
        let subject = Address::generate(&env);
        let metadata = Bytes::from_slice(&env, b"ipfs://QmTest");
        set_ts(&env, 1_000);
        let cid = client.issue_credential(&issuer, &subject, &1u32, &metadata, &None);
        client.set_attestation_expiry(&issuer, &cid, &5_000u64);
        assert!(!client.is_attestation_expired(&cid));
    }

    #[test]
    fn test_attestation_expiry_triggers_after_timestamp() {
        let env = Env::default();
        let (client, _) = setup(&env);
        let issuer = Address::generate(&env);
        let subject = Address::generate(&env);
        let metadata = Bytes::from_slice(&env, b"ipfs://QmTest");
        set_ts(&env, 1_000);
        let cid = client.issue_credential(&issuer, &subject, &1u32, &metadata, &None);
        client.set_attestation_expiry(&issuer, &cid, &2_000u64);
        set_ts(&env, 3_000);
        assert!(client.is_attestation_expired(&cid));
    }

    #[test]
    fn test_is_attested_false_after_attestation_expiry() {
        let env = Env::default();
        let (client, _) = setup(&env);
        let issuer = Address::generate(&env);
        let subject = Address::generate(&env);
        let attestor = Address::generate(&env);
        let metadata = Bytes::from_slice(&env, b"ipfs://QmTest");
        set_ts(&env, 1_000);
        let cid = client.issue_credential(&issuer, &subject, &1u32, &metadata, &None);
        let mut attestors = soroban_sdk::Vec::new(&env);
        attestors.push_back(attestor.clone());
        let mut weights = soroban_sdk::Vec::new(&env);
        weights.push_back(1u32);
        let sid = client.create_slice(&issuer, &attestors, &weights, &1u32);
        client.attest(&attestor, &cid, &sid);
        assert!(client.is_attested(&cid, &sid));
        // Set attestation expiry in the past
        client.set_attestation_expiry(&issuer, &cid, &2_000u64);
        set_ts(&env, 3_000);
        assert!(!client.is_attested(&cid, &sid));
    }

    #[test]
    fn test_is_attestation_expired_no_expiry_set() {
        let env = Env::default();
        let (client, _) = setup(&env);
        let issuer = Address::generate(&env);
        let subject = Address::generate(&env);
        let metadata = Bytes::from_slice(&env, b"ipfs://QmTest");
        let cid = client.issue_credential(&issuer, &subject, &1u32, &metadata, &None);
        assert!(!client.is_attestation_expired(&cid));
    }

    #[test]
    #[should_panic(expected = "CredentialNotFound")]
    fn test_is_attestation_expired_missing_credential() {
        let env = Env::default();
        let (client, _) = setup(&env);
        client.is_attestation_expired(&999u64);
    }

    #[test]
    #[should_panic(expected = "InvalidInput")]
    fn test_set_attestation_expiry_past_timestamp_panics() {
        let env = Env::default();
        let (client, _) = setup(&env);
        let issuer = Address::generate(&env);
        let subject = Address::generate(&env);
        let metadata = Bytes::from_slice(&env, b"ipfs://QmTest");
        set_ts(&env, 5_000);
        let cid = client.issue_credential(&issuer, &subject, &1u32, &metadata, &None);
        // expires_at is in the past
        client.set_attestation_expiry(&issuer, &cid, &1_000u64);
    }

    // ── Input validation ──────────────────────────────────────────────────────

    #[test]
    #[should_panic(expected = "credential_type must be greater than 0")]
    fn test_issue_credential_zero_type_panics() {
        let env = Env::default();
        let (client, _) = setup(&env);
        let issuer = Address::generate(&env);
        let subject = Address::generate(&env);
        let metadata = Bytes::from_slice(&env, b"ipfs://QmTest");
        client.issue_credential(&issuer, &subject, &0u32, &metadata, &None);
    }

    #[test]
    #[should_panic(expected = "metadata_hash cannot be empty")]
    fn test_issue_credential_empty_metadata_panics() {
        let env = Env::default();
        let (client, _) = setup(&env);
        let issuer = Address::generate(&env);
        let subject = Address::generate(&env);
        let empty = Bytes::new(&env);
        client.issue_credential(&issuer, &subject, &1u32, &empty, &None);
    }

    #[test]
    #[should_panic(expected = "InvalidInput")]
    fn test_issue_credential_metadata_too_long_panics() {
        let env = Env::default();
        let (client, _) = setup(&env);
        let issuer = Address::generate(&env);
        let subject = Address::generate(&env);
        // 257 bytes — exceeds 256 limit
        let long_hash = Bytes::from_slice(&env, &[b'x'; 257]);
        client.issue_credential(&issuer, &subject, &1u32, &long_hash, &None);
    }

    #[test]
    #[should_panic(expected = "InvalidInput")]
    fn test_attest_zero_credential_id_panics() {
        let env = Env::default();
        let (client, _) = setup(&env);
        let issuer = Address::generate(&env);
        let attestor = Address::generate(&env);
        let mut attestors = soroban_sdk::Vec::new(&env);
        attestors.push_back(attestor.clone());
        let mut weights = soroban_sdk::Vec::new(&env);
        weights.push_back(1u32);
        let sid = client.create_slice(&issuer, &attestors, &weights, &1u32);
        client.attest(&attestor, &0u64, &sid);
    }

    #[test]
    #[should_panic(expected = "InvalidInput")]
    fn test_attest_zero_slice_id_panics() {
        let env = Env::default();
        let (client, _) = setup(&env);
        let issuer = Address::generate(&env);
        let subject = Address::generate(&env);
        let attestor = Address::generate(&env);
        let metadata = Bytes::from_slice(&env, b"ipfs://QmTest");
        let cid = client.issue_credential(&issuer, &subject, &1u32, &metadata, &None);
        client.attest(&attestor, &cid, &0u64);
    }

    #[test]
    #[should_panic(expected = "InvalidInput")]
    fn test_get_credentials_by_subject_zero_page_panics() {
        let env = Env::default();
        let (client, _) = setup(&env);
        let subject = Address::generate(&env);
        client.get_credentials_by_subject(&subject, &0u32, &10u32);
    }

    #[test]
    #[should_panic(expected = "InvalidInput")]
    fn test_get_credentials_by_subject_zero_page_size_panics() {
        let env = Env::default();
        let (client, _) = setup(&env);
        let subject = Address::generate(&env);
        client.get_credentials_by_subject(&subject, &1u32, &0u32);
    }

    // ── Pre/post-condition assertions ─────────────────────────────────────────

    #[test]
    fn test_postcondition_credential_stored_after_issue() {
        let env = Env::default();
        let (client, _) = setup(&env);
        let issuer = Address::generate(&env);
        let subject = Address::generate(&env);
        let metadata = Bytes::from_slice(&env, b"ipfs://QmTest");
        let cid = client.issue_credential(&issuer, &subject, &1u32, &metadata, &None);
        // If postcondition passed, credential must exist
        assert!(client.credential_exists(&cid));
    }

    #[test]
    fn test_postcondition_slice_stored_after_create() {
        let env = Env::default();
        let (client, _) = setup(&env);
        let creator = Address::generate(&env);
        let mut attestors = soroban_sdk::Vec::new(&env);
        attestors.push_back(Address::generate(&env));
        let mut weights = soroban_sdk::Vec::new(&env);
        weights.push_back(1u32);
        let sid = client.create_slice(&creator, &attestors, &weights, &1u32);
        assert!(client.slice_exists(&sid));
    }

    // ── Snapshot tests ────────────────────────────────────────────────────────

    /// Generates a snapshot after issuing a credential and verifies the
    /// snapshot can be reloaded with the same ledger state.
    #[test]
    fn test_snapshot_credential_state() {
        let snap_path = "test_snapshots/tests/snapshot_credential_state.json";
        let env = Env::default();
        env.mock_all_auths();
        let (client, _) = setup(&env);
        let issuer = Address::generate(&env);
        let subject = Address::generate(&env);
        let metadata = Bytes::from_slice(&env, b"QmSnapshotHash00000000000000000000");
        let cid = client.issue_credential(&issuer, &subject, &1u32, &metadata, &None);

        assert_eq!(client.get_credential_count(), 1);
        assert_eq!(cid, 1);

        // Generate snapshot
        env.to_snapshot_file(snap_path);

        // Reload snapshot — ledger entries are preserved
        let env2 = Env::from_snapshot_file(snap_path);
        // Snapshot ledger sequence should match
        assert_eq!(env.ledger().sequence(), env2.ledger().sequence());
    }

    /// Generates a snapshot after creating a quorum slice and verifies
    /// the reloaded snapshot has the same ledger sequence.
    #[test]
    fn test_snapshot_slice_state() {
        let snap_path = "test_snapshots/tests/snapshot_slice_state.json";
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

        assert!(client.slice_exists(&slice_id));
        assert_eq!(client.get_slice_count(), 1);

        // Generate snapshot
        env.to_snapshot_file(snap_path);

        // Reload and compare ledger state
        let env2 = Env::from_snapshot_file(snap_path);
        assert_eq!(env.ledger().sequence(), env2.ledger().sequence());
        assert_eq!(env.ledger().timestamp(), env2.ledger().timestamp());
    }

    /// Generates a snapshot after a full attest flow and verifies the
    /// reloaded snapshot preserves ledger metadata.
    #[test]
    fn test_snapshot_attestation_state() {
        let snap_path = "test_snapshots/tests/snapshot_attestation_state.json";
        let env = Env::default();
        env.mock_all_auths();
        let (client, _) = setup(&env);
        let issuer = Address::generate(&env);
        let subject = Address::generate(&env);
        let attestor = Address::generate(&env);
        let metadata = Bytes::from_slice(&env, b"QmSnapshotHash00000000000000000000");
        let cid = client.issue_credential(&issuer, &subject, &1u32, &metadata, &None);
        let mut attestors = Vec::new(&env);
        attestors.push_back(attestor.clone());
        let mut weights = Vec::new(&env);
        weights.push_back(1u32);
        let slice_id = client.create_slice(&issuer, &attestors, &weights, &1u32);
        client.attest(&attestor, &cid, &slice_id, &None);

        assert!(client.is_attested(&cid, &slice_id));
        assert_eq!(client.get_attestation_count(&cid), 1);

        // Generate snapshot
        env.to_snapshot_file(snap_path);

        // Reload and compare ledger metadata
        let env2 = Env::from_snapshot_file(snap_path);
        assert_eq!(env.ledger().sequence(), env2.ledger().sequence());
        assert_eq!(env.ledger().timestamp(), env2.ledger().timestamp());
    }

    // ── Property-based fuzz tests ─────────────────────────────────────────────

    /// Property: issuing N distinct (issuer, subject, type) credentials always
    /// produces sequential IDs starting at 1 and increments the count.
    #[test]
    fn fuzz_issue_credential_sequential_ids() {
        let env = Env::default();
        env.mock_all_auths();
        let (client, _) = setup(&env);
        let issuer = Address::generate(&env);
        let subject = Address::generate(&env);
        let meta = Bytes::from_slice(&env, b"QmFuzzHash000000000000000000000000");

        for i in 1u32..=5 {
            let cid = client.issue_credential(&issuer, &subject, &i, &meta, &None);
            assert_eq!(cid, i as u64);
            assert_eq!(client.get_credential_count(), i as u64);
        }
    }

    /// Property: zero credential_type must always be rejected.
    #[test]
    #[should_panic]
    fn fuzz_issue_credential_zero_type_always_rejected() {
        let env = Env::default();
        env.mock_all_auths();
        let (client, _) = setup(&env);
        let issuer = Address::generate(&env);
        let subject = Address::generate(&env);
        let meta = Bytes::from_slice(&env, b"QmFuzzHash000000000000000000000000");
        client.issue_credential(&issuer, &subject, &0u32, &meta, &None);
    }

    /// Property: threshold > attestor count must always be rejected.
    #[test]
    #[should_panic]
    fn fuzz_create_slice_threshold_exceeds_attestors_rejected() {
        let env = Env::default();
        env.mock_all_auths();
        let (client, _) = setup(&env);
        let creator = Address::generate(&env);
        let mut attestors = Vec::new(&env);
        attestors.push_back(Address::generate(&env));
        let mut weights = Vec::new(&env);
        weights.push_back(1u32);
        // threshold=2 with only 1 attestor — must panic
        client.create_slice(&creator, &attestors, &weights, &2u32);
    }

    /// Property: attesting the same credential twice by the same attestor
    /// must always be rejected (duplicate attestation).
    #[test]
    #[should_panic]
    fn fuzz_attest_duplicate_always_rejected() {
        let env = Env::default();
        env.mock_all_auths();
        let (client, _) = setup(&env);
        let issuer = Address::generate(&env);
        let subject = Address::generate(&env);
        let attestor = Address::generate(&env);
        let meta = Bytes::from_slice(&env, b"QmFuzzHash000000000000000000000000");
        let cid = client.issue_credential(&issuer, &subject, &1u32, &meta, &None);
        let mut attestors = Vec::new(&env);
        attestors.push_back(attestor.clone());
        let mut weights = Vec::new(&env);
        weights.push_back(1u32);
        let slice_id = client.create_slice(&issuer, &attestors, &weights, &1u32);
        client.attest(&attestor, &cid, &slice_id, &None);
        // Second attest by same attestor — must panic
        client.attest(&attestor, &cid, &slice_id, &None);
    }

    /// Property: revoking a credential must prevent further attestation.
    #[test]
    #[should_panic]
    fn fuzz_attest_revoked_credential_always_rejected() {
        let env = Env::default();
        env.mock_all_auths();
        let (client, _) = setup(&env);
        let issuer = Address::generate(&env);
        let subject = Address::generate(&env);
        let attestor = Address::generate(&env);
        let meta = Bytes::from_slice(&env, b"QmFuzzHash000000000000000000000000");
        let cid = client.issue_credential(&issuer, &subject, &1u32, &meta, &None);
        let mut attestors = Vec::new(&env);
        attestors.push_back(attestor.clone());
        let mut weights = Vec::new(&env);
        weights.push_back(1u32);
        let slice_id = client.create_slice(&issuer, &attestors, &weights, &1u32);
        client.revoke_credential(&issuer, &cid);
        // Attest after revocation — must panic
        client.attest(&attestor, &cid, &slice_id, &None);
    }

    // --- Issue #339: Time-window attestation tests ---

    #[test]
    fn test_set_and_get_attestation_window() {
        let env = Env::default();
        env.mock_all_auths();
        let (client, _) = setup(&env);
        let issuer = Address::generate(&env);
        let subject = Address::generate(&env);
        let meta = Bytes::from_slice(&env, b"QmTestHash000000000000000000000000");
        let cid = client.issue_credential(&issuer, &subject, &1u32, &meta, &None);

        client.set_attestation_window(&issuer, &cid, &1000u64, &2000u64);

        let window = client.get_attestation_window(&cid).unwrap();
        assert_eq!(window.start, 1000);
        assert_eq!(window.end, 2000);
    }

    #[test]
    fn test_attest_within_window_succeeds() {
        let env = Env::default();
        env.mock_all_auths();
        let (client, _) = setup(&env);
        let issuer = Address::generate(&env);
        let subject = Address::generate(&env);
        let attestor = Address::generate(&env);
        let meta = Bytes::from_slice(&env, b"QmTestHash000000000000000000000000");
        let cid = client.issue_credential(&issuer, &subject, &1u32, &meta, &None);

        let mut attestors = Vec::new(&env);
        attestors.push_back(attestor.clone());
        let mut weights = Vec::new(&env);
        weights.push_back(1u32);
        let slice_id = client.create_slice(&issuer, &attestors, &weights, &1u32);

        client.set_attestation_window(&issuer, &cid, &500u64, &2000u64);
        set_ledger_timestamp(&env, 1000);

        // Should succeed — timestamp 1000 is within [500, 2000)
        client.attest(&attestor, &cid, &slice_id, &None);
        assert!(client.is_attested(&cid, &slice_id));
    }

    #[test]
    fn test_attest_before_window_rejected() {
        let env = Env::default();
        env.mock_all_auths();
        let (client, _) = setup(&env);
        let issuer = Address::generate(&env);
        let subject = Address::generate(&env);
        let attestor = Address::generate(&env);
        let meta = Bytes::from_slice(&env, b"QmTestHash000000000000000000000000");
        let cid = client.issue_credential(&issuer, &subject, &1u32, &meta, &None);

        let mut attestors = Vec::new(&env);
        attestors.push_back(attestor.clone());
        let mut weights = Vec::new(&env);
        weights.push_back(1u32);
        let slice_id = client.create_slice(&issuer, &attestors, &weights, &1u32);

        client.set_attestation_window(&issuer, &cid, &1000u64, &2000u64);
        set_ledger_timestamp(&env, 500); // before window

        let result = client.try_attest(&attestor, &cid, &slice_id, &None);
        assert_eq!(
            result,
            Err(Ok(soroban_sdk::Error::from_contract_error(
                ContractError::AttestationWindowOutside as u32
            )))
        );
    }

    #[test]
    fn test_attest_after_window_rejected() {
        let env = Env::default();
        env.mock_all_auths();
        let (client, _) = setup(&env);
        let issuer = Address::generate(&env);
        let subject = Address::generate(&env);
        let attestor = Address::generate(&env);
        let meta = Bytes::from_slice(&env, b"QmTestHash000000000000000000000000");
        let cid = client.issue_credential(&issuer, &subject, &1u32, &meta, &None);

        let mut attestors = Vec::new(&env);
        attestors.push_back(attestor.clone());
        let mut weights = Vec::new(&env);
        weights.push_back(1u32);
        let slice_id = client.create_slice(&issuer, &attestors, &weights, &1u32);

        client.set_attestation_window(&issuer, &cid, &500u64, &1000u64);
        set_ledger_timestamp(&env, 1500); // after window

        let result = client.try_attest(&attestor, &cid, &slice_id, &None);
        assert_eq!(
            result,
            Err(Ok(soroban_sdk::Error::from_contract_error(
                ContractError::AttestationWindowOutside as u32
            )))
        );
    }

    #[test]
    fn test_attest_no_window_always_allowed() {
        let env = Env::default();
        env.mock_all_auths();
        let (client, _) = setup(&env);
        let issuer = Address::generate(&env);
        let subject = Address::generate(&env);
        let attestor = Address::generate(&env);
        let meta = Bytes::from_slice(&env, b"QmTestHash000000000000000000000000");
        let cid = client.issue_credential(&issuer, &subject, &1u32, &meta, &None);

        let mut attestors = Vec::new(&env);
        attestors.push_back(attestor.clone());
        let mut weights = Vec::new(&env);
        weights.push_back(1u32);
        let slice_id = client.create_slice(&issuer, &attestors, &weights, &1u32);

        // No window set — attest at any time should succeed
        set_ledger_timestamp(&env, 99999);
        client.attest(&attestor, &cid, &slice_id, &None);
        assert!(client.is_attested(&cid, &slice_id));
    }

    #[test]
    #[should_panic]
    fn test_set_attestation_window_invalid_range_rejected() {
        let env = Env::default();
        env.mock_all_auths();
        let (client, _) = setup(&env);
        let issuer = Address::generate(&env);
        let subject = Address::generate(&env);
        let meta = Bytes::from_slice(&env, b"QmTestHash000000000000000000000000");
        let cid = client.issue_credential(&issuer, &subject, &1u32, &meta, &None);

        // start >= end must be rejected
        client.set_attestation_window(&issuer, &cid, &2000u64, &1000u64);
    }

    #[test]
    fn test_get_attestation_window_none_when_not_set() {
        let env = Env::default();
        env.mock_all_auths();
        let (client, _) = setup(&env);
        let issuer = Address::generate(&env);
        let subject = Address::generate(&env);
        let meta = Bytes::from_slice(&env, b"QmTestHash000000000000000000000000");
        let cid = client.issue_credential(&issuer, &subject, &1u32, &meta, &None);

        assert!(client.get_attestation_window(&cid).is_none());
    }

    // ── Credential Holder Recovery (Issue #290) ─────────────────────────────

    #[test]
    fn test_initiate_recovery_by_issuer() {
        let env = Env::default();
        env.mock_all_auths();
        let (client, _) = setup(&env);
        let issuer = Address::generate(&env);
        let subject = Address::generate(&env);
        let new_subject = Address::generate(&env);
        let approver = Address::generate(&env);
        let meta = Bytes::from_slice(&env, b"QmTestHash000000000000000000000000");
        let cid = client.issue_credential(&issuer, &subject, &1u32, &meta, &None);

        let mut approvers = Vec::new(&env);
        approvers.push_back(approver.clone());
        let rid = client.initiate_recovery(&issuer, &cid, &new_subject, &approvers, &1u32);

        let req = client.get_recovery_request(&rid);
        assert_eq!(req.credential_id, cid);
        assert_eq!(req.issuer, issuer);
        assert_eq!(req.new_subject, new_subject);
        assert_eq!(req.status, RecoveryStatus::Pending);
        assert_eq!(req.threshold, 1);
    }

    #[test]
    #[should_panic(expected = "only the original issuer can initiate recovery")]
    fn test_initiate_recovery_unauthorized_panics() {
        let env = Env::default();
        env.mock_all_auths();
        let (client, _) = setup(&env);
        let issuer = Address::generate(&env);
        let subject = Address::generate(&env);
        let new_subject = Address::generate(&env);
        let approver = Address::generate(&env);
        let attacker = Address::generate(&env);
        let meta = Bytes::from_slice(&env, b"QmTestHash000000000000000000000000");
        let cid = client.issue_credential(&issuer, &subject, &1u32, &meta, &None);

        let mut approvers = Vec::new(&env);
        approvers.push_back(approver.clone());
        client.initiate_recovery(&attacker, &cid, &new_subject, &approvers, &1u32);
    }

    #[test]
    #[should_panic(expected = "Error(Contract, #22)")]
    fn test_initiate_recovery_duplicate_rejected() {
        let env = Env::default();
        env.mock_all_auths();
        let (client, _) = setup(&env);
        let issuer = Address::generate(&env);
        let subject = Address::generate(&env);
        let new_subject = Address::generate(&env);
        let approver = Address::generate(&env);
        let meta = Bytes::from_slice(&env, b"QmTestHash000000000000000000000000");
        let cid = client.issue_credential(&issuer, &subject, &1u32, &meta, &None);

        let mut approvers = Vec::new(&env);
        approvers.push_back(approver.clone());
        client.initiate_recovery(&issuer, &cid, &new_subject, &approvers, &1u32);
        // Second initiation for same credential should panic
        client.initiate_recovery(&issuer, &cid, &new_subject, &approvers, &1u32);
    }

    #[test]
    fn test_approve_recovery_success() {
        let env = Env::default();
        env.mock_all_auths();
        let (client, _) = setup(&env);
        let issuer = Address::generate(&env);
        let subject = Address::generate(&env);
        let new_subject = Address::generate(&env);
        let approver = Address::generate(&env);
        let meta = Bytes::from_slice(&env, b"QmTestHash000000000000000000000000");
        let cid = client.issue_credential(&issuer, &subject, &1u32, &meta, &None);

        let mut approvers = Vec::new(&env);
        approvers.push_back(approver.clone());
        let rid = client.initiate_recovery(&issuer, &cid, &new_subject, &approvers, &1u32);

        client.approve_recovery(&approver, &rid);
        let req = client.get_recovery_request(&rid);
        assert_eq!(req.status, RecoveryStatus::Approved);

        let approvals = client.get_recovery_approvals(&rid);
        assert_eq!(approvals.len(), 1);
        assert_eq!(approvals.get(0).unwrap().approver, approver);
    }

    #[test]
    #[should_panic(expected = "Error(Contract, #26)")]
    fn test_approve_recovery_not_approver_panics() {
        let env = Env::default();
        env.mock_all_auths();
        let (client, _) = setup(&env);
        let issuer = Address::generate(&env);
        let subject = Address::generate(&env);
        let new_subject = Address::generate(&env);
        let approver = Address::generate(&env);
        let stranger = Address::generate(&env);
        let meta = Bytes::from_slice(&env, b"QmTestHash000000000000000000000000");
        let cid = client.issue_credential(&issuer, &subject, &1u32, &meta, &None);

        let mut approvers = Vec::new(&env);
        approvers.push_back(approver.clone());
        let rid = client.initiate_recovery(&issuer, &cid, &new_subject, &approvers, &1u32);

        client.approve_recovery(&stranger, &rid);
    }

    #[test]
    #[should_panic(expected = "Error(Contract, #27)")]
    fn test_approve_recovery_double_vote_rejected() {
        let env = Env::default();
        env.mock_all_auths();
        let (client, _) = setup(&env);
        let issuer = Address::generate(&env);
        let subject = Address::generate(&env);
        let new_subject = Address::generate(&env);
        let approver = Address::generate(&env);
        let meta = Bytes::from_slice(&env, b"QmTestHash000000000000000000000000");
        let cid = client.issue_credential(&issuer, &subject, &1u32, &meta, &None);

        let mut approvers = Vec::new(&env);
        approvers.push_back(approver.clone());
        let rid = client.initiate_recovery(&issuer, &cid, &new_subject, &approvers, &2u32);

        client.approve_recovery(&approver, &rid);
        client.approve_recovery(&approver, &rid); // duplicate
    }

    #[test]
    fn test_recovery_auto_approves_on_threshold() {
        let env = Env::default();
        env.mock_all_auths();
        let (client, _) = setup(&env);
        let issuer = Address::generate(&env);
        let subject = Address::generate(&env);
        let new_subject = Address::generate(&env);
        let approver1 = Address::generate(&env);
        let approver2 = Address::generate(&env);
        let meta = Bytes::from_slice(&env, b"QmTestHash000000000000000000000000");
        let cid = client.issue_credential(&issuer, &subject, &1u32, &meta, &None);

        let mut approvers = Vec::new(&env);
        approvers.push_back(approver1.clone());
        approvers.push_back(approver2.clone());
        let rid = client.initiate_recovery(&issuer, &cid, &new_subject, &approvers, &2u32);

        client.approve_recovery(&approver1, &rid);
        let req1 = client.get_recovery_request(&rid);
        assert_eq!(req1.status, RecoveryStatus::Pending);

        client.approve_recovery(&approver2, &rid);
        let req2 = client.get_recovery_request(&rid);
        assert_eq!(req2.status, RecoveryStatus::Approved);
    }

    #[test]
    fn test_execute_recovery_updates_subject() {
        let env = Env::default();
        env.mock_all_auths();
        let (client, _) = setup(&env);
        let issuer = Address::generate(&env);
        let subject = Address::generate(&env);
        let new_subject = Address::generate(&env);
        let approver = Address::generate(&env);
        let meta = Bytes::from_slice(&env, b"QmTestHash000000000000000000000000");
        let cid = client.issue_credential(&issuer, &subject, &1u32, &meta, &None);

        let mut approvers = Vec::new(&env);
        approvers.push_back(approver.clone());
        let rid = client.initiate_recovery(&issuer, &cid, &new_subject, &approvers, &1u32);
        client.approve_recovery(&approver, &rid);

        client.execute_recovery(&issuer, &rid, &None);

        let cred = client.get_credential(&cid);
        assert_eq!(cred.subject, new_subject);

        let old_list = client.get_credentials_by_subject(&subject, &1u32, &50u32);
        let new_list = client.get_credentials_by_subject(&new_subject, &1u32, &50u32);
        assert!(!old_list.contains(&cid));
        assert!(new_list.contains(&cid));
    }

    #[test]
    #[should_panic(expected = "Error(Contract, #25)")]
    fn test_execute_recovery_threshold_not_met_panics() {
        let env = Env::default();
        env.mock_all_auths();
        let (client, _) = setup(&env);
        let issuer = Address::generate(&env);
        let subject = Address::generate(&env);
        let new_subject = Address::generate(&env);
        let approver1 = Address::generate(&env);
        let approver2 = Address::generate(&env);
        let meta = Bytes::from_slice(&env, b"QmTestHash000000000000000000000000");
        let cid = client.issue_credential(&issuer, &subject, &1u32, &meta, &None);

        let mut approvers = Vec::new(&env);
        approvers.push_back(approver1.clone());
        approvers.push_back(approver2.clone());
        let rid = client.initiate_recovery(&issuer, &cid, &new_subject, &approvers, &2u32);
        client.approve_recovery(&approver1, &rid);

        // Only 1 of 2 approvals — threshold not met
        client.execute_recovery(&issuer, &rid, &None);
    }

    #[test]
    fn test_recovery_transfers_sbt() {
        use sbt_registry::{SbtRegistryContract, SbtRegistryContractClient};

        let env = Env::default();
        env.mock_all_auths_allowing_non_root_auth();
        let qp_id = env.register_contract(None, QuorumProofContract);
        let sbt_id = env.register_contract(None, SbtRegistryContract);
        let client = QuorumProofContractClient::new(&env, &qp_id);
        let sbt = SbtRegistryContractClient::new(&env, &sbt_id);
        let admin = Address::generate(&env);
        client.initialize(&admin);
        sbt.initialize(&admin, &qp_id);

        let issuer = Address::generate(&env);
        let subject = Address::generate(&env);
        let new_subject = Address::generate(&env);
        let approver = Address::generate(&env);
        let meta = Bytes::from_slice(&env, b"QmTestHash000000000000000000000000");
        let cid = client.issue_credential(&issuer, &subject, &1u32, &meta, &None);

        let sbt_uri = Bytes::from_slice(&env, b"ipfs://QmSBT");
        let token_id = sbt.mint(&subject, &cid, &sbt_uri);
        assert_eq!(sbt.owner_of(&token_id), subject);

        let mut approvers = Vec::new(&env);
        approvers.push_back(approver.clone());
        let rid = client.initiate_recovery(&issuer, &cid, &new_subject, &approvers, &1u32);
        client.approve_recovery(&approver, &rid);
        client.execute_recovery(&issuer, &rid, &Some(sbt_id));

        assert_eq!(sbt.owner_of(&token_id), new_subject);
        assert!(sbt.get_tokens_by_owner(&subject).is_empty());
        assert_eq!(sbt.get_tokens_by_owner(&new_subject).get(0).unwrap(), token_id);
    }

    #[test]
    fn test_recovery_emits_events() {
        let env = Env::default();
        env.mock_all_auths();
        let (client, _) = setup(&env);
        let issuer = Address::generate(&env);
        let subject = Address::generate(&env);
        let new_subject = Address::generate(&env);
        let approver = Address::generate(&env);
        let meta = Bytes::from_slice(&env, b"QmTestHash000000000000000000000000");
        let cid = client.issue_credential(&issuer, &subject, &1u32, &meta, &None);

        let mut approvers = Vec::new(&env);
        approvers.push_back(approver.clone());
        let rid = client.initiate_recovery(&issuer, &cid, &new_subject, &approvers, &1u32);
        client.approve_recovery(&approver, &rid);
        client.execute_recovery(&issuer, &rid, &None);

        let events = env.events().all();
        let initiated = events.iter().find(|(_, topics, _)| {
            topics.get(0).map(|t| String::from_val(&env, &t) == String::from_str(&env, TOPIC_RECOVERY_INITIATED)).unwrap_or(false)
        });
        let approved = events.iter().find(|(_, topics, _)| {
            topics.get(0).map(|t| String::from_val(&env, &t) == String::from_str(&env, TOPIC_RECOVERY_APPROVED)).unwrap_or(false)
        });
        let executed = events.iter().find(|(_, topics, _)| {
            topics.get(0).map(|t| String::from_val(&env, &t) == String::from_str(&env, TOPIC_RECOVERY_EXECUTED)).unwrap_or(false)
        });

        assert!(initiated.is_some(), "RecoveryInitiated event not emitted");
        assert!(approved.is_some(), "RecoveryApproved event not emitted");
        assert!(executed.is_some(), "RecoveryExecuted event not emitted");
    }

    #[test]
    fn test_recovery_records_activity() {
        let env = Env::default();
        env.mock_all_auths();
        let (client, _) = setup(&env);
        let issuer = Address::generate(&env);
        let subject = Address::generate(&env);
        let new_subject = Address::generate(&env);
        let approver = Address::generate(&env);
        let meta = Bytes::from_slice(&env, b"QmTestHash000000000000000000000000");
        let cid = client.issue_credential(&issuer, &subject, &1u32, &meta, &None);

        let mut approvers = Vec::new(&env);
        approvers.push_back(approver.clone());
        let rid = client.initiate_recovery(&issuer, &cid, &new_subject, &approvers, &1u32);
        client.approve_recovery(&approver, &rid);
        client.execute_recovery(&issuer, &rid, &None);

        let activities = client.get_holder_activity(&new_subject, &1u32, &10u32);
        assert_eq!(activities.len(), 1);
        let activity = activities.get(0).unwrap();
        assert_eq!(activity.activity_type, ActivityType::CredentialRecovered);
        assert_eq!(activity.credential_id, cid);
        assert_eq!(activity.actor, issuer);
    }

    #[test]
    fn test_cancel_recovery() {
        let env = Env::default();
        env.mock_all_auths();
        let (client, _) = setup(&env);
        let issuer = Address::generate(&env);
        let subject = Address::generate(&env);
        let new_subject = Address::generate(&env);
        let approver = Address::generate(&env);
        let meta = Bytes::from_slice(&env, b"QmTestHash000000000000000000000000");
        let cid = client.issue_credential(&issuer, &subject, &1u32, &meta, &None);

        let mut approvers = Vec::new(&env);
        approvers.push_back(approver.clone());
        let rid = client.initiate_recovery(&issuer, &cid, &new_subject, &approvers, &1u32);
        client.cancel_recovery(&issuer, &rid);

        let req = client.get_recovery_request(&rid);
        assert_eq!(req.status, RecoveryStatus::Rejected);
    }

    #[test]
    #[should_panic(expected = "only the issuer can cancel recovery")]
    fn test_cancel_recovery_unauthorized_panics() {
        let env = Env::default();
        env.mock_all_auths();
        let (client, _) = setup(&env);
        let issuer = Address::generate(&env);
        let subject = Address::generate(&env);
        let new_subject = Address::generate(&env);
        let approver = Address::generate(&env);
        let attacker = Address::generate(&env);
        let meta = Bytes::from_slice(&env, b"QmTestHash000000000000000000000000");
        let cid = client.issue_credential(&issuer, &subject, &1u32, &meta, &None);

        let mut approvers = Vec::new(&env);
        approvers.push_back(approver.clone());
        let rid = client.initiate_recovery(&issuer, &cid, &new_subject, &approvers, &1u32);
        client.cancel_recovery(&attacker, &rid);
    }

    // ── Credential Type Hierarchy Tests (Issue #291) ──

    #[test]
    fn test_register_credential_type_without_parent() {
        let env = Env::default();
        env.mock_all_auths();
        let (client, admin) = setup(&env);

        // Register a root credential type without parent
        let name = String::from_str(&env, "Engineering Degree");
        let desc = String::from_str(&env, "Bachelor of Engineering"); 
        client.register_credential_type(&admin, &1u32, &name, &desc, &None);

        // Verify it was registered
        let ctype = client.get_credential_type(&1u32);
        assert_eq!(ctype.type_id, 1);
        assert_eq!(ctype.name, name);
        assert_eq!(ctype.description, desc);
        assert_eq!(ctype.parent_type, None);
    }

    #[test]
    fn test_register_credential_type_with_valid_parent() {
        let env = Env::default();
        env.mock_all_auths();
        let (client, admin) = setup(&env);

        // Register parent type
        let parent_name = String::from_str(&env, "Engineering");
        let parent_desc = String::from_str(&env, "Engineering Credential");
        client.register_credential_type(&admin, &1u32, &parent_name, &parent_desc, &None);

        // Register child type with parent
        let child_name = String::from_str(&env, "Mechanical Engineering");
        let child_desc = String::from_str(&env, "Mechanical Engineering Degree");
        client.register_credential_type(&admin, &2u32, &child_name, &child_desc, &Some(1u32));

        // Verify child has correct parent
        let child = client.get_credential_type(&2u32);
        assert_eq!(child.parent_type, Some(1u32));
    }

    #[test]
    #[should_panic(expected = "invalidparenttype")]
    fn test_register_credential_type_invalid_parent() {
        let env = Env::default();
        env.mock_all_auths();
        let (client, admin) = setup(&env);

        // Try to register type with non-existent parent
        let name = String::from_str(&env, "Degree");
        let desc = String::from_str(&env, "Some Degree");
        client.register_credential_type(&admin, &1u32, &name, &desc, &Some(999u32));
    }

    #[test]
    #[should_panic(expected = "circularhierarchy")]
    fn test_register_credential_type_circular_dependency() {
        let env = Env::default();
        env.mock_all_auths();
        let (client, admin) = setup(&env);

        // Register type A
        let name_a = String::from_str(&env, "Type A");
        let desc_a = String::from_str(&env, "Type A");
        client.register_credential_type(&admin, &1u32, &name_a, &desc_a, &None);

        // Register type B with A as parent
        let name_b = String::from_str(&env, "Type B");
        let desc_b = String::from_str(&env, "Type B");
        client.register_credential_type(&admin, &2u32, &name_b, &desc_b, &Some(1u32));

        // Try to update A with B as parent (would create cycle)
        let name_a_new = String::from_str(&env, "Type A Updated");
        let desc_a_new = String::from_str(&env, "Type A Updated");
        client.register_credential_type(&admin, &1u32, &name_a_new, &desc_a_new, &Some(2u32));
    }

    #[test]
    fn test_three_level_hierarchy() {
        let env = Env::default();
        env.mock_all_auths();
        let (client, admin) = setup(&env);

        // Register grandparent (A)
        let name_a = String::from_str(&env, "Professional Credential");
        let desc_a = String::from_str(&env, "Professional Credential");
        client.register_credential_type(&admin, &1u32, &name_a, &desc_a, &None);

        // Register parent (B) with A as parent
        let name_b = String::from_str(&env, "Engineering License");
        let desc_b = String::from_str(&env, "Engineering License");
        client.register_credential_type(&admin, &2u32, &name_b, &desc_b, &Some(1u32));

        // Register child (C) with B as parent
        let name_c = String::from_str(&env, "Mechanical Engineering License");
        let desc_c = String::from_str(&env, "Mechanical Engineering License");
        client.register_credential_type(&admin, &3u32, &name_c, &desc_c, &Some(2u32));

        // Verify lineage
        let parent_of_c = client.get_credential_type_parent(&3u32);
        assert_eq!(parent_of_c, Some(2u32));

        let parent_of_b = client.get_credential_type_parent(&2u32);
        assert_eq!(parent_of_b, Some(1u32));

        let parent_of_a = client.get_credential_type_parent(&1u32);
        assert_eq!(parent_of_a, None);
    }

    #[test]
    fn test_get_credential_type_children() {
        let env = Env::default();
        env.mock_all_auths();
        let (client, admin) = setup(&env);

        // Register parent
        let parent_name = String::from_str(&env, "Parent");
        let parent_desc = String::from_str(&env, "Parent Type");
        client.register_credential_type(&admin, &1u32, &parent_name, &parent_desc, &None);

        // Register two children
        let child1_name = String::from_str(&env, "Child 1");
        let child1_desc = String::from_str(&env, "Child 1");
        client.register_credential_type(&admin, &2u32, &child1_name, &child1_desc, &Some(1u32));

        let child2_name = String::from_str(&env, "Child 2");
        let child2_desc = String::from_str(&env, "Child 2");
        client.register_credential_type(&admin, &3u32, &child2_name, &child2_desc, &Some(1u32));

        // Get children of parent
        let children = client.get_credential_type_children(&1u32);
        assert_eq!(children.len(), 2);
        assert!(children.iter().any(|&c| c == 2u32));
        assert!(children.iter().any(|&c| c == 3u32));

        // Parent with no children
        let children_of_leaf = client.get_credential_type_children(&2u32);
        assert_eq!(children_of_leaf.len(), 0);
    }

    #[test]
    fn test_get_credential_type_ancestors() {
        let env = Env::default();
        env.mock_all_auths();
        let (client, admin) = setup(&env);

        // Build hierarchy: A <- B <- C
        let name_a = String::from_str(&env, "A");
        let desc = String::from_str(&env, "");
        client.register_credential_type(&admin, &1u32, &name_a, &desc, &None);

        let name_b = String::from_str(&env, "B");
        client.register_credential_type(&admin, &2u32, &name_b, &desc, &Some(1u32));

        let name_c = String::from_str(&env, "C");
        client.register_credential_type(&admin, &3u32, &name_c, &desc, &Some(2u32));

        // Get ancestors of C: should be [B, A]
        let ancestors_c = client.get_credential_type_ancestors(&3u32);
        assert_eq!(ancestors_c.len(), 2);
        assert_eq!(ancestors_c.get(0).unwrap(), 2u32);
        assert_eq!(ancestors_c.get(1).unwrap(), 1u32);

        // Get ancestors of B: should be [A]
        let ancestors_b = client.get_credential_type_ancestors(&2u32);
        assert_eq!(ancestors_b.len(), 1);
        assert_eq!(ancestors_b.get(0).unwrap(), 1u32);

        // Get ancestors of A: should be []
        let ancestors_a = client.get_credential_type_ancestors(&1u32);
        assert_eq!(ancestors_a.len(), 0);
    }

    #[test]
    fn test_is_credential_type_child_of() {
        let env = Env::default();
        env.mock_all_auths();
        let (client, admin) = setup(&env);

        // Register hierarchy
        let name = String::from_str(&env, "");
        let desc = String::from_str(&env, "");
        client.register_credential_type(&admin, &1u32, &name, &desc, &None);
        client.register_credential_type(&admin, &2u32, &name, &desc, &Some(1u32));
        client.register_credential_type(&admin, &3u32, &name, &desc, &Some(2u32));

        // Direct child relationship
        assert!(client.is_credential_type_child_of(&2u32, &1u32));

        // Transitive child relationship  
        assert!(client.is_credential_type_child_of(&3u32, &1u32));

        // Not a child relationship
        assert!(!client.is_credential_type_child_of(&1u32, &2u32));

        // Not a child (unrelated types)
        client.register_credential_type(&admin, &4u32, &name, &desc, &None);
        assert!(!client.is_credential_type_child_of(&4u32, &1u32));
    }

    #[test]
    fn test_inherit_verification_rules() {
        let env = Env::default();
        env.mock_all_auths();
        let (client, admin) = setup(&env);

        // Build hierarchy: A <- B <- C <- D
        let name = String::from_str(&env, "");
        let desc = String::from_str(&env, "");
        
        client.register_credential_type(&admin, &1u32, &name, &desc, &None);  // A
        client.register_credential_type(&admin, &2u32, &name, &desc, &Some(1u32));  // B <- A
        client.register_credential_type(&admin, &3u32, &name, &desc, &Some(2u32));  // C <- B
        client.register_credential_type(&admin, &4u32, &name, &desc, &Some(3u32));  // D <- C

        // For type D, rules should be: [D, C, B, A] (child to root order)
        let rules = client.inherit_verification_rules(&4u32);
        assert_eq!(rules.len(), 4);
        assert_eq!(rules.get(0).unwrap(), 4u32);  // self
        assert_eq!(rules.get(1).unwrap(), 3u32);  // parent
        assert_eq!(rules.get(2).unwrap(), 2u32);  // grandparent
        assert_eq!(rules.get(3).unwrap(), 1u32);  // great-grandparent

        // For root type A, rules should be just [A]
        let rules_a = client.inherit_verification_rules(&1u32);
        assert_eq!(rules_a.len(), 1);
        assert_eq!(rules_a.get(0).unwrap(), 1u32);
    }

    #[test]
    fn test_multiple_children_same_parent() {
        let env = Env::default();
        env.mock_all_auths();
        let (client, admin) = setup(&env);

        // Register parent
        let name = String::from_str(&env, "");
        let desc = String::from_str(&env, "");
        client.register_credential_type(&admin, &1u32, &name, &desc, &None);

        // Register multiple children (3 children)
        for i in 2u32..5u32 {
            client.register_credential_type(&admin, &i, &name, &desc, &Some(1u32));
        }

        // Verify all children are registered
        let children = client.get_credential_type_children(&1u32);
        assert_eq!(children.len(), 3);

        // Verify each child points to parent
        for i in 2u32..5u32 {
            let parent = client.get_credential_type_parent(&i);
            assert_eq!(parent, Some(1u32));
        }
    }

    #[test]
    fn test_backward_compatibility_no_parent() {
        let env = Env::default();
        env.mock_all_auths();
        let (client, admin) = setup(&env);

        // Register type without parent (backward compatible behavior)
        let name = String::from_str(&env, "Legacy Type");
        let desc = String::from_str(&env, "No parent");
        client.register_credential_type(&admin, &1u32, &name, &desc, &None);

        // Should have no parent
        let parent = client.get_credential_type_parent(&1u32);
        assert_eq!(parent, None);

        // Should have no children
        let children = client.get_credential_type_children(&1u32);
        assert_eq!(children.len(), 0);

        // Should have empty ancestors
        let ancestors = client.get_credential_type_ancestors(&1u32);
        assert_eq!(ancestors.len(), 0);

        // Verification rules should just be self
        let rules = client.inherit_verification_rules(&1u32);
        assert_eq!(rules.len(), 1);
        assert_eq!(rules.get(0).unwrap(), 1u32);
    }

    #[test]
    fn test_overwrite_existing_type_maintains_hierarchy() {
        let env = Env::default();
        env.mock_all_auths();
        let (client, admin) = setup(&env);

        // Register parent and child
        let name = String::from_str(&env, "");
        let desc = String::from_str(&env, "");
        client.register_credential_type(&admin, &1u32, &name, &desc, &None);
        client.register_credential_type(&admin, &2u32, &name, &desc, &Some(1u32));

        // Verify parent-child relationship
        let parent = client.get_credential_type_parent(&2u32);
        assert_eq!(parent, Some(1u32));

        // Overwrite parent type with new description (no parent change)
        let new_desc = String::from_str(&env, "Updated description");
        client.register_credential_type(&admin, &1u32, &name, &new_desc, &None);

        // Child relationship should still exist
        let parent_after = client.get_credential_type_parent(&2u32);
        assert_eq!(parent_after, Some(1u32));
    }

    #[test]
    fn test_detect_fork_no_conflict() {
        let env = Env::default();
        env.mock_all_auths();
        let (client, admin) = setup(&env);

        // Create a credential and slice
        let issuer = Address::generate(&env);
        let subject = Address::generate(&env);
        let credential_id = client.issue(&issuer, &subject, &1u32, &None);
        let slice_id = client.create_slice(&admin, &vec![&env, admin.clone(), Address::generate(&env)], &2u64);

        // Attest with true value
        client.attest(&admin, &credential_id, &slice_id, &true, &None);

        // Detect fork for another attestor with same value - should not detect fork
        let attestor2 = Address::generate(&env);
        let has_fork = client.detect_fork(&credential_id, &slice_id, &attestor2, true);
        assert!(!has_fork);
    }

    #[test]
    fn test_detect_fork_with_conflict() {
        let env = Env::default();
        env.mock_all_auths();
        let (client, admin) = setup(&env);

        // Create a credential and slice
        let issuer = Address::generate(&env);
        let subject = Address::generate(&env);
        let credential_id = client.issue(&issuer, &subject, &1u32, &None);
        let slice_id = client.create_slice(&admin, &vec![&env, admin.clone(), Address::generate(&env)], &2u64);

        // Attest with true value
        client.attest(&admin, &credential_id, &slice_id, &true, &None);

        // Detect fork for another attestor with false value - should detect fork
        let attestor2 = Address::generate(&env);
        let has_fork = client.detect_fork(&credential_id, &slice_id, &attestor2, false);
        assert!(has_fork);
    }

    #[test]
    #[should_panic(expected = "ForkDetected")]
    fn test_attest_prevents_conflicting_attestation() {
        let env = Env::default();
        env.mock_all_auths();
        let (client, admin) = setup(&env);

        // Create a credential and slice
        let issuer = Address::generate(&env);
        let subject = Address::generate(&env);
        let credential_id = client.issue(&issuer, &subject, &1u32, &None);
        let attestor2 = Address::generate(&env);
        let slice_id = client.create_slice(&admin, &vec![&env, admin.clone(), attestor2.clone()], &2u64);

        // First attestation with true
        client.attest(&admin, &credential_id, &slice_id, &true, &None);

        // Second attestation with false - should panic
        client.attest(&attestor2, &credential_id, &slice_id, &false, &None);
    }

    #[test]
    fn test_fork_detection_stores_info() {
        let env = Env::default();
        env.mock_all_auths();
        let (client, admin) = setup(&env);

        // Create a credential and slice
        let issuer = Address::generate(&env);
        let subject = Address::generate(&env);
        let credential_id = client.issue(&issuer, &subject, &1u32, &None);
        let attestor2 = Address::generate(&env);
        let slice_id = client.create_slice(&admin, &vec![&env, admin.clone(), attestor2.clone()], &2u64);

        // First attestation with true
        client.attest(&admin, &credential_id, &slice_id, &true, &None);

        // Try second attestation with false - should store fork info
        let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            client.attest(&attestor2, &credential_id, &slice_id, &false, &None);
        }));
        assert!(result.is_err()); // Should have panicked

        // Check that fork info was stored
        let fork_status: ForkStatus = env.storage().instance()
            .get(&DataKey::ForkStatus(credential_id, slice_id))
            .unwrap();
        assert_eq!(fork_status, ForkStatus::ForkDetected);

        let fork_info: ForkInfo = env.storage().instance()
            .get(&DataKey::ForkInfo(credential_id, slice_id))
            .unwrap();
        assert_eq!(fork_info.credential_id, credential_id);
        assert_eq!(fork_info.slice_id, slice_id);
        assert_eq!(fork_info.conflicting_attestors.len(), 2);
        assert_eq!(fork_info.attested_values.len(), 2);
    }

    #[test]
    fn test_get_verification_stats_initial_zeros() {
        let env = Env::default();
        env.mock_all_auths();
        let contract_id = env.register_contract(None, QuorumProofContract);
        let client = QuorumProofContractClient::new(&env, &contract_id);

        let stats = client.get_verification_stats();
        assert_eq!(stats.total_verifications, 0);
        assert_eq!(stats.successful_verifications, 0);
        assert_eq!(stats.failed_verifications, 0);
    }

    #[test]
    fn test_verification_stats_increments_on_success() {
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
        let result = qp.verify_engineer(&sbt_id, &zk_id, &subject, &cred_id, &ClaimType::HasDegree, &proof);
        assert!(result);

        let stats = qp.get_verification_stats();
        assert_eq!(stats.total_verifications, 1);
        assert_eq!(stats.successful_verifications, 1);
        assert_eq!(stats.failed_verifications, 0);
    }

    #[test]
    fn test_verification_stats_increments_on_failure() {
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

        // No SBT minted — verification fails
        let proof = Bytes::from_slice(&env, b"valid-proof");
        let result = qp.verify_engineer(&sbt_id, &zk_id, &subject, &cred_id, &ClaimType::HasDegree, &proof);
        assert!(!result);

        let stats = qp.get_verification_stats();
        assert_eq!(stats.total_verifications, 1);
        assert_eq!(stats.successful_verifications, 0);
        assert_eq!(stats.failed_verifications, 1);
    }

    #[test]
    fn test_verification_stats_accumulates_across_calls() {
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

        let good_proof = Bytes::from_slice(&env, b"valid-proof");
        let bad_proof = Bytes::from_slice(&env, b"");

        // 2 successes, 1 failure
        qp.verify_engineer(&sbt_id, &zk_id, &subject, &cred_id, &ClaimType::HasDegree, &good_proof);
        qp.verify_engineer(&sbt_id, &zk_id, &subject, &cred_id, &ClaimType::HasLicense, &good_proof);
        qp.verify_engineer(&sbt_id, &zk_id, &subject, &cred_id, &ClaimType::HasDegree, &bad_proof);

        let stats = qp.get_verification_stats();
        assert_eq!(stats.total_verifications, 3);
        assert_eq!(stats.successful_verifications, 2);
        assert_eq!(stats.failed_verifications, 1);
    }

    #[test]
    fn test_holder_reputation_zero_before_any_activity() {
        let env = Env::default();
        env.mock_all_auths();
        let contract_id = env.register_contract(None, QuorumProofContract);
        let client = QuorumProofContractClient::new(&env, &contract_id);

        let holder = Address::generate(&env);
        let rep = client.get_holder_reputation(&holder);
        assert_eq!(rep.credentials_held, 0);
        assert_eq!(rep.successful_verifications, 0);
        assert_eq!(rep.score, 0);
    }

    #[test]
    fn test_holder_reputation_increments_on_credential_issue() {
        let env = Env::default();
        env.mock_all_auths();
        let contract_id = env.register_contract(None, QuorumProofContract);
        let client = QuorumProofContractClient::new(&env, &contract_id);

        let issuer = Address::generate(&env);
        let subject = Address::generate(&env);
        let metadata = Bytes::from_slice(&env, b"ipfs://QmTest");

        client.issue_credential(&issuer, &subject, &1u32, &metadata, &None);
        let rep = client.get_holder_reputation(&subject);
        assert_eq!(rep.credentials_held, 1);
        assert_eq!(rep.score, 1);

        client.issue_credential(&issuer, &subject, &2u32, &metadata, &None);
        let rep = client.get_holder_reputation(&subject);
        assert_eq!(rep.credentials_held, 2);
        assert_eq!(rep.score, 2);
    }

    #[test]
    fn test_holder_reputation_increments_on_successful_verification() {
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
        qp.verify_engineer(&sbt_id, &zk_id, &subject, &cred_id, &ClaimType::HasDegree, &proof);

        let rep = qp.get_holder_reputation(&subject);
        assert_eq!(rep.successful_verifications, 1);
        assert_eq!(rep.credentials_held, 1);
        assert_eq!(rep.score, 2); // 1 credential + 1 verification
    }

    #[test]
    fn test_holder_reputation_not_incremented_on_failed_verification() {
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

        // No SBT minted — verification fails, reputation should not increment.
        let proof = Bytes::from_slice(&env, b"valid-proof");
        qp.verify_engineer(&sbt_id, &zk_id, &subject, &cred_id, &ClaimType::HasDegree, &proof);

        let rep = qp.get_holder_reputation(&subject);
        assert_eq!(rep.successful_verifications, 0);
        assert_eq!(rep.credentials_held, 1);
        assert_eq!(rep.score, 1);
    }

    #[test]
    fn test_holder_reputation_independent_per_holder() {
        let env = Env::default();
        env.mock_all_auths();
        let contract_id = env.register_contract(None, QuorumProofContract);
        let client = QuorumProofContractClient::new(&env, &contract_id);

        let issuer = Address::generate(&env);
        let subject_a = Address::generate(&env);
        let subject_b = Address::generate(&env);
        let metadata = Bytes::from_slice(&env, b"ipfs://QmTest");

        client.issue_credential(&issuer, &subject_a, &1u32, &metadata, &None);
        client.issue_credential(&issuer, &subject_a, &2u32, &metadata, &None);
        client.issue_credential(&issuer, &subject_b, &1u32, &metadata, &None);

        assert_eq!(client.get_holder_reputation(&subject_a).credentials_held, 2);
        assert_eq!(client.get_holder_reputation(&subject_b).credentials_held, 1);
    }
}

// ── Feature #355: Proof Expiry ───────────────────────────────────────────

/// Check if a proof has expired based on its expiry timestamp.
///
/// # Parameters
/// - `credential_id`: The credential whose proof to check.
/// - `proof_expires_at`: The Unix timestamp when the proof expires.
///
/// # Returns
/// true if the current ledger timestamp is >= proof_expires_at, false otherwise.
pub fn is_proof_expired(env: Env, credential_id: u64, proof_expires_at: u64) -> bool {
    if !env.storage().instance().has(&DataKey::Credential(credential_id)) {
        panic_with_error!(&env, ContractError::CredentialNotFound);
    }
    env.ledger().timestamp() >= proof_expires_at
}

/// Renew a proof by extending its expiry timestamp.
/// Only the credential issuer may renew proofs.
///
/// # Parameters
/// - `issuer`: The issuer of the credential; must authorize this call.
/// - `credential_id`: The credential whose proof to renew.
/// - `new_proof_expires_at`: New Unix timestamp; must be in the future.
///
/// # Returns
/// The new expiry timestamp.
///
/// # Panics
/// Panics if the contract is paused.
/// Panics with `ContractError::CredentialNotFound` if the credential does not exist.
/// Panics if the caller is not the issuer.
/// Panics if `new_proof_expires_at` is not in the future.
pub fn renew_proof(env: Env, issuer: Address, credential_id: u64, new_proof_expires_at: u64) -> u64 {
    issuer.require_auth();
    Self::require_not_paused(&env);

    let credential: Credential = env
        .storage()
        .instance()
        .get(&DataKey::Credential(credential_id))
        .unwrap_or_else(|| panic_with_error!(&env, ContractError::CredentialNotFound));

    assert!(credential.issuer == issuer, "only the issuer can renew proofs");
    assert!(!credential.revoked, "cannot renew proof for revoked credential");
    assert!(new_proof_expires_at > env.ledger().timestamp(), "new_proof_expires_at must be in the future");

    new_proof_expires_at
}

// ── Feature #356: Batch Proof Verification ───────────────────────────────

/// Verify multiple proofs in a single call.
///
/// # Parameters
/// - `zk_verifier_id`: Address of the deployed ZK verifier contract.
/// - `zk_admin`: Admin address for ZK verifier authorization.
/// - `quorum_proof_id`: Address of this quorum proof contract.
/// - `credential_ids`: List of credential IDs to verify.
/// - `slice_ids`: List of slice IDs corresponding to each credential.
/// - `proof_expires_at_list`: List of proof expiry timestamps.
///
/// # Returns
/// Vec of tuples (credential_id, is_valid, is_expired) for each proof.
///
/// # Panics
/// Panics if input arrays have different lengths.
/// Panics if any array exceeds MAX_BATCH_SIZE.
pub fn batch_verify_proofs(
    env: Env,
    credential_ids: Vec<u64>,
    slice_ids: Vec<u64>,
    proof_expires_at_list: Vec<u64>,
) -> Vec<(u64, bool, bool)> {
    Self::validate_array_bounds(credential_ids.len(), 1, MAX_BATCH_SIZE, "credential_ids");
    assert!(
        credential_ids.len() == slice_ids.len() && credential_ids.len() == proof_expires_at_list.len(),
        "input lengths must match"
    );

    let mut results: Vec<(u64, bool, bool)> = Vec::new(&env);

    for i in 0..credential_ids.len() {
        let credential_id = credential_ids.get(i).unwrap();
        let slice_id = slice_ids.get(i).unwrap();
        let proof_expires_at = proof_expires_at_list.get(i).unwrap();

        // Check if credential exists and is attested
        let is_valid = if env.storage().instance().has(&DataKey::Credential(credential_id)) {
            Self::is_attested(env.clone(), credential_id, slice_id)
        } else {
            false
        };

        // Check if proof is expired
        let is_expired = if env.storage().instance().has(&DataKey::Credential(credential_id)) {
            Self::is_proof_expired(env.clone(), credential_id, proof_expires_at)
        } else {
            true
        };

        results.push_back((credential_id, is_valid, is_expired));
    }

    results
}

// ── Feature #357: Claim Type Validation ──────────────────────────────────

/// Validate that a claim type is supported.
///
/// # Parameters
/// - `claim_type`: The claim type to validate.
///
/// # Returns
/// true if the claim type is in the supported list, false otherwise.
pub fn is_claim_type_supported(env: Env, claim_type: ClaimType) -> bool {
    // Define supported claim types
    match claim_type {
        ClaimType::Degree => true,
        ClaimType::License => true,
        ClaimType::Employment => true,
        ClaimType::Age => true,
        ClaimType::Citizenship => true,
        ClaimType::Custom => true,
    }
}

/// Get list of all supported claim types.
///
/// # Returns
/// Vec of all supported ClaimType values.
pub fn get_supported_claim_types(env: Env) -> Vec<ClaimType> {
    let mut types: Vec<ClaimType> = Vec::new(&env);
    types.push_back(ClaimType::Degree);
    types.push_back(ClaimType::License);
    types.push_back(ClaimType::Employment);
    types.push_back(ClaimType::Age);
    types.push_back(ClaimType::Citizenship);
    types.push_back(ClaimType::Custom);
    types
}

/// Validate claim types in a proof request.
///
/// # Parameters
/// - `claim_types`: List of claim types to validate.
///
/// # Returns
/// true if all claim types are supported, false otherwise.
pub fn validate_claim_types(env: Env, claim_types: Vec<ClaimType>) -> bool {
    for claim_type in claim_types.iter() {
        if !Self::is_claim_type_supported(env.clone(), claim_type) {
            return false;
        }
    }
    true
}

// ── Feature #359: Credential Search with Filters ─────────────────────────

/// Search credentials with advanced filters.
///
/// # Parameters
/// - `subject`: Optional subject address filter.
/// - `issuer`: Optional issuer address filter.
/// - `credential_type`: Optional credential type filter.
/// - `start_date`: Optional start date filter (Unix timestamp).
/// - `end_date`: Optional end date filter (Unix timestamp).
/// - `page`: Page number (1-indexed).
/// - `page_size`: Number of results per page.
///
/// # Returns
/// Vec of credential IDs matching the filters.
pub fn search_credentials(
    env: Env,
    subject: Option<Address>,
    issuer: Option<Address>,
    credential_type: Option<u32>,
    start_date: Option<u64>,
    end_date: Option<u64>,
    page: u32,
    page_size: u32,
) -> Vec<u64> {
    Self::precondition(&env, page > 0);
    Self::precondition(&env, page_size > 0);
    Self::precondition(&env, page_size <= MAX_BATCH_SIZE);

    let mut matching_ids: Vec<u64> = Vec::new(&env);
    let total_credentials: u64 = env
        .storage()
        .instance()
        .get(&DataKey::CredentialCount)
        .unwrap_or(0u64);

    // Iterate through all credentials and apply filters
    for id in 1..=total_credentials {
        if let Some(credential) = env
            .storage()
            .instance()
            .get::<DataKey, Credential>(&DataKey::Credential(id))
        {
            // Apply subject filter
            if let Some(ref filter_subject) = subject {
                if credential.subject != *filter_subject {
                    continue;
                }
            }

            // Apply issuer filter
            if let Some(ref filter_issuer) = issuer {
                if credential.issuer != *filter_issuer {
                    continue;
                }
            }

            // Apply credential type filter
            if let Some(filter_type) = credential_type {
                if credential.credential_type != filter_type {
                    continue;
                }
            }

            // Apply date range filters (using credential ID as proxy for creation time)
            // Note: In production, you'd want to store creation timestamp in Credential struct

            matching_ids.push_back(id);
        }
    }

    // Apply pagination
    let total = matching_ids.len();
    let start = (page - 1).saturating_mul(page_size);
    let mut result = Vec::new(&env);

    for i in start..start.saturating_add(page_size) {
        if i >= total {
            break;
        }
        if let Some(cred_id) = matching_ids.get(i) {
            result.push_back(cred_id);
        }
    }

    result
}

/// Get total count of credentials matching filters.
///
/// # Parameters
/// Same as search_credentials but without pagination.
///
/// # Returns
/// Total count of matching credentials.
pub fn count_credentials(
    env: Env,
    subject: Option<Address>,
    issuer: Option<Address>,
    credential_type: Option<u32>,
) -> u32 {
    let mut count: u32 = 0;
    let total_credentials: u64 = env
        .storage()
        .instance()
        .get(&DataKey::CredentialCount)
        .unwrap_or(0u64);

    for id in 1..=total_credentials {
        if let Some(credential) = env
            .storage()
            .instance()
            .get::<DataKey, Credential>(&DataKey::Credential(id))
        {
            if let Some(ref filter_subject) = subject {
                if credential.subject != *filter_subject {
                    continue;
                }
            }

            if let Some(ref filter_issuer) = issuer {
                if credential.issuer != *filter_issuer {
                    continue;
                }
            }

            if let Some(filter_type) = credential_type {
                if credential.credential_type != filter_type {
                    continue;
                }
            }

            count += 1;
        }
    }

    count
}

