#!/usr/bin/env bash
#MISE description="Fail when ctl-core depends on a crate it replaced, or a feature set turns on clap features it must not"
# The render engine replaced comfy-table and anstream (terminal-render-engine,
# task 6.3). This keeps them, and the crates only they pulled in, out of the
# normal dependencies of ctl-core with app, on every target.
#
# ctl-core writes the Usage KDL itself (size-cuts, decisions 5 and 6).
# usage-lib, and kdl and winnow 0.x, which only the spec tests use, stay out
# of the normal dependencies under every feature. ctl-core declares its own
# -h/--help flag and help subcommand, so the cli feature leaves clap's help
# feature off. The usage feature turns on clap's usage feature, which renders
# the spec's usage line. With app, usage, and input, nothing turns on clap's
# defaults or the features they hold.
set -euo pipefail
status=0
app="$(cargo tree --locked --package ctl-core --edges normal --no-default-features --features app --target all --prefix none --format '{p}')"
for crate in comfy-table crossterm anstream anstyle-parse console; do
  if grep -q "^${crate} v" <<<"${app}"; then
    echo "ctl-core with app depends on ${crate}" >&2
    status=1
  fi
done
every="$(cargo tree --locked --package ctl-core --edges normal --all-features --target all --prefix none --format '{p}')"
for crate in usage-lib kdl; do
  if grep -q "^${crate} v" <<<"${every}"; then
    echo "ctl-core with every feature depends on ${crate}" >&2
    status=1
  fi
done
if grep -q '^winnow v0\.' <<<"${every}"; then
  echo "ctl-core with every feature depends on winnow below 1.0" >&2
  status=1
fi
cli="$(cargo tree --locked --package ctl-core --edges features --no-default-features --features cli --target all --invert clap_builder --prefix none)"
if grep -q 'clap_builder feature "help"' <<<"${cli}"; then
  echo "ctl-core with cli turns on clap's help feature" >&2
  status=1
fi
usage="$(cargo tree --locked --package ctl-core --edges features --no-default-features --features usage --target all --invert clap_builder --prefix none)"
if ! grep -q 'clap_builder feature "usage"' <<<"${usage}"; then
  echo "ctl-core with usage does not turn on clap's usage feature" >&2
  status=1
fi
consumer="$(cargo tree --locked --package ctl-core --edges features --no-default-features --features app,usage,input --target all --invert clap_builder --prefix none)"
for feature in default color help error-context suggestions; do
  if grep -q "clap_builder feature \"${feature}\"" <<<"${consumer}"; then
    echo "ctl-core with app, usage, and input turns on clap's ${feature} feature" >&2
    status=1
  fi
done
exit "${status}"
