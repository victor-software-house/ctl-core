# Stream records as they are produced

## Why

A ctl command returns one result, and ctl-core writes it once the handler returns.
A command that produces results over time, such as one that follows events or walks a large tree, shows nothing until the last one.

One consumer's CLI writes JSON rows to stdout itself as they arrive, so that output skips ctl-core's rules for ANSI, errors, and closed pipes.

The maintainer made format and delivery two choices: `--format` picks the encoding, and `--stream` picks whether records are written as they are produced (operator, 2026-11-25).
JSON Lines is the stream delivery of `json`, not a format of its own (operator, 2026-11-25).

This change belongs to [CTC-063][ledger] and builds on the format interface of [open-output][open-output].

## What Changes

1. A command returns a value, as today, or a `view::Records` stream: a header record, then records, each with a string `type` field.
2. `--stream` writes each record when it is produced, in that order, in every format (operator, 2026-11-25).
   A CLI's own format streams through the same interface, with no extra code.
3. `json` writes one compact object per line.
   `tsv`, `csv`, and `markdown` write a table's header once, then its rows, and `pretty` renders each record when it arrives.
4. `--stream` alone uses the stream default, `json`, which a CLI changes (operator, 2026-11-25).
5. A single value under `--stream` is a stream of one record (operator, 2026-11-25).
6. Without `--stream`, records are collected and written as one document, so `json` writes one array.
7. A failure during a stream leaves the written records on stdout and writes the error to stderr with its exit code.
   A reader that closes the pipe ends the stream quietly.
8. There is no `jsonl` format, no `--jsonl` flag, and no socket sink: a program that wants the data without the CLI uses an SDK or an API (operator, 2026-11-25).

## Capabilities

### New Capabilities

- `record-stream`: records, the `--stream` flag, each format's stream form, and the stream default.

### Modified Capabilities

- `presentation`: a stream writes `json` compact, whatever the layout.

## Impact

1. Code: `ctl-core/src/` `view.rs`, `format.rs` and the `format/` folder, `flags/output.rs`, and `app.rs`.
2. Tests: integration tests in the toy example for each scenario, with an endless stream for the closed pipe.
3. Docs: `docs/presentation.md`, `ctl-core/AGENTS.md`, and the rustdoc of `Records` and `RecordWriter`.
4. Release: a `ctl-core: minor` changeset, so it ships in the 0.8.0 candidates.

[ledger]: ../../../tasks.yaml
[open-output]: ../open-output/proposal.md
