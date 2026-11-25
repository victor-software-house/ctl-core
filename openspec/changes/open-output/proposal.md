# Proposal

## Motivation

A ctl command returns one typed result, and ctl-core decides how it becomes output.
Today that decision is a closed set: `OutputFormat` has two variants, `Pretty` and `Json`, and a CLI cannot add a third without a ctl-core release.
The cost shows in a consumer that needs more:

1. One consumer's CLI needs CSV and one JSON document.
   It writes them to stdout itself and hands ctl-core an empty result.
   So its machine output skips ctl-core's rules: no ANSI, errors on stderr, a closed pipe ends quietly.
2. Piped output is the colourless pretty view: boxed tables that cost tokens and need a parser to split into fields.
   Agents read every ctl CLI through a pipe.
3. Nine places assume the set is closed.
   They are `OutputFormat`, `is_json()`, the `--json` reconciliation, the pre-parse view, the parse-error scan, `View`, `shell::view`, the prelude, and the public-api spec.

The maintainer ruled that the output layer is an open interface (operator, 2026-11-23).
ctl-core's own formats are built-in instances of it, which any CLI can add to or replace.

This change belongs to [CTC-062][ledger].

## Changes

1. **BREAKING** `format::OutputFormat` is removed.
   A public `format::Format` trait defines how a result becomes output, and `format::FormatName` names one.
2. ctl-core ships five built-in formats, each an implementation of `Format`:
   `pretty` (people), `json` (one document), `tsv` (one record per line, for pipes), `csv`, and `markdown`.
3. A CLI registers its formats on `format::Formats`: it adds a format, replaces a built-in of the same name, or removes one.
   `App::formats` installs the set, so `--format` help, completion, the usage spec, and parse errors list exactly the registered names.
4. **BREAKING** With no `--format`, output is `pretty` when stdout is a terminal and `tsv` when it is not (operator, 2026-11-24).
   A CLI changes either default on `Formats`.
5. A format may declare a shorthand flag.
   `json` declares `--json`, which stays `-f json`.
   Two different formats on one command line are a usage error, as `--json` beside `--format pretty` is today.
   **BREAKING** Two variables that name different formats, such as `TOY_FORMAT=csv` and `TOY_JSON=1`, are a usage error too, where `TOY_JSON` won before.
6. **BREAKING** Each `is_json()` special case becomes a property of the format: a machine format never writes ANSI and writes its own error form.
   `--quiet` hides `pretty` and a default machine format, and never a machine format named by a flag, a shorthand, or a variable (operator, 2026-11-25).
7. **BREAKING** `View` holds the selected format, not an `OutputFormat`.
   `View::format()` returns its `FormatName`.

## Capabilities

### New Capabilities

- `output-formats`: the format interface, its registry, the built-in formats, the format flags, and the piped default.

### Modified Capabilities

- `presentation`: the JSON layout requirement names the `json` format and every machine format.
- `public-api`: `OutputFormat` leaves the list of open enums, and `View::format()` returns a format name.
- `completion`: a shell script prints as text in `tsv`, the piped default, as in `pretty`.

## Impact

1. Code: `ctl-core/src/` `format.rs` and a new `format/` folder, `view.rs`, `flags/output.rs`, `parser.rs`, `app.rs`, `help.rs`, `usage.rs`, `completion.rs`, `shell.rs`, `prelude.rs`, and `lib.rs`.
2. Dependencies: [erased-serde][erased-serde] 0.4 (MIT OR Apache-2.0, built on `serde_core`) joins the `view` feature (operator, 2026-11-24).
   A format serializes through it a result it receives as a trait object.
3. Tests: the tests that name `OutputFormat`, and the tests that capture piped output, which turns from pretty to `tsv`.
4. Docs: `docs/presentation.md`, `ctl-core/AGENTS.md`, the root `AGENTS.md`, and the rustdoc of every changed type.
5. Consumers: [design.md](design.md) lists the migration step by step.
6. Release: a `ctl-core: minor` changeset, so it ships as a 0.8.0 candidate.

[ledger]: ../../../tasks.yaml
[erased-serde]: https://crates.io/crates/erased-serde
