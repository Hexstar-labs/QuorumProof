// Regression test suite for QuorumProof.
//
// Convention:
//   Every test in this file is pinned to a specific closed bug or issue via
//   a `// regression: #N` comment on the `#[test]` attribute.  Future bug
//   fixes MUST add a corresponding test here (or in the per-crate test
//   module) following this pattern.
//
// See CONTRIBUTING.md § "Regression Test Convention" for the full policy.

#[cfg(test)]
mod regression_1362 {
    // regression: #1362 — enforce quorum intersection safety checks in
    // attestation flow.
    //
    // Bug description:
    //   Prior to the fix the per-(credential, slice) attestation duplicate
    //   guard was scoped globally per credential rather than per
    //   (credential, slice) pair.  This meant:
    //
    //   1. An attestor who already attested for credential C in slice S1
    //      was *blocked* from attesting for the same credential in a second
    //      slice S2 — even though that is legitimate under FBA.
    //
    //   2. Consequently `check_quorum_intersection` could never verify
    //      that `safe_nodes` form a quorum in *every* slice when the same
    //      attestor participates in multiple slices.
    //
    // The fix (see commit tagged #1362):
    //   a) Changed the duplicate-attestation guard to be keyed by
    //      `DataKey5::AttestationWeight(credential_id, slice_id, attestor)`
    //      so the same attestor may legitimately attest once per slice.
    //   b) Added `check_quorum_intersection()` which verifies that the
    //      supplied `safe_nodes` form quorum relative to EVERY listed slice,
    //      panicking with `QuorumIntersectionFailed` otherwise.
    //
    // These tests must fail if either (a) or (b) is reverted.

    use quorum_proof::{QuorumProofContract, QuorumProofContractClient};
    use soroban_sdk::{
        testutils::Address as _,
        vec, Address, Bytes, Env,
    };

    fn setup(env: &Env) -> QuorumProofContractClient<'_> {
        env.mock_all_auths();
        let id = env.register_contract(None, QuorumProofContract);
        let client = QuorumProofContractClient::new(env, &id);
        let admin = Address::generate(env);
        client.initialize(&admin);
        client
    }

    fn metadata(env: &Env) -> Bytes {
        Bytes::from_slice(env, b"QmTestHash000000000000000000000000")
    }

    // ── (a) Per-(credential, slice) duplicate guard ──────────────────────────

    /// regression: #1362
    ///
    /// An attestor who belongs to two *different* slices must be able to
    /// attest for the same credential in each slice independently.
    ///
    /// Before the fix this panicked on the second `attest()` call because the
    /// duplicate guard was keyed only on (credential, attestor) — not on
    /// (credential, slice, attestor).
    #[test]
    fn regression_1362_attestor_may_attest_once_per_slice() {
        let env = Env::default();
        let client = setup(&env);

        let issuer = Address::generate(&env);
        let holder = Address::generate(&env);
        let shared_attestor = Address::generate(&env);
        let other_attestor_s1 = Address::generate(&env);
        let other_attestor_s2 = Address::generate(&env);

        // Slice 1 — shared_attestor participates alongside other_attestor_s1
        let s1_id = client.create_slice(
            &Address::generate(&env),
            &vec![&env, shared_attestor.clone(), other_attestor_s1.clone()],
            &vec![&env, 1u32, 1u32],
            &1u32,
        );

        // Slice 2 — shared_attestor participates alongside other_attestor_s2
        let s2_id = client.create_slice(
            &Address::generate(&env),
            &vec![&env, shared_attestor.clone(), other_attestor_s2.clone()],
            &vec![&env, 1u32, 1u32],
            &1u32,
        );

        let cred_id = client.issue_credential(
            &issuer, &holder, &1u32, &metadata(&env), &None, &0u64,
        );

        // Attest in slice 1 — must succeed
        client.attest(&shared_attestor, &cred_id, &s1_id, &true, &None);
        assert!(
            client.is_attested(&cred_id, &s1_id),
            "regression #1362: credential should be attested in slice 1 after threshold met"
        );

        // Attest in slice 2 with the SAME attestor — must NOT panic.
        // Before fix this would trigger "attestor has already attested for this credential".
        client.attest(&shared_attestor, &cred_id, &s2_id, &true, &None);
        assert!(
            client.is_attested(&cred_id, &s2_id),
            "regression #1362: credential should be attested in slice 2 after threshold met"
        );
    }

    /// regression: #1362
    ///
    /// Attesting for the *same* credential in the *same* slice twice with the
    /// same attestor must still be rejected (the guard is per-slice, not
    /// removed entirely).
    #[test]
    #[should_panic(expected = "attestor has already attested for this credential")]
    fn regression_1362_duplicate_attestation_in_same_slice_still_rejected() {
        let env = Env::default();
        let client = setup(&env);

        let issuer = Address::generate(&env);
        let holder = Address::generate(&env);
        let attestor = Address::generate(&env);

        let slice_id = client.create_slice(
            &Address::generate(&env),
            &vec![&env, attestor.clone()],
            &vec![&env, 1u32],
            &1u32,
        );

        let cred_id = client.issue_credential(
            &issuer, &holder, &1u32, &metadata(&env), &None, &0u64,
        );

        client.attest(&attestor, &cred_id, &slice_id, &true, &None);
        // Second call in the SAME slice must panic — this is the unchanged part
        // of the invariant; the fix only relaxes the cross-slice case.
        client.attest(&attestor, &cred_id, &slice_id, &true, &None);
    }

    // ── (b) check_quorum_intersection safety gate ────────────────────────────

    /// regression: #1362
    ///
    /// `check_quorum_intersection` must accept a certificate whose `safe_nodes`
    /// form quorum in *every* listed slice and return `is_safe = true`.
    ///
    /// This is the positive path: a valid certificate must not be rejected.
    #[test]
    fn regression_1362_check_quorum_intersection_valid_certificate_accepted() {
        use quorum_proof::QuorumIntersectionCertificate;

        let env = Env::default();
        let client = setup(&env);

        let attestor_a = Address::generate(&env);
        let attestor_b = Address::generate(&env);
        let attestor_c = Address::generate(&env); // common node across both slices

        // Slice 1: {A, C} threshold 1
        let s1_id = client.create_slice(
            &Address::generate(&env),
            &vec![&env, attestor_a.clone(), attestor_c.clone()],
            &vec![&env, 1u32, 1u32],
            &1u32,
        );

        // Slice 2: {B, C} threshold 1
        let s2_id = client.create_slice(
            &Address::generate(&env),
            &vec![&env, attestor_b.clone(), attestor_c.clone()],
            &vec![&env, 1u32, 1u32],
            &1u32,
        );

        let slice_ids = vec![&env, s1_id, s2_id];

        // safe_nodes = [C] — C is a quorum for both slices (threshold = 1)
        let safe_nodes = vec![&env, attestor_c.clone()];

        let cert = QuorumIntersectionCertificate {
            slice_ids: slice_ids.clone(),
            safe_nodes: safe_nodes.clone(),
            proof_hash: Bytes::from_slice(&env, &[0xabu8; 32]),
            signature: None,
        };

        let report = client.check_quorum_intersection(&slice_ids, &cert);

        assert!(
            report.is_safe,
            "regression #1362: valid certificate should produce is_safe = true"
        );
        assert_eq!(
            report.common_nodes, safe_nodes,
            "regression #1362: reported common_nodes must match certificate safe_nodes"
        );
    }

    /// regression: #1362
    ///
    /// `check_quorum_intersection` must panic with `QuorumIntersectionFailed`
    /// when `safe_nodes` do NOT form quorum in at least one slice.
    ///
    /// This is the critical negative path: if this safety gate is removed,
    /// this test will no longer panic — revealing the regression.
    #[test]
    #[should_panic]
    fn regression_1362_check_quorum_intersection_rejects_invalid_safe_nodes() {
        use quorum_proof::QuorumIntersectionCertificate;

        let env = Env::default();
        let client = setup(&env);

        let attestor_a = Address::generate(&env);
        let attestor_b = Address::generate(&env);
        let attestor_c = Address::generate(&env);
        let attestor_d = Address::generate(&env);

        // Slice 1: {A, B} threshold 2 — requires BOTH A and B
        let s1_id = client.create_slice(
            &Address::generate(&env),
            &vec![&env, attestor_a.clone(), attestor_b.clone()],
            &vec![&env, 1u32, 1u32],
            &2u32,
        );

        // Slice 2: {C, D} threshold 2 — requires BOTH C and D
        let s2_id = client.create_slice(
            &Address::generate(&env),
            &vec![&env, attestor_c.clone(), attestor_d.clone()],
            &vec![&env, 1u32, 1u32],
            &2u32,
        );

        let slice_ids = vec![&env, s1_id, s2_id];

        // safe_nodes = [A] — A alone is NOT quorum for either slice (threshold 2),
        // and is not even a member of slice 2.  This certificate is fraudulent.
        let invalid_safe_nodes = vec![&env, attestor_a.clone()];

        let cert = QuorumIntersectionCertificate {
            slice_ids: slice_ids.clone(),
            safe_nodes: invalid_safe_nodes,
            proof_hash: Bytes::from_slice(&env, &[0xddu8; 32]),
            signature: None,
        };

        // Must panic with QuorumIntersectionFailed.
        // If check_quorum_intersection's safety loop is removed, this call
        // succeeds and the test fails — catching the regression.
        client.check_quorum_intersection(&slice_ids, &cert);
    }

    /// regression: #1362
    ///
    /// The `check_quorum_intersection` safe_nodes validation must check ALL
    /// slices, not just the first.  A node that satisfies slice 1 but not
    /// slice 2 must still be rejected.
    ///
    /// This tests the inner loop body: `!Self::is_quorum_impl(&env, slice_id,
    /// safe_nodes)` must iterate over every slice_id, not short-circuit after
    /// the first passing slice.
    #[test]
    #[should_panic]
    fn regression_1362_intersection_check_validates_every_slice_not_just_first() {
        use quorum_proof::QuorumIntersectionCertificate;

        let env = Env::default();
        let client = setup(&env);

        let attestor_a = Address::generate(&env);
        let attestor_b = Address::generate(&env);
        let attestor_c = Address::generate(&env);

        // Slice 1: {A} threshold 1 — A alone is quorum
        let s1_id = client.create_slice(
            &Address::generate(&env),
            &vec![&env, attestor_a.clone()],
            &vec![&env, 1u32],
            &1u32,
        );

        // Slice 2: {B, C} threshold 2 — requires BOTH B and C; A is not even
        // a member of this slice, so A cannot satisfy it.
        let s2_id = client.create_slice(
            &Address::generate(&env),
            &vec![&env, attestor_b.clone(), attestor_c.clone()],
            &vec![&env, 1u32, 1u32],
            &2u32,
        );

        let slice_ids = vec![&env, s1_id, s2_id];

        // safe_nodes = [A] passes slice 1 but fails slice 2.
        let partial_safe_nodes = vec![&env, attestor_a.clone()];

        let cert = QuorumIntersectionCertificate {
            slice_ids: slice_ids.clone(),
            safe_nodes: partial_safe_nodes,
            proof_hash: Bytes::from_slice(&env, &[0xeeu8; 32]),
            signature: None,
        };

        // Must panic — the validation loop must not stop after slice 1 passes.
        client.check_quorum_intersection(&slice_ids, &cert);
    }
}

#[cfg(test)]
mod regression_1721 {
    // regression: #1721 — authenticate `zk_verifier::initialize` against the
    // admin address.
    //
    // Bug description:
    //   `ZkVerifierContract::initialize(env, admin)` checked that no admin was
    //   stored and then stored `admin`, but never called `admin.require_auth()`.
    //   Whoever submitted the first transaction after deployment could name
    //   any address as admin and take control of the verifier. An attacker
    //   watching the deployment could win that race.
    //
    // The fix (see issue #1721):
    //   `initialize` calls `admin.require_auth()` before storing the admin, so
    //   the transaction must carry that address's authorization.
    //
    // These tests must fail if the `require_auth()` call is removed: the
    // unauthorized-caller test would then complete and store the attacker's
    // chosen admin.

    use soroban_sdk::{
        testutils::{Address as _, MockAuth, MockAuthInvoke},
        Address, Env, IntoVal, Val, Vec,
    };
    use zk_verifier::{ZkVerifierContract, ZkVerifierContractClient};

    /// Register the contract without mocking any authorization.
    fn register(env: &Env) -> (Address, ZkVerifierContractClient<'_>) {
        let id = env.register_contract(None, ZkVerifierContract);
        let client = ZkVerifierContractClient::new(env, &id);
        (id, client)
    }

    /// An attacker-submitted `initialize` that names the victim as admin must
    /// be rejected when the victim has not authorized it.
    // regression: #1721
    #[test]
    fn regression_1721_initialize_rejects_unauthorized_admin() {
        let env = Env::default();
        let (id, client) = register(&env);
        let victim = Address::generate(&env);
        let attacker = Address::generate(&env);

        // Only the attacker has authorized a call. The call names the victim
        // as admin, so the victim's `require_auth` must fail.
        let args: Vec<Val> = soroban_sdk::vec![&env, victim.clone().into_val(&env)];
        env.mock_auths(&[MockAuth {
            address: &attacker,
            invoke: &MockAuthInvoke {
                contract: &id,
                fn_name: "initialize",
                args,
                sub_invokes: &[],
            },
        }]);

        let result = client.try_initialize(&victim);
        assert!(result.is_err(), "initialize must reject an admin that did not authorize it");
    }

    /// A failed takeover attempt must leave the admin slot empty, so the real
    /// admin can still initialize the contract afterwards.
    // regression: #1721
    #[test]
    fn regression_1721_failed_takeover_leaves_admin_unset() {
        let env = Env::default();
        let (id, client) = register(&env);
        let real_admin = Address::generate(&env);
        let attacker = Address::generate(&env);

        let args: Vec<Val> = soroban_sdk::vec![&env, real_admin.clone().into_val(&env)];
        env.mock_auths(&[MockAuth {
            address: &attacker,
            invoke: &MockAuthInvoke {
                contract: &id,
                fn_name: "initialize",
                args,
                sub_invokes: &[],
            },
        }]);
        assert!(client.try_initialize(&real_admin).is_err());

        // The real admin authorizes its own initialization and succeeds.
        let args: Vec<Val> = soroban_sdk::vec![&env, real_admin.clone().into_val(&env)];
        env.mock_auths(&[MockAuth {
            address: &real_admin,
            invoke: &MockAuthInvoke {
                contract: &id,
                fn_name: "initialize",
                args,
                sub_invokes: &[],
            },
        }]);
        client.initialize(&real_admin);
    }

    /// Happy path: the admin authorizes its own initialization.
    // regression: #1721
    #[test]
    fn regression_1721_admin_can_initialize_with_own_auth() {
        let env = Env::default();
        let (id, client) = register(&env);
        let admin = Address::generate(&env);

        let args: Vec<Val> = soroban_sdk::vec![&env, admin.clone().into_val(&env)];
        env.mock_auths(&[MockAuth {
            address: &admin,
            invoke: &MockAuthInvoke {
                contract: &id,
                fn_name: "initialize",
                args,
                sub_invokes: &[],
            },
        }]);
        client.initialize(&admin);

        // A second initialization is still rejected, even by the same admin.
        let args: Vec<Val> = soroban_sdk::vec![&env, admin.clone().into_val(&env)];
        env.mock_auths(&[MockAuth {
            address: &admin,
            invoke: &MockAuthInvoke {
                contract: &id,
                fn_name: "initialize",
                args,
                sub_invokes: &[],
            },
        }]);
        assert!(client.try_initialize(&admin).is_err());
    }
}
