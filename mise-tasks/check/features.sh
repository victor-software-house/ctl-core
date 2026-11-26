#!/usr/bin/env bash
#MISE description="Compile each advertised feature set without extra features, warning-free"
# A warning fails the check: an item gated on more features than use it
# compiles unused in some set, and only that set shows it. cargo check takes
# no rustc arguments after --, so the denial goes through RUSTFLAGS.
set -euo pipefail
export RUSTFLAGS="${RUSTFLAGS:+${RUSTFLAGS} }-D warnings"
cargo check --locked --package ctl-core --no-default-features
cargo check --locked --package ctl-term --no-default-features
cargo check --locked --package ctl-input --no-default-features
for features in yaml json toml json,validate yaml,validate toml,validate yaml,toml yaml,toml,validate; do
  cargo check --locked --package ctl-input --no-default-features --features "${features}"
done
for features in document unsafe-chars render surface-model surface-serde surface-templates surface validate input-yaml input-json input-json,validate input-toml input-toml,validate input fixture served usage json cli view help app app,usage app,usage,input app,input-toml error-context app,error-context schema shell shell,view completion shell-init app,shell-init; do
  cargo check --locked --package ctl-core --no-default-features --features "${features}"
done
