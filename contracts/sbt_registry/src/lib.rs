#![no_std]
use soroban_sdk::{contract, contractimpl, contracttype, Address, Bytes, Env, Vec};

// Import the QuorumProof client for cross-contract credential validation.
use quorum_proof::QuorumProofContractClient;
/// TTL constants for persistent storage.
/// Mirrors the strategy used in quorum_proof:
/// - STANDARD_TTL: 16_384 ledgers (~3 hours at 5s/ledger) — minimum threshold before bump
/// - EXTENDED_TTL: 524_288 ledgers (~4 days) — target TTL after bump
/// SBTs represent permanent credentials, so persistent storage with TTL extension is required.
const STANDARD_TTL: u32 = 16_384;
const EXTENDED_TTL: u32 = 524_288;

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
    /// Validates that the credential exists in QuorumProofContract and is not revoked.
    pub fn mint(
        env: Env,
        owner: Address,
        credential_id: u64,
        metadata_uri: Bytes,
        quorum_proof_contract: Address,
    ) -> u64 {
        owner.require_auth();

        // Cross-contract call: fetch credential and validate it exists and is not revoked.
        let quorum_client = QuorumProofContractClient::new(&env, &quorum_proof_contract);
        let credential = quorum_client.get_credential(&credential_id);
        assert!(!credential.revoked, "credential is revoked");

    /// Token data is stored in persistent storage with TTL extension to ensure
    /// permanent credentials are not lost to storage expiry.
    pub fn mint(env: Env, owner: Address, credential_id: u64, metadata_uri: Bytes) -> u64 {
        owner.require_auth();

        // TokenCount lives in instance storage — it's a lightweight counter accessed on every mint.
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

        // Token data goes into persistent storage so it survives beyond instance TTL.
        env.storage()
            .persistent()
            .set(&DataKey::Token(id), &token);
        env.storage()
            .persistent()
            .extend_ttl(&DataKey::Token(id), STANDARD_TTL, EXTENDED_TTL);

        env.storage()
            .persistent()
            .set(&DataKey::Owner(id), &owner);
        env.storage()
            .persistent()
            .extend_ttl(&DataKey::Owner(id), STANDARD_TTL, EXTENDED_TTL);

        env.storage()
            .instance()
            .set(&DataKey::TokenCount, &id);

        // Reverse lookup: track all token IDs under the owner's address.
        let mut owner_tokens: Vec<u64> = env
            .storage()
            .persistent()
            .get(&DataKey::OwnerTokens(owner.clone()))
            .unwrap_or(Vec::new(&env));
        owner_tokens.push_back(id);
        env.storage()
            .persistent()
            .set(&DataKey::OwnerTokens(owner.clone()), &owner_tokens);
        env.storage()
            .persistent()
            .extend_ttl(&DataKey::OwnerTokens(owner), STANDARD_TTL, EXTENDED_TTL);

        id
    }

    /// Get a soulbound token by ID.
    pub fn get_token(env: Env, token_id: u64) -> SoulboundToken {
        env.storage()
            .persistent()
            .get(&DataKey::Token(token_id))
            .expect("token not found")
    }

    /// Verify ownership of a token.
    pub fn owner_of(env: Env, token_id: u64) -> Address {
        env.storage()
            .persistent()
            .get(&DataKey::Owner(token_id))
            .expect("token not found")
    }

    /// Return all token IDs owned by a given address.
    pub fn get_tokens_by_owner(env: Env, owner: Address) -> Vec<u64> {
        env.storage()
            .persistent()
            .get(&DataKey::OwnerTokens(owner))
            .unwrap_or(Vec::new(&env))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use soroban_sdk::{testutils::Address as _, testutils::Ledger, Bytes, Env};

    fn setup(env: &Env) -> (SbtRegistryContractClient, QuorumProofContractClient) {
        let sbt_id = env.register_contract(None, SbtRegistryContract);
        let qp_id = env.register_contract(None, QuorumProofContract);
        (
            SbtRegistryContractClient::new(env, &sbt_id),
            QuorumProofContractClient::new(env, &qp_id),
        )
    }

    #[test]
    fn test_mint_and_ownership() {
        let env = Env::default();
        env.mock_all_auths();
        let (sbt, qp) = setup(&env);

        let issuer = Address::generate(&env);
        let subject = Address::generate(&env);
        let metadata = Bytes::from_slice(&env, b"ipfs://QmTest");
        let cred_id = qp.issue_credential(&issuer, &subject, &1u32, &metadata);

        let uri = Bytes::from_slice(&env, b"ipfs://QmSBT");
        let token_id = sbt.mint(&subject, &cred_id, &uri, &qp.address);
        assert_eq!(token_id, 1);
        assert_eq!(sbt.owner_of(&token_id), subject);
    }

    #[test]
    #[should_panic(expected = "credential is revoked")]
    fn test_mint_rejects_revoked_credential() {
        let env = Env::default();
        env.mock_all_auths();
        let (sbt, qp) = setup(&env);

        let issuer = Address::generate(&env);
        let subject = Address::generate(&env);
        let metadata = Bytes::from_slice(&env, b"ipfs://QmTest");
        let cred_id = qp.issue_credential(&issuer, &subject, &1u32, &metadata);

        // Revoke the credential before minting.
        qp.revoke_credential(&issuer, &cred_id);

        let uri = Bytes::from_slice(&env, b"ipfs://QmSBT");
        // Should panic: credential is revoked.
        sbt.mint(&subject, &cred_id, &uri, &qp.address);
    }

    #[test]
    #[should_panic(expected = "credential not found")]
    fn test_mint_rejects_nonexistent_credential() {
        let env = Env::default();
        env.mock_all_auths();
        let (sbt, qp) = setup(&env);

        let owner = Address::generate(&env);
        let uri = Bytes::from_slice(&env, b"ipfs://QmSBT");
        // credential_id 999 was never issued — should panic.
        sbt.mint(&owner, &999u64, &uri, &qp.address);
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

    /// Verifies that token data persists in persistent storage after advancing the ledger
    /// well past the default instance TTL. The TTL extension on mint ensures the token
    /// remains accessible at EXTENDED_TTL ledgers (~4 days).
    #[test]
    fn test_token_persists_after_ttl_bump() {
        let env = Env::default();
        env.mock_all_auths();
        let contract_id = env.register_contract(None, SbtRegistryContract);
        let client = SbtRegistryContractClient::new(&env, &contract_id);

        let owner = Address::generate(&env);
        let uri = Bytes::from_slice(&env, b"ipfs://QmSBT");
        let token_id = client.mint(&owner, &42u64, &uri);

        // Advance the ledger sequence well past the default instance TTL (4096 ledgers)
        // but within the EXTENDED_TTL (524_288 ledgers) to confirm persistent storage holds.
        env.ledger().set(soroban_sdk::testutils::LedgerInfo {
            timestamp: 0,
            protocol_version: 21,
            sequence_number: 100_000,
            network_id: Default::default(),
            base_reserve: 10,
            min_temp_entry_ttl: 16,
            min_persistent_entry_ttl: STANDARD_TTL,
            max_entry_ttl: EXTENDED_TTL + 1,
        });

        // Token and ownership must still be retrievable from persistent storage.
        let token = client.get_token(&token_id);
        assert_eq!(token.id, token_id);
        assert_eq!(token.owner, owner);
        assert_eq!(token.credential_id, 42u64);

        assert_eq!(client.owner_of(&token_id), owner);

        let tokens = client.get_tokens_by_owner(&owner);
        assert_eq!(tokens.len(), 1);
        assert_eq!(tokens.get(0).unwrap(), token_id);
    }
}
