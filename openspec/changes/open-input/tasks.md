# Tasks

## 1. The fork's located reader

- [ ] 1.1 In [vsh-serde-saphyr][fork], add `located::read` as a forkctl patch.
  It returns nodes with span, value, style and tag, with each alias expanded, under the checks `from_str_with_options` applies.
  Verify: the fork's CI passes, with tests that compare both readers on refused texts.
- [ ] 1.2 Make the scalar rules public as a `scalars` module, which the fork's own Deserializer calls.
  Verify: tests for every row of [research.md](research.md), section 2, pass in the fork's CI.
- [ ] 1.3 Release the fork version that carries both patches.
  Verify: the version is published, and its changelog names both patches.

## 2. The ctl-input crate

- [ ] 2.1 Create `ctl-input` in the workspace from `ctl-core/src/input/`, with features `yaml`, `toml` and `validate`, and one integration binary `tests/it`.
  Verify: `check:features` builds each feature alone on the build host, and `cargo tree` for `yaml` lists no yamled.
- [ ] 2.2 Add the located tree, the `Format` trait, `Formats` with its tiers, and the YAML and TOML formats.
  Verify: tests for each scenario of the `input-formats` delta, including a test format `kv` written in the test.
- [ ] 2.3 Add the Deserializer, placing, `top_level`, `check`, `Tagged` and `Messages` over the tree, and remove the per-format halves.
  Verify: every existing input test passes with only the wording the message set changes, each changed line listed in the pull request.
- [ ] 2.4 Place a problem in a replayed node at its alias, and a missing field on the key that holds its mapping.
  Verify: tests for the alias and missing-field scenarios pass.
- [ ] 2.5 Add the fork's three replay limits to `Limits`, pass them to the fork's options, report them in `Loosened`, and name them in `Exceeded`.
  Verify: a test raises `max_total_replayed_events` in `.ctl/input.yaml` and sees the warning, and the nested-alias scenario names the field.

## 3. ctl-core

- [ ] 3.1 Turn `input-yaml`, `input-toml` and `validate` on through `ctl-input`, re-export it as `ctl_core::input`, and add `App::input_formats`.
  Verify: tests for the `declared-input` delta pass, and every public path under `ctl_core::input` other than `Format` resolves as before.
- [ ] 3.2 Measure every tier at the base and at the head in one box, and move each budget by the measured difference.
  Verify: `check:size` passes in CI, and the pull request lists both measurements per tier.

## 4. Docs and release

- [ ] 4.1 Update `ctl-core/AGENTS.md`, the README, `docs/`, the crate-root docs of both crates, and the rustdoc of every changed item.
  Verify: `mise run lint:prose` and `mise run lint:md` pass, and the doc tasks pass in CI.
- [ ] 4.2 Write a `ctl-core: minor` changeset that lists each break: `input::Format` as a trait, `Formats::of`, `Input::new_with_format`, and the new wording.
  Verify: `openspec validate open-input --strict` passes, and the Version PR lists the entry.

[fork]: https://github.com/victor-software-house/serde-saphyr
