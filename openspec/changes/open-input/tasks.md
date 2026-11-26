# Tasks

## 1. The fork's located reader

- [x] 1.1 In [vsh-serde-saphyr][fork], add `located::read` as a forkctl patch.
  It returns nodes with span, value, style and tag, with each alias expanded, under the checks `from_str_with_options` applies.
  Verify: the fork's CI passes, with tests that compare both readers on refused texts.
  2026-11-26 UTC: `located::read` shipped as a forkctl patch in [vsh-serde-saphyr 0.0.5][fork-0.0.5], after the fork's CI passed on the patch stack.
- [x] 1.2 Make the scalar rules public as a `scalars` module, which the fork's own Deserializer calls.
  Verify: tests for every row of [research.md](research.md), section 2, pass in the fork's CI.
  2026-11-26 UTC: the `scalars` module shipped in 0.0.5, and `Rules::string` in [0.0.6][fork-0.0.6]; the fork's Deserializer calls the same text rule.
- [x] 1.3 Release the fork version that carries both patches.
  Verify: the version is published, and its changelog names both patches.
  2026-11-26 UTC: [0.0.5][fork-0.0.5] and [0.0.6][fork-0.0.6] are published through the fork's publish workflow.

## 2. The ctl-input crate

- [x] 2.1 Create `ctl-input` in the workspace from `ctl-core/src/input/`, with features `yaml`, `toml` and `validate`, and one integration binary `tests/it`.
  Verify: `check:features` builds each feature alone on the build host, and `cargo tree` for `yaml` lists no yamled.
  2026-11-26 UTC: [#329][pr329] moved declared input into `ctl-input`; its CI passed, `check:features` included.
- [x] 2.2 Add the located tree, the `Format` trait, `Formats` with its tiers, and the YAML and TOML formats.
  Verify: tests for each scenario of the `input-formats` delta, including a test format `kv` written in the test.
  A test places every path of the existing input fixtures through the tree and through today's placing.
  Both give the same line and column.
  2026-11-26 UTC: [#333][pr333] added the tree, the trait, the registry and both formats, with a test that placed every fixture path through both placings; CI passed.
- [x] 2.3 Add the Deserializer, placing, `top_level`, `check`, `Tagged` and `Messages` over the tree, and remove the per-format halves.
  Verify: every existing input test passes with only the wording the message set changes, each changed line listed in the pull request.
  2026-11-26 UTC: [#334][pr334] added the Deserializer, `Messages` and `Tagged`, and [#336][pr336] moved `Input` onto them and removed the per-format halves; [#336][pr336] lists each changed expectation, and CI passed on both.
- [x] 2.4 Place a problem in a replayed node at its alias, and a missing field on the key that holds its mapping.
  Verify: tests for the alias and missing-field scenarios pass.
  2026-11-26 UTC: [#334][pr334] places both, with tests for the alias and missing-field scenarios; CI passed.
- [x] 2.5 Add the fork's three replay limits to `Limits`, and pass them to the fork's `Options::alias_limits`.
  Report them in `Loosened`, and name them in `Exceeded` from the fork's three alias-limit error variants.
  Verify: a test raises `max_total_replayed_events` in `.ctl/input.yaml` and sees the warning, and the nested-alias scenario names the field.
  2026-11-26 UTC: [#333][pr333] added the three replay limits through every tier, with the warning and the field name tested; CI passed.

## 3. ctl-core

- [x] 3.1 Turn `input-yaml`, `input-toml` and `validate` on through `ctl-input`, re-export it as `ctl_core::input`, and add `App::input_formats`.
  Verify: tests for the `declared-input` delta pass, and every public path under `ctl_core::input` other than `Format` resolves as before.
  2026-11-26 UTC: [#329][pr329] turned the features on through `ctl-input`, and [#336][pr336] added `App::input_formats`; CI passed.
- [x] 3.2 Measure every tier at the base and at the head in one box, and move each budget by the measured difference.
  Verify: `check:size` passes in CI, and the pull request lists both measurements per tier.
  2026-11-26 UTC: [#336][pr336] measured every tier at its parent and at its head on the build host and moved the `input` and `input-toml` budgets; the other tiers did not move. `check:size` passed in CI.

## 4. Docs and release

- [x] 4.1 Update `ctl-core/AGENTS.md`, the README, `docs/`, the crate-root docs of both crates, and the rustdoc of every changed item.
  Verify: `mise run lint:prose` and `mise run lint:md` pass, and the doc tasks pass in CI.
  2026-11-26 UTC: [#336][pr336] updated the docs; [#337][pr337] split `ctl-input/src/lib.rs` by concern. The doc and prose tasks passed in CI.
- [x] 4.2 Write a `ctl-core: minor` changeset that lists each break: `input::Format` as a trait, `Formats::of`, `Input::new_with_format`, and the new wording.
  Verify: `openspec validate open-input --strict` passes, and the Version PR lists the entry.
  2026-11-26 UTC: the fragments are `ctl-core: patch!`, not `minor`: inside the 0.8.0 candidate train only `patch!` writes the Breaking heading the Version PR's semver check reads ([#335][pr335]). [Version PR #332][pr332] lists the entries.

[fork]: https://github.com/victor-software-house/serde-saphyr
[fork-0.0.5]: https://github.com/victor-software-house/serde-saphyr/releases/tag/v0.0.5
[fork-0.0.6]: https://github.com/victor-software-house/serde-saphyr/releases/tag/v0.0.6
[pr329]: https://github.com/victor-software-house/ctl-core/pull/329
[pr332]: https://github.com/victor-software-house/ctl-core/pull/332
[pr333]: https://github.com/victor-software-house/ctl-core/pull/333
[pr334]: https://github.com/victor-software-house/ctl-core/pull/334
[pr335]: https://github.com/victor-software-house/ctl-core/pull/335
[pr336]: https://github.com/victor-software-house/ctl-core/pull/336
[pr337]: https://github.com/victor-software-house/ctl-core/pull/337
