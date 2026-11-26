#!/usr/bin/env bash
#MISE description="Run the help and spec tests with app and no input-document"
# The input flags exist only with input-document, so a build without it must
# print the help, the usage spec and the missing-argument help it always
# printed. The all-features test run turns input-document on and holds those
# pages to the snapshots with the Input section, so this builds ctl-core with
# app and usage alone and holds them to the snapshots of that build.
set -euo pipefail
cargo test --locked --package ctl-core --no-default-features --features app,usage --test help_sections --test formats --test usage_spec --test clap_display --no-fail-fast
