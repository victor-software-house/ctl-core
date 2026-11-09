#!/usr/bin/env bash
#MISE description="Run the TOML input tests with only the input-toml feature"
# A consumer that reads only TOML enables input-toml alone and gets no YAML
# parser and no garde. A crate that turns on input-yaml for the whole build
# would hide a gap, and would change the limit messages. This builds ctl-core
# with input-toml alone and holds a TOML text to its byte, depth, and node
# limits (split-input-formats, decisions 3 and 5).
set -euo pipefail
cargo test --locked --package ctl-core --no-default-features --features input-toml --test input_toml_alone --no-fail-fast
