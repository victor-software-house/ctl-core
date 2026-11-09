# Tasks

- [x] 1. Plan: this change, its design decisions approved, and the
  [ledger row for TOML input without YAML][ledger] pointed at it with
  `qctl edit --plan`.
  Proof: `openspec validate split-input-formats --strict` and
  `mise run q check` pass.
  - 2026-11-09 UTC: the lead approved the six decisions. Both commands pass
    on the change's branch, and the row's `plan` names the proposal.
- [x] 2. Split the features: `input-yaml`, `input-toml` without `input`, and
  `input` as `input-yaml` plus `validate`; gate the module, `Format`, the
  constructors, `frontmatter`, `check`, `load`, and the field placing.
  Proof: `check:features` compiles `input-yaml` alone and `input-toml` alone
  with warnings denied, in CI.
  - 2026-11-09 UTC: `check:features` passed in [CI run 42622544979][run] on
    [0376a89][head] of [PR 253][pr253]. Clippy with `input-toml` alone then
    found three unfulfilled `expect` attributes, which the next PR narrows to
    the `schema` feature.
- [x] 3. Own `Limits`. The three shared fields exist in every build, and
  the 17 YAML-only fields with `input-yaml`. Add the three default
  constants, make `budget` private, and pass resolved limits to the TOML
  path.
  Proof: the limit tests pass in CI. They include the TOML-only tier and
  default scenarios, and the test that the constants equal serde-saphyr's
  defaults.
  - 2026-11-09 UTC: `the_shared_defaults_match_serde_saphyr` and the 7
    TOML-only tests passed in [CI run 42622544979][run].
- [x] 4. Gate `REPO_FILE`, `use_repo_limits`, and `Loosened` on
  `input-yaml`, and add the TOML-only limit message.
  Proof: the scenario "A limit in a TOML-only build" passes in CI.
  - 2026-11-09 UTC: the byte, depth, and node limit tests in
    `tests/input_toml_alone.rs` passed in [CI run 42622544979][run].
- [x] 5. Add `Input::new_with_format`, and refuse a `.yaml` file in a
  TOML-only build by naming `input-yaml`.
  Proof: the scenarios "TOML text in memory without YAML" and "A YAML file
  in a TOML-only build" pass in CI.
  - 2026-11-09 UTC: `text_in_memory_parses_without_yaml` and
    `a_yaml_file_names_the_feature_it_needs` passed in
    [CI run 42622544979][run].
- [x] 6. Extend `check:tree` to refuse `serde-saphyr`, `yamled`, and `garde`
  under `input-toml` alone.
  Proof: `check:tree` passes in CI on the pushed commit.
  - 2026-11-09 UTC: `check:tree` passed in [CI run 42622544979][run].
- [x] 7. Add the `input-toml` size tier at `app,input-toml`, and move the
  `input` tier if CI measures a change.
  Proof: `check:size` passes in CI with the `.text` it measured.
  - 2026-11-09 UTC: CI measured `input` at 1,041,402 and `input-toml` at
    592,638 bytes, the budgets the commit set, in [CI run 42622544979][run].
- [x] 8. Update the crate docs, `AGENTS.md`, and the feature docs in
  `Cargo.toml`, and add a `ctl-core: patch` changeset that names the break.
  Proof: `mise run verify` passes in CI on the pushed commit.
  - 2026-11-09 UTC: Verify passed in [CI run 42622544979][run].
- [x] 9. Run `cargo semver-checks` against the last release and put its
  output in the PR body.
  Proof: CI's semver step reports the change, and the PR body holds the
  output.
  - 2026-11-09 UTC: CI's `ver semver` skipped ctl-core, because a minor
    fragment already makes 0.7.0 a minor. `cargo semver-checks` 0.51.0 ran
    against 0.6.28 on a linux-x64 host instead. [PR 253][pr253] holds its
    output for the default features, the consumer set, and `input-toml` alone.
- [x] 10. Open the PR ready on the stack top, wait for CI and Kody, and
  answer each thread.
  Proof: CI green on the last push.
  - 2026-11-09 UTC: [PR 253][pr253] opened ready on stack 251. CI was green on
    [0376a89][head], and Kody's review reported no open findings.

[ledger]: ../../../../tasks.yaml
[run]: https://github.com/victor-software-house/ctl-core/actions/runs/42622544979
[head]: https://github.com/victor-software-house/ctl-core/commit/0376a89c0721b11045cd25f5dfc5091adf6fb0a2
[pr253]: https://github.com/victor-software-house/ctl-core/pull/253
