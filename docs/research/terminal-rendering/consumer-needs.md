# What the consumers ask of ctl-core's presentation layer

Read-only survey. Nothing was compiled and no checkout's working tree or
branch was changed. Each repository's default branch was fetched and its tree
extracted from `origin/main`.

It covers the four public consumers: verctl, qctl, forkctl and rawr. Private
consumers of ctl-core were checked as well and are not named or described
here; they use the same primitives and need the same gaps listed in Part D.

## 0. How to read the counts

- Counts are regex hits over the extracted tree, not parsed calls. Counts for
  ctl-core primitives are limited to files that import `ctl_core`, and to
  `src/` unless a row says "tests".
- A count of `.row(` mixes `Fields` rows and `Table` rows. `Fields::new(` and
  `Table::new(` are given separately.
- "Hand-rolled" means the consumer draws terminal output without ctl-core.
- Cargo.lock presence is not proof the crate is linked into every binary of the
  workspace. It shows what the resolver would fetch for that lockfile.

### 0.1 Commits read

| Repo | Branch | Commit |
|:--|:--|:--|
| ctl-core | `origin/main` | `7dc8060e2578af605f4ae858947693cb52477d6f` |
| verctl | `origin/main` | `41757015205f335135dfc84274b0330bcb9be4b0` |
| qctl | `origin/main` | `33a74b56af84f5ce237a8468fbe7a3b26f8081ec` |
| forkctl | `origin/main` | `7b987070f9069842c2e9afdd17fca28353b81bc5` |
| rawr | `origin/main` | `40abc93a3ef4bb293540bfce6320a69d4ebbc5d9` |

Paths below are relative to the repository root at those commits.

## Part A. ctl-core at `7dc8060e2578` (version 0.6.28)

### A.1 Module sizes

Line counts include the unit tests at the bottom of each file. The second
number is where `#[cfg(test)]` starts, so it is the size of the shipped code.

| Module | Lines | Code lines | Feature gate |
|:--|--:|--:|:--|
| [src/document.rs][ctl-core-document] | 567 | 546 | `document` |
| [src/render.rs][ctl-core-render] | 1090 | 752 | `render` |
| [src/view.rs][ctl-core-view] | 450 | 398 | `view` |
| [src/help.rs][ctl-core-help] | 428 | 284 | `help` |
| [src/layout.rs][ctl-core-layout] | 64 | 37 | `render` (private module) |
| [src/style.rs][ctl-core-style] | 61 | 46 | `render` (private module) |
| [src/table.rs][ctl-core-table] | 86 | 46 | `render` |
| [src/color.rs][ctl-core-color] | 96 | 78 | always |
| [src/format.rs][ctl-core-format] | 79 | 69 | always |
| [src/model.rs][ctl-core-model] | 114 | 80 | always (serde derive needs `json`) |
| [src/app.rs][ctl-core-app] | 645 | 353 | `app` |
| [src/flags.rs][ctl-core-flags] plus `src/flags/*.rs` | 27 + 30 + 15 + 413 + 111 + 266 | n/a | `cli` |
| [src/parser.rs][ctl-core-parser] | 164 | 119 | `cli` |
| [src/prelude.rs][ctl-core-prelude] | 54 | n/a | always |
| [src/lib.rs][ctl-core-lib] | 167 | n/a | n/a |

Other modules, not part of presentation: [surface.rs][ctl-core-surface] 710, [input.rs][ctl-core-input] 489 plus
`input/*` 538, [served.rs][ctl-core-served] 204, [usage.rs][ctl-core-usage] 148, [validate.rs][ctl-core-validate] 70, [fixture.rs][ctl-core-fixture]
138. The `tests/` directory, [examples/gallery.rs][ctl-core-examples-gallery] and [docs/presentation.md][ctl-core-docs-presentation]
together are 2,923 lines.

### A.2 Feature graph that matters for rendering

From [Cargo.toml][ctl-core-Cargo]:

- `document`: no dependencies. The semantic types only.
- `render` = `document` + `anstyle` + `comfy-table` + `unicode-bidi` +
  `unicode-general-category` + `unicode-width`.
- `json` = `serde` + `serde_json`.
- `view` = `render` + `json` + `anstream`.
- `help` = `cli` + `render` + `anstream`.
- `app` = `cli` + `view` + `help` + `anyhow`. This is the default feature.
- comfy-table is built with `default-features = false` and
  `features = ["custom_styling", "tty"]`.

### A.3 Public API by module

#### `document` ([src/document.rs][ctl-core-document])

Re-exported at the crate root and in `prelude` (the block types).

- `Document`: `new()`, `block(impl Into<Block>)`, `heading(impl Into<Text>)`,
  `paragraph(impl Into<Text>)`, `verbatim(impl Into<String>)`,
  `fields(Fields)`, `table(Table)`, `section(Section)`, `notice(Notice)`,
  `rule(Option<Text>)`, `is_empty()`, `blocks() -> &[Block]`. Also
  `render(&self, RenderOptions) -> String`, added in [render.rs][ctl-core-render].
- `Block`: `Heading(Text)`, `Paragraph(Text)`, `Verbatim(String)`,
  `Fields(Fields)`, `Table(Table)`, `Section(Section)`, `Notice(Notice)`,
  `Rule(Rule)`. `From` impls for `Fields`, `Table`, `Section`, `Notice`,
  `Rule`.
- `Text`: `new()`, `plain(..)`, `span(Role, ..)`, `then(..)`, `token(..)`,
  `id(..)`, `value(..)`, `muted(..)`, `success(..)`, `warning(..)`,
  `error(..)`, `spans()`, `is_empty()`. `From<&str>` and `From<String>`.
- `Span`: `role()`, `value()`.
- `Role` (`#[non_exhaustive]`): `Plain`, `Heading`, `Success`, `Warning`,
  `Error`, `Value`, `Muted`, `Token`, `Id`.
- `Fields`: `new()`, `row(label: impl Into<String>, value: impl Into<Text>)`
  (the label becomes a `Token` span), `text_row(Text, Text)`, `rows()`,
  `is_empty()`.
- `Table`: `plain()` (headerless), `new(headers)`, `token_column(usize)`,
  `id_column(usize)`, `stacked_below(width: u16, label_columns: usize)`,
  `row(cells)`, `headers()`, `rows()`, `token_column_index()`,
  `id_column_index()`, `stacked() -> Option<Stacked>`, `is_empty()`.
- `Stacked`: `width()`, `label_columns()`.
- `Section`: `new(title, Document)`, `title()`, `body()`.
- `Notice`: `new(NoticeLevel, message)`, `code(..)`, `level()`,
  `code_value()`, `message()`.
- `NoticeLevel`: `Success`, `Warning`, `Error`.
- `Rule`: `title()`. It has no public constructor; `Document::rule` builds it.

Not present: nested lists, a tree, an inline link, a progress or live block, a
code block with a language, or a per-cell alignment setting.

#### `render` ([src/render.rs][ctl-core-render])

- Constants: `DEFAULT_COLUMN_BUFFER` (2), `DEFAULT_COLUMN_BUFFER_ENV`
  (`CTL_CORE_COLUMN_BUFFER`), `DEFAULT_COLUMN_BUFFER_ENVS`,
  `DEFAULT_MINIMUM_AUTOMATIC_WIDTH` (20), `DEFAULT_FALLBACK_WIDTH` (80),
  `DEFAULT_RECORD_STYLE_ENV` / `_ENVS` (`CTL_CORE_RECORD_STYLE`),
  `DEFAULT_LIST_STYLE_ENV` / `_ENVS` (`CTL_CORE_LIST_STYLE`),
  `DEFAULT_ROW_SEPARATION_ENV` / `_ENVS` (`CTL_CORE_ROW_SEPARATION`).
- `RecordStyle`: `Boxed`, `KeysRight` (default), `KeysLeft`.
- `ListStyle`: `Grid` (default), `HeaderRule`, `Plain`.
- `RowSeparation`: `None` (default), `Rule`, `Blank`.
- `RenderOptions` (Copy): `new(ColorMode)` and builders `record_style`,
  `list_style`, `row_separation`, `record_style_envs`, `list_style_envs`,
  `row_separation_envs`, `width`, `automatic_width_buffer`,
  `automatic_width_buffer_envs`, `minimum_automatic_width`, `fallback_width`.
  Getters: `record()`, `list()`, `separation()`, `fallback()`, `color()`,
  `explicit_width()`, `explicit_automatic_width_buffer()`,
  `automatic_width_buffer_env_names()`, `automatic_width_minimum()`.
  Precedence for each style: the owner's setting, then the environment
  variable list, then the default.
- `Renderer`: `new(RenderOptions)`, `render(self, &Document) -> String`.
- Crate-private: `EnvChoice`, `env_choice`, `env_choice_warnings`,
  `process_env`.

#### `view` ([src/view.rs][ctl-core-view])

- `Present: Serialize`: `present() -> Document`, `message_kind()`
  (default `Success`), `exit_code()` (default from `message_kind`).
- `MessageKind`: `Success`, `Error`; `default_exit_code()` (0 or 1).
- `Stream`: `None`, `Stdout`, `Stderr`.
- `Captured`: `stream()`, `bytes()`, `text()`, `exit_code()`.
- `JsonLayout`: `Compact`, `Pretty` (default), `PrettyOnTerminal`.
  Constants `DEFAULT_JSON_LAYOUT_ENV` (`CTL_CORE_JSON_LAYOUT`) and `_ENVS`.
- `View` (Copy): public fields `format`, `color`, `quiet`. `new(OutputFormat,
  ColorMode)`, builders `json_layout`, `json_layout_envs`, `styles(RenderOptions)`,
  `quiet`, `width`, `automatic_width_buffer`, `automatic_width_buffer_envs`,
  `minimum_automatic_width`, `fallback_width`. Getters `layout()`,
  `explicit_automatic_width_buffer()`, `automatic_width_buffer_env_names()`,
  `automatic_width_minimum()`. Actions: `capture(&impl Present) ->
  io::Result<Captured>`, `show(&impl Present) -> io::Result<ExitCode>`,
  `emit_err(bin, message) -> io::Result<ExitCode>`.
- Crate-private: `write_stdout`, `write_stderr`.

#### `help` ([src/help.rs][ctl-core-help])

- `try_emit::<C>()`, `try_emit_with_options::<C>(RenderOptions)`,
  `try_emit_from::<C>(&[String])`,
  `try_emit_from_with_options::<C>(&[String], RenderOptions)`: print help and
  return `true` when help was asked for.
- `emit_bare_with_options::<C>(RenderOptions)`: root help to stderr.
- `document(clap::Command) -> Document`: the help as a semantic document.

#### `layout` (private, [src/layout.rs][ctl-core-layout])

- `terminal_width(buffer, minimum) -> Option<u16>`, `column_buffer(names)`.
  Not public.

#### `style` (private, [src/style.rs][ctl-core-style])

Eight constants of `anstyle::Style`, plus `styled(style, value) -> String`.

| Role | Style |
|:--|:--|
| `Heading` | bold cyan |
| `Success` | bold green |
| `Warning` | bold yellow |
| `Error` | bold red |
| `Value` | bold yellow |
| `Muted` | bright black |
| `Token` | bold green |
| `Id` | bold, terminal foreground |

Only the sixteen named ANSI colours are used. There is no 256-colour or RGB
style, and no theme object.

#### `table` ([src/table.rs][ctl-core-table], legacy)

- `kv(ColorMode, rows) -> String` and
  `grid(ColorMode, headers, rows) -> String`. Both build a `Document` and render
  it with `RenderOptions::new(color)`, then trim the trailing newline. No
  consumer in the survey calls either.

#### `color` ([src/color.rs][ctl-core-color])

- `ColorMode`: `Auto` (default), `Always`, `Never`. Implements `FromStr`,
  `Display`, `clap::ValueEnum` (feature `cli`), serde (feature `json`),
  `JsonSchema` (feature `schema`). Crate-private `choice()` maps to
  `anstream::ColorChoice`.
- `ParseColorError`.

#### `format` ([src/format.rs][ctl-core-format])

- `OutputFormat`: `Pretty` (default), `Json`; `is_json()`. Same trait set as
  `ColorMode`. `ParseFormatError`.

#### `model` ([src/model.rs][ctl-core-model])

- `SCHEMA_VERSION` (1).
- `Envelope<T>`: `Ok { schema_version, data }`, `Err { schema_version, error }`;
  `ok(data)`, `err(ErrorBody)`. Serialized with a `status` tag.
- `ErrorBody { bin, message }`; `new(bin, message)`.

#### Neighbours consumers touch (not requested, listed for completeness)

- `App<C>` ([src/app.rs][ctl-core-app]): `new(bin)`, `view`, `automatic_width_buffer`,
  `automatic_width_buffer_envs`, `minimum_automatic_width`, `fallback_width`,
  `input_limits`, `styles`, `json_layout`, `json_layout_envs`, `before_parse`,
  `mounted_as`, `usage_spec`, `run`, `run_from`.
- Flags (`src/flags/*`): `OutputArgs`, `FormatArgs`, `FormatLong`, `ColorLong`,
  `DryRunArgs`, `EnvPrefix`, `resolve_color` (`--no-color` wins), warnings.
- `Surface` (feature `surface-model`) for committed operator documents.

### A.4 Where each terminal crate is used

| Crate | File:line | What for |
|:--|:--|:--|
| comfy-table | [src/render.rs:5-9][ctl-core-render-5-9] | Imports presets `NOTHING`, `UTF8_FULL`, `UTF8_FULL_CONDENSED`, `Cell`, `CellAlignment`, `Column`, `ColumnConstraint`, `ContentArrangement`, `LineStyle`, `Table`, `TableStyle`, `Width`. |
| comfy-table | [src/render.rs:461-492][ctl-core-render-461-492] (`fields`) | Key/value records: a two-column table with the `NOTHING` preset, or `UTF8_FULL_CONDENSED` for the boxed style. Keys are right- or left-aligned by column padding and alignment. |
| comfy-table | [src/render.rs:494-542][ctl-core-render-494-542] (`table`, `list_table`) | Lists: header row, token and id column styling, `Grid` / `HeaderRule` / `Plain` presets, blank row between rows for `RowSeparation::Blank`, `RULE` line style ([src/render.rs:42][ctl-core-render-42]). |
| comfy-table | [src/render.rs:609-629][ctl-core-render-609-629] (`wrap`) | Word wrap of paragraphs, headings and notices: a one-cell, borderless, dynamic-width table with the indentation subtracted from the width. |
| comfy-table | [src/render.rs:598-607][ctl-core-render-598-607] (`engine_table`) | Every table is `ContentArrangement::Dynamic` and gets `set_width` when a width is known. |
| comfy-table | [src/render.rs:745-750][ctl-core-render-745-750] (`keep_words_whole`) | A `LowerBoundary` column constraint so a word wider than the column (URL, path) is not broken. |
| comfy-table | [src/layout.rs:16-24][ctl-core-layout-16-24] (`detected_width`) | `Table::new().width()` is the terminal-width probe. With the `tty` feature it returns the table's set width, else, when stdout is a terminal, `crossterm::terminal::size()`, else `None`. |
| anstyle | [src/style.rs:3-39][ctl-core-style-3-39] | The eight `Style` constants; `styled()` renders `style.render()`, the text, then `style.render_reset()`. |
| anstream | [src/view.rs:6][ctl-core-view-6],[385-396][ctl-core-view-385-396] | `AutoStream::new(stdout/stderr lock, ColorChoice)` around `write_all` in `write_stdout` and `write_stderr`. |
| anstream | [src/help.rs:67][ctl-core-help-67],[80][ctl-core-help-80] | The same wrap for help to stdout and bare help to stderr. |
| anstream | [src/color.rs:34-40][ctl-core-color-34-40] | `ColorMode::choice()` to `anstream::ColorChoice`. |
| unicode-width | [src/render.rs:12][ctl-core-render-12],[438][ctl-core-render-438],[737][ctl-core-render-737] | Title width for `Rule` (`UnicodeWidthStr::width`), and word width in `longest_word`. |
| unicode-bidi | [src/render.rs:10][ctl-core-render-10],[714][ctl-core-render-714] | The bidi control constants `ALM FSI LRE LRI LRM LRO PDF PDI RLE RLI RLM RLO`, removed from `Verbatim` text. |
| unicode-general-category | [src/render.rs:11][ctl-core-render-11],[716-718][ctl-core-render-716-718] | `LineSeparator` and `ParagraphSeparator` removed from `Verbatim` text. |

[src/render.rs][ctl-core-render] also contains `char::is_control` (line 711), so `Verbatim` keeps
`\n` and `\t` only and drops every other control character, including ESC.

### A.5 Pipeline from `Document` to bytes

1. A command returns a value that implements `Present` ([src/view.rs:24][ctl-core-view-24]). JSON
   serializes the value itself, never the `Document`.
2. `View::show` calls `View::capture` ([src/view.rs:319][ctl-core-view-319]). For JSON it writes
   `serde_json` output (pretty or compact per `JsonLayout`, resolved from the
   owner's choice, then `CTL_CORE_JSON_LAYOUT`, then `Pretty`). `--quiet` turns a
   successful pretty result into `Stream::None`. Otherwise it calls
   `present()` and `Document::render(RenderOptions)`.
3. `Renderer::render` ([src/render.rs:405][ctl-core-render-405]) renders each block to a string and
   joins non-empty ones with a blank line, then one trailing `\n`.
4. Painting. `Renderer::paint` ([src/render.rs:662][ctl-core-render-662]) turns each span into
   `styled(style, text)`. This happens before layout, so SGR escape sequences
   are inside the strings handed to comfy-table. With `ColorMode::Never` or
   `Role::Plain` no sequence is added. `Auto` and `Always` both paint. The
   decision to strip is made later, at the write.
5. Re-measuring. comfy-table with `custom_styling` measures a cell by stripping
   ANSI (`ansi_str` strip, then unicode width) and splits lines with `console`'s
   ANSI-aware iterator. ctl-core's own `longest_word` ([src/render.rs:724][ctl-core-render-724]) also
   strips SGR, but only sequences that end in `m`, to size the no-break column
   bound.
6. Layout. `Heading`, `Paragraph` and `Notice` go through `wrap`. `Fields` and
   `Table` go through `engine_table`. `Section` renders its heading, then the
   rendered body with its trailing newline removed. `Rule` draws `─` repeated to
   the width (40 when no width), with an optional `── title ──...` form.
   `Verbatim` is sanitised and not wrapped. A table with `stacked_below` renders
   as indented label and description lines when the width is under the
   threshold ([src/render.rs:551-582][ctl-core-render-551-582]).
7. Trailing spaces are trimmed line by line (`trim_line_ends`,
   [src/render.rs:691][ctl-core-render-691]), because borderless presets leave cell padding.
8. Write. `write_stdout` and `write_stderr` wrap `io::stdout().lock()` or
   `io::stderr().lock()` in `anstream::AutoStream` with the `ColorChoice`
   mapped from `ColorMode`. `AutoStream` strips SGR when the stream is not a
   terminal or the environment says no colour, and passes it when `Always`.
   Success goes to stdout, errors (`MessageKind::Error`) to stderr; JSON always
   goes to stdout. Write errors, including a closed pipe, are returned as
   `io::Error`; `App::run_from` turns that into `ExitCode::FAILURE`
   ([src/app.rs:281][ctl-core-app-281]).
9. Consequence: `Captured` content for `ColorMode::Auto` or `Always` contains
   SGR bytes. Only `Never` makes it clean. Tests in the consumers all use
   `ColorMode::Never` for that reason.

### A.6 Width detection

Order, implemented across [src/render.rs:631-639][ctl-core-render-631-639] and [src/layout.rs][ctl-core-layout]:

1. `RenderOptions::width` or `View::width`, if set. It is exact: no buffer, no
   floor.
2. Otherwise, `layout::terminal_width`:
   - `comfy_table::Table::width()`. With the `tty` feature this checks
     whether **stdout** is a terminal (`Table::is_tty`, stdout, not stderr). If
     it is, it calls `crossterm::terminal::size()`. On Unix that opens
     `/dev/tty` and runs the window-size ioctl on it, and falls back to the
     stdout descriptor when `/dev/tty` cannot be opened.
   - If that returns nothing (stdout not a terminal, or the call failed), the
     `COLUMNS` environment variable, accepted when it parses as `u16` and is
     at least 1. When stdout is a terminal and the ioctl succeeds, `COLUMNS` is
     not read.
   - The detected width is reduced by the buffer (default 2, from
     `automatic_width_buffer`, else the env name list starting with
     `CTL_CORE_COLUMN_BUFFER`, else 2), but not below the minimum (20) unless the
     terminal itself is narrower, and never below 1
     (`effective_width`, [src/layout.rs:26][ctl-core-layout-26]).
3. If still nothing, the fallback width (80). `fallback_width(None)` turns it
   off, so tables render at natural width and prose is not wrapped
   (`wrap` returns the input when no width is known).

Help to stderr (`emit_bare_with_options`) still probes **stdout**, because the
probe is shared. That is a mismatch when only one of the two is redirected.

### A.7 How help is drawn

- `App::run_from` ([src/app.rs][ctl-core-app]) calls `help::try_emit_from_with_options` first
  ([src/app.rs:257][ctl-core-app-257]), using the `View`'s `render_options()`.
- `try_emit_from_with_options` ([src/help.rs:54][ctl-core-help-54]) asks `parser::wants_help` if
  `-h` or `--help` is present, picks the subcommand named in argv
  (`select_command`), builds a `Document` with `help::document`, renders it
  with `Document::render`, and writes to stdout through `AutoStream`.
- `document(Command)` ([src/help.rs:124][ctl-core-help-124]) builds: a heading-role paragraph with
  clap's rendered usage line; the command's `about`; a `Commands` section whose
  table is `Table::plain().token_column(0).stacked_below(64, 1)`; an
  `Arguments` section (positional rows, stacked below 64 with 3 label
  columns); one section per `help_heading` for options, with rows
  `[-s] [--long] [value label] [description + default + env]`; and a trailing
  `Help` section. Value labels show `[a|b|c]` for enumerated values. Defaults
  and env names are appended as `Muted` text.
- Help text itself never goes through clap's renderer. Clap's colour setting is
  `Auto` on the real parse ([src/parser.rs:90][ctl-core-parser-90]) and `Never` on the lenient
  pre-parse that reads `--color` and `--format` ([src/parser.rs:32][ctl-core-parser-32],[66][ctl-core-parser-66]). Clap
  errors that are not help or version go through `View::emit_err`
  (`{bin}: {message}`), or a JSON envelope when `--format json` appears earlier
  in argv; help and version display errors are printed by clap's own
  `error.print()` ([src/app.rs:288-293][ctl-core-app-288-293]).

## Part B. Consumers

### B.1 Pins and features

| Consumer | ctl-core | Features on the dependency | Notes |
|:--|:--|:--|:--|
| verctl | `=0.6.23` | default (so `app`), plus `input`, `surface`, `usage` | Runtime dependency; dev-dependency adds `surface`. |
| qctl | `=0.6.24` | `app`, `input`, `usage` | Dev-dependency adds `surface`. |
| forkctl | `=0.6.23` | `app`, `input`, `usage` | Dev-dependency adds `surface`. |
| rawr | `=0.6.28` | `app` | Public. Also depends on `ratatui` 0.30.2 and `textwrap`. |

### B.2 Use of ctl-core primitives, by primitive

Counts are for `src/` files that import `ctl_core`. Dashes mean zero.

| Primitive | verctl | qctl | forkctl | rawr |
|:--|--:|--:|--:|--:|
| `Document::new(` | 23 | 31 | 7 | 3 |
| `Fields::new(` | 17 | 6 | 18 | - |
| `Table::new/plain(` | 10 | 6 | 1 | - |
| `token_column` / `id_column` / `stacked_below` | 10 | 6 | 1 | - |
| `Section::new(` | - | 3 | 1 | - |
| `Notice::new(` | 1 | 7 | 5 | - |
| `.rule(` / `Rule` | - | - | - | - |
| `.verbatim(` | 1 | 2 | 4 | 2 |
| `.heading(` | 1 | 3 | 2 | - |
| `.paragraph(` | 1 | 9 | 2 | - |
| `Text::new/plain(` | - | 16 | 3 | - |
| Role span builders (`token id muted success warning error value`) | - | 27 | 3 | - |
| `Role::` used directly | - | - | - | - |
| `impl Present` | 13 | 1 | 1 | 1 |
| `App::<C>::new` | 1 | 1 | 1 (alias `Chassis`) | 1 |
| `View::new(` | 10 (in-file tests) | - | 2 | 1 (in-file test) |
| `OutputArgs/FormatArgs/FormatLong/ColorLong/DryRunArgs` | 5 | 3 | 15 | - |
| `RenderOptions`, `RecordStyle`, `ListStyle`, `RowSeparation` | - | - | - | - |
| `kv(` / `grid(` legacy | - | - | - | - |
| `.capture(` / `Captured` | 10 | - | 1 | 1 |
| `help::` entry points called directly | - | - | - | - |
| `Envelope` / `ErrorBody` used directly | - | - | - | - |
| `Surface` | 4 | 2 | 1 | - |
| `input::` / `Input::` | 6 | 13 | 3 | - |
| `EnvPrefix` (file count) | 3 | 2 | 2 | - |

Role span counts are matches of `.token(`, `.id(`, `.muted(` and so on in files
that build documents, so they include a few non-ctl-core method calls. Direct
`Role::` use is zero everywhere; consumers reach roles only through the `Text`
builders and the `token_column` / `id_column` table options.

#### B.2.1 Representative call sites

##### Fields (key/value record)

- verctl [src/presentation.rs:66][verctl-presentation-66]:
  `Document::new().fields(Fields::new().row("no-op", "no crate to check"))`.
- forkctl [src/presentation.rs:135][forkctl-presentation-135], [:148][forkctl-presentation-148], [:151][forkctl-presentation-151]: multi-row records for
  operation results; [:468][forkctl-presentation-468] puts a `Text::new().token(command)` into a row.

##### Table (list)

- verctl [src/presentation.rs:69][verctl-presentation-69]:
  `Table::new(vec!["name", "version", "docs.rs"]).id_column(0)`.
- qctl [src/presentation.rs:101-103][qctl-presentation-101-103]:
  `Table::new(["priority", "id", "title"]).id_column(1).stacked_below(80, 2)`.

##### Section and heading

- qctl [src/presentation.rs:117][qctl-presentation-117], [:127][qctl-presentation-127], [:145][qctl-presentation-145]:
  `document.section(Section::new("queue", ...))`, also `horizon` and `problems`.
- qctl [src/presentation.rs:162][qctl-presentation-162], [:167][qctl-presentation-167], [:180][qctl-presentation-180]:
  `.heading(Text::new().id(&task.id).then("  ").then(&task.title))`.
- forkctl [src/presentation.rs:109][forkctl-presentation-109], [:459][forkctl-presentation-459], [:471][forkctl-presentation-471]:
  `heading(format!("forkctl · {command}"))`, `Section::new("operation", body)`.

##### Notice

- verctl [src/presentation.rs:191][verctl-presentation-191]: `Notice::new(NoticeLevel::Error, message)`.
- qctl [src/presentation.rs:134][qctl-presentation-134], [:146][qctl-presentation-146], [:218][qctl-presentation-218].
- forkctl [src/presentation.rs:111][forkctl-presentation-111], [:119][forkctl-presentation-119], [:124][forkctl-presentation-124]: warnings, including the
  update notice and the declared-limit warnings.

##### Verbatim

- qctl [src/presentation.rs:54][qctl-presentation-54]: the bundled instructions Markdown, printed as is.
- forkctl [src/presentation.rs:58-59][forkctl-presentation-58-59]: a JSON value or a text blob; [:107][forkctl-presentation-107]:
  rendered Markdown.
- rawr [src/main.rs:98-99][rawr-main-98-99]: the single command on stdout, so a shell can run
  `$(rawr pick)`.

##### Paragraph and styled text

- qctl [src/presentation.rs:15-20][qctl-presentation-15-20]:
  `Text::new().success("active").then("  ").id(id)`.

##### Present

- verctl [src/presentation.rs:22][verctl-presentation-22], [:63][verctl-presentation-63], [:92][verctl-presentation-92]: 13 report types, one per verb.
- forkctl [src/presentation.rs:51][forkctl-presentation-51]: one `Report` enum for every command.

##### View and App

- verctl [src/main.rs:26-28][verctl-main-26-28]:
  `App::<Cli>::new("verctl").mounted_as("ver").view(|cli| cli.format.view(cli.color.color()))`.
- forkctl [src/main.rs:57-60][forkctl-main-57-60]: `mounted_as("fork")`, `usage_spec(..)`,
  `before_parse(complete_from_environment)`, `view(select_view)`;
  [:81][forkctl-main-81] picks `View::new(OutputFormat::Json, ColorMode::Never)` for its `Api`
  command and `Pretty` otherwise.

##### Tests that capture

- verctl [src/presentation.rs:546-576][verctl-presentation-546-576]:
  `View::new(Pretty, Never).width(80).capture(&report)`.

### B.3 Terminal output done without ctl-core

#### verctl, qctl, forkctl (public)

These are close to clean. qctl and forkctl each carry an architecture test
([tests/architecture.rs][forkctl-tests-architecture]) that forbids the four print macros and the
dependencies `anstream`, `anstyle` and `comfy-table` (qctl also
`unicode-width`). qctl's test also allows `ctl_core::` only in [main.rs][qctl-main],
[cli.rs][qctl-cli], [operator_docs.rs][qctl-operator-docs] and [presentation.rs][qctl-presentation].

| Where | What |
|:--|:--|
| forkctl [src/main.rs:143][forkctl-main-143] | `eprintln!("forkctl: warning: {warning}")` for each declared-limit warning, before any view exists. |
| forkctl [src/update.rs:26][forkctl-update-26] | `io::stdout().is_terminal()` gates an update notice (not a rendering concern, but a terminal probe). |
| forkctl [src/process.rs:160][forkctl-process-160], verctl [src/process.rs:112][verctl-process-112] | A child process's stderr is copied to the parent's stderr as it arrives, chunk by chunk. |
| forkctl [src/report.rs:12][forkctl-report-12],[30][forkctl-report-30], [src/presentation.rs][forkctl-presentation] | `range_diff` is a string produced by git and shown verbatim. |
| qctl [tests/mutations.rs:24][qctl-tests-mutations-24],[94][qctl-tests-mutations-94] | `similar::TextDiff` in a test, to assert on file edits. Not output. |
| verctl [src/cli.rs:87][verctl-cli-87],[103][verctl-cli-103],[136][verctl-cli-136] | Writes `$GITHUB_OUTPUT` lines. File output, not terminal output. |
| qctl, verctl, forkctl | Bundled Markdown instructions and changelog text are printed as `verbatim` or `paragraph` with no Markdown rendering. |

Escape sequence mentions in these three are only test assertions that JSON
carries no ESC byte ([verctl src/presentation.rs:605][verctl-presentation-605], [qctl tests/cli.rs:927][qctl-tests-cli-927],[963][qctl-tests-cli-963],[1011][qctl-tests-cli-1011],
[forkctl tests/cli.rs:194][forkctl-tests-cli-194]).

#### rawr (public)

rawr draws a full-screen picker without ctl-core, and uses ctl-core only for the
one-line result.

| Where | What |
|:--|:--|
| [src/picker.rs][rawr-picker] (124 lines), [src/picker/state.rs][rawr-picker-state] (365), [src/picker/terminal.rs][rawr-picker-terminal] (144), [src/picker/view.rs][rawr-picker-view] (444) | A ratatui + crossterm picker: alternate screen, raw mode, mouse capture, key and mouse handling, scrollbar, row hit-testing, key-hint line, search line. |
| [src/picker/terminal.rs:41-42][rawr-picker-terminal-41-42] | Writes OSC 22 pointer-shape sequences by hand (`\x1b]22;pointer\x1b\\`). |
| [src/picker/terminal.rs][rawr-picker-terminal] | Draws to **stderr** so stdout holds only the chosen command (rule 3 of its [AGENTS.md][rawr-AGENTS]). |
| [src/picker/view.rs:14][rawr-picker-view-14],[51][rawr-picker-view-51] | `textwrap::wrap` with a `WordSplitter` to wrap titles in a column. |
| [src/session.rs:185][rawr-session-185], [src/catalog.rs:147][rawr-catalog-147] | `sanitize` and `title` strip escape sequences from untrusted session titles before they are drawn. |
| [src/main.rs:172][rawr-main-172],[185][rawr-main-185] | `stderr().is_terminal()` decides whether a picker can open at all. |

### B.4 Overlapping renderer crates in each Cargo.lock

Source: the root `Cargo.lock` of each tree. None of `tabled`, `indicatif`,
`owo-colors` or `terminal_size` appear. `anstyle` is 1.0.14, `anstream` 1.0.0,
`anstyle-parse` 1.0.0 and `unicode-segmentation` 1.13.3 in every lockfile. A dash
means the package is absent.

| Lockfile | crossterm | ratatui | comfy-table | console | textwrap | unicode-width | terminal_size | rustix | libc | miette |
|:--|:--|:--|:--|:--|:--|:--|:--|:--|:--|:--|
| ctl-core | 0.29.0 | - | 8.0.0 | 0.16.4 | - | 0.1.14, 0.2.2 | - | 1.1.4 | 0.2.189 | 7.6.0 |
| verctl | 0.29.0 | - | 8.0.0 | 0.16.4 | - | 0.1.14, 0.2.2 | - | 1.1.4 | 0.2.189 | 7.6.0 |
| qctl | 0.29.0 | - | 8.0.0 | 0.16.4 | - | 0.1.14, 0.2.2 | - | 1.1.4 | 0.2.189 | 7.6.0 |
| forkctl | 0.29.0 | - | 8.0.0 | 0.16.4 | - | 0.1.14, 0.2.2 | - | 1.1.4 | 0.2.189 | 7.6.0 |
| rawr | 0.29.0 | 0.30.2 | 8.0.1 | 0.16.6 | 0.16.4 | 0.2.3 | - | 1.1.6 | 0.2.191 | - |

Other renderer-adjacent packages in the same lockfiles:

| Package | Where it appears |
|:--|:--|
| `similar` | qctl 2.7.0 and 3.2.0; forkctl 2.7.0. |
| `unicode-bidi` 0.3.18, `unicode-general-category` 1.1.0 | Every lockfile, from ctl-core's `render` feature. |

How these arrive:

- `crossterm` and `console` come from comfy-table (`tty` and `custom_styling`),
  in every ctl-core consumer. In the ctl-core lockfile the only package that
  depends on `console` is `comfy-table`. comfy-table also pulls `ansi-str`.
- `miette` in ctl-core's lockfile is reached only through `usage-lib` and `kdl`
  (the `usage` feature). No consumer source uses it.
- Direct dependencies that overlap a renderer, from the manifests: rawr
  `ratatui`, `textwrap`; qctl `similar` (dev). verctl and forkctl declare none.

## Part C. Needs matrix

Legend:

- **core** means used through ctl-core (count in brackets where useful).
- **H** means hand-rolled, followed by a few words on what.
- **X:crate** means done with that crate directly.
- Blank means no use found.

| Capability | verctl | qctl | forkctl | rawr |
|:--|:--|:--|:--|:--|
| Record / fields | core (17) | core (6) | core (18) | |
| List table | core (10) | core (6) | core (1) | X:ratatui rows |
| Table: narrow-width stacking | | core (`stacked_below(80,2)`) | | |
| Sections / headings | core (1 heading) | core (3 sections, 3 headings) | core (1, 2) | |
| Notices | core (1) | core (7) | core (5) | |
| Verbatim text | core (1) | core (2) | core (4) | core (2) |
| Styled inline text / roles | | core (~27 spans) | core (3) | |
| Help | core | core | core | core |
| JSON | core | core | core (Api command) | |
| Wrapping | core | core | core | X:textwrap [src/picker/view.rs:51][rawr-picker-view-51] |
| Width detection | core | core | core | X:crossterm via ratatui |
| Color policy | core | core | core | |
| Panel / box | | | | X:ratatui |
| Live redraw | | | | X:ratatui |
| Markdown | prints raw | prints raw | prints raw | |
| Pretty-print of data | | | verbatim JSON [src/presentation.rs:58][forkctl-presentation-58] | |
| Error report rendering | core (`{bin}: msg`) | core | core | core |
| Prompts: interactive pick | | | | X:ratatui [src/picker.rs][rawr-picker] |
| Sanitise untrusted text | verbatim only | verbatim only | verbatim only | H: [src/session.rs:185][rawr-session-185] |
| Relay a child process's output | H: [src/process.rs:112][verctl-process-112] | | H: [src/process.rs:160][forkctl-process-160] | |
| Raw byte pass-through to stdout | | | | core (`verbatim` of one command) |
| Pre-view warnings on stderr | | | H: [src/main.rs:143][forkctl-main-143] | |

## Part D. What the data says

### D.1 Capabilities by use

By count of consumers and call sites:

1. Key/value record (`Fields`): 3 of 4 CLIs, 41 constructors.
2. List table (`Table`): 3 of 4 CLIs, 17 constructors. `id_column` is by far the
   most used option, then `stacked_below` (qctl).
3. `Present` plus `App` plus `View`: all 4. Every CLI follows the same shape:
   return a typed value, implement `Present`, call `App::run`.
4. Verbatim text: 4 of 4 CLIs. It is the escape hatch for Markdown, JSON and
   one-line machine output.
5. Notices: 3 CLIs (verctl, qctl, forkctl), 13 constructors.
6. Paragraph / heading / section / styled spans: qctl and forkctl. Roles seen:
   `id`, `success`, `muted`, `token`, `warning`, `error`, `value`. Never
   `Heading` or `Plain` directly.
7. Never used: `Rule`, `kv`, `grid`, `Role::` by name, the help entry points,
   `RowSeparation`, `ListStyle` and `RecordStyle`.

### D.2 Hand-rolled gaps that the library could obviously offer

Gaps between what ctl-core offers and what consumers write themselves:

1. A table with per-column width policy: natural, fixed with ellipsis
   truncation, shrink-to-minimum when the table is wider than the screen, and
   left or right justification. ctl-core has dynamic wrapping only; no
   ellipsis truncation and no per-column alignment.
2. A width rule with its own policy for piped output. ctl-core has
   `fallback_width`, but nothing that distinguishes "terminal" from "piped"
   beyond that.
3. A live view that redraws in place on a terminal and prints plain lines
   otherwise, with a progress or tally grid.
4. Closed-pipe tolerance on write. ctl-core's `write_stdout` returns the error
   and `App` exits with failure.
5. Escape-sequence sanitising of untrusted text in `Text` spans, not only in
   `Verbatim`. rawr wrote its own ([src/session.rs:185][rawr-session-185]).
6. A confirmation prompt (`[y/N]`) and an echo-off secret read.
7. Hyperlinks (OSC 8), where a URL is printed bare today.
8. A tree or indented-hierarchy block, where a hierarchy is flattened into
   repeated cells today.
9. Numeric alignment inside a cell (`{:>4}`, `{percent:>3}%`).
10. A styled-span `Line` that can be truncated and measured. ctl-core `Text`
    has the spans but no width or fit.

Full-screen work (Markdown, syntax highlighting, diff, images, selection,
scrollbar, mouse, pager, search) is a different product from a CLI chassis.
rawr is the only public CLI that crosses over, with a picker.

### D.3 Behaviours consumers depend on

These are visible in tests and comments, so a replacement renderer must keep
them:

- JSON never contains an ESC byte, whatever `CLICOLOR_FORCE`, `FORCE_COLOR` or
  `COLORTERM` say (tests in verctl, qctl, forkctl, [ctl-core/tests/style_ansi.rs][ctl-core-tests-style-ansi]).
- Parse errors become JSON when `--format json` appears earlier in argv
  ([src/app.rs:300-340][ctl-core-app-300-340]).
- A single-line, newline-terminated output with no trailing spaces; tests compare
  rendered text byte for byte with `ColorMode::Never` and a fixed width.
- A word wider than its column is not broken (URLs, paths).
- Output is deterministic for a given width, so snapshot tests work.
- stdout carries data, stderr carries diagnostics, and a picker draws on stderr
  (rawr).
- Narrow terminals switch a list to stacked label and description lines at 64
  (help), 72 and 80 columns.

## Not read, or read only in part

- ctl-core's [docs/presentation.md][ctl-core-docs-presentation] and [examples/gallery.rs][ctl-core-examples-gallery] were not read.
- Counts are regex hits, as stated in section 0.

---

[ctl-core-document]: https://github.com/victor-software-house/ctl-core/blob/7dc8060e2578af605f4ae858947693cb52477d6f/src/document.rs
[ctl-core-render]: https://github.com/victor-software-house/ctl-core/blob/7dc8060e2578af605f4ae858947693cb52477d6f/src/render.rs
[ctl-core-view]: https://github.com/victor-software-house/ctl-core/blob/7dc8060e2578af605f4ae858947693cb52477d6f/src/view.rs
[ctl-core-help]: https://github.com/victor-software-house/ctl-core/blob/7dc8060e2578af605f4ae858947693cb52477d6f/src/help.rs
[ctl-core-layout]: https://github.com/victor-software-house/ctl-core/blob/7dc8060e2578af605f4ae858947693cb52477d6f/src/layout.rs
[ctl-core-style]: https://github.com/victor-software-house/ctl-core/blob/7dc8060e2578af605f4ae858947693cb52477d6f/src/style.rs
[ctl-core-table]: https://github.com/victor-software-house/ctl-core/blob/7dc8060e2578af605f4ae858947693cb52477d6f/src/table.rs
[ctl-core-color]: https://github.com/victor-software-house/ctl-core/blob/7dc8060e2578af605f4ae858947693cb52477d6f/src/color.rs
[ctl-core-format]: https://github.com/victor-software-house/ctl-core/blob/7dc8060e2578af605f4ae858947693cb52477d6f/src/format.rs
[ctl-core-model]: https://github.com/victor-software-house/ctl-core/blob/7dc8060e2578af605f4ae858947693cb52477d6f/src/model.rs
[ctl-core-app]: https://github.com/victor-software-house/ctl-core/blob/7dc8060e2578af605f4ae858947693cb52477d6f/src/app.rs
[ctl-core-flags]: https://github.com/victor-software-house/ctl-core/blob/7dc8060e2578af605f4ae858947693cb52477d6f/src/flags.rs
[ctl-core-parser]: https://github.com/victor-software-house/ctl-core/blob/7dc8060e2578af605f4ae858947693cb52477d6f/src/parser.rs
[ctl-core-prelude]: https://github.com/victor-software-house/ctl-core/blob/7dc8060e2578af605f4ae858947693cb52477d6f/src/prelude.rs
[ctl-core-lib]: https://github.com/victor-software-house/ctl-core/blob/7dc8060e2578af605f4ae858947693cb52477d6f/src/lib.rs
[ctl-core-surface]: https://github.com/victor-software-house/ctl-core/blob/7dc8060e2578af605f4ae858947693cb52477d6f/src/surface.rs
[ctl-core-input]: https://github.com/victor-software-house/ctl-core/blob/7dc8060e2578af605f4ae858947693cb52477d6f/src/input.rs
[ctl-core-served]: https://github.com/victor-software-house/ctl-core/blob/7dc8060e2578af605f4ae858947693cb52477d6f/src/served.rs
[ctl-core-usage]: https://github.com/victor-software-house/ctl-core/blob/7dc8060e2578af605f4ae858947693cb52477d6f/src/usage.rs
[ctl-core-validate]: https://github.com/victor-software-house/ctl-core/blob/7dc8060e2578af605f4ae858947693cb52477d6f/src/validate.rs
[ctl-core-fixture]: https://github.com/victor-software-house/ctl-core/blob/7dc8060e2578af605f4ae858947693cb52477d6f/src/fixture.rs
[ctl-core-examples-gallery]: https://github.com/victor-software-house/ctl-core/blob/7dc8060e2578af605f4ae858947693cb52477d6f/examples/gallery.rs
[ctl-core-docs-presentation]: https://github.com/victor-software-house/ctl-core/blob/7dc8060e2578af605f4ae858947693cb52477d6f/docs/presentation.md
[ctl-core-Cargo]: https://github.com/victor-software-house/ctl-core/blob/7dc8060e2578af605f4ae858947693cb52477d6f/Cargo.toml
[ctl-core-render-5-9]: https://github.com/victor-software-house/ctl-core/blob/7dc8060e2578af605f4ae858947693cb52477d6f/src/render.rs#L5-L9
[ctl-core-render-461-492]: https://github.com/victor-software-house/ctl-core/blob/7dc8060e2578af605f4ae858947693cb52477d6f/src/render.rs#L461-L492
[ctl-core-render-494-542]: https://github.com/victor-software-house/ctl-core/blob/7dc8060e2578af605f4ae858947693cb52477d6f/src/render.rs#L494-L542
[ctl-core-render-42]: https://github.com/victor-software-house/ctl-core/blob/7dc8060e2578af605f4ae858947693cb52477d6f/src/render.rs#L42
[ctl-core-render-609-629]: https://github.com/victor-software-house/ctl-core/blob/7dc8060e2578af605f4ae858947693cb52477d6f/src/render.rs#L609-L629
[ctl-core-render-598-607]: https://github.com/victor-software-house/ctl-core/blob/7dc8060e2578af605f4ae858947693cb52477d6f/src/render.rs#L598-L607
[ctl-core-render-745-750]: https://github.com/victor-software-house/ctl-core/blob/7dc8060e2578af605f4ae858947693cb52477d6f/src/render.rs#L745-L750
[ctl-core-layout-16-24]: https://github.com/victor-software-house/ctl-core/blob/7dc8060e2578af605f4ae858947693cb52477d6f/src/layout.rs#L16-L24
[ctl-core-style-3-39]: https://github.com/victor-software-house/ctl-core/blob/7dc8060e2578af605f4ae858947693cb52477d6f/src/style.rs#L3-L39
[ctl-core-view-6]: https://github.com/victor-software-house/ctl-core/blob/7dc8060e2578af605f4ae858947693cb52477d6f/src/view.rs#L6
[ctl-core-view-385-396]: https://github.com/victor-software-house/ctl-core/blob/7dc8060e2578af605f4ae858947693cb52477d6f/src/view.rs#L385-L396
[ctl-core-help-67]: https://github.com/victor-software-house/ctl-core/blob/7dc8060e2578af605f4ae858947693cb52477d6f/src/help.rs#L67
[ctl-core-help-80]: https://github.com/victor-software-house/ctl-core/blob/7dc8060e2578af605f4ae858947693cb52477d6f/src/help.rs#L80
[ctl-core-color-34-40]: https://github.com/victor-software-house/ctl-core/blob/7dc8060e2578af605f4ae858947693cb52477d6f/src/color.rs#L34-L40
[ctl-core-render-12]: https://github.com/victor-software-house/ctl-core/blob/7dc8060e2578af605f4ae858947693cb52477d6f/src/render.rs#L12
[ctl-core-render-438]: https://github.com/victor-software-house/ctl-core/blob/7dc8060e2578af605f4ae858947693cb52477d6f/src/render.rs#L438
[ctl-core-render-737]: https://github.com/victor-software-house/ctl-core/blob/7dc8060e2578af605f4ae858947693cb52477d6f/src/render.rs#L737
[ctl-core-render-10]: https://github.com/victor-software-house/ctl-core/blob/7dc8060e2578af605f4ae858947693cb52477d6f/src/render.rs#L10
[ctl-core-render-714]: https://github.com/victor-software-house/ctl-core/blob/7dc8060e2578af605f4ae858947693cb52477d6f/src/render.rs#L714
[ctl-core-render-11]: https://github.com/victor-software-house/ctl-core/blob/7dc8060e2578af605f4ae858947693cb52477d6f/src/render.rs#L11
[ctl-core-render-716-718]: https://github.com/victor-software-house/ctl-core/blob/7dc8060e2578af605f4ae858947693cb52477d6f/src/render.rs#L716-L718
[ctl-core-view-24]: https://github.com/victor-software-house/ctl-core/blob/7dc8060e2578af605f4ae858947693cb52477d6f/src/view.rs#L24
[ctl-core-view-319]: https://github.com/victor-software-house/ctl-core/blob/7dc8060e2578af605f4ae858947693cb52477d6f/src/view.rs#L319
[ctl-core-render-405]: https://github.com/victor-software-house/ctl-core/blob/7dc8060e2578af605f4ae858947693cb52477d6f/src/render.rs#L405
[ctl-core-render-662]: https://github.com/victor-software-house/ctl-core/blob/7dc8060e2578af605f4ae858947693cb52477d6f/src/render.rs#L662
[ctl-core-render-724]: https://github.com/victor-software-house/ctl-core/blob/7dc8060e2578af605f4ae858947693cb52477d6f/src/render.rs#L724
[ctl-core-render-551-582]: https://github.com/victor-software-house/ctl-core/blob/7dc8060e2578af605f4ae858947693cb52477d6f/src/render.rs#L551-L582
[ctl-core-render-691]: https://github.com/victor-software-house/ctl-core/blob/7dc8060e2578af605f4ae858947693cb52477d6f/src/render.rs#L691
[ctl-core-app-281]: https://github.com/victor-software-house/ctl-core/blob/7dc8060e2578af605f4ae858947693cb52477d6f/src/app.rs#L281
[ctl-core-render-631-639]: https://github.com/victor-software-house/ctl-core/blob/7dc8060e2578af605f4ae858947693cb52477d6f/src/render.rs#L631-L639
[ctl-core-layout-26]: https://github.com/victor-software-house/ctl-core/blob/7dc8060e2578af605f4ae858947693cb52477d6f/src/layout.rs#L26
[ctl-core-app-257]: https://github.com/victor-software-house/ctl-core/blob/7dc8060e2578af605f4ae858947693cb52477d6f/src/app.rs#L257
[ctl-core-help-54]: https://github.com/victor-software-house/ctl-core/blob/7dc8060e2578af605f4ae858947693cb52477d6f/src/help.rs#L54
[ctl-core-help-124]: https://github.com/victor-software-house/ctl-core/blob/7dc8060e2578af605f4ae858947693cb52477d6f/src/help.rs#L124
[ctl-core-parser-90]: https://github.com/victor-software-house/ctl-core/blob/7dc8060e2578af605f4ae858947693cb52477d6f/src/parser.rs#L90
[ctl-core-parser-32]: https://github.com/victor-software-house/ctl-core/blob/7dc8060e2578af605f4ae858947693cb52477d6f/src/parser.rs#L32
[ctl-core-parser-66]: https://github.com/victor-software-house/ctl-core/blob/7dc8060e2578af605f4ae858947693cb52477d6f/src/parser.rs#L66
[ctl-core-app-288-293]: https://github.com/victor-software-house/ctl-core/blob/7dc8060e2578af605f4ae858947693cb52477d6f/src/app.rs#L288-L293
[verctl-presentation-66]: https://github.com/victor-software-house/verctl/blob/41757015205f335135dfc84274b0330bcb9be4b0/src/presentation.rs#L66
[forkctl-presentation-135]: https://github.com/victor-software-house/forkctl/blob/7b987070f9069842c2e9afdd17fca28353b81bc5/src/presentation.rs#L135
[forkctl-presentation-148]: https://github.com/victor-software-house/forkctl/blob/7b987070f9069842c2e9afdd17fca28353b81bc5/src/presentation.rs#L148
[forkctl-presentation-151]: https://github.com/victor-software-house/forkctl/blob/7b987070f9069842c2e9afdd17fca28353b81bc5/src/presentation.rs#L151
[verctl-presentation-69]: https://github.com/victor-software-house/verctl/blob/41757015205f335135dfc84274b0330bcb9be4b0/src/presentation.rs#L69
[qctl-presentation-101-103]: https://github.com/victor-software-house/qctl/blob/33a74b56af84f5ce237a8468fbe7a3b26f8081ec/src/presentation.rs#L101-L103
[qctl-presentation-117]: https://github.com/victor-software-house/qctl/blob/33a74b56af84f5ce237a8468fbe7a3b26f8081ec/src/presentation.rs#L117
[qctl-presentation-127]: https://github.com/victor-software-house/qctl/blob/33a74b56af84f5ce237a8468fbe7a3b26f8081ec/src/presentation.rs#L127
[qctl-presentation-145]: https://github.com/victor-software-house/qctl/blob/33a74b56af84f5ce237a8468fbe7a3b26f8081ec/src/presentation.rs#L145
[qctl-presentation-162]: https://github.com/victor-software-house/qctl/blob/33a74b56af84f5ce237a8468fbe7a3b26f8081ec/src/presentation.rs#L162
[qctl-presentation-167]: https://github.com/victor-software-house/qctl/blob/33a74b56af84f5ce237a8468fbe7a3b26f8081ec/src/presentation.rs#L167
[qctl-presentation-180]: https://github.com/victor-software-house/qctl/blob/33a74b56af84f5ce237a8468fbe7a3b26f8081ec/src/presentation.rs#L180
[forkctl-presentation-109]: https://github.com/victor-software-house/forkctl/blob/7b987070f9069842c2e9afdd17fca28353b81bc5/src/presentation.rs#L109
[forkctl-presentation-459]: https://github.com/victor-software-house/forkctl/blob/7b987070f9069842c2e9afdd17fca28353b81bc5/src/presentation.rs#L459
[forkctl-presentation-471]: https://github.com/victor-software-house/forkctl/blob/7b987070f9069842c2e9afdd17fca28353b81bc5/src/presentation.rs#L471
[forkctl-presentation-468]: https://github.com/victor-software-house/forkctl/blob/7b987070f9069842c2e9afdd17fca28353b81bc5/src/presentation.rs#L468
[verctl-presentation-191]: https://github.com/victor-software-house/verctl/blob/41757015205f335135dfc84274b0330bcb9be4b0/src/presentation.rs#L191
[qctl-presentation-134]: https://github.com/victor-software-house/qctl/blob/33a74b56af84f5ce237a8468fbe7a3b26f8081ec/src/presentation.rs#L134
[qctl-presentation-146]: https://github.com/victor-software-house/qctl/blob/33a74b56af84f5ce237a8468fbe7a3b26f8081ec/src/presentation.rs#L146
[qctl-presentation-218]: https://github.com/victor-software-house/qctl/blob/33a74b56af84f5ce237a8468fbe7a3b26f8081ec/src/presentation.rs#L218
[forkctl-presentation-111]: https://github.com/victor-software-house/forkctl/blob/7b987070f9069842c2e9afdd17fca28353b81bc5/src/presentation.rs#L111
[forkctl-presentation-119]: https://github.com/victor-software-house/forkctl/blob/7b987070f9069842c2e9afdd17fca28353b81bc5/src/presentation.rs#L119
[forkctl-presentation-124]: https://github.com/victor-software-house/forkctl/blob/7b987070f9069842c2e9afdd17fca28353b81bc5/src/presentation.rs#L124
[qctl-presentation-54]: https://github.com/victor-software-house/qctl/blob/33a74b56af84f5ce237a8468fbe7a3b26f8081ec/src/presentation.rs#L54
[forkctl-presentation-58-59]: https://github.com/victor-software-house/forkctl/blob/7b987070f9069842c2e9afdd17fca28353b81bc5/src/presentation.rs#L58-L59
[forkctl-presentation-107]: https://github.com/victor-software-house/forkctl/blob/7b987070f9069842c2e9afdd17fca28353b81bc5/src/presentation.rs#L107
[rawr-main-98-99]: https://github.com/victor-software-house/rawr/blob/40abc93a3ef4bb293540bfce6320a69d4ebbc5d9/src/main.rs#L98-L99
[qctl-presentation-15-20]: https://github.com/victor-software-house/qctl/blob/33a74b56af84f5ce237a8468fbe7a3b26f8081ec/src/presentation.rs#L15-L20
[verctl-presentation-22]: https://github.com/victor-software-house/verctl/blob/41757015205f335135dfc84274b0330bcb9be4b0/src/presentation.rs#L22
[verctl-presentation-63]: https://github.com/victor-software-house/verctl/blob/41757015205f335135dfc84274b0330bcb9be4b0/src/presentation.rs#L63
[verctl-presentation-92]: https://github.com/victor-software-house/verctl/blob/41757015205f335135dfc84274b0330bcb9be4b0/src/presentation.rs#L92
[forkctl-presentation-51]: https://github.com/victor-software-house/forkctl/blob/7b987070f9069842c2e9afdd17fca28353b81bc5/src/presentation.rs#L51
[verctl-main-26-28]: https://github.com/victor-software-house/verctl/blob/41757015205f335135dfc84274b0330bcb9be4b0/src/main.rs#L26-L28
[forkctl-main-57-60]: https://github.com/victor-software-house/forkctl/blob/7b987070f9069842c2e9afdd17fca28353b81bc5/src/main.rs#L57-L60
[verctl-presentation-546-576]: https://github.com/victor-software-house/verctl/blob/41757015205f335135dfc84274b0330bcb9be4b0/src/presentation.rs#L546-L576
[forkctl-tests-architecture]: https://github.com/victor-software-house/forkctl/blob/7b987070f9069842c2e9afdd17fca28353b81bc5/tests/architecture.rs
[qctl-main]: https://github.com/victor-software-house/qctl/blob/33a74b56af84f5ce237a8468fbe7a3b26f8081ec/src/main.rs
[qctl-cli]: https://github.com/victor-software-house/qctl/blob/33a74b56af84f5ce237a8468fbe7a3b26f8081ec/src/cli.rs
[qctl-operator-docs]: https://github.com/victor-software-house/qctl/blob/33a74b56af84f5ce237a8468fbe7a3b26f8081ec/src/operator_docs.rs
[qctl-presentation]: https://github.com/victor-software-house/qctl/blob/33a74b56af84f5ce237a8468fbe7a3b26f8081ec/src/presentation.rs
[forkctl-main-143]: https://github.com/victor-software-house/forkctl/blob/7b987070f9069842c2e9afdd17fca28353b81bc5/src/main.rs#L143
[forkctl-main-81]: https://github.com/victor-software-house/forkctl/blob/7b987070f9069842c2e9afdd17fca28353b81bc5/src/main.rs#L81
[forkctl-update-26]: https://github.com/victor-software-house/forkctl/blob/7b987070f9069842c2e9afdd17fca28353b81bc5/src/update.rs#L26
[forkctl-process-160]: https://github.com/victor-software-house/forkctl/blob/7b987070f9069842c2e9afdd17fca28353b81bc5/src/process.rs#L160
[verctl-process-112]: https://github.com/victor-software-house/verctl/blob/41757015205f335135dfc84274b0330bcb9be4b0/src/process.rs#L112
[forkctl-report-12]: https://github.com/victor-software-house/forkctl/blob/7b987070f9069842c2e9afdd17fca28353b81bc5/src/report.rs#L12
[forkctl-report-30]: https://github.com/victor-software-house/forkctl/blob/7b987070f9069842c2e9afdd17fca28353b81bc5/src/report.rs#L30
[forkctl-presentation]: https://github.com/victor-software-house/forkctl/blob/7b987070f9069842c2e9afdd17fca28353b81bc5/src/presentation.rs
[qctl-tests-mutations-24]: https://github.com/victor-software-house/qctl/blob/33a74b56af84f5ce237a8468fbe7a3b26f8081ec/tests/mutations.rs#L24
[qctl-tests-mutations-94]: https://github.com/victor-software-house/qctl/blob/33a74b56af84f5ce237a8468fbe7a3b26f8081ec/tests/mutations.rs#L94
[verctl-cli-87]: https://github.com/victor-software-house/verctl/blob/41757015205f335135dfc84274b0330bcb9be4b0/src/cli.rs#L87
[verctl-cli-103]: https://github.com/victor-software-house/verctl/blob/41757015205f335135dfc84274b0330bcb9be4b0/src/cli.rs#L103
[verctl-cli-136]: https://github.com/victor-software-house/verctl/blob/41757015205f335135dfc84274b0330bcb9be4b0/src/cli.rs#L136
[verctl-presentation-605]: https://github.com/victor-software-house/verctl/blob/41757015205f335135dfc84274b0330bcb9be4b0/src/presentation.rs#L605
[qctl-tests-cli-927]: https://github.com/victor-software-house/qctl/blob/33a74b56af84f5ce237a8468fbe7a3b26f8081ec/tests/cli.rs#L927
[qctl-tests-cli-963]: https://github.com/victor-software-house/qctl/blob/33a74b56af84f5ce237a8468fbe7a3b26f8081ec/tests/cli.rs#L963
[qctl-tests-cli-1011]: https://github.com/victor-software-house/qctl/blob/33a74b56af84f5ce237a8468fbe7a3b26f8081ec/tests/cli.rs#L1011
[forkctl-tests-cli-194]: https://github.com/victor-software-house/forkctl/blob/7b987070f9069842c2e9afdd17fca28353b81bc5/tests/cli.rs#L194
[rawr-picker]: https://github.com/victor-software-house/rawr/blob/40abc93a3ef4bb293540bfce6320a69d4ebbc5d9/src/picker.rs
[rawr-picker-state]: https://github.com/victor-software-house/rawr/blob/40abc93a3ef4bb293540bfce6320a69d4ebbc5d9/src/picker/state.rs
[rawr-picker-terminal]: https://github.com/victor-software-house/rawr/blob/40abc93a3ef4bb293540bfce6320a69d4ebbc5d9/src/picker/terminal.rs
[rawr-picker-view]: https://github.com/victor-software-house/rawr/blob/40abc93a3ef4bb293540bfce6320a69d4ebbc5d9/src/picker/view.rs
[rawr-picker-terminal-41-42]: https://github.com/victor-software-house/rawr/blob/40abc93a3ef4bb293540bfce6320a69d4ebbc5d9/src/picker/terminal.rs#L41-L42
[rawr-AGENTS]: https://github.com/victor-software-house/rawr/blob/40abc93a3ef4bb293540bfce6320a69d4ebbc5d9/AGENTS.md
[rawr-picker-view-14]: https://github.com/victor-software-house/rawr/blob/40abc93a3ef4bb293540bfce6320a69d4ebbc5d9/src/picker/view.rs#L14
[rawr-picker-view-51]: https://github.com/victor-software-house/rawr/blob/40abc93a3ef4bb293540bfce6320a69d4ebbc5d9/src/picker/view.rs#L51
[rawr-session-185]: https://github.com/victor-software-house/rawr/blob/40abc93a3ef4bb293540bfce6320a69d4ebbc5d9/src/session.rs#L185
[rawr-catalog-147]: https://github.com/victor-software-house/rawr/blob/40abc93a3ef4bb293540bfce6320a69d4ebbc5d9/src/catalog.rs#L147
[rawr-main-172]: https://github.com/victor-software-house/rawr/blob/40abc93a3ef4bb293540bfce6320a69d4ebbc5d9/src/main.rs#L172
[rawr-main-185]: https://github.com/victor-software-house/rawr/blob/40abc93a3ef4bb293540bfce6320a69d4ebbc5d9/src/main.rs#L185
[forkctl-presentation-58]: https://github.com/victor-software-house/forkctl/blob/7b987070f9069842c2e9afdd17fca28353b81bc5/src/presentation.rs#L58
[ctl-core-tests-style-ansi]: https://github.com/victor-software-house/ctl-core/blob/7dc8060e2578af605f4ae858947693cb52477d6f/tests/style_ansi.rs
[ctl-core-app-300-340]: https://github.com/victor-software-house/ctl-core/blob/7dc8060e2578af605f4ae858947693cb52477d6f/src/app.rs#L300-L340
