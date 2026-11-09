#!/usr/bin/env bash
#MISE description="Run the parser tests with only the cli feature"
# Clap drops its -h/--help flag when its help feature is off, and a crate that
# turns that feature on for the whole build hides the gap. This builds
# ctl-core with the cli feature alone and checks that -h, --help, and -V
# still parse (size-cuts, design decision 6).
set -euo pipefail
cargo test --locked --package ctl-core --no-default-features --features cli --test parse --no-fail-fast
