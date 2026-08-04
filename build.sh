#!/bin/bash
set -e

DEPLOY=false
if [ "$1" = "--release" ]; then
  DEPLOY=true
fi

cargo build --release --target wasm32-unknown-unknown

echo "Plugins have been built"

if [ "$DEPLOY" = true ]; then
  cp target/wasm32-unknown-unknown/release/*.wasm ~/.openagent/plugins/
  echo "Plugins have been copied to ~/.openagent/plugins/"
fi
