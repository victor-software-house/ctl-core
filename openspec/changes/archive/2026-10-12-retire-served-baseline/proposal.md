# Proposal: retire the served baseline

## Why

Two generators rendered the same Rust configuration. `baseline::render`
(CTC-015) served `rust-toolchain.toml`, `rustfmt.toml`, `clippy.toml`,
`deny.toml`, `lefthook.yml`, and the `Cargo.toml` lints and `mise.dev.toml`
tools blocks from `templates/baseline/`. The shared Copier template (kind
`rust-crate`) renders the same files, and since its `v0.0.2` also the mise
profiles and the hooks. Each rule had to change in both places, and
`tests/baseline.rs` failed whenever they drifted. No consumer enables the
`baseline` feature, and no runner ever wired `baseline::render` into a CLI.
The operator chose Copier as the only generator (CTC-018).

## What Changes

1. The `baseline` feature and module, `templates/baseline/`,
   `tests/baseline.rs`, and `.ctl/rust.yaml` are removed, together with
   `Served::with_body`, which only the baseline used.
2. The `served-baseline` spec is removed.
3. `check:features` no longer compiles the `baseline` feature set, and
   `.copier-answers.yml` no longer lists it.

## Capabilities

### New Capabilities

None.

### Modified Capabilities

- `served-baseline`: removed.

## Impact

1. A crate that enables `ctl-core/baseline` stops compiling. None does:
   verctl, qctl, and forkctl pin older releases without it.
2. A repository takes the shared Rust configuration from the Copier
   template instead.
