# Tasks

- [x] 1. Plan: this change, and the CTC-021 row added with `qctl add`. Proof:
  `openspec validate toml-input` and `mise run q check` pass.
- [x] 2. Add the `toml` dependency and the `input-toml` feature, list the
  feature in `check:features` and `.copier-answers.yml`. Proof:
  `cargo check --locked --no-default-features --features input-toml`
  passes on a build runner, and `cargo tree` for `input` alone lists no
  `toml` crate.
- [x] 3. Split the garde path once (`field_steps`) and make the YAML locator
  use it. Proof: the existing `tests/input.rs` passes unchanged.
- [x] 4. Add `Format`, `Input::format`, `Input::open`, and the TOML parse,
  placement, and limit paths. Proof: `tests/input_toml.rs` passes.
- [x] 5. Document the feature and the format rule in AGENTS.md and the
  crate docs, and add the changeset. Proof: `mise run verify` passes on a
  build runner, including `test:doc` and `check:features`.
- [ ] 6. Open the PR, wait for CI, and answer each review thread with the
  commit that fixes it. Proof: CI green on the last push, no open thread.

## Evidence

2026-10-14 UTC, tasks 1 to 5. On a macOS arm64 build runner with mise
2026.10.9, `MISE_ENV=ci mise run verify` exited 0 on the tree of the commit
that adds the loader: format, clippy, `check:features` (including
`input-toml` alone), 180 unit and integration tests (16 in
`tests/input_toml.rs`, the 6 in `tests/input.rs` and the 3 in
`tests/input_limits.rs` unchanged), doc-tests, cargo-deny licenses, bans,
and sources (all `ok`), and cargo-machete.

`cargo tree --locked -e normal --no-default-features --features input` lists
no `toml` crate. With `--features input-toml` the `toml` subtree is
`serde_core`, `serde_spanned`, `toml_datetime`, `toml_parser`, and `winnow`
1.0.4; `toml_writer` and `indexmap` are in `Cargo.lock` but not in that
graph. cargo-deny now warns about two `winnow` versions (0.7.15 under
serde-saphyr's parser, 1.0.4 under `toml`), a warning by `deny.toml`'s
`multiple-versions = "warn"`.

`mise run semver` against ctl-core 0.6.18 reported "no semver update
required" (202 checks, 202 pass).

A scratch copy of the tree passed the CI `Template` job's check: `copier
update --defaults` changed only `.copier-answers.yml` (`_commit` v0.0.18 to
v0.0.19).
