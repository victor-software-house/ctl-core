# Tasks

## 1. The format interface and registry

- [x] 1.1 Add `format::Format`, `Output`, `Sink`, `FormatName`, and `Formats` with `builtin`, `add`, `remove`, `terminal_default`, `piped_default`, and `augment`; add `erased-serde` to the `view` feature; remove `OutputFormat`.
  Verify: unit tests add, replace, and remove a format and resolve each default; the feature-set checks pass in CI.
  Evidence, 2026-11-25 (UTC): [PR 316][ctl-core#316] merged as `f192812` with `verify / Build and test`, `verify / Static checks` and `verify / Verify` green in [its CI run][run-316].
  `tests/formats.rs` adds, replaces and removes a format and resolves each default; the feature sets build in `verify / Static checks`.
- [x] 1.2 Write `pretty` and `json` as `Format` implementations, with today's output byte for byte.
  Verify: `tests/view.rs` and `tests/json_failures.rs` pass unchanged apart from the type names.
  Evidence, 2026-11-25 (UTC): [PR 316][ctl-core#316] merged as `f192812` with `verify / Build and test`, `verify / Static checks` and `verify / Verify` green in [its CI run][run-316].
  `tests/view.rs` passes unchanged; `tests/json_failures.rs` changed type names only, and later passed `--format pretty` in one test once [PR 318][ctl-core#318] made `tsv` the piped default.
- [x] 1.3 Document `Format`, `Output`, `Sink`, and `Formats` in their rustdoc and in the output section of `docs/presentation.md`.
  Verify: `mise run lint:prose` and `mise run lint:md` pass locally, and the doc tasks pass in CI.
  Evidence, 2026-11-25 (UTC): [PR 316][ctl-core#316] merged as `f192812` with `verify / Build and test`, `verify / Static checks` and `verify / Verify` green in [its CI run][run-316].
  `mise run lint:prose` and `mise run lint:md` pass in this repository once this change splits three long sentences elsewhere.

## 2. Flags and the run

- [x] 2.1 Make `OutputArgs`, `FormatArgs`, and `FormatLong` hold `format: Option<FormatName>`; add built-in shorthands in `augment_args`; reject two different formats as a usage error.
  Verify: `tests/cli/flags_cli.rs` and `tests/env_flags.rs` cover `--json`, `TOY_FORMAT`, `TOY_JSON`, and the conflict.
  Evidence, 2026-11-25 (UTC): [PR 316][ctl-core#316] merged as `f192812` with `verify / Build and test`, `verify / Static checks` and `verify / Verify` green in [its CI run][run-316].
  `tests/env_flags.rs` covers `TOY_FORMAT`, `TOY_JSON` and the conflict between them, and `tests/cli/flags_cli.rs` covers `--json`.
- [x] 2.2 Add `Formats::augment`, which binds each flag it adds under the prefix of `--format`'s variable.
  Make help, completion, the usage spec, and the pre-parse view take the command as a value, built once by `App` from `formats.augment(C::command())`.
  Verify: tests find a custom format and its shorthand in help, completion, the usage spec, a `Surface`, a parse error, and `TOY_<LONG>`.
  Evidence, 2026-11-25 (UTC): [PR 316][ctl-core#316] merged as `f192812` with `verify / Build and test`, `verify / Static checks` and `verify / Verify` green in [its CI run][run-316].
  `tests/formats.rs` finds a custom format and its shorthand in help, completion, the usage spec, a `Surface`, a parse error, and `TOY_<LONG>`, including a shorthand a subcommand already declares.
- [x] 2.3 Resolve the default from the terminal check, and replace every `is_json()` case with `machine()`; let `--quiet` hide only `pretty` and a default machine format.
  Verify: a test runs the toy binary piped and reads `tsv`; the colour test with `CLICOLOR_FORCE=1` finds no ESC byte in any machine format; the quiet scenarios of the `output-formats` spec pass.
  Evidence, 2026-11-25 (UTC): [PR 318][ctl-core#318] merged as `aa2fa39` with every check green in [its CI run][run-318].
  `a_pipe_reads_tsv_when_no_format_is_named` reads `tsv` from the piped toy.
  The colour test runs every machine format with `CLICOLOR_FORCE=1` and finds no ESC byte.
  `quiet_hides_pretty_and_a_default_but_not_a_named_machine_format` covers the quiet scenarios.
- [x] 2.4 Update `ctl-core/AGENTS.md`, the root `AGENTS.md`, the crate docs, and the prelude for the format flags, the defaults, and the quiet rule.
  Verify: `mise run lint:prose` and `mise run lint:md` pass locally, and the doc tasks pass in CI.
  Evidence, 2026-11-25 (UTC): [PR 316][ctl-core#316] merged as `f192812` with `verify / Build and test`, `verify / Static checks` and `verify / Verify` green in [its CI run][run-316].
  [PR 318][ctl-core#318] added the piped default to `ctl-core/AGENTS.md`, the crate docs and `docs/presentation.md`; `lint:prose` and `lint:md` pass as under 1.3.

## 3. Tabular built-ins

- [x] 3.1 Write `tsv`, `csv`, and `markdown` from the `Document`, with the block rules and escaping of design decision 3.
  Verify: snapshot tests for a table, fields, a sectioned report, and values with a tab, a quote, a comma, a pipe, and a line break.
  Evidence, 2026-11-25 (UTC): [PR 318][ctl-core#318] merged as `aa2fa39` with every check green in [its CI run][run-318].
  `tests/tabular.rs` snapshots each format on a table, fields, a sectioned report, and values with a tab, a quote, a comma, a pipe, a backslash, a line break, terminal control characters and bidi controls.
- [x] 3.2 List in the rustdoc of `tsv`, `csv`, and `markdown` the blocks each one writes and drops.
  Verify: the doc tasks pass in CI.
  Evidence, 2026-11-25 (UTC): the rustdoc of `tsv`, `csv` and `markdown` lists the blocks each writes and leaves out, and the doc tasks passed in [the CI run of PR 318][run-318].

## 4. Release

- [ ] 4.1 Write a `ctl-core: minor` changeset that lists each break and the migration of design.md.
  Verify: `openspec validate open-output --strict` passes and the Version PR lists the entry.
  Not done, 2026-11-25 (UTC): the changesets of [PR 316][ctl-core#316] and [PR 318][ctl-core#318] are on `main`, and `openspec validate open-output --strict` passes.
  No Version PR lists them until the maintainer releases the candidate.
- [x] 4.2 Record the release binary size of the toy example before and after.
  Verify: the numbers are under this task with the CI run linked.
  Evidence, 2026-11-25 (UTC): `.text` bytes of the size example, from fresh builds in a Linux box, each pull request against its parent.
  [PR 316][ctl-core#316]: `cli` -112, `help` -928, `view` +25,491, `app` +29,955, `usage` +29,891, `input` +29,472, `input-toml` +29,456.
  [PR 318][ctl-core#318]: `cli` +112, `help` +128, `view` +9,056, `app` +9,056, `usage` +9,040, `input` +9,056, `input-toml` +9,056.
  `check:size` passed against the new budgets in [the CI run of PR 316][run-316] and [the CI run of PR 318][run-318].

## Workflow follow-up

- After the maintainer releases the candidate, open one pull request each on qctl, verctl, forkctl, hakkiri, and rgbctl, following design.md's migration plan.
- In each of those pull requests, update the CLI's bundled skill and installed instructions where they call output pretty by default or name `OutputFormat`.
  Update the shared agent skills that document the format flags too.

[ctl-core#316]: https://github.com/victor-software-house/ctl-core/pull/316
[ctl-core#318]: https://github.com/victor-software-house/ctl-core/pull/318
[run-316]: https://github.com/victor-software-house/ctl-core/actions/runs/45679971892
[run-318]: https://github.com/victor-software-house/ctl-core/actions/runs/45691100555
