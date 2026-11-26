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
# defaults or the features they hold. Only input-document turns on error-context,
# through ctl-core's own feature of that name, so a bad value in an input
# document is placed on its key (CTC-043).
#
# input-toml alone reads TOML with no YAML parser and no garde
# (split-input-formats, decision 6). input-yaml alone reads YAML with no TOML
# parser, and neither it nor input links yamled. input-json alone reads JSON
# with no TOML or YAML parser and no yamled: the reader is ctl-input's own.
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
toml="$(cargo tree --locked --package ctl-core --edges normal --no-default-features --features input-toml --target all --prefix none --format '{p}')"
for crate in serde-saphyr vsh-serde-saphyr garde; do
  if grep -q "^${crate} v" <<<"${toml}"; then
    echo "ctl-core with input-toml alone depends on ${crate}" >&2
    status=1
  fi
done
yaml="$(cargo tree --locked --package ctl-core --edges normal --no-default-features --features input-yaml --target all --prefix none --format '{p}')"
for crate in yamled toml; do
  if grep -q "^${crate} v" <<<"${yaml}"; then
    echo "ctl-core with input-yaml alone depends on ${crate}" >&2
    status=1
  fi
done
json="$(cargo tree --locked --package ctl-core --edges normal --no-default-features --features input-json --target all --prefix none --format '{p}')"
for crate in toml yamled serde-saphyr; do
  if grep -q "^${crate} v" <<<"${json}"; then
    echo "ctl-core with input-json alone depends on ${crate}" >&2
    status=1
  fi
done
input="$(cargo tree --locked --package ctl-core --edges normal --no-default-features --features input --target all --prefix none --format '{p}')"
if grep -q '^yamled v' <<<"${input}"; then
  echo "ctl-core with input depends on yamled" >&2
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
documented="$(cargo tree --locked --package ctl-core --edges features --no-default-features --features app,input,input-document --target all --invert clap_builder --prefix none)"
if ! grep -q 'clap_builder feature "error-context"' <<<"${documented}"; then
  echo "ctl-core with app, input, and input-document does not turn on clap's error-context feature" >&2
  status=1
fi
for features in app,input-toml app,input-json; do
  undocumented="$(cargo tree --locked --package ctl-core --edges features --no-default-features --features "${features}" --target all --invert clap_builder --prefix none)"
  if grep -q 'clap_builder feature "error-context"' <<<"${undocumented}"; then
    echo "ctl-core with ${features} turns on clap's error-context feature" >&2
    status=1
  fi
done
exit "${status}"
