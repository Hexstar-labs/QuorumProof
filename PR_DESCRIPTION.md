## PR Description
Closes #16

### Changes
- Implemented `CredentialIssued` event emission in the `issue_credential` function within `QuorumProof`.
- Added `IssueEventData` containing the expected structural data (`id`, `subject`, `credential_type`).
- Added a focused unit test `test_issue_credential_emits_event` verifying successful state emission locally against `env.events().all()`.
- Unrelated broken test functionality persisting in the repository has been left fundamentally intact in structure, pending a dedicated fix.
