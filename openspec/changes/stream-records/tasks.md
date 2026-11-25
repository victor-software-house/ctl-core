# Tasks

## 1. Records and the flag

- [x] 1.1 Add `stream: bool` to `OutputArgs`, `FormatArgs`, and `FormatLong`, and `Formats::stream_default`.
  Verify: `tests/env_flags.rs` covers `TOY_STREAM`, and a test reads JSON lines from `toy events --stream` on a terminal.
  Evidence, 2026-11-25 (UTC): [PR 321][ctl-core#321] merged as `11e9a24` with every check green in [its CI run][run-321].
  `a_variable_sets_stream` in `tests/env_flags.rs` covers `TOY_STREAM`.
  `stream_alone_writes_json_lines_on_a_pipe` reads the JSON lines from a pipe, and `a_stream_on_a_terminal_defaults_to_json` in `tests/formats.rs` shows the stream default beating the terminal default.
- [x] 1.2 Add `view::Records`, `format::RecordWriter`, and `Format::stream` with the default writer; let `App::run` take a value or records.
  Verify: integration tests for each scenario of the `record-stream` spec, including a toy CLI's own format and `| head -n 1` on an endless stream.
  Evidence, 2026-11-25 (UTC): `tests/records.rs` has a test for each scenario of the `record-stream` spec, run as a child process, including a test format with no stream form of its own and `an_endless_stream_ends_quietly_when_the_reader_closes_the_pipe`.
  The lockstep tests fail at their deadline when records are collected before they are written.
- [x] 1.3 Update `docs/presentation.md`, `ctl-core/AGENTS.md`, and the rustdoc of `Records` and `RecordWriter`.
  Verify: `mise run lint:prose` and `mise run lint:md` pass locally, and the doc tasks pass in CI.
  Evidence, 2026-11-25 (UTC): `mise run lint:prose` (no error) and `mise run lint:md` pass on `main` at `79a248a`, and the doc tasks passed in [the CI run of PR 321][run-321].

## 2. Stream forms

- [x] 2.1 Write the stream form of `json`, `tsv`, `csv`, `markdown`, and `pretty` from design decision 1.
  Verify: snapshot tests of each built-in under `--stream`, with records of one table and of two tables.
  Evidence, 2026-11-25 (UTC): [PR 323][ctl-core#323] merged as `91d34f6` with every check green in [its CI run][run-323].
  `tests/records.rs` snapshots each built-in under `--stream` with records of one table, of two alternating tables, and of fields between two tables.
- [x] 2.2 Describe each built-in's stream form in its rustdoc.
  Verify: the doc tasks pass in CI.
  Evidence, 2026-11-25 (UTC): the rustdoc of `json`, `tsv`, `csv`, `markdown` and `pretty` states each stream form, and the doc tasks passed in [the CI run of PR 323][run-323].

## 3. Release

- [ ] 3.1 Write a `ctl-core: minor` changeset for records and `--stream`.
  Verify: `openspec validate stream-records --strict` passes and the Version PR lists the entry.
  Not done, 2026-11-25 (UTC): `.changeset/stream-records.md` is on `main`, and `openspec validate stream-records --strict` passes.
  No Version PR lists it until the maintainer releases the candidate.

[ctl-core#321]: https://github.com/victor-software-house/ctl-core/pull/321
[ctl-core#323]: https://github.com/victor-software-house/ctl-core/pull/323
[run-321]: https://github.com/victor-software-house/ctl-core/actions/runs/45703656518
[run-323]: https://github.com/victor-software-house/ctl-core/actions/runs/45710653756
