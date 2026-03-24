#!/bin/bash

# Script to push branch and create PR for Issue #21

echo "=========================================="
echo "Issue #21 Fix - Push and Create PR"
echo "=========================================="
echo ""

# Check if we're on the correct branch
CURRENT_BRANCH=$(git branch --show-current)
if [ "$CURRENT_BRANCH" != "fix/issue-21-issuer-revocation" ]; then
    echo "❌ Error: Not on the correct branch"
    echo "Current branch: $CURRENT_BRANCH"
    echo "Expected: fix/issue-21-issuer-revocation"
    exit 1
fi

echo "✅ On correct branch: $CURRENT_BRANCH"
echo ""

# Show commit summary
echo "📝 Commit Summary:"
git log --oneline -1
echo ""

# Push to origin (your fork)
echo "🚀 Pushing to origin (your fork)..."
git push -u origin fix/issue-21-issuer-revocation

if [ $? -eq 0 ]; then
    echo ""
    echo "✅ Successfully pushed to origin!"
    echo ""
    echo "=========================================="
    echo "Next Steps:"
    echo "=========================================="
    echo ""
    echo "1. Go to: https://github.com/Chibey-max/QuorumProof/pull/new/fix/issue-21-issuer-revocation"
    echo ""
    echo "2. Or go to: https://github.com/QuorumProof/QuorumProof/compare/main...Chibey-max:QuorumProof:fix/issue-21-issuer-revocation"
    echo ""
    echo "3. Use the PR description from PR_DESCRIPTION.md"
    echo ""
    echo "4. Set the following:"
    echo "   - Base repository: QuorumProof/QuorumProof"
    echo "   - Base branch: main"
    echo "   - Head repository: Chibey-max/QuorumProof"
    echo "   - Compare branch: fix/issue-21-issuer-revocation"
    echo ""
    echo "5. Title: Fix #21: Add issuer revocation support"
    echo ""
    echo "6. Add labels: bug, smart-contract, high-priority"
    echo ""
    echo "=========================================="
else
    echo ""
    echo "❌ Push failed. You may need to authenticate."
    echo ""
    echo "Options:"
    echo "1. Set up SSH key: https://docs.github.com/en/authentication/connecting-to-github-with-ssh"
    echo "2. Use GitHub CLI: gh auth login"
    echo "3. Use Personal Access Token"
    echo ""
    echo "After authentication, run: git push -u origin fix/issue-21-issuer-revocation"
fi
