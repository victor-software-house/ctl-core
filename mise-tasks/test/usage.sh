#!/usr/bin/env bash
#MISE description="Run the Usage spec tests with only the usage feature"
# The spec's bytes must not depend on a Clap feature that another crate in a
# consumer's graph turns on. This builds ctl-core with the usage feature
# alone, as a consumer of the plain tier does, and holds the spec to the same
# snapshots the full build checks (size-cuts, design decision 3).
set -euo pipefail
cargo test --locked --package ctl-core --no-default-features --features usage --test usage_spec --no-fail-fast
