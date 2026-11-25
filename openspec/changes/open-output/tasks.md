# Tasks

## 1. The format interface and registry

- [ ] 1.1 Add `format::Format`, `Output`, `Sink`, `FormatName`, and `Formats` with `builtin`, `add`, `remove`, `terminal_default`, `piped_default`, and `augment`; add `erased-serde` to the `view` feature; remove `OutputFormat`.
  Verify: unit tests add, replace, and remove a format and resolve each default; the feature-set checks pass in CI.
- [ ] 1.2 Write `pretty` and `json` as `Format` implementations, with today's output byte for byte.
  Verify: `tests/view.rs` and `tests/json_failures.rs` pass unchanged apart from the type names.

## 2. Flags and the run

- [ ] 2.1 Make `OutputArgs`, `FormatArgs`, and `FormatLong` hold `format: Option<FormatName>` and `stream: bool`; add built-in shorthands in `augment_args`; reject two different formats as a usage error.
  Verify: `tests/cli/flags_cli.rs` and `tests/env_flags.rs` cover `--json`, `TOY_FORMAT`, `TOY_STREAM`, and the conflict.
- [ ] 2.2 Add `Formats::augment`, which binds each flag it adds under the prefix of `--format`'s variable.
  Make help, completion, the usage spec, and the pre-parse view take the command as a value, built once by `App` from `formats.augment(C::command())`.
  Verify: tests find a custom format and its shorthand in help, completion, the usage spec, a `Surface`, a parse error, and `TOY_<LONG>`.
- [ ] 2.3 Resolve the default from the terminal check, and replace every `is_json()` case with `machine()`.
  Verify: a test runs the toy binary piped and reads `tsv`; the colour test with `CLICOLOR_FORCE=1` finds no ESC byte in any machine format.

## 3. Tabular built-ins

- [ ] 3.1 Write `tsv`, `csv`, and `markdown` from the `Document`, with the block rules and escaping of design decision 3.
  Verify: snapshot tests for a table, fields, a sectioned report, and values with a tab, a quote, a comma, a pipe, and a line break.

## 4. Records and --stream

- [ ] 4.1 Add `view::Records` and let `App::run` take a value or records; write whole and streamed forms, the error mid-stream, and the closed pipe.
  Verify: integration tests for each scenario of the `output-formats` spec under --stream, including `| head -n 1` on an endless stream.
- [ ] 4.2 Refuse `--stream` before the handler runs when the format has no stream form.
  Verify: a test whose handler panics if called exits 2 under `--stream --format pretty`.

## 5. Docs and release

- [ ] 5.1 Update `docs/presentation.md`, `ctl-core/AGENTS.md`, the root `AGENTS.md`, the crate docs, and the prelude.
  Verify: `mise run lint:prose` and `mise run lint:md` pass locally, and the doc tasks pass in CI.
- [ ] 5.2 Write a `ctl-core: minor` changeset that lists each break and the migration of design.md.
  Verify: `openspec validate open-output --strict` passes and the Version PR lists the entry.
- [ ] 5.3 Record the release binary size of the toy example before and after.
  Verify: the numbers are under this task with the CI run linked.

## Workflow follow-up

- After the maintainer releases the candidate, open one pull request each on qctl, verctl, forkctl, hakkiri, and rgbctl, following design.md's migration plan.
- In each of those pull requests, update the CLI's bundled skill and installed instructions where they call output pretty by default or name `OutputFormat`.
  Update the shared agent skills that document the format flags too.
