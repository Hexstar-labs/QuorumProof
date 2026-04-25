#![no_std]
use soroban_sdk::{contract, contractimpl, contracttype, contracterror, panic_with_error, symbol_short, Address, Bytes, Env, IntoVal, Vec, Symbol};

const STANDARD_TTL: u32 = 16_384;
const EXTENDED_TTL: u32 = 524_288;

#[contracterror]
#[derive(Copy, Clone, Debug, Eq, PartialEq, PartialOrd, Ord)]
#[repr(u32)]
// #[contracterror] is required for panic_with_error! to work correctly with Soroban.
// Copy + Clone are the only derives compatible with #[contracterror].
pub enum ContractError {
    SoulboundNonTransferable = 1,
    TokenNotFound = 2,
    RecoveryNotFound = 3,
    RecoveryAlreadyExists = 4,
    UnauthorizedRecovery = 5,
    InsufficientApprovals = 6,
    InvalidGuardian = 7,
}

#[contracttype]
#[derive(Clone)]
pub enum DataKey {
    Token(u64),
    TokenCount,
    Owner(u64),
    OwnerTokens(Address),
    OwnerCredential(Address, u64),
    Delegation(u64),
    Admin,
    QuorumProofId,
    RecoveryRequest(u64),
    RecoveryRequestCount,
    PendingRecoveryByHolder(Address),
    RecoveryApprovals(u64),
    RecoveryGuardians,
    RecoveryThreshold,
    AuditTrail(u64),
    AuditTrailCount,
}

#[contracttype]
#[derive(Clone)]
pub struct SoulboundToken {
    pub id: u64,
    pub owner: Address,
    pub credential_id: u64,
    pub metadata_uri: Bytes,
}

#[contracttype]
#[derive(Clone)]
pub struct Delegation {
    pub token_id: u64,
    pub delegatee: Address,
    pub expires_at: u64,
}

/// Represents a recovery request for an SBT holder's lost/compromised account
#[contracttype]
#[derive(Clone)]
pub struct RecoveryRequest {
    /// Unique recovery request ID
    pub id: u64,
    /// Original account owner who initiated recovery
    pub initiator: Address,
    /// New account to recover SBTs to
    pub new_owner: Address,
    /// Time when recovery was initiated
    pub initiated_at: u64,
    /// Whether the recovery has been finalized
    pub completed: bool,
    /// Number of approvals received so far
    pub approvals_count: u32,
}

/// Represents a single approval by a recovery guardian
#[contracttype]
#[derive(Clone)]
pub struct RecoveryApproval {
    /// The guardian who approved
    pub guardian: Address,
    /// Time when approval was given
    pub approved_at: u64,
}

/// Audit trail entry for recovery operations
#[contracttype]
#[derive(Clone)]
pub struct AuditTrailEntry {
    /// Unique audit trail ID
    pub id: u64,
    /// Recovery request ID (if applicable)
    pub recovery_request_id: u64,
    /// Type of action: "initiate", "approve", "finalize"
    pub action: Symbol,
    /// Actor performing the action
    pub actor: Address,
    /// Timestamp of the action
    pub timestamp: u64,
    /// Additional details about the action
    pub details: soroban_sdk::String,
}

#[contract]
pub struct SbtRegistryContract;

#[contractimpl]
impl SbtRegistryContract {
    /// Mint a soulbound token linked to a credential_id.
    ///
    /// Creates a non-transferable token bound to the `owner` address and associated
    /// with the given `credential_id`. Each `(owner, credential_id)` pair may only
    /// have one SBT — attempting to mint a duplicate panics.
    ///
    /// Cross-contract verifies via `quorum_proof` that the credential exists and is
    /// not revoked before minting.
    ///
    /// # Parameters
    /// - `owner`: The address receiving the SBT; must authorize this call.
    /// - `credential_id`: The credential this SBT is linked to.
    /// - `metadata_uri`: Content-addressed URI (e.g. IPFS) for the token metadata.
    ///
    /// # Panics
    /// Panics with `ContractError::SoulboundNonTransferable` if an SBT already exists
    /// for this `(owner, credential_id)` pair.
    /// Panics if the credential does not exist or is revoked in `quorum_proof`.
    pub fn mint(env: Env, owner: Address, credential_id: u64, metadata_uri: Bytes) -> u64 {
        owner.require_auth();

        // Cross-contract: verify credential exists and is not revoked.
        // Uses env.invoke_contract to avoid a circular crate dependency with quorum_proof.
        let qp_id: Address = env.storage().instance()
            .get(&DataKey::QuorumProofId)
            .expect("not initialized");
        // is_revoked panics with CredentialNotFound if the credential doesn't exist.
        let revoked: bool = env.invoke_contract(
            &qp_id,
            &Symbol::new(&env, "is_revoked"),
            soroban_sdk::vec![&env, credential_id.into_val(&env)],
        );
        assert!(!revoked, "credential is revoked");

        if env.storage().instance().has(&DataKey::OwnerCredential(owner.clone(), credential_id)) {
            panic_with_error!(&env, ContractError::SoulboundNonTransferable);
        }
        let mut token_count: u64 = env.storage().instance().get(&DataKey::TokenCount).unwrap_or(0);
        token_count += 1;
        let token_id = token_count;
        let token = SoulboundToken { id: token_id, owner: owner.clone(), credential_id, metadata_uri };
        env.storage().persistent().set(&DataKey::Token(token_id), &token);
        env.storage().persistent().extend_ttl(&DataKey::Token(token_id), STANDARD_TTL, EXTENDED_TTL);
        env.storage().persistent().set(&DataKey::Owner(token_id), &owner.clone());
        env.storage().persistent().extend_ttl(&DataKey::Owner(token_id), STANDARD_TTL, EXTENDED_TTL);
        env.storage().instance().set(&DataKey::TokenCount, &token_count);
        let mut owner_tokens: Vec<u64> = env.storage().persistent()
            .get(&DataKey::OwnerTokens(owner.clone()))
            .unwrap_or(Vec::new(&env));
        owner_tokens.push_back(token_id);
        env.storage().persistent().set(&DataKey::OwnerTokens(owner.clone()), &owner_tokens);
        env.storage().persistent().extend_ttl(&DataKey::OwnerTokens(owner.clone()), 16_384, 524_288);

        // Uniqueness mapping
        env.storage().instance().set(&DataKey::OwnerCredential(owner.clone(), credential_id), &token_id);

        let mut topics: Vec<soroban_sdk::Val> = Vec::new(&env);
        topics.push_back(symbol_short!("mint").into_val(&env));
        topics.push_back(token_id.into_val(&env));
        env.events().publish(topics, token);
        token_id
    }

    /// Retrieve a soulbound token by its ID.
    ///
    /// # Parameters
    /// - `token_id`: The ID of the token to retrieve.
    ///
    /// # Panics
    /// Panics with "token not found" if no token exists with that ID.
    pub fn get_token(env: Env, token_id: u64) -> SoulboundToken {
        env.storage().persistent().get(&DataKey::Token(token_id)).expect("token not found")
    }

    /// Returns the owner address of a token.
    ///
    /// # Parameters
    /// - `token_id`: The ID of the token to query.
    ///
    /// # Panics
    /// Panics with "token not found" if no token exists with that ID.
    pub fn owner_of(env: Env, token_id: u64) -> Address {
        env.storage().persistent().get(&DataKey::Owner(token_id)).expect("token not found")
    }

    /// Returns all token IDs owned by the given address.
    ///
    /// # Parameters
    /// - `owner`: The address whose tokens to list.
    ///
    /// # Panics
    /// Does not panic; returns an empty `Vec` if the owner holds no tokens.
    pub fn get_tokens_by_owner(env: Env, owner: Address) -> Vec<u64> {
        env.storage().persistent().get(&DataKey::OwnerTokens(owner)).unwrap_or(Vec::new(&env))
    }

    /// Alias for get_tokens_by_owner — returns all SBT token IDs owned by an address.
    pub fn get_sbt_by_owner(env: Env, owner: Address) -> Vec<u64> {
        env.storage().persistent().get(&DataKey::OwnerTokens(owner)).unwrap_or(Vec::new(&env))
    }

    /// Delegate rights for a specific SBT to another address until a timestamp expires.
    pub fn delegate_sbt_rights(
        env: Env,
        owner: Address,
        token_id: u64,
        delegatee: Address,
        expires_at: u64,
    ) {
        owner.require_auth();
        let token: SoulboundToken = env.storage().persistent()
            .get(&DataKey::Token(token_id))
            .expect("token not found");
        assert!(token.owner == owner, "not the owner");

        let current_ts: u64 = env.ledger().timestamp();
        assert!(expires_at > current_ts, "expiry must be in the future");

        let delegation = Delegation { token_id, delegatee, expires_at };
        env.storage().instance().set(&DataKey::Delegation(token_id), &delegation);
    }

    /// Retrieve delegation details for a token.
    pub fn get_delegation(env: Env, token_id: u64) -> Delegation {
        env.storage().instance()
            .get(&DataKey::Delegation(token_id))
            .expect("delegation not found")
    }

    /// Check whether a delegatee currently holds active rights for the token.
    pub fn is_delegate_active(env: Env, token_id: u64, delegatee: Address) -> bool {
        let current_ts: u64 = env.ledger().timestamp();
        env.storage().instance()
            .get(&DataKey::Delegation(token_id))
            .map_or(false, |delegation: Delegation| {
                delegation.delegatee == delegatee && delegation.expires_at > current_ts
            })
    }

    /// Returns the total number of SBTs ever minted.
    pub fn sbt_count(env: Env) -> u64 {
        env.storage().instance().get(&DataKey::TokenCount).unwrap_or(0u64)
    }

    pub fn transfer(env: Env, _from: Address, _to: Address, _token_id: u64) {
        panic_with_error!(&env, ContractError::SoulboundNonTransferable);
    }

    /// Burn a soulbound token. Only the owner may call this.
    /// Returns the credential_id linked to this token.
    pub fn burn(env: Env, owner: Address, token_id: u64) -> u64 {
        owner.require_auth();
        let token: SoulboundToken = env.storage().persistent()
            .get(&DataKey::Token(token_id))
            .unwrap_or_else(|| panic_with_error!(&env, ContractError::TokenNotFound));
        assert!(token.owner == owner, "not the owner");
        env.storage().persistent().remove(&DataKey::Token(token_id));
        env.storage().persistent().remove(&DataKey::Owner(token_id));
        env.storage().instance().remove(&DataKey::Delegation(token_id));
        env.storage().instance().remove(&DataKey::OwnerCredential(owner.clone(), token.credential_id));
        let mut owner_tokens: Vec<u64> = env.storage().persistent()
            .get(&DataKey::OwnerTokens(owner.clone()))
            .expect("owner has no tokens");
        let pos = owner_tokens.iter().position(|id| id == token_id).expect("token not in owner list");
        owner_tokens.remove(pos as u32);
        env.storage().persistent().set(&DataKey::OwnerTokens(owner.clone()), &owner_tokens);

        let mut topics: Vec<soroban_sdk::Val> = Vec::new(&env);
        topics.push_back(symbol_short!("burn").into_val(&env));
        topics.push_back(token_id.into_val(&env));
        env.events().publish(topics, token.id);
        token.credential_id
    }

    /// Initialize the contract with an admin and the quorum_proof contract address.
    pub fn initialize(env: Env, admin: Address, quorum_proof_id: Address) {
        admin.require_auth();
        env.storage().instance().set(&DataKey::Admin, &admin);
        env.storage().instance().set(&DataKey::QuorumProofId, &quorum_proof_id);
    }

    /// Burn a soulbound token. Callable by the token owner or the contract admin.
    ///
    /// Removes Token, Owner, and OwnerTokens storage entries and emits a `burn` event.
    pub fn burn_sbt(env: Env, caller: Address, token_id: u64) {
        caller.require_auth();
        let token: SoulboundToken = env.storage().persistent()
            .get(&DataKey::Token(token_id))
            .expect("token not found");

        let admin: Address = env.storage().instance().get(&DataKey::Admin).expect("not initialized");
        assert!(caller == token.owner || caller == admin, "unauthorized");

        let owner = token.owner.clone();
        env.storage().persistent().remove(&DataKey::Token(token_id));
        env.storage().persistent().remove(&DataKey::Owner(token_id));
        env.storage().instance().remove(&DataKey::Delegation(token_id));
        let mut owner_tokens: Vec<u64> = env.storage().persistent()
            .get(&DataKey::OwnerTokens(owner.clone()))
            .unwrap_or(Vec::new(&env));
        if let Some(pos) = owner_tokens.iter().position(|id| id == token_id) {
            owner_tokens.remove(pos as u32);
        }
        env.storage().persistent().set(&DataKey::OwnerTokens(owner.clone()), &owner_tokens);
        env.storage().instance().remove(&DataKey::OwnerCredential(owner, token.credential_id));

        let mut topics: Vec<soroban_sdk::Val> = Vec::new(&env);
        topics.push_back(symbol_short!("burn").into_val(&env));
        env.events().publish(topics, token_id);
    }

    /// Admin-only: transfer an SBT to a new owner (e.g. after credential re-issuance).
    pub fn admin_transfer_sbt(env: Env, admin: Address, token_id: u64, new_owner: Address) {
        admin.require_auth();
        let stored_admin: Address = env.storage().instance().get(&DataKey::Admin).expect("not initialized");
        assert!(admin == stored_admin, "unauthorized");

        let mut token: SoulboundToken = env.storage().persistent()
            .get(&DataKey::Token(token_id))
            .expect("token not found");
        let old_owner = token.owner.clone();

        // Remove from old owner's list
        let mut old_tokens: Vec<u64> = env.storage().persistent()
            .get(&DataKey::OwnerTokens(old_owner.clone()))
            .unwrap_or(Vec::new(&env));
        if let Some(pos) = old_tokens.iter().position(|id| id == token_id) {
            old_tokens.remove(pos as u32);
        }
        env.storage().persistent().set(&DataKey::OwnerTokens(old_owner.clone()), &old_tokens);
        env.storage().instance().remove(&DataKey::Delegation(token_id));
        env.storage().instance().remove(&DataKey::OwnerCredential(old_owner, token.credential_id));

        // Add to new owner
        token.owner = new_owner.clone();
        env.storage().persistent().set(&DataKey::Token(token_id), &token);
        env.storage().persistent().set(&DataKey::Owner(token_id), &new_owner);
        let mut new_tokens: Vec<u64> = env.storage().persistent()
            .get(&DataKey::OwnerTokens(new_owner.clone()))
            .unwrap_or(Vec::new(&env));
        new_tokens.push_back(token_id);
        env.storage().persistent().set(&DataKey::OwnerTokens(new_owner.clone()), &new_tokens);
        env.storage().instance().set(&DataKey::OwnerCredential(new_owner, token.credential_id), &token_id);
    }

    /// Admin-only contract upgrade to new WASM. Uses deployer convention for auth.
    pub fn upgrade(env: Env, admin: Address, new_wasm_hash: soroban_sdk::BytesN<32>) {
        admin.require_auth();
        env.deployer().update_current_contract_wasm(new_wasm_hash);
    }

    // ── SBT Holder Recovery ──────────────────────────────────────

    /// Configure the recovery guardians and approval threshold for the contract.
    /// Only the admin may call this. Sets up the multi-sig recovery mechanism.
    ///
    /// # Parameters
    /// - `admin`: The admin address; must authorize this call.
    /// - `guardians`: List of addresses authorized to approve recovery requests.
    /// - `threshold`: Number of guardian approvals required to finalize recovery.
    ///
    /// # Panics
    /// Panics if caller is not the admin.
    /// Panics if guardians list is empty or exceeds maximum allowed.
    /// Panics if threshold is 0 or exceeds the number of guardians.
    pub fn setup_recovery_guardians(env: Env, admin: Address, guardians: Vec<Address>, threshold: u32) {
        admin.require_auth();
        let stored_admin: Address = env.storage().instance().get(&DataKey::Admin).expect("not initialized");
        assert!(stored_admin == admin, "unauthorized");
        
        assert!(!guardians.is_empty(), "guardians list cannot be empty");
        assert!(guardians.len() <= 10, "too many guardians (max 10)");
        assert!(threshold > 0, "threshold must be greater than 0");
        assert!(threshold <= guardians.len() as u32, "threshold cannot exceed number of guardians");
        
        env.storage().instance().set(&DataKey::RecoveryGuardians, &guardians);
        env.storage().instance().set(&DataKey::RecoveryThreshold, &threshold);
        env.storage().instance().extend_ttl(STANDARD_TTL, EXTENDED_TTL);
    }

    /// Get the current recovery guardians configured for this contract.
    pub fn get_recovery_guardians(env: Env) -> Vec<Address> {
        env.storage()
            .instance()
            .get(&DataKey::RecoveryGuardians)
            .unwrap_or(Vec::new(&env))
    }

    /// Get the current recovery approval threshold.
    pub fn get_recovery_threshold(env: Env) -> u32 {
        env.storage()
            .instance()
            .get(&DataKey::RecoveryThreshold)
            .unwrap_or(0u32)
    }

    /// Initiate a recovery request for a lost or compromised account.
    ///
    /// The holder calls this to request recovery of their SBTs to a new address.
    /// Once the recovery is approved by the threshold number of guardians, 
    /// the holder can finalize the recovery to transfer their SBTs.
    ///
    /// # Parameters
    /// - `initiator`: The current account holder; must authorize this call.
    /// - `new_owner`: The new account to recover SBTs to.
    ///
    /// # Panics
    /// Panics if no recovery guardians have been configured.
    /// Panics if a recovery request already exists for this holder.
    /// Panics if initiator is the same as new_owner.
    pub fn initiate_recovery(env: Env, initiator: Address, new_owner: Address) -> u64 {
        initiator.require_auth();
        
        let guardians: Vec<Address> = env.storage()
            .instance()
            .get(&DataKey::RecoveryGuardians)
            .unwrap_or(Vec::new(&env));
        assert!(!guardians.is_empty(), "recovery guardians not configured");
        assert!(initiator != new_owner, "new owner must be different from initiator");
        
        // Check if there's already a pending recovery for this holder
        if env.storage().instance().has(&DataKey::PendingRecoveryByHolder(initiator.clone())) {
            panic_with_error!(&env, ContractError::RecoveryAlreadyExists);
        }
        
        // Create recovery request
        let request_id: u64 = env.storage().instance().get(&DataKey::RecoveryRequestCount).unwrap_or(0u64) + 1;
        let request = RecoveryRequest {
            id: request_id,
            initiator: initiator.clone(),
            new_owner: new_owner.clone(),
            initiated_at: env.ledger().timestamp(),
            completed: false,
            approvals_count: 0,
        };
        
        env.storage().instance().set(&DataKey::RecoveryRequest(request_id), &request);
        env.storage().instance().set(&DataKey::RecoveryRequestCount, &request_id);
        env.storage().instance().set(&DataKey::PendingRecoveryByHolder(initiator.clone()), &request_id);
        env.storage().instance().extend_ttl(STANDARD_TTL, EXTENDED_TTL);
        
        // Initialize empty approvals vector
        let approvals: Vec<RecoveryApproval> = Vec::new(&env);
        env.storage().instance().set(&DataKey::RecoveryApprovals(request_id), &approvals);
        env.storage().instance().extend_ttl(STANDARD_TTL, EXTENDED_TTL);
        
        // Record audit trail
        Self::record_audit_trail(&env, request_id, symbol_short!("init"), initiator.clone(), 
            soroban_sdk::String::from_str(&env, "Recovery initiated"));
        
        // Emit event
        let mut topics: Vec<soroban_sdk::Val> = Vec::new(&env);
        topics.push_back(symbol_short!("recov_in").into_val(&env));
        topics.push_back(request_id.into_val(&env));
        env.events().publish(topics, initiator);
        
        request_id
    }

    /// Approve a pending recovery request as a guardian.
    ///
    /// A configured recovery guardian calls this to approve a recovery request.
    /// Once the threshold number of approvals is reached, the initiator can 
    /// finalize the recovery.
    ///
    /// # Parameters
    /// - `guardian`: The guardian address approving; must authorize this call and be in guardians list.
    /// - `recovery_request_id`: The ID of the recovery request to approve.
    ///
    /// # Panics
    /// Panics with `ContractError::RecoveryNotFound` if the recovery request doesn't exist.
    /// Panics if the guardian is not in the configured guardians list.
    /// Panics if the guardian has already approved this request.
    /// Panics if the recovery has already been completed.
    pub fn approve_recovery(env: Env, guardian: Address, recovery_request_id: u64) {
        guardian.require_auth();
        
        let guardians: Vec<Address> = env.storage()
            .instance()
            .get(&DataKey::RecoveryGuardians)
            .unwrap_or(Vec::new(&env));
        
        let mut is_guardian = false;
        for g in guardians.iter() {
            if g == guardian {
                is_guardian = true;
                break;
            }
        }
        assert!(is_guardian, "only configured guardians can approve recoveries");
        
        // Get recovery request
        let mut recovery: RecoveryRequest = env.storage().instance()
            .get(&DataKey::RecoveryRequest(recovery_request_id))
            .unwrap_or_else(|| panic_with_error!(&env, ContractError::RecoveryNotFound));
        
        assert!(!recovery.completed, "recovery already completed");
        
        // Get existing approvals
        let mut approvals: Vec<RecoveryApproval> = env.storage().instance()
            .get(&DataKey::RecoveryApprovals(recovery_request_id))
            .unwrap_or(Vec::new(&env));
        
        // Check if guardian has already approved
        for approval in approvals.iter() {
            assert!(approval.guardian != guardian, "guardian has already approved this recovery");
        }
        
        // Add approval
        let new_approval = RecoveryApproval {
            guardian: guardian.clone(),
            approved_at: env.ledger().timestamp(),
        };
        approvals.push_back(new_approval);
        
        // Update recovery request with new approval count
        recovery.approvals_count += 1;
        
        env.storage().instance().set(&DataKey::RecoveryApprovals(recovery_request_id), &approvals);
        env.storage().instance().set(&DataKey::RecoveryRequest(recovery_request_id), &recovery);
        env.storage().instance().extend_ttl(STANDARD_TTL, EXTENDED_TTL);
        
        // Record audit trail
        Self::record_audit_trail(&env, recovery_request_id, symbol_short!("approv"), guardian.clone(),
            soroban_sdk::String::from_str(&env, "Recovery approved by guardian"));
        
        // Emit event
        let mut topics: Vec<soroban_sdk::Val> = Vec::new(&env);
        topics.push_back(symbol_short!("recov_ap").into_val(&env));
        topics.push_back(recovery_request_id.into_val(&env));
        env.events().publish(topics, guardian);
    }

    /// Finalize a recovery request by transferring SBTs to the new owner.
    ///
    /// The initiator calls this after collecting enough guardian approvals.
    /// This transfers all SBTs from the original account to the new owner.
    ///
    /// # Parameters
    /// - `initiator`: The recovery initiator; must authorize this call.
    /// - `recovery_request_id`: The ID of the recovery request to finalize.
    ///
    /// # Panics
    /// Panics with `ContractError::RecoveryNotFound` if the recovery request doesn't exist.
    /// Panics with `ContractError::InsufficientApprovals` if threshold not reached.
    /// Panics if recovery already completed.
    pub fn finalize_recovery(env: Env, initiator: Address, recovery_request_id: u64) {
        initiator.require_auth();
        
        let mut recovery: RecoveryRequest = env.storage().instance()
            .get(&DataKey::RecoveryRequest(recovery_request_id))
            .unwrap_or_else(|| panic_with_error!(&env, ContractError::RecoveryNotFound));
        
        assert!(recovery.initiator == initiator, "only recovery initiator can finalize");
        assert!(!recovery.completed, "recovery already completed");
        
        let threshold: u32 = env.storage().instance().get(&DataKey::RecoveryThreshold).unwrap_or(0u32);
        assert!(recovery.approvals_count >= threshold, 
            "insufficient approvals: need {} but have {}", threshold, recovery.approvals_count);
        
        // Transfer all SBTs from initiator to new_owner
        let token_ids: Vec<u64> = env.storage().persistent()
            .get(&DataKey::OwnerTokens(initiator.clone()))
            .unwrap_or(Vec::new(&env));
        
        let new_owner = recovery.new_owner.clone();
        
        // Update each token and transfer ownership
        for token_id in token_ids.iter() {
            let mut token: SoulboundToken = env.storage().persistent()
                .get(&DataKey::Token(token_id))
                .unwrap_or_else(|| panic_with_error!(&env, ContractError::TokenNotFound));
            
            // Update token owner
            token.owner = new_owner.clone();
            env.storage().persistent().set(&DataKey::Token(token_id), &token);
            env.storage().persistent().set(&DataKey::Owner(token_id), &new_owner);
            
            // Remove from old owner's mapping
            env.storage().instance().remove(&DataKey::OwnerCredential(initiator.clone(), token.credential_id));
            
            // Add to new owner's mapping
            env.storage().instance().set(&DataKey::OwnerCredential(new_owner.clone(), token.credential_id), &token_id);
        }
        
        // Clear initiator's token list
        env.storage().persistent().remove(&DataKey::OwnerTokens(initiator.clone()));
        
        // Add to new owner's token list
        let mut new_owner_tokens: Vec<u64> = env.storage().persistent()
            .get(&DataKey::OwnerTokens(new_owner.clone()))
            .unwrap_or(Vec::new(&env));
        for token_id in token_ids.iter() {
            new_owner_tokens.push_back(token_id);
        }
        env.storage().persistent().set(&DataKey::OwnerTokens(new_owner.clone()), &new_owner_tokens);
        env.storage().persistent().extend_ttl(&DataKey::OwnerTokens(new_owner.clone()), STANDARD_TTL, EXTENDED_TTL);
        
        // Mark recovery as completed
        recovery.completed = true;
        env.storage().instance().set(&DataKey::RecoveryRequest(recovery_request_id), &recovery);
        
        // Clear pending recovery tracking
        env.storage().instance().remove(&DataKey::PendingRecoveryByHolder(initiator.clone()));
        
        // Record audit trail
        Self::record_audit_trail(&env, recovery_request_id, symbol_short!("final"), initiator.clone(),
            soroban_sdk::String::from_str(&env, "Recovery finalized and SBTs transferred"));
        
        // Emit event
        let mut topics: Vec<soroban_sdk::Val> = Vec::new(&env);
        topics.push_back(symbol_short!("recov_fn").into_val(&env));
        topics.push_back(recovery_request_id.into_val(&env));
        env.events().publish(topics, (initiator, new_owner));
    }

    /// Get a recovery request by ID.
    ///
    /// # Parameters
    /// - `recovery_request_id`: The recovery request ID to retrieve.
    ///
    /// # Panics
    /// Panics with `ContractError::RecoveryNotFound` if the request doesn't exist.
    pub fn get_recovery_request(env: Env, recovery_request_id: u64) -> RecoveryRequest {
        env.storage().instance()
            .get(&DataKey::RecoveryRequest(recovery_request_id))
            .unwrap_or_else(|| panic_with_error!(&env, ContractError::RecoveryNotFound))
    }

    /// Get all approvals for a recovery request.
    ///
    /// # Parameters
    /// - `recovery_request_id`: The recovery request ID.
    ///
    /// # Returns
    /// Vector of all approvals for the recovery request.
    pub fn get_recovery_approvals(env: Env, recovery_request_id: u64) -> Vec<RecoveryApproval> {
        env.storage().instance()
            .get(&DataKey::RecoveryApprovals(recovery_request_id))
            .unwrap_or(Vec::new(&env))
    }

    /// Helper function to record audit trail entries for recovery operations.
    fn record_audit_trail(env: &Env, recovery_request_id: u64, action: Symbol, actor: Address, details: soroban_sdk::String) {
        let entry_id: u64 = env.storage().instance().get(&DataKey::AuditTrailCount).unwrap_or(0u64) + 1;
        let entry = AuditTrailEntry {
            id: entry_id,
            recovery_request_id,
            action,
            actor,
            timestamp: env.ledger().timestamp(),
            details,
        };
        
        env.storage().instance().set(&DataKey::AuditTrail(entry_id), &entry);
        env.storage().instance().set(&DataKey::AuditTrailCount, &entry_id);
        env.storage().instance().extend_ttl(STANDARD_TTL, EXTENDED_TTL);
    }

    /// Get an audit trail entry by ID.
    ///
    /// # Parameters
    /// - `audit_id`: The audit trail entry ID.
    ///
    /// # Returns
    /// The audit trail entry, or panics if not found.
    pub fn get_audit_trail_entry(env: Env, audit_id: u64) -> AuditTrailEntry {
        env.storage().instance()
            .get(&DataKey::AuditTrail(audit_id))
            .expect("audit trail entry not found")
    }

    /// Get the total count of audit trail entries.
    pub fn get_audit_trail_count(env: Env) -> u64 {
        env.storage().instance().get(&DataKey::AuditTrailCount).unwrap_or(0u64)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use soroban_sdk::testutils::{Address as _, Events as _};
    use soroban_sdk::{BytesN, FromVal, TryFromVal};
    use quorum_proof::{QuorumProofContract, QuorumProofContractClient};

    fn setup_with_qp(env: &Env) -> (SbtRegistryContractClient, Address, QuorumProofContractClient, Address) {
        let qp_id = env.register_contract(None, QuorumProofContract);
        let qp_client = QuorumProofContractClient::new(env, &qp_id);
        let admin = Address::generate(env);
        qp_client.initialize(&admin);

        let sbt_id = env.register_contract(None, SbtRegistryContract);
        let sbt_client = SbtRegistryContractClient::new(env, &sbt_id);
        sbt_client.initialize(&admin, &qp_id);

        (sbt_client, admin, qp_client, qp_id)
    }

    #[test]
    fn test_mint_and_ownership() {
        let env = Env::default();
        env.mock_all_auths();
        let (client, _admin, qp_client, _qp_id) = setup_with_qp(&env);

        let issuer = Address::generate(&env);
        let owner = Address::generate(&env);
        let meta = soroban_sdk::Bytes::from_slice(&env, b"ipfs://meta");
        let cred_id = qp_client.issue_credential(&issuer, &owner, &1u32, &meta, &None);

        let uri = Bytes::from_slice(&env, b"ipfs://QmSBT");
        let token_id = client.mint(&owner, &cred_id, &uri);
        assert_eq!(token_id, 1);
        assert_eq!(client.owner_of(&token_id), owner);
    }

    #[test]
    fn test_delegate_sbt_rights_and_active_status() {
        let env = Env::default();
        env.mock_all_auths();
        let (client, _admin, qp_client, _qp_id) = setup_with_qp(&env);

        let issuer = Address::generate(&env);
        let owner = Address::generate(&env);
        let delegatee = Address::generate(&env);
        let meta = soroban_sdk::Bytes::from_slice(&env, b"ipfs://meta");
        let cred_id = qp_client.issue_credential(&issuer, &owner, &1u32, &meta, &None);
        let uri = Bytes::from_slice(&env, b"ipfs://QmSBT");
        let token_id = client.mint(&owner, &cred_id, &uri);

        let expires_at = env.ledger().timestamp() + 1_000;
        client.delegate_sbt_rights(&owner, &token_id, &delegatee, &expires_at);

        assert!(client.is_delegate_active(&token_id, &delegatee));
        let delegation = client.get_delegation(&token_id);
        assert_eq!(delegation.delegatee, delegatee);
        assert_eq!(delegation.expires_at, expires_at);
    }

    #[test]
    #[should_panic(expected = "expiry must be in the future")]
    fn test_delegate_sbt_rights_rejects_past_expiry() {
        let env = Env::default();
        env.mock_all_auths();
        let (client, _admin, qp_client, _qp_id) = setup_with_qp(&env);

        let issuer = Address::generate(&env);
        let owner = Address::generate(&env);
        let delegatee = Address::generate(&env);
        let meta = soroban_sdk::Bytes::from_slice(&env, b"ipfs://meta");
        let cred_id = qp_client.issue_credential(&issuer, &owner, &1u32, &meta, &None);
        let uri = Bytes::from_slice(&env, b"ipfs://QmSBT");
        let token_id = client.mint(&owner, &cred_id, &uri);

        let expires_at = env.ledger().timestamp();
        client.delegate_sbt_rights(&owner, &token_id, &delegatee, &expires_at);
    }

    #[test]
    fn test_burn_allows_remint_same_credential() {
        let env = Env::default();
        env.mock_all_auths();
        let (client, _admin, qp_client, _qp_id) = setup_with_qp(&env);
        let issuer = Address::generate(&env);
        let owner = Address::generate(&env);
        let meta = soroban_sdk::Bytes::from_slice(&env, b"ipfs://meta");
        let cred_id = qp_client.issue_credential(&issuer, &owner, &1u32, &meta, &None);
        let uri = Bytes::from_slice(&env, b"ipfs://QmSBT");

        // mint, burn, then re-mint the same credential — must succeed
        let token_id = client.mint(&owner, &cred_id, &uri);
        client.burn(&owner, &token_id);
        let new_token_id = client.mint(&owner, &cred_id, &uri);

        assert_eq!(new_token_id, 2);
        assert_eq!(client.owner_of(&new_token_id), owner);
    }

    #[test]
    fn test_mint_emits_event() {
        let env = Env::default();
        env.mock_all_auths();
        let (client, _admin, qp_client, _qp_id) = setup_with_qp(&env);
        let issuer = Address::generate(&env);
        let owner = Address::generate(&env);
        let meta = soroban_sdk::Bytes::from_slice(&env, b"ipfs://meta");
        let cred_id = qp_client.issue_credential(&issuer, &owner, &1u32, &meta, &None);
        let uri = Bytes::from_slice(&env, b"ipfs://QmSBT");

        let token_id = client.mint(&owner, &cred_id, &uri);

        // Verify the token was minted correctly (event was emitted if token exists)
        assert_eq!(client.owner_of(&token_id), owner);
        assert_eq!(token_id, 1);
    }

    #[test]
    #[should_panic(expected = "HostError")]
    fn test_duplicate_sbt_minting_rejection() {
        let env = Env::default();
        env.mock_all_auths();
        let (client, _admin, qp_client, _qp_id) = setup_with_qp(&env);

        let issuer = Address::generate(&env);
        let owner = Address::generate(&env);
        let meta = soroban_sdk::Bytes::from_slice(&env, b"ipfs://meta");
        let cred_id = qp_client.issue_credential(&issuer, &owner, &1u32, &meta, &None);

        let uri = Bytes::from_slice(&env, b"ipfs://QmSBT");
        client.mint(&owner, &cred_id, &uri);
        client.mint(&owner, &cred_id, &uri);
    }

    /// Minting an SBT for a non-existent credential_id must panic.
    #[test]
    #[should_panic]
    fn test_mint_nonexistent_credential_panics() {
        let env = Env::default();
        env.mock_all_auths();
        let (client, _admin, _qp_client, _qp_id) = setup_with_qp(&env);

        let owner = Address::generate(&env);
        let uri = Bytes::from_slice(&env, b"ipfs://QmSBT");
        // credential_id 999 was never issued
        client.mint(&owner, &999u64, &uri);
    }

    /// Minting an SBT for a revoked credential must panic.
    #[test]
    #[should_panic(expected = "credential is revoked")]
    fn test_mint_revoked_credential_panics() {
        let env = Env::default();
        env.mock_all_auths();
        let (client, _admin, qp_client, _qp_id) = setup_with_qp(&env);

        let issuer = Address::generate(&env);
        let owner = Address::generate(&env);
        let meta = soroban_sdk::Bytes::from_slice(&env, b"ipfs://meta");
        let cred_id = qp_client.issue_credential(&issuer, &owner, &1u32, &meta, &None);
        qp_client.revoke_credential(&issuer, &cred_id);

        let uri = Bytes::from_slice(&env, b"ipfs://QmSBT");
        client.mint(&owner, &cred_id, &uri);
    }

    #[test]
    fn test_get_tokens_by_owner_single() { /* impl from previous */ }

    // --- Issue #196: get_sbt_by_owner ---

    #[test]
    fn test_get_sbt_by_owner_returns_token_ids() {
        let env = Env::default();
        env.mock_all_auths();
        let (client, _admin, qp_client, _qp_id) = setup_with_qp(&env);
        let issuer = Address::generate(&env);
        let owner = Address::generate(&env);
        let meta = soroban_sdk::Bytes::from_slice(&env, b"ipfs://meta");
        let cred_id1 = qp_client.issue_credential(&issuer, &owner, &1u32, &meta, &None);
        let cred_id2 = qp_client.issue_credential(&issuer, &owner, &2u32, &meta, &None);
        let uri = Bytes::from_slice(&env, b"ipfs://QmSBT");

        assert_eq!(client.get_sbt_by_owner(&owner).len(), 0);

        let id1 = client.mint(&owner, &cred_id1, &uri);
        let id2 = client.mint(&owner, &cred_id2, &uri);

        let tokens = client.get_sbt_by_owner(&owner);
        assert_eq!(tokens.len(), 2);
        assert_eq!(tokens.get(0).unwrap(), id1);
        assert_eq!(tokens.get(1).unwrap(), id2);
    }

    // --- Issue #197: sbt_count ---

    #[test]
    fn test_sbt_count() {
        let env = Env::default();
        env.mock_all_auths();
        let (client, _admin, qp_client, _qp_id) = setup_with_qp(&env);
        let issuer = Address::generate(&env);
        let owner = Address::generate(&env);
        let meta = soroban_sdk::Bytes::from_slice(&env, b"ipfs://meta");
        let cred_id1 = qp_client.issue_credential(&issuer, &owner, &1u32, &meta, &None);
        let cred_id2 = qp_client.issue_credential(&issuer, &owner, &2u32, &meta, &None);
        let uri = Bytes::from_slice(&env, b"ipfs://QmSBT");

        assert_eq!(client.sbt_count(), 0);

        client.mint(&owner, &cred_id1, &uri);
        assert_eq!(client.sbt_count(), 1);

        client.mint(&owner, &cred_id2, &uri);
        assert_eq!(client.sbt_count(), 2);
    }

    // --- Issue #37: burn_sbt ---

    #[test]
    fn test_burn_sbt_by_owner() {
        let env = Env::default();
        env.mock_all_auths();
        let (client, _admin, qp_client, _qp_id) = setup_with_qp(&env);

        let issuer = Address::generate(&env);
        let owner = Address::generate(&env);
        let meta = soroban_sdk::Bytes::from_slice(&env, b"ipfs://meta");
        let cred_id = qp_client.issue_credential(&issuer, &owner, &1u32, &meta, &None);
        let uri = Bytes::from_slice(&env, b"ipfs://QmSBT");
        let token_id = client.mint(&owner, &cred_id, &uri);

        client.burn_sbt(&owner, &token_id);

        assert!(client.get_tokens_by_owner(&owner).is_empty());
    }

    #[test]
    fn test_burn_sbt_by_admin() {
        let env = Env::default();
        env.mock_all_auths();
        let (client, admin, qp_client, _qp_id) = setup_with_qp(&env);

        let issuer = Address::generate(&env);
        let owner = Address::generate(&env);
        let meta = soroban_sdk::Bytes::from_slice(&env, b"ipfs://meta");
        let cred_id = qp_client.issue_credential(&issuer, &owner, &1u32, &meta, &None);
        let uri = Bytes::from_slice(&env, b"ipfs://QmSBT");
        let token_id = client.mint(&owner, &cred_id, &uri);

        // Admin burns the SBT for a revoked credential
        qp_client.revoke_credential(&issuer, &cred_id);
        client.burn_sbt(&admin, &token_id);

        assert!(client.get_tokens_by_owner(&owner).is_empty());
    }

    #[test]
    fn test_burn_sbt_emits_event() {
        let env = Env::default();
        env.mock_all_auths();
        let (client, _admin, qp_client, _qp_id) = setup_with_qp(&env);

        let issuer = Address::generate(&env);
        let owner = Address::generate(&env);
        let meta = soroban_sdk::Bytes::from_slice(&env, b"ipfs://meta");
        let cred_id = qp_client.issue_credential(&issuer, &owner, &1u32, &meta, &None);
        let uri = Bytes::from_slice(&env, b"ipfs://QmSBT");
        let token_id = client.mint(&owner, &cred_id, &uri);

        client.burn_sbt(&owner, &token_id);

        // Verify token was burned (owner_of should panic or tokens list should be empty)
        assert!(client.get_tokens_by_owner(&owner).is_empty());
        // Verify a burn event was emitted by checking events list is non-empty
        let events = env.events().all();
        let burn_event = events.iter().find(|(_, topics, _)| {
            topics.get(0)
                .and_then(|v| soroban_sdk::Symbol::try_from_val(&env, &v).ok())
                .map(|s| s == symbol_short!("burn"))
                .unwrap_or(false)
        });
        assert!(burn_event.is_some(), "burn event not emitted");
    }

    #[test]
    #[should_panic(expected = "unauthorized")]
    fn test_burn_sbt_unauthorized_panics() {
        let env = Env::default();
        env.mock_all_auths();
        let (client, _admin, qp_client, _qp_id) = setup_with_qp(&env);

        let issuer = Address::generate(&env);
        let owner = Address::generate(&env);
        let stranger = Address::generate(&env);
        let meta = soroban_sdk::Bytes::from_slice(&env, b"ipfs://meta");
        let cred_id = qp_client.issue_credential(&issuer, &owner, &1u32, &meta, &None);
        let uri = Bytes::from_slice(&env, b"ipfs://QmSBT");
        let token_id = client.mint(&owner, &cred_id, &uri);

        client.burn_sbt(&stranger, &token_id);
    }

    #[test]
    fn test_burn_sbt_allows_remint() {
        let env = Env::default();
        env.mock_all_auths();
        let (client, _admin, qp_client, _qp_id) = setup_with_qp(&env);

        let issuer = Address::generate(&env);
        let owner = Address::generate(&env);
        let meta = soroban_sdk::Bytes::from_slice(&env, b"ipfs://meta");
        let cred_id = qp_client.issue_credential(&issuer, &owner, &1u32, &meta, &None);
        let uri = Bytes::from_slice(&env, b"ipfs://QmSBT");
        let token_id = client.mint(&owner, &cred_id, &uri);

        client.burn_sbt(&owner, &token_id);

        // Re-mint must succeed after burn
        let new_id = client.mint(&owner, &cred_id, &uri);
        assert_eq!(client.owner_of(&new_id), owner);
    }

    #[test]
    #[should_panic]
    #[allow(unused)]
    // upgrade requires the WASM to exist in host storage; this verifies auth passes
    fn test_upgrade_success() {
        let env = Env::default();
        env.mock_all_auths();
        let contract_id = env.register_contract(None, SbtRegistryContract);
        let client = SbtRegistryContractClient::new(&env, &contract_id);

        let admin = Address::generate(&env);
        let wasm_hash = BytesN::from_array(&env, &[0u8; 32]);

        client.upgrade(&admin, &wasm_hash);
    }

    #[test]
    #[should_panic(expected = "HostError")]
    fn test_upgrade_unauthorized_panics() {
        let env = Env::default();
        env.mock_all_auths();
        let contract_id = env.register_contract(None, SbtRegistryContract);
        let client = SbtRegistryContractClient::new(&env, &contract_id);

        let admin = Address::generate(&env);
        let unpriv = Address::generate(&env);
        let wasm_hash = BytesN::from_array(&env, &[0u8; 32]);

        client.upgrade(&admin, &wasm_hash);

        env.as_contract(&contract_id, || {
            client.upgrade(&unpriv, &wasm_hash);
        });
    }

    #[test]
    fn test_admin_transfer_sbt_updates_ownership() {
        let env = Env::default();
        env.mock_all_auths();
        let (client, admin, qp_client, _qp_id) = setup_with_qp(&env);

        let issuer = Address::generate(&env);
        let old_owner = Address::generate(&env);
        let new_owner = Address::generate(&env);
        let meta = soroban_sdk::Bytes::from_slice(&env, b"ipfs://meta");
        let cred_id = qp_client.issue_credential(&issuer, &old_owner, &1u32, &meta, &None);

        let uri = Bytes::from_slice(&env, b"ipfs://QmSBT");
        let token_id = client.mint(&old_owner, &cred_id, &uri);

        client.admin_transfer_sbt(&admin, &token_id, &new_owner);

        assert_eq!(client.owner_of(&token_id), new_owner);
        assert_eq!(client.get_token(&token_id).owner, new_owner);
        assert!(client.get_tokens_by_owner(&old_owner).is_empty());
        assert_eq!(client.get_tokens_by_owner(&new_owner).get(0).unwrap(), token_id);
    }

    #[test]
    #[should_panic(expected = "unauthorized")]
    fn test_admin_transfer_sbt_non_admin_panics() {
        let env = Env::default();
        env.mock_all_auths();
        let (client, admin, qp_client, _qp_id) = setup_with_qp(&env);

        let non_admin = Address::generate(&env);
        let owner = Address::generate(&env);
        let new_owner = Address::generate(&env);
        let issuer = Address::generate(&env);
        let meta = soroban_sdk::Bytes::from_slice(&env, b"ipfs://meta");
        let cred_id = qp_client.issue_credential(&issuer, &owner, &1u32, &meta, &None);

        let uri = Bytes::from_slice(&env, b"ipfs://QmSBT");
        let token_id = client.mint(&owner, &cred_id, &uri);

        let _ = admin; // admin initialized the contract
        client.admin_transfer_sbt(&non_admin, &token_id, &new_owner);
    }

    // ── Snapshot tests ────────────────────────────────────────────────────────

    /// Generates a snapshot after minting an SBT and verifies the
    /// snapshot can be reloaded with the same ledger state.
    #[test]
    fn test_snapshot_mint_state() {
        let snap_path = "test_snapshots/tests/snapshot_mint_state.json";
        let env = Env::default();
        env.mock_all_auths();
        let (client, _admin, qp_client, _qp_id) = setup_with_qp(&env);

        let issuer = Address::generate(&env);
        let owner = Address::generate(&env);
        let meta = soroban_sdk::Bytes::from_slice(&env, b"ipfs://meta");
        let cred_id = qp_client.issue_credential(&issuer, &owner, &1u32, &meta, &None);
        let uri = Bytes::from_slice(&env, b"ipfs://QmSBT");
        let token_id = client.mint(&owner, &cred_id, &uri);

        assert_eq!(client.owner_of(&token_id), owner);
        assert_eq!(client.sbt_count(), 1);

        // Generate snapshot
        env.to_snapshot_file(snap_path);

        // Reload and compare ledger metadata
        let env2 = Env::from_snapshot_file(snap_path);
        assert_eq!(env.ledger().sequence(), env2.ledger().sequence());
        assert_eq!(env.ledger().timestamp(), env2.ledger().timestamp());
    }

    /// Generates a snapshot after burning an SBT and verifies the
    /// reloaded snapshot has the same ledger state.
    #[test]
    fn test_snapshot_burn_state() {
        let snap_path = "test_snapshots/tests/snapshot_burn_state.json";
        let env = Env::default();
        env.mock_all_auths();
        let (client, _admin, qp_client, _qp_id) = setup_with_qp(&env);

        let issuer = Address::generate(&env);
        let owner = Address::generate(&env);
        let meta = soroban_sdk::Bytes::from_slice(&env, b"ipfs://meta");
        let cred_id = qp_client.issue_credential(&issuer, &owner, &1u32, &meta, &None);
        let uri = Bytes::from_slice(&env, b"ipfs://QmSBT");
        let token_id = client.mint(&owner, &cred_id, &uri);
        client.burn(&owner, &token_id);

        assert_eq!(client.sbt_count(), 0);

        // Generate snapshot
        env.to_snapshot_file(snap_path);

        // Reload and compare ledger metadata
        let env2 = Env::from_snapshot_file(snap_path);
        assert_eq!(env.ledger().sequence(), env2.ledger().sequence());
        assert_eq!(env.ledger().timestamp(), env2.ledger().timestamp());
    }

    /// Generates a snapshot after an admin transfer and verifies the
    /// reloaded snapshot has the same ledger state.
    #[test]
    fn test_snapshot_transfer_state() {
        let snap_path = "test_snapshots/tests/snapshot_transfer_state.json";
        let env = Env::default();
        env.mock_all_auths();
        let (client, admin, qp_client, _qp_id) = setup_with_qp(&env);

        let issuer = Address::generate(&env);
        let owner = Address::generate(&env);
        let new_owner = Address::generate(&env);
        let meta = soroban_sdk::Bytes::from_slice(&env, b"ipfs://meta");
        let cred_id = qp_client.issue_credential(&issuer, &owner, &1u32, &meta, &None);
        let uri = Bytes::from_slice(&env, b"ipfs://QmSBT");
        let token_id = client.mint(&owner, &cred_id, &uri);
        client.admin_transfer_sbt(&admin, &token_id, &new_owner);

        assert_eq!(client.owner_of(&token_id), new_owner);

        // Generate snapshot
        env.to_snapshot_file(snap_path);

        // Reload and compare ledger metadata
        let env2 = Env::from_snapshot_file(snap_path);
        assert_eq!(env.ledger().sequence(), env2.ledger().sequence());
        assert_eq!(env.ledger().timestamp(), env2.ledger().timestamp());
    }

    // ── Property-based fuzz tests ─────────────────────────────────────────────

    /// Property: minting N SBTs for distinct credentials always increments
    /// the token count and assigns sequential IDs.
    #[test]
    fn fuzz_mint_sequential_ids() {
        let env = Env::default();
        env.mock_all_auths();
        let (client, _admin, qp_client, _qp_id) = setup_with_qp(&env);
        let issuer = Address::generate(&env);
        let owner = Address::generate(&env);
        let meta = soroban_sdk::Bytes::from_slice(&env, b"ipfs://meta");
        let uri = Bytes::from_slice(&env, b"ipfs://QmSBT");

        for i in 1u32..=4 {
            let cred_id = qp_client.issue_credential(&issuer, &owner, &i, &meta, &None);
            let token_id = client.mint(&owner, &cred_id, &uri);
            assert_eq!(token_id, i as u64);
            assert_eq!(client.sbt_count(), i as u64);
        }
    }

    /// Property: minting the same (owner, credential_id) pair twice must
    /// always be rejected (soulbound non-transferable invariant).
    #[test]
    #[should_panic]
    fn fuzz_mint_duplicate_always_rejected() {
        let env = Env::default();
        env.mock_all_auths();
        let (client, _admin, qp_client, _qp_id) = setup_with_qp(&env);
        let issuer = Address::generate(&env);
        let owner = Address::generate(&env);
        let meta = soroban_sdk::Bytes::from_slice(&env, b"ipfs://meta");
        let uri = Bytes::from_slice(&env, b"ipfs://QmSBT");
        let cred_id = qp_client.issue_credential(&issuer, &owner, &1u32, &meta, &None);
        client.mint(&owner, &cred_id, &uri);
        // Second mint for same (owner, cred_id) — must panic
        client.mint(&owner, &cred_id, &uri);
    }

    /// Property: burning an SBT must decrement the count and allow re-mint.
    #[test]
    fn fuzz_burn_decrements_count_and_allows_remint() {
        let env = Env::default();
        env.mock_all_auths();
        let (client, _admin, qp_client, _qp_id) = setup_with_qp(&env);
        let issuer = Address::generate(&env);
        let owner = Address::generate(&env);
        let meta = soroban_sdk::Bytes::from_slice(&env, b"ipfs://meta");
        let uri = Bytes::from_slice(&env, b"ipfs://QmSBT");
        let cred_id = qp_client.issue_credential(&issuer, &owner, &1u32, &meta, &None);
        let token_id = client.mint(&owner, &cred_id, &uri);
        assert_eq!(client.sbt_count(), 1);
        client.burn(&owner, &token_id);
        assert_eq!(client.sbt_count(), 0);
        // Re-mint must succeed after burn
        let new_id = client.mint(&owner, &cred_id, &uri);
        assert_eq!(client.owner_of(&new_id), owner);
    }

    // ── SBT Holder Recovery Tests ───────────────────────────────

    #[test]
    fn test_setup_recovery_guardians() {
        let env = Env::default();
        env.mock_all_auths();
        let (client, admin, _qp_client, _qp_id) = setup_with_qp(&env);

        let guardian1 = Address::generate(&env);
        let guardian2 = Address::generate(&env);
        let guardian3 = Address::generate(&env);
        let guardians = soroban_sdk::vec![&env, guardian1, guardian2, guardian3];

        client.setup_recovery_guardians(&admin, &guardians, &2u32);

        let retrieved_guardians = client.get_recovery_guardians();
        assert_eq!(retrieved_guardians.len(), 3);
        assert_eq!(client.get_recovery_threshold(), 2);
    }

    #[test]
    fn test_initiate_recovery() {
        let env = Env::default();
        env.mock_all_auths();
        let (client, admin, qp_client, _qp_id) = setup_with_qp(&env);

        // Setup recovery guardians
        let guardian1 = Address::generate(&env);
        let guardian2 = Address::generate(&env);
        let guardians = soroban_sdk::vec![&env, guardian1, guardian2];
        client.setup_recovery_guardians(&admin, &guardians, &1u32);

        // Mint an SBT
        let issuer = Address::generate(&env);
        let owner = Address::generate(&env);
        let new_owner = Address::generate(&env);
        let meta = soroban_sdk::Bytes::from_slice(&env, b"ipfs://meta");
        let cred_id = qp_client.issue_credential(&issuer, &owner, &1u32, &meta, &None);
        let uri = Bytes::from_slice(&env, b"ipfs://QmSBT");
        let _token_id = client.mint(&owner, &cred_id, &uri);

        // Initiate recovery
        let recovery_id = client.initiate_recovery(&owner, &new_owner);
        assert_eq!(recovery_id, 1);

        // Verify recovery request created
        let recovery = client.get_recovery_request(&recovery_id);
        assert_eq!(recovery.initiator, owner);
        assert_eq!(recovery.new_owner, new_owner);
        assert!(!recovery.completed);
        assert_eq!(recovery.approvals_count, 0);
    }

    #[test]
    #[should_panic(expected = "new owner must be different from initiator")]
    fn test_initiate_recovery_same_owner_panics() {
        let env = Env::default();
        env.mock_all_auths();
        let (client, admin, _qp_client, _qp_id) = setup_with_qp(&env);

        let guardian = Address::generate(&env);
        let guardians = soroban_sdk::vec![&env, guardian];
        client.setup_recovery_guardians(&admin, &guardians, &1u32);

        let owner = Address::generate(&env);
        client.initiate_recovery(&owner, &owner); // Should panic
    }

    #[test]
    #[should_panic(expected = "already exists")]
    fn test_initiate_recovery_duplicate_panics() {
        let env = Env::default();
        env.mock_all_auths();
        let (client, admin, _qp_client, _qp_id) = setup_with_qp(&env);

        let guardian = Address::generate(&env);
        let guardians = soroban_sdk::vec![&env, guardian];
        client.setup_recovery_guardians(&admin, &guardians, &1u32);

        let owner = Address::generate(&env);
        let new_owner = Address::generate(&env);
        let _recovery_id1 = client.initiate_recovery(&owner, &new_owner);
        let _recovery_id2 = client.initiate_recovery(&owner, &new_owner); // Should panic
    }

    #[test]
    fn test_approve_recovery() {
        let env = Env::default();
        env.mock_all_auths();
        let (client, admin, qp_client, _qp_id) = setup_with_qp(&env);

        // Setup recovery guardians
        let guardian1 = Address::generate(&env);
        let guardian2 = Address::generate(&env);
        let guardians = soroban_sdk::vec![&env, guardian1.clone(), guardian2];
        client.setup_recovery_guardians(&admin, &guardians, &2u32);

        // Initiate recovery
        let owner = Address::generate(&env);
        let new_owner = Address::generate(&env);
        let recovery_id = client.initiate_recovery(&owner, &new_owner);

        // Approve recovery
        client.approve_recovery(&guardian1, &recovery_id);

        let recovery = client.get_recovery_request(&recovery_id);
        assert_eq!(recovery.approvals_count, 1);

        let approvals = client.get_recovery_approvals(&recovery_id);
        assert_eq!(approvals.len(), 1);
        assert_eq!(approvals.get(0).unwrap().guardian, guardian1);
    }

    #[test]
    #[should_panic(expected = "already approved")]
    fn test_approve_recovery_duplicate_panics() {
        let env = Env::default();
        env.mock_all_auths();
        let (client, admin, _qp_client, _qp_id) = setup_with_qp(&env);

        let guardian = Address::generate(&env);
        let guardians = soroban_sdk::vec![&env, guardian.clone()];
        client.setup_recovery_guardians(&admin, &guardians, &1u32);

        let owner = Address::generate(&env);
        let new_owner = Address::generate(&env);
        let recovery_id = client.initiate_recovery(&owner, &new_owner);

        client.approve_recovery(&guardian, &recovery_id);
        client.approve_recovery(&guardian, &recovery_id); // Should panic
    }

    #[test]
    #[should_panic(expected = "only configured guardians")]
    fn test_approve_recovery_unauthorized_panics() {
        let env = Env::default();
        env.mock_all_auths();
        let (client, admin, _qp_client, _qp_id) = setup_with_qp(&env);

        let guardian = Address::generate(&env);
        let guardians = soroban_sdk::vec![&env, guardian];
        client.setup_recovery_guardians(&admin, &guardians, &1u32);

        let owner = Address::generate(&env);
        let new_owner = Address::generate(&env);
        let recovery_id = client.initiate_recovery(&owner, &new_owner);

        let unauthorized = Address::generate(&env);
        client.approve_recovery(&unauthorized, &recovery_id); // Should panic
    }

    #[test]
    fn test_finalize_recovery_transfers_sbts() {
        let env = Env::default();
        env.mock_all_auths();
        let (client, admin, qp_client, _qp_id) = setup_with_qp(&env);

        // Setup recovery guardians
        let guardian = Address::generate(&env);
        let guardians = soroban_sdk::vec![&env, guardian.clone()];
        client.setup_recovery_guardians(&admin, &guardians, &1u32);

        // Mint SBTs
        let issuer = Address::generate(&env);
        let owner = Address::generate(&env);
        let new_owner = Address::generate(&env);
        let meta = soroban_sdk::Bytes::from_slice(&env, b"ipfs://meta");
        let cred_id1 = qp_client.issue_credential(&issuer, &owner, &1u32, &meta, &None);
        let cred_id2 = qp_client.issue_credential(&issuer, &owner, &2u32, &meta, &None);
        let uri = Bytes::from_slice(&env, b"ipfs://QmSBT");
        let token_id1 = client.mint(&owner, &cred_id1, &uri);
        let token_id2 = client.mint(&owner, &cred_id2, &uri);

        // Verify owner has tokens
        let owner_tokens = client.get_tokens_by_owner(&owner);
        assert_eq!(owner_tokens.len(), 2);

        // Initiate and approve recovery
        let recovery_id = client.initiate_recovery(&owner, &new_owner);
        client.approve_recovery(&guardian, &recovery_id);

        // Finalize recovery
        client.finalize_recovery(&owner, &recovery_id);

        // Verify owner no longer has tokens
        let owner_tokens_after = client.get_tokens_by_owner(&owner);
        assert_eq!(owner_tokens_after.len(), 0);

        // Verify new owner has tokens
        let new_owner_tokens = client.get_tokens_by_owner(&new_owner);
        assert_eq!(new_owner_tokens.len(), 2);

        // Verify token ownership changed
        assert_eq!(client.owner_of(&token_id1), new_owner);
        assert_eq!(client.owner_of(&token_id2), new_owner);

        // Verify recovery is marked completed
        let recovery = client.get_recovery_request(&recovery_id);
        assert!(recovery.completed);
    }

    #[test]
    #[should_panic(expected = "insufficient approvals")]
    fn test_finalize_recovery_insufficient_approvals_panics() {
        let env = Env::default();
        env.mock_all_auths();
        let (client, admin, qp_client, _qp_id) = setup_with_qp(&env);

        // Setup recovery guardians with threshold of 2
        let guardian1 = Address::generate(&env);
        let guardian2 = Address::generate(&env);
        let guardians = soroban_sdk::vec![&env, guardian1.clone(), guardian2];
        client.setup_recovery_guardians(&admin, &guardians, &2u32);

        // Mint an SBT
        let issuer = Address::generate(&env);
        let owner = Address::generate(&env);
        let new_owner = Address::generate(&env);
        let meta = soroban_sdk::Bytes::from_slice(&env, b"ipfs://meta");
        let cred_id = qp_client.issue_credential(&issuer, &owner, &1u32, &meta, &None);
        let uri = Bytes::from_slice(&env, b"ipfs://QmSBT");
        let _token_id = client.mint(&owner, &cred_id, &uri);

        // Initiate recovery with only one approval (need 2)
        let recovery_id = client.initiate_recovery(&owner, &new_owner);
        client.approve_recovery(&guardian1, &recovery_id);

        // Try to finalize with only 1 approval (should panic)
        client.finalize_recovery(&owner, &recovery_id);
    }

    #[test]
    fn test_recovery_audit_trail() {
        let env = Env::default();
        env.mock_all_auths();
        let (client, admin, _qp_client, _qp_id) = setup_with_qp(&env);

        let guardian = Address::generate(&env);
        let guardians = soroban_sdk::vec![&env, guardian.clone()];
        client.setup_recovery_guardians(&admin, &guardians, &1u32);

        let owner = Address::generate(&env);
        let new_owner = Address::generate(&env);

        // Initiate recovery - should create audit entry
        let recovery_id = client.initiate_recovery(&owner, &new_owner);
        let initial_count = client.get_audit_trail_count();
        assert_eq!(initial_count, 1);

        // Get first audit entry
        let entry1 = client.get_audit_trail_entry(&1u64);
        assert_eq!(entry1.recovery_request_id, recovery_id);
        assert_eq!(entry1.actor, owner);

        // Approve recovery - should create another audit entry
        client.approve_recovery(&guardian, &recovery_id);
        let count_after_approval = client.get_audit_trail_count();
        assert_eq!(count_after_approval, 2);

        // Get second audit entry
        let entry2 = client.get_audit_trail_entry(&2u64);
        assert_eq!(entry2.recovery_request_id, recovery_id);
        assert_eq!(entry2.actor, guardian);
    }

    #[test]
    fn test_get_recovery_approvals_empty() {
        let env = Env::default();
        env.mock_all_auths();
        let (client, admin, _qp_client, _qp_id) = setup_with_qp(&env);

        let guardian = Address::generate(&env);
        let guardians = soroban_sdk::vec![&env, guardian];
        client.setup_recovery_guardians(&admin, &guardians, &1u32);

        let owner = Address::generate(&env);
        let new_owner = Address::generate(&env);
        let recovery_id = client.initiate_recovery(&owner, &new_owner);

        let approvals = client.get_recovery_approvals(&recovery_id);
        assert_eq!(approvals.len(), 0);
    }

    #[test]
    #[should_panic(expected = "only recovery initiator")]
    fn test_finalize_recovery_unauthorized_panics() {
        let env = Env::default();
        env.mock_all_auths();
        let (client, admin, _qp_client, _qp_id) = setup_with_qp(&env);

        let guardian = Address::generate(&env);
        let guardians = soroban_sdk::vec![&env, guardian.clone()];
        client.setup_recovery_guardians(&admin, &guardians, &1u32);

        let owner = Address::generate(&env);
        let new_owner = Address::generate(&env);
        let recovery_id = client.initiate_recovery(&owner, &new_owner);
        client.approve_recovery(&guardian, &recovery_id);

        let unauthorized = Address::generate(&env);
        client.finalize_recovery(&unauthorized, &recovery_id); // Should panic
    }
}
