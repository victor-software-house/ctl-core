#!/usr/bin/env bash
#MISE description="Run the parse-error tests with app and no error-context"
# Clap's error-context feature is opt-in: the app feature does not turn it on,
# and a parse error then has its reason and its usage line but no argument,
# value, or tip. A crate that turns error-context on for the whole build would
# hide a gap, so this builds ctl-core with app alone and holds an unknown
# argument to that envelope.
set -euo pipefail
cargo test --locked --package ctl-core --no-default-features --features app --test error_context_off --no-fail-fast
