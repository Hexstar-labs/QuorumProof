# Contributing to QuorumProof

Thank you for contributing to QuorumProof!  This guide covers the
conventions we use so that the codebase stays maintainable and every past
incident has discoverable regression coverage.

---

## Regression Test Convention (Issue #1479)

Whenever a bug is fixed, a regression test **must** be added so the bug
cannot silently return.

### Where to put regression tests

| What regressed | Where the test goes |
|---|---|
| Contract logic in `contracts/quorum_proof` | `contracts/integration_tests/src/regressions.rs` (preferred) or a `#[cfg(test)]` block in the relevant source file |
| API-server behaviour | `api-server/tests/` alongside existing tests |
| Anything else | Nearest `*_tests.rs` or `tests/` directory, with a clear `// regression: #N` comment |

### How to write a regression test

1. **Name the test after the bug.**  Prefix with `regression_<issue_number>_`:

   ```rust
   #[test]
   fn regression_1362_attestor_may_attest_once_per_slice() { … }
   ```

2. **Pin the test to the issue with a comment on the `#[test]` attribute:**

   ```rust
   // regression: #1362
   #[test]
   fn regression_1362_attestor_may_attest_once_per_slice() { … }
   ```

3. **Describe the bug in the function doc-comment.**  Include:
   - What the bug was.
   - How the fix changed the code (briefly).
   - Why removing the fix would make *this specific test* fail.

4. **Add the test to `contracts/integration_tests/src/regressions.rs`** under
   a named submodule `regression_<issue_number>`.  The module comment
   must include the issue title.

5. **Verify the test is red without the fix.**  Temporarily revert the fix
   locally and confirm the test fails before submitting your PR.

### Minimal example

```rust
// contracts/integration_tests/src/regressions.rs

mod regression_1362 {
    // regression: #1362 — enforce quorum intersection safety checks in
    // attestation flow.
    use quorum_proof::{QuorumProofContract, QuorumProofContractClient};
    use soroban_sdk::{testutils::Address as _, vec, Address, Bytes, Env};

    fn setup(env: &Env) -> QuorumProofContractClient<'_> {
        env.mock_all_auths();
        let id = env.register_contract(None, QuorumProofContract);
        let client = QuorumProofContractClient::new(env, &id);
        client.initialize(&Address::generate(env));
        client
    }

    // regression: #1362
    /// An attestor who belongs to two *different* slices must be able to
    /// attest for the same credential in each slice independently.
    /// Before the fix this panicked because the duplicate guard was keyed
    /// globally per (credential, attestor) instead of per
    /// (credential, slice, attestor).
    #[test]
    fn regression_1362_attestor_may_attest_once_per_slice() {
        // … test body …
    }
}
```

### Existing regressions

The current regression inventory lives in
`contracts/integration_tests/src/regressions.rs`.  Each module in that
file corresponds to one closed issue.

| Issue | Module | Description |
|---|---|---|
| #1362 | `regression_1362` | Per-(credential,slice) attestation duplicate guard; `check_quorum_intersection` validates safe_nodes across all slices |

---

## Contribution Process (Issue #1682)

1. **Find or open an issue.** Check the existing issues first. For anything
   beyond a typo, open an issue describing the problem and proposed change
   so effort is not duplicated.
2. **Fork and branch.** Work on a feature branch off `main`. Name it after
   the issue, for example `fix/1721-zk-verifier-initialize-auth`. Several
   issues can share one branch when they are closed by one PR; list every
   issue number in the branch name and in the PR description.
3. **Make focused commits.** Each commit should be one logical change with
   a message in the form `type(scope): summary (#N)`, where `type` is one of
   `feat`, `fix`, `docs`, `chore`, `test` or `refactor`.
4. **Run the checks locally** (see [Testing](#testing) and
   [docs/linting.md](docs/linting.md)).
5. **Open a pull request** using the template in
   `.github/PULL_REQUEST_TEMPLATE.md`. Link the issues with
   `Closes #N` so they close on merge.
6. **Respond to review.** Push fixes as new commits so reviewers can see what
   changed. Squash only if a maintainer asks.

## Coding Standards (Issue #1682)

- **Lint and format.** Follow [docs/linting.md](docs/linting.md). It covers
  clippy for Rust, ESLint for the frontend and dashboard, ruff for Python and
  `.editorconfig` for whitespace. `./scripts/lint.sh` runs whichever linters
  are installed.
- **Rust contracts.** No bare `.unwrap()` in production contract code. Return
  a typed `ContractError` instead (see `scripts/check_unwraps.sh`, #1391).
  Every public entry point needs a doc-comment covering parameters, panics and
  side effects.
- **Authorization.** Any state-changing entry point must check who is calling.
  Call `require_auth()` on the address whose authority is being used, and add a
  test that an unauthorized caller is rejected.
- **TypeScript.** Keep types explicit at module boundaries. Do not add
  `eslint-disable` without a comment giving the reason.
- **Python.** Keep scripts runnable with the standard library where practical.
  Use `if __name__ == "__main__":` for entry points.
- **Documentation.** Update the relevant file under `docs/` when behaviour or
  an interface changes. Add new docs to `docs/README.md` when they are created.

## Testing

Run the tests that match the code you touched. CI runs all of them, so
running them locally first saves a review cycle.

| Area | Command |
|---|---|
| Rust contracts | `cargo test --target x86_64-unknown-linux-gnu --workspace` |
| Rust integration and regressions | `cargo test --target x86_64-unknown-linux-gnu -p integration_tests` |
| api-server | `cd api-server && npm test` |
| frontend | `cd frontend && npm run lint && npm test` |
| dashboard | `cd dashboard && npm run lint && npm test` |

Rust tests run against the host target. Contract WASM builds use
`--target wasm32-unknown-unknown --release`.

## Pull Request Checklist (Issue #1682)

Every PR should satisfy the checklist in `.github/PULL_REQUEST_TEMPLATE.md`.
In short:

- [ ] Linked to the issue(s) it closes, using `Closes #N`.
- [ ] Tests added or updated, including a regression test for any bug fix
      (see [Regression Test Convention](#regression-test-convention-issue-1479)).
- [ ] Local lint and tests pass.
- [ ] Public functions and types are documented.
- [ ] Docs updated for any behaviour or interface change.
- [ ] No secrets, keys or `.env` values committed.

## Review Process (Issue #1682)

- **Reviewers.** Merging needs approval from a maintainer, as enforced by
  the default branch's protection rules.
  Changes to contracts, authorization, deployment scripts or CI need review
  from someone who has worked on that area.
- **Waiting on review.** If a PR has no response, ask in the PR thread and
  tag a maintainer.
- **What reviewers check.** Correctness and tests first, then security and
  authorization, then API and documentation, then style. Style comments that
  a linter would catch should not be raised by hand.
- **Resolving comments.** The author resolves a thread after pushing the
  change it asked for, or replies explaining why no change was made. Reviewers
  resolve threads they are satisfied with.
- **Merging.** PRs are merged by a maintainer once CI is green and approvals
  are in place. Do not merge your own PR without a review.
- **Security.** Do not discuss suspected vulnerabilities in public issues or
  PR threads. Follow [SECURITY.md](SECURITY.md).

## General Guidelines

- Fork the repository and work on a feature branch.
- Run `cargo test --target x86_64-unknown-linux-gnu --workspace` before
  opening a PR.
- Keep PR titles under 70 characters; use the description for details.
- Open a GitHub issue before starting non-trivial work so effort isn't
  duplicated.

## Code Style

- Follow the Rust API guidelines.
- Keep `#[contracttype]` structs in `lib.rs`; business logic in separate
  modules under `src/`.
- All public contract entry points must have a doc-comment explaining
  parameters, panics, and side effects.

## Security

See [SECURITY.md](SECURITY.md) for the vulnerability disclosure policy.
Do not open public issues for security vulnerabilities.
