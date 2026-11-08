#!/usr/bin/env bash
#MISE description="Fail when ctl-core's app tier depends on a terminal or table crate it replaced"
# The render engine replaced comfy-table and anstream (terminal-render-engine,
# task 6.3). This keeps them, and the crates only they pulled in, out of the
# normal dependencies of ctl-core with app, on every target.
set -euo pipefail
tree="$(cargo tree --locked --package ctl-core --edges normal --no-default-features --features app --target all --prefix none --format '{p}')"
status=0
for crate in comfy-table crossterm anstream anstyle-parse console; do
  if grep -q "^${crate} v" <<<"${tree}"; then
    echo "ctl-core with app depends on ${crate}" >&2
    status=1
  fi
done
exit "${status}"
