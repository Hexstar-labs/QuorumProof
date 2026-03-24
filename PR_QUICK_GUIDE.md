# Quick Guide: Creating PR for Issue #21

## ✅ What's Been Done

1. **Branch Created**: `fix/issue-21-issuer-revocation`
2. **Changes Committed**: All code changes and tests committed
3. **Tests Passing**: All 5 tests pass successfully
4. **Build Verified**: WASM compilation successful

## 🚀 To Push and Create PR

### Option 1: Using the Script
```bash
./create_pr.sh
```

### Option 2: Manual Push
```bash
# Push to your fork
git push -u origin fix/issue-21-issuer-revocation
```

### Option 3: Using GitHub CLI (if installed)
```bash
# Push and create PR in one command
gh pr create --base main --head fix/issue-21-issuer-revocation \
  --title "Fix #21: Add issuer revocation support" \
  --body-file PR_DESCRIPTION.md \
  --repo QuorumProof/QuorumProof
```

## 📝 Creating the PR on GitHub

1. **After pushing**, GitHub will show a link to create PR, or go to:
   - Your fork: https://github.com/Chibey-max/QuorumProof
   - Click "Compare & pull request" button

2. **Or manually create PR**:
   - Go to: https://github.com/QuorumProof/QuorumProof/compare
   - Click "compare across forks"
   - Set:
     - Base repository: `QuorumProof/QuorumProof`
     - Base branch: `main`
     - Head repository: `Chibey-max/QuorumProof`
     - Compare branch: `fix/issue-21-issuer-revocation`

3. **Fill in PR details**:
   - **Title**: `Fix #21: Add issuer revocation support`
   - **Description**: Copy content from `PR_DESCRIPTION.md`
   - **Labels**: `bug`, `smart-contract`, `high-priority`
   - **Reviewers**: Add relevant team members
   - **Link Issue**: Mention "Fixes #21" in description

## 🔐 If Authentication Required

### SSH Key (Recommended)
```bash
# Generate SSH key
ssh-keygen -t ed25519 -C "your_email@example.com"

# Add to ssh-agent
eval "$(ssh-agent -s)"
ssh-add ~/.ssh/id_ed25519

# Copy public key and add to GitHub
cat ~/.ssh/id_ed25519.pub
# Go to GitHub Settings > SSH Keys > New SSH Key

# Update remote to use SSH
git remote set-url origin git@github.com:Chibey-max/QuorumProof.git
```

### Personal Access Token
```bash
# Create token at: https://github.com/settings/tokens
# Select scopes: repo (all)

# Use token when pushing
git push -u origin fix/issue-21-issuer-revocation
# Username: your-github-username
# Password: your-personal-access-token
```

### GitHub CLI
```bash
# Install gh CLI: https://cli.github.com/
gh auth login
# Follow prompts
```

## 📊 PR Checklist

Before submitting, verify:
- [x] Branch name: `fix/issue-21-issuer-revocation`
- [x] All tests pass (5/5)
- [x] Build successful
- [x] Commit message descriptive
- [x] PR description complete
- [x] Issue #21 referenced
- [ ] Branch pushed to fork
- [ ] PR created on main repo
- [ ] Labels added
- [ ] Reviewers assigned

## 📁 Files in This PR

- `contracts/quorum_proof/src/lib.rs` - Main implementation
- `contracts/quorum_proof/test_snapshots/tests/*.json` - Test snapshots
- `ISSUE_21_FIX_SUMMARY.md` - Technical documentation
- `PR_DESCRIPTION.md` - PR description template
- `create_pr.sh` - Helper script
- `PR_QUICK_GUIDE.md` - This guide

## 🎯 Summary

**Issue**: Only subject could revoke credentials  
**Solution**: Allow both subject AND issuer to revoke  
**Tests**: 3 new tests, all passing  
**Breaking Change**: Function signature updated (requires caller parameter)  

## 💡 Need Help?

If you encounter issues:
1. Check git status: `git status`
2. Check remote: `git remote -v`
3. Check branch: `git branch --show-current`
4. View commit: `git log --oneline -1`
5. Test locally: `cargo test --package quorum_proof`
