---
ctl-core: minor
---

A command can return records, and `--stream` writes each one when the command produces it.
`view::Records` is a header record then records, each a `Present` value that serializes with a string `type` field.
`view::Reply` is a value or records.
`App::run` takes either through the sealed `view::Respond`, so a handler that returns a value compiles as before.
`View::show_records` writes records for a consumer without `App`.
`View::stream(bool)` sets the stream setting and `View::is_stream()` reads it.
Under it, `View::show` writes a value through the format's stream form, as a stream of one record.

Without `--stream`, records are collected and written as one result, so `json` writes one array.
With `--stream`, `json` writes each record compact on one line.
`--stream` alone uses the stream default, `json`, which `Formats::stream_default` changes.
`Format::stream` returns the `format::RecordWriter` that writes each record.
Its default writes each record as `write` does, so existing formats compile unchanged.
The `view` feature now depends on `anyhow`, the error type of a stream of records.

Breaking changes:

1. `OutputArgs`, `FormatArgs`, and `FormatLong` gain a `stream` field, and `--stream` with the variable `<PREFIX>_STREAM`, which takes `true` or `false`.
   A CLI that declares its own `--stream` must rename it.
2. `--stream` has no short flag, so a CLI that calls `Surface::require_shorts` adds `--stream` to its allow list.
3. `View`, with its `PartialEq` and `Debug`, carries the stream setting.

Migration: a CLI that writes its own rows to stdout returns `Records` instead and drops that code.
