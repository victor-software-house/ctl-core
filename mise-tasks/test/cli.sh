#!/usr/bin/env bash
#MISE description="Run the parser tests with only the cli feature"
# Clap drops -h/--help and the help subcommand when its help feature is off,
# so ctl-core declares them itself. A crate that turns that feature on for the
# whole build would hide a gap. This builds ctl-core with the cli feature
# alone and checks that -h, --help, -V, and help <command> still parse
# (size-cuts, design decision 6).
set -euo pipefail
cargo test --locked --package ctl-core --no-default-features --features cli --test parse --no-fail-fast
