#!/usr/bin/env bash
# Build, test and deploy the FlexiPay escrow ("Safe pay") contract.
# Needs: Rust + `rustup target add wasm32v1-none`, and the Stellar CLI
#   cargo install --locked stellar-cli   (or: brew install stellar-cli)
#
#   NETWORK=testnet ./scripts/deploy.sh
#
# Prints the contract ID. Put it in the frontend's .env as VITE_ESCROW_CONTRACT_ID.
set -euo pipefail
NETWORK="${NETWORK:-testnet}"
IDENTITY="${IDENTITY:-flexipay-deployer}"
cd "$(dirname "$0")/.."

echo "▸ Testing"
cargo test --quiet

echo "▸ Building wasm"
stellar contract build
WASM=target/wasm32v1-none/release/flexipay_escrow.wasm

if ! stellar keys address "$IDENTITY" >/dev/null 2>&1; then
  echo "▸ Creating deployer key '$IDENTITY'"
  if [ "$NETWORK" = "testnet" ]; then
    stellar keys generate "$IDENTITY" --network testnet --fund
  else
    stellar keys generate "$IDENTITY"
    echo "Fund $(stellar keys address "$IDENTITY") on $NETWORK, then re-run."; exit 1
  fi
fi

echo "▸ Deploying to $NETWORK"
ID=$(stellar contract deploy --wasm "$WASM" --source-account "$IDENTITY" --network "$NETWORK")
echo
echo "✓ Escrow contract: $ID"
echo "  Frontend: VITE_ESCROW_CONTRACT_ID=$ID"
echo "$ID" > ".contract-id.$NETWORK"
