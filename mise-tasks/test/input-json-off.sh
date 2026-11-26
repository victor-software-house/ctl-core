#!/usr/bin/env bash
#MISE description="Run the declared-input tests with input and no input-json"
# JSON input is opt-in: input is input-yaml with validate, and does not turn on
# input-json. A crate that turns on input-json for the whole build would hide a
# gap, so this builds ctl-core with input alone and holds a .json file to an
# unsupported extension (json-input, the JSON is off scenario).
set -euo pipefail
cargo test --locked --package ctl-core --no-default-features --features input --test input_json_off --no-fail-fast
