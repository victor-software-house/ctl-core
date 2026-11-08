#!/usr/bin/env bash
#MISE description="Compile each advertised feature set without extra features, warning-free"
# A warning fails the check: an item gated on more features than use it
# compiles unused in some set, and only that set shows it. cargo check takes
# no rustc arguments after --, so the denial goes through RUSTFLAGS.
set -euo pipefail
export RUSTFLAGS="${RUSTFLAGS:+${RUSTFLAGS} }-D warnings"
cargo check --locked --package ctl-core --no-default-features
for features in document verbatim render surface-model surface-serde surface-templates surface validate input input-toml fixture served usage json cli view help app app,usage schema shell shell,view completion shell-init app,shell-init; do
  cargo check --locked --package ctl-core --no-default-features --features "${features}"
done
