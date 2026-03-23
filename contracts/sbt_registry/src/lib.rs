#![no_std]
use soroban_sdk::{contract, contractimpl, contracttype, Address, Bytes, Env, Vec};

#[contracttype]
#[derive(Clone)]
pub enum DataKey {
    Token(u64),
    TokenCount,
    Owner(u64),
    OwnerTokens(Address),
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
    /// Mint a soulbound token. Non-transferable by design.
    pub fn mint(env: Env, owner: Address, credential_id: u64, metadata_uri: Bytes) -> u64 {
        owner.require_auth();
        let id: u64 = env
            .storage()
            .instance()
            .get(&DataKey::TokenCount)
            .unwrap_or(0u64)
            + 1;
        let token = SoulboundToken {
            id,
            owner: owner.clone(),
            credential_id,
            metadata_uri,
        };
        env.storage()
            .instance()
            .set(&DataKey::Token(id), &token);
        env.storage()
            .instance()
            .set(&DataKey::Owner(id), &owner);
        env.storage()
            .instance()
            .set(&DataKey::TokenCount, &id);
        // Track token ID under the owner's address for reverse lookup
        let mut owner_tokens: Vec<u64> = env
            .storage()
            .instance()
            .get(&DataKey::OwnerTokens(owner.clone()))
            .unwrap_or(Vec::new(&env));
        owner_tokens.push_back(id);
        env.storage()
            .instance()
            .set(&DataKey::OwnerTokens(owner), &owner_tokens);
        id
    }

    /// Get a soulbound token by ID.
    pub fn get_token(env: Env, token_id: u64) -> SoulboundToken {
        env.storage()
            .instance()
            .get(&DataKey::Token(token_id))
            .expect("token not found")
    }

    /// Verify ownership of a token.
    pub fn owner_of(env: Env, token_id: u64) -> Address {
        env.storage()
            .instance()
            .get(&DataKey::Owner(token_id))
            .expect("token not found")
    }

    /// Return all token IDs owned by a given address.
    pub fn get_tokens_by_owner(env: Env, owner: Address) -> Vec<u64> {
        env.storage()
            .instance()
            .get(&DataKey::OwnerTokens(owner))
            .unwrap_or(Vec::new(&env))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use soroban_sdk::{testutils::Address as _, Bytes, Env};

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
    fn test_get_tokens_by_owner_single() {
        let env = Env::default();
        env.mock_all_auths();
        let contract_id = env.register_contract(None, SbtRegistryContract);
        let client = SbtRegistryContractClient::new(&env, &contract_id);

        let owner = Address::generate(&env);
        let uri = Bytes::from_slice(&env, b"ipfs://QmSBT");
        let token_id = client.mint(&owner, &1u64, &uri);

        let tokens = client.get_tokens_by_owner(&owner);
        assert_eq!(tokens.len(), 1);
        assert_eq!(tokens.get(0).unwrap(), token_id);
    }

    #[test]
    fn test_get_tokens_by_owner_multiple() {
        let env = Env::default();
        env.mock_all_auths();
        let contract_id = env.register_contract(None, SbtRegistryContract);
        let client = SbtRegistryContractClient::new(&env, &contract_id);

        let owner = Address::generate(&env);
        let uri = Bytes::from_slice(&env, b"ipfs://QmSBT");
        let id1 = client.mint(&owner, &1u64, &uri);
        let id2 = client.mint(&owner, &2u64, &uri);
        let id3 = client.mint(&owner, &3u64, &uri);

        let tokens = client.get_tokens_by_owner(&owner);
        assert_eq!(tokens.len(), 3);
        assert_eq!(tokens.get(0).unwrap(), id1);
        assert_eq!(tokens.get(1).unwrap(), id2);
        assert_eq!(tokens.get(2).unwrap(), id3);
    }

    #[test]
    fn test_get_tokens_by_owner_empty() {
        let env = Env::default();
        env.mock_all_auths();
        let contract_id = env.register_contract(None, SbtRegistryContract);
        let client = SbtRegistryContractClient::new(&env, &contract_id);

        let owner = Address::generate(&env);
        let tokens = client.get_tokens_by_owner(&owner);
        assert_eq!(tokens.len(), 0);
    }

    #[test]
    fn test_get_tokens_by_owner_isolated_per_owner() {
        let env = Env::default();
        env.mock_all_auths();
        let contract_id = env.register_contract(None, SbtRegistryContract);
        let client = SbtRegistryContractClient::new(&env, &contract_id);

        let owner_a = Address::generate(&env);
        let owner_b = Address::generate(&env);
        let uri = Bytes::from_slice(&env, b"ipfs://QmSBT");

        let id_a1 = client.mint(&owner_a, &1u64, &uri);
        let id_a2 = client.mint(&owner_a, &2u64, &uri);
        let id_b1 = client.mint(&owner_b, &3u64, &uri);

        let tokens_a = client.get_tokens_by_owner(&owner_a);
        assert_eq!(tokens_a.len(), 2);
        assert_eq!(tokens_a.get(0).unwrap(), id_a1);
        assert_eq!(tokens_a.get(1).unwrap(), id_a2);

        let tokens_b = client.get_tokens_by_owner(&owner_b);
        assert_eq!(tokens_b.len(), 1);
        assert_eq!(tokens_b.get(0).unwrap(), id_b1);
    }
}
