#!/usr/bin/env bash
#MISE description="Compile each advertised feature set without extra features"
set -euo pipefail
cargo check --locked --no-default-features
for features in document render surface-model surface-serde surface-templates surface validate input input-toml fixture served usage json cli view help app app,usage schema; do
  cargo check --locked --no-default-features --features "${features}"
done
