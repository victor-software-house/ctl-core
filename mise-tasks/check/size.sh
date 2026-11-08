#!/usr/bin/env bash
#MISE description="Fail when the toy CLI's .text at a feature tier grows past its budget"
# Builds ctl-core's `size` example with the release profile at each tier in
# [workspace.metadata.size] of the root Cargo.toml, reads `.text` with
# binutils' size, and fails when it exceeds the tier's budget by more than
# tolerance-percent. The budgets are linux-x64 numbers, so other platforms
# skip the check.
set -euo pipefail
platform="$(uname -s)-$(uname -m)"
if [[ "${platform}" != "Linux-x86_64" ]]; then
  echo "check:size: the budgets are linux-x64 numbers; skipped on ${platform}" >&2
  exit 0
fi
config="$(cargo metadata --format-version 1 --no-deps --locked | jq -c '.metadata.size')"
tolerance="$(jq -r '.["tolerance-percent"]' <<<"${config}")"
status=0
while IFS=$'\t' read -r tier features budget; do
  binary="$(cargo build --locked --release --package ctl-core --example size \
    --no-default-features --features "${features}" --message-format json-render-diagnostics |
    jq -r 'select(.reason == "compiler-artifact" and .target.name == "size") | .executable')"
  text="$(size -A "${binary}" | awk '$1 == ".text" { print $2 }')"
  if [[ ! "${text}" =~ ^[0-9]+$ ]]; then
    echo "check:size: ${tier}: size -A found no .text size in ${binary}" >&2
    status=1
    continue
  fi
  limit=$((budget + budget * tolerance / 100))
  echo "check:size: ${tier} (${features}): .text ${text} bytes, budget ${budget}, limit ${limit}"
  if ((text > limit)); then
    echo "check:size: ${tier} is ${text} bytes, over its limit of ${limit}" >&2
    status=1
  fi
done < <(jq -r '.tiers | to_entries[] | [.key, .value.features, .value.text] | @tsv' <<<"${config}")
exit "${status}"
