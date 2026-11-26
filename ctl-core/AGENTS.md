# ctl-core

The shared clap chassis and presentation kernel for the `*ctl` CLIs.
**Not a command.** Import as `ctl_core`.
This guide covers this crate only; the repository's root [`AGENTS.md`](../AGENTS.md) holds the workspace rules, and they apply here too.

## What it owns

- `-h` / `--help` and `-V` / `--version` (never `disable_help_flag`)
- Shared flags: `-c`/`--color`, `-f`/`--format`, `--json` (`--format json`), `-n`/`--dry-run`/`--preview`, `-q`/`--quiet`, `--stream`, plus `--foo` / `--no-foo` negation helpers
  `--json` beside another `--format` on the command line is a usage error.
  `TOY_JSON` beside a `TOY_FORMAT` that names another format is a usage error too, and the command line wins over any variable.
- The open output interface: `format::Format`, `Output`, `Sink`, and `Formats` with feature `view`, and `FormatName` (the value `--format` parses into).
  `pretty`, `json`, `tsv`, `csv`, and `markdown` are built-in `Format` implementations.
  `pretty` is the default on a terminal and `tsv` is the default when stdout is not one.
  A CLI that keeps `pretty` when piped sets `Formats::builtin().piped_default(FormatName::PRETTY)`.
  A CLI registers its own on `Formats` and installs them with `App::formats`
- Environment binding: `#[command(env_prefix = "VERCTL")]` on a CLI's root parser binds every long option, shared flags included, to `<PREFIX>_<LONG>` through the `EnvPrefix` trait in the prelude.
  - A flag on the command line wins over its variable, the variable over a config file, and the file over the default.
  - `App` runs `flags::env_under_flags` before the parse.
    It unbinds the variable of an option that the command line set, negated, or conflicts with.
    It also unbinds a variable that is empty or reads as a boolean flag's default.
  - A CLI test that parses its own command calls it too.
  - A caller that holds settings in the environment, such as an action, sets them and runs the bare verb.
    It never rebuilds argv in a shell.
- `App`: pre-parse hooks, Usage, help, parsing, execution, streams, errors, and exit codes.
  When the reader closes stdout early, it keeps the command's own exit code and never reports success for a failure (`App::broken_pipe`).
- `Document` and the fluent semantic primitives used by help and command output
- `View` over one serializable domain model, written through the `Format` it holds: pretty, colorless, JSON, or a format the CLI registered
- The one `Theme`, and the width, framing, and colour settings a CLI hands the engine (`RenderOptions`)
- Wire envelope: `Envelope` / `ErrorBody` / `SCHEMA_VERSION`, written to stdout on success and to stderr on failure.
  `ErrorBody` keeps a field per part of a failure: `message`, `causes`, `problems`, `usage`, and `parse`
- `CommandError`: a handler failure that carries its exit code and the envelope's `kind`, which `App` finds anywhere in the `anyhow` chain
- Mise Usage spec (`usage` feature): `--usage-spec[=BIN]`, `mount_directives`, and `App::usage_spec` enrichment, so consumers run `mise run q status` with no `--`.
  - `mount_directives` are `raw`, `raw_args`, and `mount_line`, the block every served CLI-proxy task carries, so piped output stays byte for byte what the CLI wrote.
  - Completion is its own `completion` feature, below.
- Clap-derived `Surface` (`surface-model` feature): binary and mounted names, recursive commands, aliases, visibility, arguments, flags, Usage KDL, and optional audience notes.
  `surface-serde` adds serialization without a template engine; `surface-templates` adds shared MiniJinja fragments; `surface` remains the compatibility aggregate enabling all three.
- Declared input (`validate`, `input-yaml`, `input-json`, `input-toml`, `input` features): shared garde rules, and one loader for YAML, JSON or TOML.
  The loader lives in the `ctl-input` crate, which ctl-core re-exports as `input`.
  It validates once with `validate`, and places every problem on its file and line.
  - A public `FieldPath` names a field by its steps, and `Input::place` and `Input::problem_at` place it, for garde's problems and for a CLI's own.
  - `InputError::try_new(name, problems)`, `None` for an empty list, and `InputError::whole(name, message)` build a CLI's own refusal; the I/O failure stays private, so only ctl-input builds an unread file's error.
  - `Input::top_level` lists the root's entries with each value's byte range, style and string, from the tree `place` keeps. The format's own scalar rules say whether a value is a string. It reads the tree as a listing, so a repeated key, a merge key and a `.inf` or `.NaN` are entries where `parse` refuses them. A key that is a number, a boolean or a null is listed by its text as written.
    `FieldPath::from(&garde::Path)` reads garde's path parts and never its printed string, so a key that holds a `.` is one step.
  - garde is pinned exactly in the root `Cargo.toml`, because that conversion calls the doc-hidden `Path::__iter`.
    A bump is a reviewed change, and `a_garde_path_converts_from_its_parts` fails first if the iteration order moves.
  - An `Input` of any format reads its text into the located tree once and keeps it, with the limits and the reading it was read under. `parse`, `place`, `place_key`, `check` and `top_level` share that tree. A strict tree serves every one of them. A listing tree, which only `place`, `place_key` and `top_level` read, is read only when no strict tree is kept and never serves `parse`. A change of limits reads the text again.
  - `Input::place_key` places the key of the last step where `place` places its value, for a fault about the key itself. An item and the root give their value.
  - `Input::flattened` declares a schema field that `#[serde(flatten)]` puts under its parent. `check` drops its step from each garde path before placing and printing, because serde gives no sign of the flattening at run time.
  - `input::Tagged` is ctl-input's own `Tagged<T>`, so a field keeps its YAML tag beside its value, such as `!override`. A tag inside a value serde buffers, such as an untagged enum or a flattened field, is not passed on.
- Served files (`served` feature): render a template that declares its own target through `{%- set -%}` exports, check the declarations once, and write the file with its mode.
- Shell completion (`completion` feature): the `completion <SHELL>` verb as a flattenable `CompletionCommand`, and the registration script for bash, zsh, and fish.
  - `App::completion` answers `COMPLETE=<shell>` before parsing; the feature enables `app` for it.
  - `Shell` lives in its own `shell` feature, which later shell verbs share.
  - `clap_complete` is pinned exactly because its dynamic engine is unstable.
    A bump is a reviewed change that reruns the script tests.
  - The zsh script carries three lines so it completes on the first TAB from `fpath` ([research](../docs/research/shell-completion.md), section 6).
    Drop them when a pinned `clap_complete` writes its own.
  - `Script` and its `shell::view` live in `shell` behind `view`, shared by both shell verbs.
- Shell integration (`shell-init` feature): the `init <SHELL>` verb as a flattenable `InitCommand`, and the bash, zsh, and fish script a `Hook` writes.
  - The hook runs `<bin> <subcommand>` once, before the first prompt, when its variable is non-empty and the binary is on `PATH`, then removes itself.
  - ctl-core writes all shell syntax.
    `Hook::new` refuses any part that is not a plain word, and the script calls the binary by name, never by path ([research](../docs/research/shell-init.md)).
- Fixture trees (`fixture` feature): a temporary directory per test suite and per scenario, and one call per fixture file, for consumers' integration tests.

Domain verbs and result types stay in each CLI.
Domain handlers return data and never print, inspect the terminal, choose a view, or construct engine tables.
The engine is this workspace's `ctl-text`, `ctl-layout`, and `ctl-term`, re-exported as `ctl_core::{text, layout, term}`.
A consumer composes `Document` blocks; no engine widget type appears in `Document`'s API, and `Theme` is the one place a role becomes a style.

Boolean pairs that are domain-specific (`--pr` / `--no-pr`) stay in the CLI.
Use clap `overrides_with` both ways and `warn_opposites` so the last flag wins without silence.

## What it must not own

- Domain verbs, domain result types, and domain-specific boolean pairs.
  They stay in each CLI.
- Text measurement, wrapping, and the unsafe-character rule: `ctl-text`.
- Widget layout, such as how a table chooses column widths: `ctl-layout`.
- Terminal detection, colour depth, and escape bytes: `ctl-term`.
  ctl-core chooses a role's style through `Theme` and hands lines to `ctl-term`.
- A terminal crate the engine replaced.
  `check:tree` fails the `app` tier when comfy-table, crossterm, anstream, anstyle-parse, or console returns.

## Public API

- A public item is a promise to every consumer.
  Keep a helper `pub(crate)` until a consumer needs it.
- Every public item has a doc that adds something beyond its name, and every fallible public function documents `# Errors`.
- No engine widget type appears in `Document`'s API; a consumer reaches the engine crates through `ctl_core::{text, layout, term}`.
- A setter on a builder type takes the name of its setting (`RenderOptions::width`, `View::quiet`, `Theme::style`), never a `with_` prefix.
  A reader of the same setting gets another name, such as `Theme::role_style`, or no method at all.
  A method that reads the environment starts with `resolved_`, so a caller sees which methods are not pure.
- A public enum that can gain a variant is `#[non_exhaustive]`, as `Block` and `NoticeLevel` are.
  The set of output formats is open through the `Format` trait, so no enum names them; `FormatName` is a value.
  An enum whose set is closed by what it means stays exhaustive, and its rustdoc says why in one sentence, as `Shell` and `Outcome` do.
- A public struct whose fields can grow is `#[non_exhaustive]`, and ctl-core builds it through a constructor.
  This covers the envelope types, `FlagWarning`, the `Surface` types, both error types, and every clap `Args` struct a CLI flattens.
- A field whose meaning depends on another field stays private behind one reader.
  `OutputArgs::color()` and `ColorLong::color()` combine `--color` with `--no-color`; `View::format()` and `View::is_quiet()` read the view.
- An error caused by an I/O failure keeps its `io::ErrorKind`, read through `io_failure()`, and stays `Clone` and `Eq` ([design](../openspec/changes/archive/2026-11-09-lock-public-api/design.md#1-an-error-keeps-the-ioerrorkind-not-the-ioerror)).
- A trait that only ctl-core implements is sealed by a private supertrait, as `EnvPrefix` is.
- A break needs a `minor` changeset at the repository root while the workspace is below 1.0.
  Inside a release-candidate train, where the minor already moved, mark a break `patch!` instead.
  The Version PR consumes every fragment, and verctl's `semver` then skips a crate only when the candidate's CHANGELOG section holds the Breaking heading that `patch!` writes.
  CI runs `mise run semver` against the last release before packaging.
  The crate-docs rules under [Cargo features](#cargo-features) keep traits visible to that check.
- A change to what a user sees moves each CLI's skill and instructions in the pull request that adopts the release.
  Examples are help text, a variable name, or an error's wording.

## Tests

| Path | Covers | Accept a change with |
|:--|:--|:--|
| `tests/snapshots/` | Rendered help, scripts, and documents, as insta file snapshots | `cargo insta review` |
| `tests/parity/` | The consumer corpus against the 0.6.x baselines | `PARITY_UPDATE=1 cargo test --locked -p ctl-core --test parity` |
| `tests/support/` | Shared helpers: shell checks and pseudo-terminals | (none) |
| `examples/size.rs` | The toy CLI that `check:size` measures | a budget change in the root `Cargo.toml` |

Read the diff of every changed snapshot or baseline before accepting it.
The sections [Snapshots](#snapshots) and [Parity corpus](#parity-corpus) hold the details.

## Decisions that bind it

These sections of [`design.md`](../openspec/changes/archive/2026-11-10-terminal-render-engine/design.md) state the reasons; change the code and the decision together.

- [12.
  Roles map to styles through a public theme](../openspec/changes/archive/2026-11-10-terminal-render-engine/design.md#12-roles-map-to-styles-through-a-public-theme)
- [13.
  A closed pipe ends the program quietly](../openspec/changes/archive/2026-11-10-terminal-render-engine/design.md#13-a-closed-pipe-ends-the-program-quietly)
- [15.
  Public API](../openspec/changes/archive/2026-11-10-terminal-render-engine/design.md#15-public-api)
- [16.
  Feature tiers and their measured sizes](../openspec/changes/archive/2026-11-10-terminal-render-engine/design.md#16-feature-tiers-and-their-measured-sizes)
- [17.
  Tests](../openspec/changes/archive/2026-11-10-terminal-render-engine/design.md#17-tests)
- [18.
  The rule against writing these from scratch is replaced](../openspec/changes/archive/2026-11-10-terminal-render-engine/design.md#18-the-rule-against-writing-these-from-scratch-is-replaced)

## Records and --stream

A command returns a value or `view::Records`, a header record then records, each `Present` and serializing with a string `type` field (a debug build panics on one without).
`view::Reply` is a value or records, and the sealed `view::Respond` lets `App::run` take either.
`View::show_records` writes records without `App` and returns the iterator's error, or the `io::Error` of a write to stdout, which a caller tells apart by downcasting.
Only `Records::new` and `Reply::value` are generic: each record is erased at once, and `View::run_records` writes records collected or streamed in two separate functions.
A value under `--stream` writes through the format's `RecordWriter` without that driver, so a CLI that returns only values does not link the collecting path.

- Without `--stream`, records are collected and written as one result: `json` writes one array, and adjacent one-table documents with equal headers and the same column count merge into one table.
- With `--stream`, `Format::stream` returns the `RecordWriter` that writes each record as the iterator yields it.
  The writer writes into a buffer the driver owns; the driver then locks stdout, writes the record whole, flushes, and unlocks.
  No lock is held while the iterator, `present`, or a serialization runs, and a record that fails to write reaches stdout in no part.
  The default writer calls `write` and writes nothing between records.
  `json` writes each record compact on one line, whatever the layout.
- `--stream` alone uses `Formats::stream_default`, `json` unless the CLI changes it; one that is not registered is a usage error.
- A value under `--stream` is a stream of one record with no `type` added, and keeps its own stream and exit code when it failed.
- Every record goes to stdout, and its outcome and exit code are ignored; the run exits 0 unless the iterator yields an error.
  An error stops the iterator, closes the output, then goes to stderr as a handler error does.
  A closed stdout stops the pull, drops the iterator, and exits through `App::written`, unless the iterator already failed: then that failure goes to stderr with its exit code, and the closed stdout never hides it.
  Any other write error, in either path, goes to stderr as a failure with exit 1, unless the iterator already failed: its failure and exit code win.
- `--quiet` with no format named drains the iterator and writes nothing; a named machine format is written.
- There is no `jsonl` format: JSON Lines is the stream form of `json`.

A CLI that calls `Surface::require_shorts` adds `--stream` to its allow list: the flag has no short.

## Cargo features

Features are tree-shaking.
`document` carries no terminal engine.
`render` adds the engine and the `ctl-term` encoder, with no window query.
`view` adds JSON, `help` adds Clap help, and each turns on `ctl-term`'s `detect` feature.
`app` composes the lifecycle.

`surface-model` adds only the Clap and Usage metadata that the rendered documents read.
`surface-serde` adds ctl-core's direct Serde edge without a template engine.
`surface-templates` adds MiniJinja, and `surface` preserves the full legacy aggregate.
`surface-model` pulls in no Serde: ctl-core writes the Usage KDL itself.

Prefer the narrowest feature set a consumer needs.
Never add serialization or rendering behaviour to a lower tier.

`features = ["usage"]` does not pull `help`.
Do not add `help` only to make the prelude compile.

ctl-core asks a dependency only for the features its own code calls.
Cargo unifies features, so a consumer cannot turn off one ctl-core turned on.

`clap` is declared with `default-features = false`: `cli` asks for `std`, `derive`, `env`, and `string`, and `help` adds `usage`.
Clap's `color`, `help`, `error-context`, and `suggestions` are the consumer's choice.
ctl-core's own `error-context` feature is how a CLI makes that choice for the JSON error: it turns on Clap's `error-context` and fills `ErrorBody::parse` with one field per kind of Clap's error context: the argument, the missing arguments, the value, the valid values, the subcommands, the suggestions by kind, the conflicting arguments, the value counts, and the tips.
ctl-core reads every `ContextKind` but `Usage`, which ctl-core renders itself, and a unit test in `app/error_body.rs` lists each kind of Clap 4.6.6 with its field.
Without it `parse` holds the reason alone, and `usage` is there either way.
No other feature turns it on, and `mise run test:error-context-off` tests the build without it.
Without Clap's `help` feature, Clap turns off `-h`/`--help` and the `help` subcommand, so `parser::apply_defaults` and `help::document` declare both (`parser::with_help`), worded as Clap words them.
ctl-core renders every help Clap stops for and never calls `Error::print` or `Command::color`.

Measure a new dependency or feature with `cargo bloat` on a release build before adding it (`openspec/changes/archive/2026-11-08-narrow-clap-features/design.md`).

The crate-root `//!` sections in `ctl-core/src/lib.rs` are the docs.rs landing page, and `document_features` there is authoritative for the feature graph.
The README is the GitHub page and does not copy them.
Crate-root re-exports stay `#[doc(hidden)]`, so the page lists modules once; the prelude documents the flat surface.

Traits are the exception, with every type a required trait method takes or returns.
cargo-semver-checks 0.51.0 treats a trait as public-API-sealed (`trait_newly_sealed`) when any `#[doc(hidden)]` path reaches it or such a type.
This holds even when another path is public.
So `Present`, `EnvPrefix`, and `Document` (returned by `Present::present`) stay visible at the root.

Each module's summary lives in its own `//!`, never also on `pub mod`.
Every fallible public function documents `# Errors`, with no `missing_errors_doc` expect.

## Machine output is never coloured

A machine format, which is every format whose `Format::machine` is true, stays plain whatever the environment says: `Sink` takes its colour away, so no format can write an escape sequence by mistake.
Agent shells set `CLICOLOR_FORCE=1`, `FORCE_COLOR=3`, and `COLORTERM=truecolor`; pretty output may honour them, and a machine format never does.

A parse error under a long `--format <name>`, or the shorthand of a registered format such as `--json`, is that format's error form even when the bad argument comes first.
A parse error reads `<bin>: <message>`, without Clap's `error:` prefix.
A consumer test runs the binary with `CLICOLOR_FORCE=1` and asserts that machine output holds no ESC byte.

## Library control

ctl-core supplies sane defaults, never hidden policy.
Every automatic behavior that can change rendered output has a public library override and an explicit disable path.
Environment-backed defaults expose their canonical name and let a consumer replace the ordered name list with its own aliases or disable lookup.
An explicit render width is exact and bypasses automatic-width buffering.

The automatic terminal-width buffer defaults to two columns and reads `CTL_CORE_COLUMN_BUFFER`; the automatic minimum defaults to 20.
When no width is detected, output lays out to a fallback width of 80, and `fallback_width(None)` disables it.
`RenderOptions`, `View`, and `App` expose the buffer, ordered environment names, minimum, and fallback.
`View` returns its settings as a `RenderOptions` from `render_options`.

The buffer, its names, and the minimum exist only with `view` or `help`, since nothing reads them without a stream.
The defaults are `ctl-term`'s constants, not second copies.

Buffer zero disables subtraction; an empty environment slice disables lookup.
Explicit widths remain exact.
Public option-aware help entry points keep direct help callers under the same policy.

The record style, list frame, row separation, column floor, table fit, and JSON layout read `CTL_CORE_RECORD_STYLE`, `CTL_CORE_LIST_STYLE`, `CTL_CORE_ROW_SEPARATION`, `CTL_CORE_COLUMN_FLOOR`, `CTL_CORE_TABLE_FIT`, and `CTL_CORE_JSON_LAYOUT` when the consumer has not set them.
Each name list is replaceable, and an empty slice disables lookup.

The column floor is 8 cells.
No column of a table narrows below the larger of its header width and the floor, and the floor is never more than the column's natural width.
`RenderOptions::column_floor(0)` or `CTL_CORE_COLUMN_FLOOR=0` turns it off.

A table whose floors and frame do not fit the width stacks each row as a record.
`Table::stacking(TableFit::Wrap)`, `RenderOptions::table_fit(TableFit::Wrap)`, or `CTL_CORE_TABLE_FIT=wrap` keeps the grid at its floors instead.
A table with `stacked_below` has no floor and keeps its own form.

## View contract

1. **Models first.** Each command returns a serializable result; the view does not own domain data.
2. **One model, every mode.** `Present` maps that model to a semantic `Document`.
   JSON serializes the model directly, and every format reads it through `Output`: `data()` is the serializable face, `document()` the document, built on first use.
   Colorless renders the same document.
3. **Semantic composition only.** Consumers use `Fields`, `Table`, `Section`, `Notice`, `Text`, and `Document::verbatim`; they never choose borders, ANSI, width, or streams.
   - Verbatim is for preformatted Markdown or protocol text whose line boundaries must survive rendering.
   - It removes what `unsafe_chars::is_unsafe` refuses (every Cc control other than newline and tab, the 12 UAX #9 directional formatting characters, and U+2028 and U+2029).
     It preserves the other format characters (Cf).
     Then it normalizes trailing newlines to the document.
   - A CLI that prints text it did not write filters with the same predicate.
   - The predicate and `strip_unsafe` live in `ctl-text` (`ctl_text::unsafe_chars`), and `ctl_core::unsafe_chars` re-exports them under the `unsafe-chars` feature.
4. **Quiet hides human success and defaults only.** `--quiet` hides a successful result in `pretty` and in a default format. It never hides errors, and it never hides a machine format that a flag, a shorthand, or a variable named.
5. **A JSON error keeps a field per part.** `message` is one sentence.
   A handler error's further causes are `causes`, an input's problems are `problems`, a parse error's usage line is `usage`, and Clap's parts of it are `parse`.
   Each is left out when empty, and none contains ANSI.
   `pretty` writes `ErrorBody::text`, which `App` sets to what a failure printed before it had fields, so pretty output is unchanged; JSON never carries it.
6. **Help is a document.** Clap remains the grammar, and help uses the same renderer as command output.
   Usage lines wrap without splitting tokens.
   `-h` shows `about`, each argument's `help`, and `after_help` as an Examples section; `--help` and the `help` subcommand show `long_about`, `long_help`, and `after_long_help`, each falling back to its short form.
   A command's visible aliases follow its name.
   Clap has no heading for a subcommand, so `App::command_group` names the root's groups, and the root's other commands follow under Commands.

See [`docs/presentation.md`](../docs/presentation.md).

## Operator documents

Consumer skills and installed instructions are committed rendered artifacts.
Their source templates import `ctl/version.md.jinja`, `ctl/invocation.md.jinja`, or `ctl/commands.md.jinja` and retain domain prose.
Tests build `Surface` from the consumer's Clap type and byte-compare the render with the committed file.
Do not restore token scans, copied command lists, or a second no-`--` rule.

## Consumers

Migrate one consumer at a time by pinning a released ctl-core version, never a path dependency.
Verctl goes first, then forkctl retains its protocol while deleting its local view/help/layout, then qctl turns direct printing into typed results.
The migration is tracked as `CTC-010` here and `QCTL-008` in qctl.

## Declared input

The code lives in [`ctl-input`](../ctl-input/AGENTS.md), and `ctl_core::input` is that crate re-exported, so every path under it is unchanged.
ctl-core's `input-yaml`, `input-json`, `input-toml`, and `validate` turn on `ctl-input`'s `yaml`, `json`, `toml`, and `validate`.
Each format is its own feature, and validation is a third:

| Feature | Adds | Links |
|:--|:--|:--|
| `input-yaml` | YAML, with strict booleans and duplicate and merge keys refused | serde-saphyr |
| `input-json` | JSON by the strict RFC 8259 grammar, read by ctl-input's own reader into the located tree; a strict reading refuses a duplicate key, and `input` does not turn it on | none |
| `input-toml` | TOML, parsed into the span-carrying `DeTable` that the located tree is built from | toml |
| `validate` | The shared garde rules, and with a format, `Input::check` and `Input::load` | garde |
| `input` | `input-yaml` with `validate`, the set every current consumer enables | serde-saphyr, garde |

`check:tree` fails when `input-toml` alone links serde-saphyr or garde.
`input-json` links no parser crate, and a CLI that reads JSON names it beside `input`.

A message carries no position of its own, except that a problem in a node an alias replayed names where its value is defined, counted from the file.
`Messages` words every problem once, whatever the format, and `set_messages` replaces the wording for the process.
`Input::frontmatter` splits YAML frontmatter from a Markdown body.

`Limits` is ctl-core's own type, overridable field by field, in this order:

1. the call (`Input::limits`);
2. the repository (`limits:` in `.ctl/input.yaml`, installed by `use_repo_limits(root)` at startup), with `input-yaml` only;
3. the process (`set_default_limits` or `App::input_limits`);
4. ctl-core's default.

Three fields hold every format: `max_reader_input_bytes`, `max_depth`, and `max_nodes`.
Their defaults are the `Limits::DEFAULT_*` constants, which a unit test holds to serde-saphyr's.
The other fields name YAML syntax, exist only with `input-yaml`, and default to serde-saphyr's.
The field set follows the enabled features, so a caller builds `Limits` with `..Limits::NONE`.

ctl-core enforces depth and nodes on a JSON text as its reader goes, when an array or object opens and when a value is added.
It enforces them on a TOML text itself, because the `toml` crate takes no depth or node limit from a caller.
Its parser bounds nesting at a fixed 80 levels, which no limit raises.

`enforce_alias_anchor_ratio` is not overridable and stays on.
`use_repo_limits` returns each limit the repository raises above the default; the CLI prints each as a warning.
Without `input-yaml`, a limit message ends "the tool sets this limit".

A CLI parses config only through the input features; it keeps no `schema.rs` and no YAML crate of its own.
`CTC-008` tracks the consumer moves and the template renderer.

The format is chosen, never guessed, and it is a value: `input::Format` is a trait, `input::YAML`, `input::JSON` and `input::TOML` are the built-in formats, and a CLI adds its own to a `Formats` registry.
`Input::open(path)` takes the format from the file's extension (`.toml`, `.json`, `.yaml`, `.yml`, any case) through the process default, which `set_default_formats` or `App::input_formats` installs, else `Formats::builtin()`.
`Formats::open(path)` does the same through a registry of the caller's.
Both refuse every extension no registered format reads before reading, and name the feature a built-in extension needs when its feature is off.

`Input::new` and `Input::read` need `input-yaml` and stay YAML whatever the name is.
`Input::new_with_format(name, text, format)` or `Input::format(format)` names the format for text in memory, with `&input::TOML` or a format of the CLI's own.
The repository limits file stays `.ctl/input.yaml` for a TOML-config repository.
`openspec/specs/declared-input` holds the contract, and `CTC-021` and `CTC-025` the rows.

## Served files

`served::render(template, source, suffix, context)` renders one template through `surface::environment()` and reads `path`, `name`, and `executable` from its top-level exports; `Served::write(root)` writes it, sets 0755 or 0644 on Unix, and refuses a symlinked target.
Which templates a repository serves and what context they see stay with the CLI. verctl moves onto it inside its redesign.

## Fixtures

Enable `fixture` on the ctl-core dev-dependency.
`Suite::new(name)` owns a temporary directory for a test file, and `suite.scenario(name)` owns one inside it; both are removed on drop.
`root.file(path, body)` writes a fixture and its parent directories and returns the path; a path outside the root panics.
A CLI keeps no fixture wrapper of its own.
`CTC-006` tracks the consumer moves.

## Snapshots

Rendered output a reviewer should read, such as a shell script or a help page, is an insta file snapshot under `ctl-core/tests/snapshots/`.
It is never a string literal or a `contains` check: a change to the output shows as a diff of the snapshot.

`insta` is pinned exactly; a bump is a reviewed change.
CI sets `INSTA_UPDATE=no` in `mise.ci.toml`, so it never writes a snapshot.
Review a change locally with `cargo insta review` (`cargo-insta` is in `mise.dev.toml`).
A test that writes a temporary file uses `fixture`, not its own temporary directory.

A rendered shell script also passes its shell's check through `ctl-core/tests/support/shells.rs`: `bash -n` and `shellcheck --enable=all`, `zsh -n`, `fish -n`.

- zsh, fish, and shellcheck are pinned in `mise.test.toml`, the `test` profile, and locked in `mise.test.lock`.
- `.miserc.toml` loads it locally after `dev`.
  CI loads it only in the Build and test job (`MISE_ENV=ci,test`, through the shared workflow's `test-profiles` input), so no other job fetches them.
- Their install directories are on the tests' `PATH`, which mr-boxington's command wrapper keeps; only mise's shims are taken off it.
- Generate the locks with the default profiles, `mise lock --platform linux-x64,macos-arm64`.
  mise also writes the pinned catalogs' task tools into `mise.test.lock`.
  The pre-push lock check runs with `MISE_ENV=dev,test`.
- Write a new lock with the pinned mise, 2026.10.25.
  A newer mise writes a new file as lockfile version 4.
  CI's mise cannot read that version.
- A missing tool fails the test.
- A script ctl-core writes passes shellcheck with no exclusions.
  Only a third-party template's lines may be excluded, by code, with the reason beside the list.

A script that changes what a shell does at run time also runs in that shell.
`ctl-core/tests/shell_init.rs` starts bash, zsh, and fish on a pseudo-terminal through `pty-process` (`ctl-core/tests/support/terminal.rs`), types each line only after the prompt appears, and reads what a fake binary logged.
A pipe is not enough: fish draws no prompt when it reads one.

## Parity corpus

`ctl-core/tests/parity/` renders a fixed corpus of the documents that verctl, qctl, forkctl, and rawr produce.

- `corpus.rs` builds each case with the public API, as a consumer does.
- `main.rs` renders every case through `Document::render` at widths 20, 40, 80, and 120, with `ColorMode::Never` and `ColorMode::Always`, and compares the output byte for byte with `ctl-core/tests/parity/baseline/<case>-w<width>-<plain|color>.txt`.
- A baseline writes each ESC byte as the six characters `\u{1b}`.
- A missing or different baseline fails the test with a line diff.
- `PARITY_UPDATE=1 cargo test --locked -p ctl-core --test parity` writes the missing and changed files.
- The baselines are plain files, not insta snapshots, because the 0.6.x files stay frozen once committed.
  The render-engine change adds the new engine's outputs beside them and lists each difference in its pull request.

Add a case to `CASES` with its baselines in the same commit.
