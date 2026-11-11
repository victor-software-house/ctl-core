# Tasks

Compile, format, lint, and test steps run in an ephemeral build box, given a git clone of the branch.
The usual build host did not answer on 2026-11-11, so the box was a cloud one (4 vCPU, 8 GB).

- [x] 1. Plan: this change, validated, with the ledger row [CTC-053][ledger] pointing at it.
  Proof: `openspec validate field-path-place --strict` and `mise run q check` pass.
  - 2026-11-11 (UTC), both commands pass on the branch.
- [x] 2. Add `FieldPath`, `Step` and `Place`, `From<&garde::Path>` with the exact garde pin and the comment at `__iter`, and delete `field_steps`.
  Proof: `check:features` compiles `input-yaml` alone, `input-toml` alone, and `input` with warnings denied.
  - 2026-11-11 (UTC), `check:features` and `lint` (clippy with warnings denied on every target) passed in the box.
- [x] 3. Add `Input::place` and `Input::problem_at`, keep the YAML document in the input, and route `check` through `place`.
  Proof: the tests for YAML and TOML pass: a key holding dots, stars and slashes, a later `place` after `check`, and a second `place` that reuses the document.
  - 2026-11-11 (UTC), `test` and `test:doc` passed in the box.
    The tests are `a_problem_under_a_key_that_holds_dots_stars_and_slashes_lands_on_that_key` and `a_later_check_is_placed_through_place` in `tests/input.rs` and `tests/input_toml.rs`, `a_path_is_placed_without_yaml_and_without_garde` in `tests/input_toml_alone.rs`, and `a_second_placing_reuses_the_document` in `yaml_text.rs`.
- [x] 4. Move the docs: rustdoc of the new items, the `input` module docs, `ctl-core/AGENTS.md`, the crate README where it names placing, and the spec.
  Proof: `check:docs`, `doc`, and `lint:prose` pass, and a grep for `field_steps` and `nearest ancestor` in the repository finds only the archive.
  - 2026-11-11 (UTC), `doc`, `doc:default-features`, `doc:no-default-features`, `check:docs`, `check:repo` and `lint:prose` passed in the box.
    The first run of the two doc tasks failed on unresolved links in the crate docs, which the link definitions for a build without `input` fixed.
- [x] 5. Measure the `input` and `input-toml` tiers and move their budgets.
  Proof: `check:size` passes.
  - 2026-11-11 (UTC), the `input` tier measured 1,043,133 bytes against 1,041,402, and `input-toml` 594,206 against 592,638, with the pinned rustc on linux-x64.
    Both budgets in the root `Cargo.toml` moved to those numbers, and `check:size` passed.
    The `app` tier measured 471,990 against its budget of 471,974, inside the tolerance, and was left alone.
- [x] 6. Add the changeset.
  Proof: `mise run semver` reports no break.
  - 2026-11-11 (UTC), `semver` passed in the box against the last release with the patch changeset.
- [x] 7. Keep placing within the input's limits: a text past `max_reader_input_bytes` is not placed, and a YAML text is read under the effective budget before yamled gets it.
  Proof: the tests for an oversize YAML text, a YAML text past `max_depth` or `max_nodes`, and an oversize TOML text pass, and none of them builds a document.
  A refusal and a placed document follow the limits in force: two clone tests in `yaml_text.rs` and a step of the limits sequence test cover a change of the call, repository and process tiers.
  - 2026-11-11 (UTC), against the source of the previous commit in a box, the limits sequence test fails at the step that tightens the process default after a placing, and all of them pass with the change.
    `format`, `lint`, `check:features`, `test`, `test:input-toml`, `test:doc`, `doc`, `check:size`, `specs`, `lint:prose`, `check:repo` and `semver` pass.
  - 2026-11-11 (UTC), `test`, `test:input-toml`, `lint`, `check:features`, `doc` and `check:size` passed in the box.
    The tests are `a_text_over_the_size_limit_builds_no_document` and `a_text_over_the_depth_or_node_limit_builds_no_document` in `yaml_text.rs`, and `a_text_over_the_size_limit_is_not_placed` in `tests/input_toml.rs`.

[ledger]: ../../../tasks.yaml
