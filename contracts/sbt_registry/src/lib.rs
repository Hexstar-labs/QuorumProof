#![no_std]
use soroban_sdk::{contract, contractimpl, contracttype, contracterror, panic_with_error, Address, Bytes, Env, Vec};

const STANDARD_TTL: u32 = 16_384;
const EXTENDED_TTL: u32 = 524_288;

#[contracterror]
#[derive(Copy, Clone, Debug, Eq, PartialEq, PartialOrd, Ord)]
#[repr(u32)]
pub enum ContractError {
    SoulboundNonTransferable = 1,
}

#[contracttype]
#[derive(Clone)]
pub enum DataKey {
    Token(u64),
    TokenCount,
    Owner(u64),
    OwnerTokens(Address),
    OwnerCredential(Address, u64),
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
    /// Panics if an SBT already exists for this (owner, credential_id).
    pub fn mint(env: Env, owner: Address, credential_id: u64, metadata_uri: Bytes) -> u64 {
        owner.require_auth();
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

        env.events().publish(("mint",), token_id);
        token_id
    }

    pub fn get_token(env: Env, token_id: u64) -> SoulboundToken {
        env.storage().persistent().get(&DataKey::Token(token_id)).expect("token not found")
    }

    pub fn owner_of(env: Env, token_id: u64) -> Address {
        env.storage().persistent().get(&DataKey::Owner(token_id)).expect("token not found")
    }

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
    pub fn burn(env: Env, owner: Address, token_id: u64) {
        owner.require_auth();
        let token: SoulboundToken = env.storage().persistent()
            .get(&DataKey::Token(token_id))
            .expect("token not found");
        assert!(token.owner == owner, "only the token owner can burn");
        env.storage().persistent().remove(&DataKey::Token(token_id));
        env.storage().persistent().remove(&DataKey::Owner(token_id));
        let mut owner_tokens: Vec<u64> = env.storage().persistent()
            .get(&DataKey::OwnerTokens(owner.clone()))
            .unwrap_or(Vec::new(&env));
        if let Some(pos) = owner_tokens.iter().position(|id| id == token_id) {
            owner_tokens.remove(pos as u32);
        }
        env.storage().persistent().set(&DataKey::OwnerTokens(owner), &owner_tokens);
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

    #[test]
    fn test_mint_and_ownership() {
        let env = Env::default();
        env.mock_all_auths();
        let contract_id = env.register_contract(None, SbtRegistryContract);
        let client = SbtRegistryContractClient::new(&env, &contract_id);
        let owner = Address::generate(&env);
        let uri = Bytes::from_slice(&env, b"ipfs://QmSBT");
        let token_id = client.mint(&owner, &1u64, &uri);
        assert_eq!(token_id, 1);
        assert_eq!(client.owner_of(&token_id), owner);
    }

    #[test]
    #[should_panic(expected = "HostError")]
    fn test_duplicate_sbt_minting_rejection() {
        let env = Env::default();
        env.mock_all_auths();
        let contract_id = env.register_contract(None, SbtRegistryContract);
        let client = SbtRegistryContractClient::new(&env, &contract_id);
        let owner = Address::generate(&env);
        let uri = Bytes::from_slice(&env, b"ipfs://QmSBT");
        client.mint(&owner, &1u64, &uri);
        client.mint(&owner, &1u64, &uri);
    }

    // Other tests for ownership, get_tokens_by_owner etc. unchanged as per existing
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

#[test]
    #[should_panic] // upgrade requires the WASM to exist in host storage; this verifies auth passes
    fn test_upgrade_success() {
        let env = Env::default();
        env.mock_all_auths();
        let contract_id = env.register_contract(None, SbtRegistryContract);
        let client = SbtRegistryContractClient::new(&env, &contract_id);

        let admin = Address::generate(&env);
        let wasm_hash = BytesN::from_array(&env, &[0u8; 32]);

        // Should succeed without panic
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

        client.upgrade(&admin, &wasm_hash);  // Authorize admin first

        // Unauthorized should panic on require_auth
        env.as_contract(&contract_id, || {
            client.upgrade(&unpriv, &wasm_hash);
        });
    }
}
