#!/usr/bin/env bash
#MISE description="Run the parse-error tests with app and no error-context"
# Clap's error-context feature is opt-in: the app feature does not turn it on,
# and a parse error then has its reason and its usage line but no argument,
# value, or tip, and its text is shorter. The all-features test run turns
# ctl-core's error-context on, so this builds ctl-core with app alone and runs
# the tests that read a parse error and a pretty error, which hold the text to
# the snapshots a build without Clap's error-context has always had.
set -euo pipefail
cargo test --locked --package ctl-core --no-default-features --features app --test error_context_off --test clap_display --test json_failures --no-fail-fast
