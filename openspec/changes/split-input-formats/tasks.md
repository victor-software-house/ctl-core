# Tasks

- [x] 1. Plan: this change, its design decisions approved, and the
  [ledger row for TOML input without YAML][ledger] pointed at it with
  `qctl edit --plan`.
  Proof: `openspec validate split-input-formats --strict` and
  `mise run q check` pass.
  2026-11-09 UTC: the lead approved the six decisions. Both commands pass
  on the change's branch, and the row's `plan` names the proposal.
- [ ] 2. Split the features: `input-yaml`, `input-toml` without `input`, and
  `input` as `input-yaml` plus `validate`; gate the module, `Format`, the
  constructors, `frontmatter`, `check`, `load`, and the field placing.
  Proof: `check:features` compiles `input-yaml` alone and `input-toml` alone
  with warnings denied, in CI.
- [ ] 3. Own `Limits`. The three shared fields exist in every build, and
  the 17 YAML-only fields with `input-yaml`. Add the three default
  constants, make `budget` private, and pass resolved limits to the TOML
  path.
  Proof: the limit tests pass in CI. They include the TOML-only tier and
  default scenarios, and the test that the constants equal serde-saphyr's
  defaults.
- [ ] 4. Gate `REPO_FILE`, `use_repo_limits`, and `Loosened` on
  `input-yaml`, and add the TOML-only limit message.
  Proof: the scenario "A limit in a TOML-only build" passes in CI.
- [ ] 5. Add `Input::new_with_format`, and refuse a `.yaml` file in a
  TOML-only build by naming `input-yaml`.
  Proof: the scenarios "TOML text in memory without YAML" and "A YAML file
  in a TOML-only build" pass in CI.
- [ ] 6. Extend `check:tree` to refuse `serde-saphyr`, `yamled`, and `garde`
  under `input-toml` alone.
  Proof: `check:tree` passes in CI on the pushed commit.
- [ ] 7. Add the `input-toml` size tier at `app,input-toml`, and move the
  `input` tier if CI measures a change.
  Proof: `check:size` passes in CI with the `.text` it measured.
- [ ] 8. Update the crate docs, `AGENTS.md`, and the feature docs in
  `Cargo.toml`, and add a `ctl-core: patch` changeset that names the break.
  Proof: `mise run verify` passes in CI on the pushed commit.
- [ ] 9. Run `cargo semver-checks` against the last release and put its
  output in the PR body.
  Proof: CI's semver step reports the change, and the PR body holds the
  output.
- [ ] 10. Open the PR ready on the stack top, wait for CI and Kody, and
  answer each thread.
  Proof: CI green on the last push.

[ledger]: ../../../tasks.yaml
