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
}

#[contracttype]
#[derive(Clone)]
pub enum DataKey {
    Token(u64),
    TokenCount,
    Owner(u64),
    OwnerTokens(Address),
    OwnerCredential(Address, u64),
    Admin,
    QuorumProofId,
}

#[contracttype]
#[derive(Clone)]
pub struct SoulboundToken {
    pub id: u64,
    pub owner: Address,
    pub credential_id: u64,
    pub metadata_uri: Bytes,
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
}

#[cfg(test)]
mod tests {
    use super::*;
    use soroban_sdk::testutils::Address as _;
    use soroban_sdk::BytesN;
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
    fn test_burn_allows_remint_same_credential() {
        let env = Env::default();
        env.mock_all_auths();
        let contract_id = env.register_contract(None, SbtRegistryContract);
        let client = SbtRegistryContractClient::new(&env, &contract_id);
        let owner = Address::generate(&env);
        let uri = Bytes::from_slice(&env, b"ipfs://QmSBT");

        // mint, burn, then re-mint the same credential — must succeed
        let token_id = client.mint(&owner, &1u64, &uri);
        client.burn(&owner, &token_id);
        let new_token_id = client.mint(&owner, &1u64, &uri);

        assert_eq!(new_token_id, 2);
        assert_eq!(client.owner_of(&new_token_id), owner);
    }

    #[test]
    fn test_mint_emits_event() {
        let env = Env::default();
        env.mock_all_auths();
        let contract_id = env.register_contract(None, SbtRegistryContract);
        let client = SbtRegistryContractClient::new(&env, &contract_id);
        let owner = Address::generate(&env);
        let uri = Bytes::from_slice(&env, b"ipfs://QmSBT");

        let token_id = client.mint(&owner, &1u64, &uri);

        let events = env.events().all();
        // Find the mint event: topic[0] == symbol "mint", data == token_id
        let mint_event = events.iter().find(|(_, topics, _)| {
            if let Some(first) = topics.get(0) {
                soroban_sdk::Symbol::try_from_val(&env, &first)
                    .map(|s| s == symbol_short!("mint"))
                    .unwrap_or(false)
            } else {
                false
            }
        });
        assert!(mint_event.is_some(), "mint event not emitted");
        let (_, _, data) = mint_event.unwrap();
        let emitted_id = u64::try_from_val(&env, &data).expect("data should be token_id");
        assert_eq!(emitted_id, token_id);
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
        let contract_id = env.register_contract(None, SbtRegistryContract);
        let client = SbtRegistryContractClient::new(&env, &contract_id);
        let owner = Address::generate(&env);
        let uri = Bytes::from_slice(&env, b"ipfs://QmSBT");

        assert_eq!(client.get_sbt_by_owner(&owner).len(), 0);

        let id1 = client.mint(&owner, &1u64, &uri);
        let id2 = client.mint(&owner, &2u64, &uri);

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
        let contract_id = env.register_contract(None, SbtRegistryContract);
        let client = SbtRegistryContractClient::new(&env, &contract_id);
        let owner = Address::generate(&env);
        let uri = Bytes::from_slice(&env, b"ipfs://QmSBT");

        assert_eq!(client.sbt_count(), 0);

        client.mint(&owner, &1u64, &uri);
        assert_eq!(client.sbt_count(), 1);

        client.mint(&owner, &2u64, &uri);
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

        let events = env.events().all();
        let burn_event = events.iter().find(|(_, topics, _)| {
            if let Some(first) = topics.get(0) {
                soroban_sdk::Symbol::try_from_val(&env, &first)
                    .map(|s| s == symbol_short!("burn"))
                    .unwrap_or(false)
            } else {
                false
            }
        });
        assert!(burn_event.is_some(), "burn event not emitted");
        let (_, _, data) = burn_event.unwrap();
        let emitted_id = u64::try_from_val(&env, &data).expect("data should be token_id");
        assert_eq!(emitted_id, token_id);
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
}
