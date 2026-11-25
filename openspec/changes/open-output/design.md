# Design

## Context

[proposal.md](proposal.md) gives the reasons.
Today a result reaches the terminal through four pieces:

1. `Present` in [view.rs][view-rs]: a result is `Serialize` and builds a `Document`.
2. `View` in the same file: it holds an `OutputFormat`, and `capture` branches on `is_json()`.
3. `OutputArgs`, `FormatArgs`, and `FormatLong` in [flags/output.rs][output-rs]: `-f/--format` is a Clap value enum, and `resolved_format` folds `--json` into it.
4. `App::run` in [app.rs][app-rs]: it reads the format from the raw arguments for help and parse errors, picks the `View` after the parse, and writes one value.

Three constraints shape the approach:

1. A CLI that owns `-f` or `-c` flattens a narrower struct, such as `FormatLong` or `ColorLong`, so the format flags must work as Clap-derived flags, with or without `App`.
2. The handler stays a black box: it returns typed data and never prints, inspects the terminal, or picks a format.
3. JSON never carries ANSI, whatever `CLICOLOR_FORCE` or `FORCE_COLOR` say ([AGENTS.md][agents] rule 6).

## Goals / Non-Goals

**Goals:**

1. A format is a value that implements a public trait, and ctl-core's own formats are such values.
2. One registry per CLI decides which formats exist, which is the default, and which flags select them.
3. Delivery is a second choice: a whole document at the end, or records as they are produced.

**Non-Goals:**

1. A socket or network sink (operator, 2026-11-25).
2. Projection and filtering on the JSON form, such as `--query`.
3. A stream form for a built-in other than `json`; any format can add one later without a break.
4. Validated input from flags, files and stdin ([CTC-043][ledger]), which follows as its own change (operator, 2026-11-24).
5. The table fit rules, which are [CTC-063][ledger].
6. Printing a command's handle bare when stdout is not a terminal, which is [CTC-061][ledger]; this change's pull request stacks on that one (operator, 2026-11-25).

## Decisions

### 1. A format is a `&'static dyn Format`

`format::Format` is an object-safe trait:

| Method | Default | Meaning |
|:--|:--|:--|
| `name()` | none | The `--format` value, lowercase, such as `tsv` |
| `about()` | none | One line for help and completion |
| `shorthand()` | `None` | A long flag that selects it, such as `json` for `--json` |
| `machine()` | `true` | Never writes ANSI, and `--quiet` does not hide it when named |
| `write(output, sink)` | none | Write one whole result |
| `stream(sink)` | `None` | A `RecordWriter` for `--stream`, or `None` when the format has no stream form |
| `write_error(error, sink)` | `bin: message` | Write a failure to stderr |

A consumer declares a format as a `static` and registers a reference to it, so `View` stays `Copy` and no format is allocated.

The alternative was a generic method `write<T: Present>`.
It lost because a generic method makes the trait not object-safe, so a registry could not hold formats chosen at run time.

### 2. A format reads the result through `Output`

`write` receives `format::Output`, which offers each face of the result once:

1. `data()`: the result as `&dyn erased_serde::Serialize`, which any serde serializer writes in the result's own field order.
2. `document()`: the `Document` that `present()` builds, made on first use.
3. `outcome()`: success or failure.

`Sink` carries the writer and the decisions made for its stream: colour, width, the `RenderOptions`, the JSON layout, and the binary name.
`sink.render(&document)` lays out a document as `pretty` does, so a format that only restyles text reuses the engine.

The alternative was a `serde_json::Value` built once per result.
It lost because it sorts object keys unless `preserve_order` pulls in `indexmap`, and it copies every result.
The maintainer chose `erased-serde` (operator, 2026-11-24).

### 3. Tabular formats read the view, not the data

`tsv`, `csv`, and `markdown` take their rows and columns from the `Document`, so the columns a person sees are the columns a script gets.
Serde data has no column order or labels for a nested value, and a view already chose them.

| Block | `tsv` | `csv` | `markdown` |
|:--|:--|:--|:--|
| `Table` | header line, then one line per row | header record, then rows | pipe table |
| `Fields` | one `label\tvalue` line per field | `field,value` header, then a record per field | two-column pipe table |
| `Paragraph`, `Verbatim` | the plain text | left out | the text |
| `Notice` | `level\tmessage` | left out | a quoted line, `> **warning**: message` |
| `Heading`, `Section` title | left out | left out | a heading, one level deeper per section |
| `Rule` | left out | left out | `---` |

1. Blocks are separated by one blank line in every tabular format.
2. Styling is dropped: a cell is the text of its spans.
3. `tsv` writes a tab, a line feed, a carriage return, and a backslash inside a value as `\t`, `\n`, `\r`, and `\\`, so a record stays on one line.
4. `csv` quotes a field that holds a comma, a quote, or a line break, as [RFC 4180][rfc4180] says, and ends records with CRLF.
5. `markdown` escapes `|` in a cell and writes a line break in a cell as `<br>`.

The alternative was rows taken from the serialized data, with nested keys flattened to dotted names.
It lost because the column order would follow struct fields rather than the view, and a result whose view hides a field would show it.

### 4. One registry per CLI: `Formats`

`Formats::builtin()` holds `pretty`, `json`, `tsv`, `csv`, and `markdown`, with `pretty` the terminal default, `tsv` the piped default, and `json` the stream default.

1. `add(format)` adds a format, or replaces the one with the same name.
2. `remove(name)` takes one out.
3. `terminal_default(name)`, `piped_default(name)`, and `stream_default(name)` set the default for each case.
   Setting both to `pretty` turns the piped default off.
4. `App::formats(formats)` installs the set for help, completion, the usage spec, parse errors, and the run.

Without `App`, `View` resolves a name against `Formats::builtin()`, and `Formats::augment(command)` applies a custom set to a Clap command.

### 5. The format flags take a name, and `App` fills the list

`OutputArgs`, `FormatArgs`, and `FormatLong` hold `format: Option<FormatName>` and `stream: bool`.

1. The derived `--format` accepts the built-in names, so a CLI that parses with Clap alone still rejects a typo.
2. `Formats::augment(command)` sets `--format`'s accepted names to the registry's, and adds one global flag per shorthand that the command lacks.
3. The flags' `augment_args` adds the built-in shorthands, `--json` among them, so a shorthand is not a field that each new format would need.
4. Every flag reads `<PREFIX>_<LONG>`, such as `QCTL_FORMAT` and `QCTL_STREAM`.
   Clap's derive applies `env_prefix` after it has added every argument ([env.rs][env-rs]), so the derived flags are bound before `augment` runs.
   `augment` binds each flag it adds under the prefix it reads from `--format`'s variable, and adds none when `--format` has no variable.

`App` builds one command, `formats.augment(C::command())`, and hands that command to help, completion, the usage spec, the pre-parse view, and the parse.
Today each of those calls `C::command()` itself; they take the command as a value instead.

A consumer whose test builds a `Surface` or its operator docs without `App` builds them from `formats.augment(Cli::command())`, from the same `formats()` function it hands `App`.
Then the committed docs list the CLI's own formats.

Each piece sits in the narrowest feature that needs it:

| Public item | Feature | Why |
|:--|:--|:--|
| `FormatName` and the built-in names | `cli` | The flag structs parse names without the renderer |
| The flag structs' `format` and `stream` | `cli` | As today |
| `Format`, `RecordWriter`, `Output`, `Sink`, `Formats`, and the built-ins | `view` | They render a `Document` and serialize data |
| `erased-serde` | `view` | Only `Output::data` uses it |

The format in force is, in order:

1. `--format` or a shorthand on the command line; naming two different formats is a usage error, exit 2.
2. `<PREFIX>_FORMAT` or a shorthand's variable.
3. The terminal default when stdout is a terminal, and the piped default otherwise.

A format variable and a shorthand variable that name different formats, such as `TOY_FORMAT=csv` and `TOY_JSON=1`, are a usage error that names both.
`env-flags` refuses other conflicting variables by name the same way today.

The alternative was a process-wide registry set before parsing.
It lost because a test that builds two CLIs in one process would share it.

### 6. The stream form is part of the format interface

A command returns either a value, as today, or `view::Records`: a header record, then an iterator of records, each `Result`.
Each record is `Present` and serializes with a string field `type`; a consumer writes `#[serde(tag = "type")]` on an enum of record types.

A format gives a stream form by returning a `format::RecordWriter` from `stream(sink)`.
ctl-core hands the writer each record as an `Output` when the iterator yields it, then calls `finish`.
ctl-core fixes no list of formats that stream: a CLI's own format streams on the same terms as a built-in (operator, 2026-11-25).

| Result | Without `--stream` | With `--stream` |
|:--|:--|:--|
| A value | the format's whole output | the format's whole output |
| `Records` | collected, then written whole by `write` | each record written by the format's `RecordWriter` when the iterator yields it |

1. Collected records reach `write` as one `Output`.
   Its `data()` serializes the records as one sequence, header first, and its `document()` is the header's document, then each record's.
   So `json` writes one array that one parser reads like the stream, and adjacent one-table documents with equal headers merge into one table.
2. `json` is the built-in with a stream form: each record is compact JSON on its own line, split on LF only.
3. `--stream` with no format named uses the registry's stream default, `json`, which `Formats::stream_default(name)` changes.
4. `--stream` with a format whose `stream` returns `None` is a usage error, exit 2, raised before the handler runs.
5. When the iterator yields an error, the records before it are written, streamed or collected.
   The error goes to stderr in the format's error form, and the run exits with the error's code.
   So a collected `json` array is closed and stdout still parses.
6. When the reader closes stdout, the run stops pulling records, drops the iterator, and exits as the `BrokenPipe` policy says.

The alternative was a fixed set of deliveries per built-in, declared by a flag such as `streams()`.
It lost because a CLI's format could then only stream in the ways ctl-core foresaw.

There is no `jsonl` format: JSON Lines is the framing of a stream of typed records, not a second rendering of one value (operator, 2026-11-25).

### 7. Machine formats replace each `is_json()` case

| Today | After |
|:--|:--|
| JSON never carries ANSI | A `machine()` format never carries ANSI |
| `--quiet` never hides JSON | `--quiet` never hides a `machine()` format named by a flag, a shorthand, or a variable; it hides `pretty` and a default format (operator, 2026-11-25) |
| A JSON failure is an envelope on stderr | Each format writes its own error form; `json` keeps the envelope |
| `--json` before a bad argument makes the parse error JSON | Any registered format or shorthand before it picks the error form |

### 8. `View` keeps the requested name and the resolved format

`OutputArgs::view()` records the requested name, which may be unset.
`App` resolves it against its registry and the terminal check before the handler runs.
A `View` used without `App` resolves against the built-ins.
`View::format()` returns the resolved `FormatName`, and `View::is_machine()` replaces `format().is_json()`.

## Risks / Trade-offs

1. [Piped output changes for every consumer] → The changeset names this first, and a consumer that needs the old output sets `piped_default("pretty")`.
2. [A tabular format drops text a person reads, such as a heading] → `markdown` keeps every block.
   The rustdoc of `tsv` and `csv` lists the blocks each one drops.
3. [`erased-serde` adds code to every binary] → It sits behind `view`, which already carries `serde_json`; the size check on the build host records the change.
4. [A record without a `type` field breaks the stream contract] → A debug assertion and a test helper check the field.

## Migration Plan

Each consumer pins the released candidate in its own pull request, one at a time.
The maintainer releases the candidate first (operator, 2026-11-24).

1. Replace `OutputFormat::Json` with `FormatName::JSON`, and `format.is_json()` with `view.is_machine()`.
2. Replace `View::new(OutputFormat::X, color)` with `View::new(&format::X, color)`.
3. Update tests that capture piped output: they now read `tsv`, or pass `--format pretty`.
4. Return typed values or `Records` where the CLI writes machine output itself, and drop that code.
5. Register any format of the CLI's own through `App::formats`.

| Consumer | Pins | What moves |
|:--|:--|:--|
| [qctl][qctl] | 0.6.17 | `cli.rs` and `presentation.rs`; `-f` stays `--file` through `FormatLong` |
| [verctl][verctl] | 0.6.9 | `cli.rs` and `presentation.rs` |
| [forkctl][forkctl] | 0.6.23 | `cli.rs`, `main.rs`, and `presentation.rs` |
| [hakkiri][hakkiri] | 0.7.5 | `cli.rs` and `report.rs` |
| [rgbctl][rgbctl] | 0.6.28 | `cli.rs` and `outcome.rs` |

After the candidate is released, one pull request per row above moves that CLI (operator, 2026-11-24).
Private consumers move in their own work, from these notes.

One private CLI writes its own JSON and CSV today, to keep the byte-exact output of an earlier tool.
It registers its own `json` and `csv` in place of the built-ins.
Those formats read the result through `Output::data()`, which hands any serde serializer every field.
So the CLI no longer writes to stdout itself.

Rollback: a consumer keeps its current pin.

[view-rs]: ../../../ctl-core/src/view.rs
[env-rs]: ../../../ctl-core/src/flags/env.rs
[output-rs]: ../../../ctl-core/src/flags/output.rs
[app-rs]: ../../../ctl-core/src/app.rs
[agents]: ../../../AGENTS.md
[ledger]: ../../../tasks.yaml
[rfc4180]: https://www.rfc-editor.org/rfc/rfc4180
[qctl]: https://github.com/victor-software-house/qctl
[verctl]: https://github.com/victor-software-house/verctl
[forkctl]: https://github.com/victor-software-house/forkctl
[hakkiri]: https://github.com/victor-software-house/hakkiri
[rgbctl]: https://github.com/victor-software-house/rgbctl
