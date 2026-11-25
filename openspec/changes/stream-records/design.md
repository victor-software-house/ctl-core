# Design

## Context

[proposal.md](proposal.md) gives the reasons.
[open-output][open-output] makes a format a `&'static dyn Format` that writes one result through `Output` and `Sink`.
This change adds a second delivery to that interface.

## Goals / Non-Goals

**Goals:**

1. Records reach stdout in the order the command produces them, in any format.
2. A CLI's own format streams on the same terms as a built-in.

**Non-Goals:**

1. Streamed `pretty` output with columns aligned across records: a stream writes a row before later rows exist.
2. A socket or network sink (operator, 2026-11-25).

## Decisions

### 1. Records and the stream form

A command returns either a value, as today, or `view::Records`: a header record, then an iterator of records, each `Result`.
Each record is `Present` and serializes with a string field `type`; a consumer writes `#[serde(tag = "type")]` on an enum of record types.

Every format streams, and a stream writes records in the order they are produced (operator, 2026-11-25).
`stream(sink)` returns the format's `format::RecordWriter`.
ctl-core hands it each record as an `Output` when the iterator yields it, then calls `finish`.
The default writer calls `write` for each record and writes nothing between records, so a CLI's own format streams with no extra code, and overrides `stream` only to write records differently.

| Result | Without `--stream` | With `--stream` |
|:--|:--|:--|
| A value | the format's whole output | a stream of one record, as the value serializes; ctl-core adds no `type` field (operator, 2026-11-25) |
| `Records` | collected, then written whole by `write` | each record written by the format's `RecordWriter` when the iterator yields it |

| Format | Stream form |
|:--|:--|
| `json` | each record compact on its own line |
| `tsv`, `csv` | a table's header line once, then one line per row; the header again when the columns change |
| `markdown` | a table's header and separator once, then rows; other blocks as they arrive |
| `pretty` | its own writer: each record's document rendered when it arrives, with a blank line between records |

1. Collected records reach `write` as one `Output`.
   Its `data()` serializes the records as one sequence, header first, and its `document()` is the header's document, then each record's.
   So `json` writes one array that one parser reads like the stream, and adjacent one-table documents with equal headers merge into one table.
2. `json` splits records on LF only.
3. `--stream` with no format named uses the registry's stream default, `json`, which `Formats::stream_default(name)` changes (operator, 2026-11-25).
4. When the iterator yields an error, the records before it are written, streamed or collected.
   The error goes to stderr in the format's error form, and the run exits with the error's code.
   So a collected `json` array is closed and stdout still parses.
5. When the reader closes stdout, the run stops pulling records, drops the iterator, and exits as the `BrokenPipe` policy says.

The alternative was a stream form only in some formats, declared by a flag such as `streams()`.
It lost because the maintainer ruled that every format streams (operator, 2026-11-25).

There is no `jsonl` format: JSON Lines is the framing of a stream of typed records, not a second rendering of one value (operator, 2026-11-25).

### 2. The flag

`OutputArgs`, `FormatArgs`, and `FormatLong` gain `stream: bool`, read from `<PREFIX>_STREAM` like every other flag.
The format in force under `--stream` is the one named on the command line, then by a variable, then the stream default.
`--quiet` treats the stream default as a default format, so it hides it, and never hides a machine format named for the stream.

| Behaviour | Library override | Disable |
|:--|:--|:--|
| `--stream` alone picks `json` | `Formats::stream_default(name)` | name a format |
| A format's stream form | override `Format::stream` | keep the default writer, which calls `write` |

## Risks / Trade-offs

1. [A record of `Records` without a `type` field breaks the stream contract] → A debug assertion and a test helper check the field; a single value is exempt.
2. [A stream of tables in `tsv` repeats a header when the columns change] → A reader splits on header lines, and the rustdoc says when one repeats.

## Migration Plan

Nothing breaks: a command that returns a value writes as before.
A CLI that writes its own rows to stdout returns `Records` instead, and drops that code.

[open-output]: ../open-output/design.md
