# Unified ctl presentation

## Decision

Typed domain data is the source of every command presentation.
ctl-core owns the Rust CLI chassis and rendering framework once; consumer CLIs own domain models and domain-specific information hierarchy.

The public boundary is semantic:

```text
Clap graph ──→ App ──→ domain handler ──→ Result model
                                              │
                                   ┌──────────┴──────────┐
                                   │                     │
                             serde JSON             Present
                                                         │
                                                     Document
                                                         │
                                             pretty or colorless
```

## Ownership

ctl-core owns:

- Usage mounts, optional consumer enrichment through `App::usage_spec`, pre-parse short-circuits, help, Clap parsing, exit codes, and stream selection;
- `OutputArgs`, `FormatArgs`, `FormatLong`, `ColorLong`, `ColorMode`, `FormatName`, the `Format` interface with its `pretty`, `json`, `tsv`, `csv`, and `markdown` implementations, and quiet behavior;
- `Document`, `Text`, `Fields`, `Table`, `Section`, `Notice`, `Rule`, and unwrapped verbatim blocks that remove terminal controls, bidi controls, and Unicode line separators while preserving other Unicode formatting;
- semantic roles and the one visual theme;
- width detection, wrapping (including Usage lines), table layout, ANSI policy, stdout, and stderr, through the workspace's `ctl-text`, `ctl-layout`, and `ctl-term`;
- JSON emission and generic error envelopes.

A consumer owns:

- Clap-derived domain verbs and parameters;
- serializable request, result, notice, and domain error types;
- the adapter that maps one result to semantic document nodes;
- operator prose in its skill and installed instructions.

A domain handler never prints, constructs a terminal table, reads terminal width, or selects pretty versus JSON.

## Engine boundary

A document renders through three crates of this workspace:

- `ctl-text` holds styled spans, lines, and text, with widths in terminal cells, wrapping, and truncation;
- `ctl-layout` holds the widgets a document becomes, such as tables, records, and rules, and never sees an escape sequence;
- `ctl-term` decides colour, colour depth, and width for one stream, and holds the encoder, the one place an escape sequence is written.

`Theme` maps each role to an `anstyle::Style` when a document becomes widgets, so the layout crate never sees a role.
Consumers compose `Document` blocks; a CLI that needs a widget directly uses `ctl_core::{text, layout, term}`.

What ctl-core writes itself and what it takes from a crate:

1. Taken from crates: Unicode width and segmentation from `unicode-width` and `unicode-segmentation`, the window size from `terminal_size`, and the terminal check from `std::io::IsTerminal`.
2. Not written at all: ANSI stripping.
   Nothing is painted before the decision to paint, so there is nothing to strip.
3. Written in `ctl-layout`: table sizing.
   No crate sizes styled cells without a terminal crate and a second escape parser, and the closest one ignores minimum widths.
   The algorithm is specified and property-tested.
4. Written in `ctl-term`: the colour policy, as a pure function.
   No crate reads both `NO_COLOR` and `FORCE_COLOR` with an explicit override, per stream, without global state.

## Automatic width policy

An automatically detected TTY or `COLUMNS` width reserves two columns before wrapping, stacking, help, fields, and table layout.
This default protects hosts that indent tool output or whose frame consumes the rightmost column.
The configured floor is 20 when the detected width permits it.
A narrower terminal caps that floor, so ctl-core never claims columns the terminal does not have.

Sometimes neither a terminal nor `COLUMNS` gives a width.
Agent and CI captures, where every stream is piped, are examples.
Then ctl-core lays out to a fallback width of 80.
`fallback_width(Some(n))` changes it and `fallback_width(None)` renders tables at their natural width.
The fallback is separate from the minimum, which applies only to a detected width.

Wrapping breaks at whitespace.
A word wider than its column, such as a URL or a path, is not split in records, paragraphs, and headings.
It runs past the width.
That keeps it copyable whole.
Grid tables still split it, to keep their borders intact.

`CTL_CORE_COLUMN_BUFFER` replaces the default buffer.
Library consumers have full control through `RenderOptions`, `View`, and `App`.
`RenderOptions`, `View`, and `App` name a setter after its setting:

- `automatic_width_buffer(n)` overrides the environment, including zero to disable buffering;
- `automatic_width_buffer_envs(&[...])` replaces the ordered names, so a consumer can add aliases or disable lookup with an empty slice;
- `minimum_automatic_width(n)` replaces the default preferred floor of 20, still capped by the detected terminal width;
- `fallback_width(..)` sets or disables the undetected-width fallback.

These settings exist with `view` or `help`, which detect a width.
The `render` tier lays out to the explicit width or the fallback, so it has no buffer or minimum.

Option-aware help entry points accept the same `RenderOptions`.
An explicit `width(n)` is always exact and ignores every automatic-width setting.

## Default look

Records render borderless with keys right-aligned (`RecordStyle::KeysRight`).
Lists keep the full grid (`Frame::Grid`) with rows together (`RowSeparation::None`).
JSON is indented by two spaces (`JsonLayout::Pretty`).
Identifiers render bold in the terminal foreground (`Role::Id`).
Each is an option on `RenderOptions` (`record_style`, `frame`, `row_separation`), on `View` (`styles`, `json_layout`), and on `App` (`styles`, `json_layout`), so a consumer that needs the box or compact JSON sets it.

An operator picks any option the consumer left open through these variables:

- `CTL_CORE_RECORD_STYLE` (`keys-right`, `keys-left`, `boxed`)
- `CTL_CORE_LIST_STYLE` (`grid`, `header-rule`, `plain`)
- `CTL_CORE_ROW_SEPARATION` (`none`, `rule`, `blank`)
- `CTL_CORE_COLUMN_FLOOR` (a whole number of cells, `0` turns the floor off)
- `CTL_CORE_TABLE_FIT` (`stack`, `wrap`)
- `CTL_CORE_JSON_LAYOUT` (`pretty`, `compact`, `pretty-on-terminal`)

The consumer's setter wins over the variable.
`record_style_envs`, `frame_envs`, `row_separation_envs`, `column_floor_envs`, `table_fit_envs`, and `json_layout_envs` replace each ordered name list, and an empty slice disables lookup.
A value outside the list is ignored, and `App` prints one warning line that lists the accepted values.
For the floor, the line says a whole number of cells.

## Table columns and stacking

No column of a table narrows below its floor, whatever its overflow.
A column's floor is its header width or the floor setting, whichever is larger, and never more than the column's natural width.
The setting is 8 cells.
So a header never wraps mid-word, and a short column is never padded.
An ellipsis or fold column stops at 8 cells where it once went down to 1.

`RenderOptions::column_floor(cells)` sets the setting, and `View::styles` and `App::styles` carry it.
`CTL_CORE_COLUMN_FLOOR` sets it when the consumer leaves it open.
A setting of `0` gives every column a floor of 0, so each goes down to the limit its overflow sets.

A table whose columns at their floors, with its frame, do not fit the width stacks.
Each row draws as a record, with a blank line between rows.
The record has the first cell as a title, then one line of header and value for each other column, in the record style.
A table with no header draws each row as its first cell over the others, since it has no column names to show.
A table with no rows draws its grid, since there is nothing to stack.

`RenderOptions::table_fit(TableFit)` sets the fit, and `Table::stacking(TableFit)` sets it for one table.
`CTL_CORE_TABLE_FIT` sets it when neither does.
`TableFit::Wrap` (`wrap`) keeps the grid at its floors instead, and lets it run past the width, framed or not.
A table with `Table::stacked_below`, such as a help table, has no floor and keeps its own threshold and form.

## Output formats

A format is a value that implements `format::Format`, and `FormatName` names it.
`pretty`, `json`, `tsv`, `csv`, and `markdown` are built-in implementations, such as `format::PRETTY` and `format::TSV`, and a CLI registers its own on `format::Formats`.

1. `Format::write` receives an `Output` and a `Sink`.
   `Output::data` is the result as `&dyn erased_serde::Serialize`, `Output::document` is the document `present` builds, made on first use, and `Output::outcome` says whether the result succeeded.
   `Sink` is the writer for the stream the result goes to.
   Its `render` lays a document out as `pretty` does, and its colour is `never` when the format is a machine format.
2. `Format::write_error` writes a failure.
   The default is `{bin}: {message}`; `json` writes the error envelope.
3. `Formats::builtin()` holds the five built-ins.
   `add` registers a format or replaces the one of the same name, `remove` takes one out, and `terminal_default` and `piped_default` set the default for each case.
   The terminal default is `pretty` and the piped default is `tsv`.
   A CLI that keeps `pretty` when piped sets `Formats::builtin().piped_default(FormatName::PRETTY)`.
   `App::formats` installs the set.
4. `Formats::augment` gives a command the registered names for `--format`, and one global flag for each shorthand the command lacks.
   Help, completion, the Usage spec, a `Surface` built from the augmented command, and parse errors then list the same names.
   A shorthand flag reads `<PREFIX>_<LONG>`, with the prefix taken from the variable `--format` reads.
5. The format in force is the one named on the command line, then the one a variable names, then the default for the terminal check.
   Two flags, or two variables, that name different formats are a usage error that names both.
6. `View::new` takes a registered format, and `View::requested` takes a name that `View::resolve` turns into one.
   `View::format()` returns the name, and `View::is_machine()` says whether the format never writes ANSI.
7. `tsv`, `csv`, and `markdown` read `Output::document`, so the columns a script gets are the columns a person sees.
   `tsv` writes tables, fields, paragraphs, verbatim text, and notices; `csv` writes only tables and fields; `markdown` writes every block.
   A cell is the text of its spans with the styling dropped.
   `tsv` writes a tab, line feed, carriage return, and backslash in a cell as `\t`, `\n`, `\r`, and `\\`.
   `csv` follows RFC 4180 and ends each record in CRLF.
   `markdown` writes a backslash in a cell as `\\`, `|` as `\|`, `&` as `&amp;`, `<` as `&lt;`, and a line break as `<br>`.
   A control character, directional formatting character, and line or paragraph separator is dropped from every text, except a tab, line feed, and carriage return.
   Blocks that write something are separated by one blank line.

## Records and --stream

A command returns one value, as before, or `view::Records`: a header record, then records, each a `Result`.
Every record is `Present` and serializes as a map with a string `type` field, which `#[serde(tag = "type")]` on an enum of the record types gives.
A debug build panics on a record without one, the header included.
A single value is exempt.
A handler whose subcommands return a value and records returns `view::Reply`, and `view::Respond` is the sealed trait that lets `App::run` take either.
`View::show_records` writes records for a consumer without `App`, and returns the error the iterator ended with.

1. Without `--stream`, records are collected and written as one result.
   Its data is one sequence, header first, so `json` writes one array.
   Its document is the header's document followed by each record's, where adjacent documents that are each one table with equal headers and the same column count merge into one table.
2. With `--stream`, `Format::stream` returns the `RecordWriter` that writes each record when the iterator yields it.
   ctl-core takes what the writer wrote for one record, locks stdout, writes it whole, flushes, and unlocks.
   It holds no lock while the iterator or a serialization runs, and a record that fails to write leaves no partial line on stdout.
   The default writer calls `write` for each record and writes nothing between records, so a CLI's own format streams with no extra code.
   `json` writes each record compact on one line ending in LF, whatever the layout.
   `tsv`, `csv`, and `markdown` share one stream writer, and `pretty` has its own.
   The tabular writer writes each record's document as the whole output writes it, with the same blank line between blocks, and keeps the header of the table it last wrote.
   The first table of a record, when its header row equals that header and it has the same column count, goes on with only its rows and no blank line before them.
   Inside one record every table is written as the whole output writes it.
   A plain table goes on from the last plain table of the same column count.
   Any other table writes its header again after a blank line, and in `markdown` its separator row too.
   A block that writes something other than a table ends the table, so the same table after it writes its header again.
   A block the format leaves out writes nothing, adds no blank line, and does not end the table.
   So a stream of records that are each one row of one table with the same column count writes the same bytes as the collected output.
   `pretty` renders each record's document as the whole output renders it, with a blank line between records and none before the first or after the last.
   A record that renders to nothing writes nothing and adds no blank line.
   Its tables are not aligned across records, and colour and width are decided for each record, so a terminal resized mid-stream changes the width of later records.
3. A value under `--stream` is a stream of one record, as the value serializes, with no `type` field added.
   A value that failed keeps its own stream, stderr, and exit code.
4. Every record goes to stdout.
   A record's `outcome()` and `exit_code()` are ignored, and the run exits 0 unless the iterator yields an error.
5. `--stream` with no format named uses the stream default, `json`, in place of the terminal and piped defaults.
   `Formats::stream_default` changes it.
   A stream default that is not registered makes `--stream` with no format a usage error that lists the registered names.
6. An error from the iterator stops it.
   The records already pulled are written and the output is closed, so a `json` array is complete.
   Then the error goes to stderr in the format's error form, with the exit code and `kind` of the first `CommandError` in its chain, or 1.
   Any other error that ends the stream, such as a record that does not serialize, goes there the same way with exit 1.
7. A reader that closes stdout ends the stream: ctl-core stops pulling, drops the iterator, and exits as the `BrokenPipe` policy says.
   When the iterator had already failed, the closed stdout does not hide that failure: its error goes to stderr, with its exit code.
8. `--quiet` counts the stream default as a default format.
   With no format named, it drains the iterator, writes no record, and still writes an error and its code.
   A machine format named on the command line or by a variable is written.
9. A parse error and a handler error under `--stream` with no format named use the stream default's error form, whether `--stream` came from the flag or from `<PREFIX>_STREAM`.
   The variable takes `true` or `false`, as every shared flag's variable does.
   A `--stream` after the bad argument counts, as a format after it does.

There is no `jsonl` format and no `--jsonl` flag: JSON Lines is the stream form of `json`.

## Output law

One serializable model feeds all modes:

1. JSON serializes the model directly, writes one newline to stdout, and never contains ANSI.
   Every format that is a machine format has the same last property: its `Sink` takes the colour away.
2. Pretty asks `Present` for a semantic document and renders it with color policy `auto` or `always`.
3. Colorless renders the same document with color policy `never`; it does not maintain a second layout.
4. Errors use stderr, in each format's own error form, so stdout stays empty after a failure.
   That covers the error envelope, a parse error, and a result whose outcome is an error.
   A reader that parses stdout never mistakes an error for a result.
5. Quiet suppresses a successful result in `pretty` and in a default format, the stream default included.
   It never hides errors, and never hides a machine format that a flag, a shorthand, or a variable named.
6. A reader that closes stdout or stderr early, as `head -1` does, leaves the exit code as the command set it and puts nothing on stderr:
   - A command that succeeded exits 0.
   - A failed command or a parse error keeps its code, so a closed pipe never turns a failure into success.
   - `App::broken_pipe` with `BrokenPipe::Fail` makes every closed pipe a failure, keeping a failure's own code.

## Help law

Clap derive types are the command grammar.
ctl-core extracts help from that graph and renders it through the same document renderer.
No consumer maintains a second help parameter model or a local terminal-width implementation.

## Fluent boundary

Consumer presentation code composes meaning:

```rust,ignore
Document::new()
    .heading("status")
    .fields(
        Fields::new()
            .row("active", result.active.as_deref().unwrap_or("none"))
            .row("queued", result.queued.to_string()),
    )
    .table(
        Table::new(["id", "title"])
            .token_column(0)
            .row([result.id.as_str(), result.title.as_str()]),
    )
    .notice(Notice::new(NoticeLevel::Warning, "one stale row"))
```

It does not choose borders, spacing, ANSI styles, width, or streams.

## Operator surface

Clap is also the source for committed operator documents.
`Surface` extracts binary and mounted names, package metadata, recursive commands, aliases, visibility, locally declared argument and flag metadata, and mounted Usage KDL.

That metadata and its validation methods require only `surface-model`.
`surface-serde` adds serialization without a template engine, and `surface-templates` adds MiniJinja.
`surface-model` pulls in no Serde: ctl-core writes the Usage KDL itself.
The existing `surface` feature remains the compatibility aggregate.

Ancestor globals stay normalized on their declaring command and appear separately as each descendant's `inherited_arguments`.
Templates can render the effective flag set without losing declaration provenance.
`Surface` records hidden commands for verification, while the shared command-inventory fragment renders visible commands only.
`Surface::note` carries optional skill- or instruction-specific prose when Clap's `about` addresses another audience.

ctl-core ships three MiniJinja fragments:

- `ctl/version.md.jinja` renders a skill frontmatter version line;
- `ctl/invocation.md.jinja` renders mounted examples and the no-`--` rule;
- `ctl/commands.md.jinja` renders the visible top-level command inventory.

A consumer template imports those fragments and keeps its own domain prose.
Tests render with the consumer's Clap type and byte-compare the result with the committed skill or installed instructions.
Adding or renaming a Clap command therefore changes `Surface`.
The committed-render test then fails until the operator document moves with it.
No token scan or copied command list remains.

## Migration order

1. Publish the ctl-core presentation kernel.
2. Migrate verctl, which already has typed report models and uses ctl-core `View`.
3. Migrate forkctl, retaining its protocol types while deleting local help, view, layout, and terminal dependencies.
4. Migrate qctl handlers from direct printing to typed results, then add shared pretty, colorless, and JSON views.
5. Pin the released `surface` feature in each consumer.
   Render its skill and instructions from shared fragments plus domain prose.
   Delete copied verb scans and invocation rules.

Each consumer pins a released ctl-core version.
No path dependencies connect repositories.
