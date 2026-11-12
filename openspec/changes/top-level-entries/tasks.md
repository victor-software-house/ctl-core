# Tasks

Compile, format, lint and test steps run in an ephemeral build box, given a git clone of the branch.

- [ ] 1. Plan: this change, validated, with the ledger row [CTC-055][ledger] pointing at it.
  Proof: `openspec validate top-level-entries --strict` and `mise run q check` pass.
- [ ] 2. Add `Entry`, `Written` and `Input::top_level`, with the YAML half over serde-saphyr and yamled and the TOML half over the `toml` tree.
  Proof: the tests for YAML and TOML in `tests/input.rs`, `tests/input_toml.rs` and `tests/input_toml_alone.rs` pass, and none lists a key that is no string.
- [ ] 3. Share the yamled document between listing and placing.
  Proof: `listing_the_top_level_builds_the_document_that_placing_reuses` in `yaml_text.rs` passes.
- [ ] 4. Move yamled to 0.0.13 and the `input` and `input-toml` budgets.
  Proof: `check:size` passes with the budgets at the measured sizes.
- [ ] 5. Move the docs and add the changeset.
  Proof: `check:docs`, `doc` and `lint:prose` pass, and `semver` reports no break.
- [ ] 6. Verify the pull request's CI passes.

[ledger]: ../../../tasks.yaml
