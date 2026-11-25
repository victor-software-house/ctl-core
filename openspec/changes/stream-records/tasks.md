# Tasks

## 1. Records and the flag

- [ ] 1.1 Add `stream: bool` to `OutputArgs`, `FormatArgs`, and `FormatLong`, and `Formats::stream_default`.
  Verify: `tests/env_flags.rs` covers `TOY_STREAM`, and a test reads JSON lines from `toy events --stream` on a terminal.
- [ ] 1.2 Add `view::Records`, `format::RecordWriter`, and `Format::stream` with the default writer; let `App::run` take a value or records.
  Verify: integration tests for each scenario of the `record-stream` spec, including a toy CLI's own format and `| head -n 1` on an endless stream.

## 2. Stream forms

- [ ] 2.1 Write the stream form of `json`, `tsv`, `csv`, and `markdown` from design decision 1; `pretty` keeps the default writer.
  Verify: snapshot tests of each built-in under `--stream`, with records of one table and of two tables.

## 3. Docs and release

- [ ] 3.1 Update `docs/presentation.md`, `ctl-core/AGENTS.md`, and the rustdoc of `Records` and `RecordWriter`.
  Verify: `mise run lint:prose` and `mise run lint:md` pass locally, and the doc tasks pass in CI.
- [ ] 3.2 Write a `ctl-core: minor` changeset for records and `--stream`.
  Verify: `openspec validate stream-records --strict` passes and the Version PR lists the entry.
