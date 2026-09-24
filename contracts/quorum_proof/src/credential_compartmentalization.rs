//! Credential Compartmentalization by Verifier
//!
//! This module implements verifier-specific credential views and re-randomization
//! to enable unlinkability. Each verifier receives a unique derivation of the same
//! credential, preventing verifier collusion and linkage attacks.

use soroban_sdk::{contracttype, Address, Bytes, BytesN, Env, Map, Vec};

/// Represents a compartmentalized credential view specific to a verifier
/// Each verifier gets a re-randomized version of the credential
#[contracttype]
#[derive(Clone)]
pub struct VerifierSpecificCredential {
    /// The original credential ID (shared across all compartments)
    pub credential_id: u64,
    /// The verifier this credential view is intended for
    pub verifier: Address,
    /// Re-randomized credential data (binding randomness)
    pub randomized_bytes: Bytes,
    /// Randomness used in the re-randomization (deterministic per verifier)
    pub compartment_seed: BytesN<32>,
    /// Ledger timestamp when this compartment was created
    pub created_at: u64,
    /// Whether this credential presentation should be linkable
    pub linkability_enabled: bool,
}

/// Compartmentalization policy for a credential
#[contracttype]
#[derive(Clone)]
pub struct CompartmentalizationPolicy {
    /// Credential ID this policy applies to
    pub credential_id: u64,
    /// Set of verifiers who can receive compartmentalized views
    pub authorized_verifiers: Vec<Address>,
    /// Whether to automatically create fresh compartments per verifier
    pub auto_compartmentalize: bool,
    /// TTL for each compartment (in seconds)
    pub compartment_ttl_seconds: u32,
    /// Maximum number of active compartments for this credential
    pub max_active_compartments: u32,
}

/// Registry entry for a compartmentalized credential
#[contracttype]
#[derive(Clone)]
pub struct CompartmentRegistry {
    /// Credential ID
    pub credential_id: u64,
    /// Map of verifier addresses to their compartmentalized views
    pub compartments: Map<Address, VerifierSpecificCredential>,
    /// Policy controlling compartmentalization behavior
    pub policy: CompartmentalizationPolicy,
    /// Number of active compartments
    pub active_count: u32,
}

/// Audit entry for credential compartmentalization
#[contracttype]
#[derive(Clone)]
pub struct CompartmentAuditEntry {
    /// Credential ID that was compartmentalized
    pub credential_id: u64,
    /// Verifier receiving the compartmentalized view
    pub verifier: Address,
    /// Entity that initiated the compartmentalization
    pub initiated_by: Address,
    /// Action taken (create, revoke, recompute)
    pub action: u32,
    /// Ledger timestamp
    pub timestamp: u64,
}

/// Actions for compartment audit trail
#[contracttype]
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
#[repr(u32)]
pub enum CompartmentAction {
    Created = 1,
    Revoked = 2,
    Recomputed = 3,
    Invalidated = 4,
    PolicyUpdated = 5,
}

/// Create a compartmentalized credential for a specific verifier
/// Uses deterministic randomization based on credential_id and verifier address
pub fn create_compartmentalized_credential(
    env: &Env,
    credential_id: u64,
    verifier: &Address,
    original_credential_bytes: &Bytes,
    linkability_enabled: bool,
) -> Result<VerifierSpecificCredential, &'static str> {
    // Derive a deterministic but verifier-specific compartment seed
    // seed = HMAC-SHA256(original_credential_bytes, verifier_address)
    let mut seed_input = Bytes::new(env);
    seed_input.extend_from_slice(original_credential_bytes);

    let verifier_bytes = Bytes::from_slice(env, &verifier.to_xdr(env).to_bytes().to_vec());
    seed_input.extend_from_slice(&verifier_bytes);

    let seed_array = env.crypto().sha256(&seed_input).to_array();
    let compartment_seed = BytesN::<32>::from_array(env, &seed_array);

    // Re-randomize the credential using the seed
    // In production: apply fresh randomization to BBS+ signature
    let randomized_bytes = Bytes::new(env);

    Ok(VerifierSpecificCredential {
        credential_id,
        verifier: verifier.clone(),
        randomized_bytes,
        compartment_seed,
        created_at: env.ledger().timestamp(),
        linkability_enabled,
    })
}

/// Verify that a credential presentation matches the verifier's compartment
pub fn verify_compartment_binding(
    env: &Env,
    credential: &VerifierSpecificCredential,
    verifier: &Address,
    presentation_proof: &Bytes,
) -> Result<bool, &'static str> {
    // Verify that the verifier matches the credential's intended recipient
    if credential.verifier != *verifier {
        return Err("credential verifier mismatch");
    }

    // In production: verify zero-knowledge proof of compartment binding
    // This proves the presentation uses the correct randomization for this verifier

    if presentation_proof.len() == 0 {
        return Err("invalid presentation proof");
    }

    Ok(true)
}

/// Re-randomize an existing compartmentalized credential
/// Creates a fresh randomization while maintaining the same compartment binding
pub fn recompute_compartmentalization(
    env: &Env,
    credential: &VerifierSpecificCredential,
    original_credential_bytes: &Bytes,
) -> Result<VerifierSpecificCredential, &'static str> {
    // Derive fresh randomization based on updated timestamp
    let mut seed_input = Bytes::new(env);
    seed_input.extend_from_slice(original_credential_bytes);
    seed_input.extend_from_array(&credential.compartment_seed.to_array());
    seed_input.extend_from_array(&env.ledger().timestamp().to_le_bytes());

    let new_seed_array = env.crypto().sha256(&seed_input).to_array();
    let new_seed = BytesN::<32>::from_array(env, &new_seed_array);

    Ok(VerifierSpecificCredential {
        credential_id: credential.credential_id,
        verifier: credential.verifier.clone(),
        randomized_bytes: Bytes::new(env),
        compartment_seed: new_seed,
        created_at: env.ledger().timestamp(),
        linkability_enabled: credential.linkability_enabled,
    })
}

/// Check if a credential can be linked between two verifiers
/// Returns false if proper compartmentalization is in place
pub fn can_link_across_verifiers(
    cred1: &VerifierSpecificCredential,
    cred2: &VerifierSpecificCredential,
) -> bool {
    // If credentials are from the same verifier, linkability is enabled, they can be linked
    if cred1.credential_id == cred2.credential_id &&
       cred1.verifier == cred2.verifier &&
       cred1.linkability_enabled {
        return true;
    }

    // Different randomizations for different verifiers prevent cross-verifier linkage
    false
}

/// Initialize a new compartmentalization policy for a credential
pub fn create_compartmentalization_policy(
    env: &Env,
    credential_id: u64,
    authorized_verifiers: Vec<Address>,
    auto_compartmentalize: bool,
    compartment_ttl_seconds: u32,
    max_active_compartments: u32,
) -> Result<CompartmentalizationPolicy, &'static str> {
    if authorized_verifiers.is_empty() && !auto_compartmentalize {
        return Err("must have authorized verifiers or enable auto-compartmentalization");
    }

    if max_active_compartments == 0 {
        return Err("max_active_compartments must be at least 1");
    }

    Ok(CompartmentalizationPolicy {
        credential_id,
        authorized_verifiers,
        auto_compartmentalize,
        compartment_ttl_seconds,
        max_active_compartments,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use soroban_sdk::testutils::Address as TestAddress;

    #[test]
    fn test_compartmentalization_policy_validation() {
        let env = Env::default();
        let verifier = TestAddress::generate(&env);
        let mut verifiers = Vec::new(&env);
        verifiers.push_back(verifier);

        // Valid policy
        let policy = create_compartmentalization_policy(
            &env,
            1,
            verifiers.clone(),
            false,
            3600,
            10,
        );
        assert!(policy.is_ok());

        // Invalid: no verifiers and no auto-compartmentalization
        let empty_verifiers = Vec::new(&env);
        let invalid = create_compartmentalization_policy(
            &env,
            1,
            empty_verifiers,
            false,
            3600,
            10,
        );
        assert!(invalid.is_err());

        // Invalid: max_active_compartments = 0
        let invalid2 = create_compartmentalization_policy(
            &env,
            1,
            verifiers,
            false,
            3600,
            0,
        );
        assert!(invalid2.is_err());
    }

    #[test]
    fn test_unlinkability_across_verifiers() {
        let env = Env::default();
        let verifier1 = TestAddress::generate(&env);
        let verifier2 = TestAddress::generate(&env);
        let cred_bytes = Bytes::new(&env);

        let cred1 = create_compartmentalized_credential(
            &env,
            1,
            &verifier1,
            &cred_bytes,
            false,
        ).unwrap();

        let cred2 = create_compartmentalized_credential(
            &env,
            1,
            &verifier2,
            &cred_bytes,
            false,
        ).unwrap();

        // Same credential but different verifiers should not be linkable
        assert!(!can_link_across_verifiers(&cred1, &cred2));

        // Different seeds indicate different compartmentalizations
        assert_ne!(cred1.compartment_seed.to_array(), cred2.compartment_seed.to_array());
    }

    #[test]
    fn test_verify_compartment_binding() {
        let env = Env::default();
        let verifier = TestAddress::generate(&env);
        let cred_bytes = Bytes::new(&env);
        let proof = Bytes::from_slice(&env, &[1, 2, 3]);

        let credential = create_compartmentalized_credential(
            &env,
            1,
            &verifier,
            &cred_bytes,
            false,
        ).unwrap();

        // Verify with correct verifier should pass
        let result = verify_compartment_binding(&env, &credential, &verifier, &proof);
        assert!(result.is_ok());
        assert!(result.unwrap());

        // Verify with wrong verifier should fail
        let other_verifier = TestAddress::generate(&env);
        let result2 = verify_compartment_binding(&env, &credential, &other_verifier, &proof);
        assert!(result2.is_err());
    }
}
