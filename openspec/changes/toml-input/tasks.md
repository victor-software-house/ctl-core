# Tasks

- [ ] 1. Plan: this change, and the CTC-021 row added with `qctl add`. Proof:
  `openspec validate toml-input` and `mise run q check` pass.
- [ ] 2. Add the `toml` dependency and the `input-toml` feature, list the
  feature in `check:features` and `.copier-answers.yml`. Proof:
  `cargo check --locked --no-default-features --features input-toml`
  passes on a build runner, and `cargo tree` for `input` alone lists no
  `toml` crate.
- [ ] 3. Split the garde path once (`field_steps`) and make the YAML locator
  use it. Proof: the existing `tests/input.rs` passes unchanged.
- [ ] 4. Add `Format`, `Input::format`, `Input::open`, and the TOML parse,
  placement, and limit paths. Proof: `tests/input_toml.rs` passes.
- [ ] 5. Document the feature and the format rule in AGENTS.md and the
  crate docs, and add the changeset. Proof: `mise run verify` passes on a
  build runner, including `test:doc` and `check:features`.
- [ ] 6. Open the PR, wait for CI, and answer each review thread with the
  commit that fixes it. Proof: CI green on the last push, no open thread.

## Evidence

Recorded as each task completes.
