# zk_verifier Initialization

`zk_verifier::initialize(admin)` sets the contract admin. It can run once per
deployment (Issue #1721).

## Authorization

`initialize` calls `admin.require_auth()` before it stores the admin. The
transaction must therefore carry the authorization of the address being set as
admin. Before this fix, `initialize` stored any address it was given, so whoever
submitted the first transaction after deployment could become admin.

| Call | Result |
|---|---|
| `initialize(A)` signed by `A` | Admin set to `A` |
| `initialize(A)` signed by anyone else | Rejected; admin unchanged |
| Second `initialize` (any signer) | Rejected with `already initialized` |

## Deployment

`scripts/deploy_testnet.sh` runs `initialize` immediately after deploying the
verifier, with the deployer as admin.

### Residual window

The deploy and initialize calls are separate transactions. Soroban SDK 21 has
no constructor, so the two cannot be made atomic. Between them the contract
exists with no admin.

The `require_auth` check closes the case where an attacker names a victim as
admin. It does not close the case where an attacker, watching the deployment,
initializes the contract with the attacker's own key first. Only a short window
and a post-deploy check limit that risk:

1. Keep the deploy and initialize calls back-to-back in one script run.
2. Verify the admin after deployment (below). If the check fails, the contract
   was claimed by someone else: discard that contract ID and redeploy.
3. For production, run deploy and initialize from a machine not shared with
   other operators, and record the contract ID in the deployment manifest.

### Verifying the admin

There is no admin getter. `update_admin` requires the current admin's
authorization, so this call succeeds only when the deployer is the admin. It
is a no-op when it succeeds (the admin is set to itself):

```bash
stellar contract invoke --id "$CONTRACT_ZK_VERIFIER" --source deployer \
  --network testnet -- update_admin \
  --admin "$DEPLOYER_ADDRESS" --new_admin "$DEPLOYER_ADDRESS"
```

A second `initialize` call must fail with `already initialized`.

Regression tests for this behaviour are in
`contracts/integration_tests/src/regressions.rs` under `regression_1721`.
