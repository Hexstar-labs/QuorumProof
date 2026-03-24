#!/bin/bash

# QuorumProof Dashboard Project Verification

echo "╔════════════════════════════════════════════════════════════╗"
echo "║        QuorumProof Dashboard - Project Verification       ║"
echo "╚════════════════════════════════════════════════════════════╝"
echo ""

cd "$(dirname "$0")" || exit 1

ERRORS=0
WARNINGS=0

# Check 1: Git branch
echo "• Checking git branch..."
if git branch | grep -q "feature/credential-card-component"; then
    echo "  ✓ Feature branch exists: feature/credential-card-component"
else
    echo "  ✗ Feature branch not found"
    ((ERRORS++))
fi

# Check 2: Files exist
echo ""
echo "• Checking required files..."
REQUIRED_FILES=(
    "dashboard/src/components/CredentialCard.tsx"
    "dashboard/src/types/credential.ts"
    "dashboard/src/styles/credentialCard.css"
    "dashboard/src/App.tsx"
    "dashboard/package.json"
    "dashboard/tsconfig.json"
    "dashboard/vite.config.ts"
    "dashboard/README.md"
    "dashboard/SETUP.md"
)

for file in "${REQUIRED_FILES[@]}"; do
    if [ -f "$file" ]; then
        echo "  ✓ $file"
    else
        echo "  ✗ $file missing"
        ((ERRORS++))
    fi
done

# Check 3: Git commits
echo ""
echo "• Checking git commits..."
COMMIT_COUNT=$(git rev-list --count feature/credential-card-component ^main 2>/dev/null || echo "0")
if [ "$COMMIT_COUNT" -ge 3 ]; then
    echo "  ✓ Found $COMMIT_COUNT commits on feature branch"
else
    echo "  ✗ Expected at least 3 commits, found $COMMIT_COUNT"
    ((ERRORS++))
fi

# Check 4: TypeScript config
echo ""
echo "• Checking TypeScript configuration..."
if grep -q '"jsx": "react-jsx"' dashboard/tsconfig.json; then
    echo "  ✓ JSX support configured"
else
    echo "  ✗ JSX configuration incomplete"
    ((WARNINGS++))
fi

# Check 5: Component structure
echo ""
echo "• Checking component structure..."
if grep -q "export.*CredentialCard" dashboard/src/components/CredentialCard.tsx; then
    echo "  ✓ CredentialCard properly exported"
else
    echo "  ✗ CredentialCard export issue"
    ((ERRORS++))
fi

if grep -q "export interface CredentialCardProps" dashboard/src/components/CredentialCard.tsx; then
    echo "  ✓ CredentialCardProps interface exported"
else
    echo "  ✗ CredentialCardProps not exported"
    ((ERRORS++))
fi

# Check 6: Styling
echo ""
echo "• Checking CSS styling..."
if grep -q "credential-card {" dashboard/src/styles/credentialCard.css; then
    echo "  ✓ Component styles present"
else
    echo "  ✗ Component styles missing"
    ((ERRORS++))
fi

# Check 7: Package.json dependencies
echo ""
echo "• Checking dependencies..."
DEPS="react react-dom lucide-react clsx date-fns"
for dep in $DEPS; do
    if grep -q "\"$dep\"" dashboard/package.json; then
        echo "  ✓ $dep"
    else
        echo "  ✗ $dep missing from package.json"
        ((ERRORS++))
    fi
done

# Summary
echo ""
echo "╔════════════════════════════════════════════════════════════╗"
if [ $ERRORS -eq 0 ]; then
    echo "║              ✓ All checks passed!                         ║"
else
    echo "║              ✗ Found $ERRORS error(s)                         ║"
fi
if [ $WARNINGS -gt 0 ]; then
    echo "║              ⚠ Found $WARNINGS warning(s)                      ║"
fi
echo "╚════════════════════════════════════════════════════════════╝"
echo ""

if [ $ERRORS -eq 0 ]; then
    echo "Next steps:"
    echo "  1. cd dashboard"
    echo "  2. npm install"
    echo "  3. npm run dev"
    echo "  4. Open http://localhost:5173"
    echo ""
    echo "To create PR on GitHub:"
    echo "  gh pr create --base main --head feature/credential-card-component"
    exit 0
else
    echo "Please fix the errors listed above before proceeding."
    exit 1
fi
