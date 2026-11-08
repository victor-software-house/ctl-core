#!/usr/bin/env bash
#MISE description="Compile each advertised feature set without extra features"
set -euo pipefail
cargo check --locked --package ctl-core --no-default-features
for features in document verbatim render surface-model surface-serde surface-templates surface validate input input-toml fixture served usage json cli view help app app,usage schema shell shell,view completion shell-init app,shell-init; do
  cargo check --locked --package ctl-core --no-default-features --features "${features}"
done
