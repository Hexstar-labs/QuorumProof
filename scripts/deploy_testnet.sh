#!/bin/bash
# scripts/deploy_testnet.sh — Deploy all contracts to testnet.
#
# In CI (see .github/workflows/testnet-deploy.yml) this is followed by
# scripts/testnet_smoke_test.sh; on smoke-test failure
# scripts/testnet_rollback.sh restores the previous manifest so downstream
# services keep pointing at the last known-good deployment.
set -e

if [ -f .env ]; then
  source .env
fi

WASM_DIR="${WASM_DIR:-target/wasm32-unknown-unknown/release}"
MANIFEST_PATH="${MANIFEST_PATH:-testnet-deployment.json}"

echo "Deploying to testnet..."

stellar keys generate deployer --network testnet --fund 2>/dev/null || true
DEPLOYER_ADDRESS=$(stellar keys address deployer)

CONTRACT_QUORUM_PROOF=$(stellar contract deploy \
  --wasm "$WASM_DIR/quorum_proof.wasm" \
  --source deployer \
  --network testnet)

CONTRACT_SBT_REGISTRY=$(stellar contract deploy \
  --wasm "$WASM_DIR/sbt_registry.wasm" \
  --source deployer \
  --network testnet)

CONTRACT_ZK_VERIFIER=$(stellar contract deploy \
  --wasm "$WASM_DIR/zk_verifier.wasm" \
  --source deployer \
  --network testnet)

# Issue #1721: claim the zk_verifier admin role immediately after deployment.
# `initialize` requires the admin's authorization, so the deployer (the signing
# source) becomes admin and no one else can. Soroban SDK 21 cannot deploy and
# initialize in one transaction, so there is a short window between the two
# calls. See docs/zk-verifier-initialization.md for the residual risk and the
# post-deploy check.
stellar contract invoke \
  --id "$CONTRACT_ZK_VERIFIER" \
  --source deployer \
  --network testnet \
  -- initialize --admin "$DEPLOYER_ADDRESS"

echo "CONTRACT_QUORUM_PROOF=$CONTRACT_QUORUM_PROOF"
echo "CONTRACT_SBT_REGISTRY=$CONTRACT_SBT_REGISTRY"
echo "CONTRACT_ZK_VERIFIER=$CONTRACT_ZK_VERIFIER"

# Preserve the previous manifest (if any) so a failed smoke test can restore
# it via testnet_rollback.sh — this is a fresh deployment, not an in-place
# upgrade, so "rollback" means "point everyone back at the last known-good
# contract IDs," not reverting on-chain state.
if [ -f "$MANIFEST_PATH" ]; then
  cp "$MANIFEST_PATH" "${MANIFEST_PATH}.previous"
fi

cat > "$MANIFEST_PATH" <<EOF
{
  "network": "testnet",
  "deployed_at": "$(date -u +%Y-%m-%dT%H:%M:%SZ)",
  "deployer": "$DEPLOYER_ADDRESS",
  "contracts": {
    "quorum_proof": "$CONTRACT_QUORUM_PROOF",
    "sbt_registry": "$CONTRACT_SBT_REGISTRY",
    "zk_verifier": "$CONTRACT_ZK_VERIFIER"
  }
}
EOF

echo "Wrote deployment manifest to $MANIFEST_PATH"
