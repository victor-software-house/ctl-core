# Changelog

## ctl-core 0.8.0-rc.2

### Breaking

- `Input` reads every text into the located tree once, through its format, and keeps the tree.
  `parse`, `place`, `place_key`, `check` and `top_level` all work on it, for YAML, TOML and any format a CLI adds.
  A strict tree serves every one of them, and a listing tree, which `top_level` and placing read, serves only placing and listing.
  yamled leaves ctl-core's graph: `input-yaml` links serde-saphyr only, and `input-toml` links the `toml` crate only ([#336](https://github.com/victor-software-house/ctl-core/pull/336)).

  `App::input_formats(Formats)` sets the formats `Input::open` reads, as `App::input_limits` sets the limits.
  `Formats::open(path)` reads a file through a registry of the caller's, whatever the process default is.

  Breaking changes:

  1. `input::Format` is the format trait, and the enum is gone. `Format::Toml` becomes `&input::TOML`, and `Format::Yaml` becomes `&input::YAML`. A `match` on the enum has no replacement: ask the format for its `name()`.
  2. `Format::of(path)` becomes `Formats::of(path)`, which returns the format as a `&'static dyn Format`. `Input::open` takes it from the process registry that `set_default_formats` or `App::input_formats` installs, else `Formats::builtin()`.
  3. `Input::new_with_format` and `Input::format` take a `&'static dyn Format`, not the enum.
  4. `input::Tagged` is ctl-input's own type, not serde-saphyr's, with the same spelling: `!override` for a local tag. A tag inside a value serde buffers, such as an untagged enum or a flattened field, is not passed on.
  5. `Input` is compared by its format's name, and its `Debug` shows that name and no longer shows a parsed document.
  6. A problem is worded by `Messages` whatever the format, so each problem type has one wording. Each changed one reads:
     - a repeated key: `duplicate mapping key: prefix, set DuplicateKeyPolicy in Options if acceptable` becomes `duplicate key \`prefix\``;
     - a merge key: `YAML merge keys are not allowed by configured policy` becomes `merge key \`<<\` is not allowed`;
     - a YAML 1.1 boolean: `invalid boolean (strict mode expects true/false)` becomes `expected true or false`;
     - a number field given a word: `invalid u16` in YAML, and `invalid type: string "…", expected u16` in TOML, become `expected an integer`;
     - an unknown field or variant: `unknown field \`x\`, expected one of name` becomes `unknown field \`x\`, expected one of \`name\``, with each name in backticks;
     - a problem in a node an alias replayed keeps `(defined at line L, column C)` after the new wording, and a limit a replay raised gains that suffix too;
     - a node budget breached inside a replay: `budget breached: Nodes { nodes: 10 }` becomes `exceeds max_nodes (9); raise limits.max_nodes in .ctl/input.yaml`;
     - a TOML repeated key: `duplicate key` becomes `duplicate key \`prefix\``, the same words as YAML, on the repeated key;
     - a YAML text with more than one document: `multiple YAML documents detected; use from_str_multiple or from_str_multiple_with_options` becomes `the text holds more than one YAML document`;
     - a refused non-finite float: `non-finite float \`.inf\` rejected by the configured policy` becomes `non-finite float \`.inf\` is not allowed`, with the scalar's own text in the backticks;
     - a text over `max_reader_input_bytes` is a `Fault::TooLarge`, worded by `Messages` like every other problem. Its default words are unchanged.
  7. Six positions move:
     - a field the root mapping lacks sits on line 1, column 1, where it sat on the last key of the root;
     - a field a nested mapping lacks sits on the key that holds the mapping, in YAML and TOML alike;
     - a YAML problem on line 1 of a file that starts with a byte-order mark sits one column further left, because the mark is no column;
     - a TOML text is read in the order it is written, so the first problem is the first in the file, where it was the first by key;
     - a problem in a node an alias replayed sits at the alias, and its words name where the value is defined;
     - a problem in a block scalar sits at its indicator, `|` or `>`.
  8. Reads that change:
     - an ignored non-finite float, such as `.inf`, is no longer refused: a read that asks for its value still refuses it;
     - a TOML datetime reads as its text, so it fills a `String` and a typeless read gives the text;
     - a `toml::value::Datetime` field no longer reads: read it as a `String`;
     - TOML placing past `max_depth` or `max_nodes` gives no place, as YAML placing does;
     - each control character in a problem's words, such as one in a key or a value the text wrote, is escaped as Rust's `escape_debug` does, in every format.

  Migration: replace the enum with the built-in format values, call `Formats::builtin().of(path)` where `Format::of(path)` was called, and move each pinned message to the wording above.
  A CLI with its own format implements `input::Format` and installs it with `Formats::builtin().add(&FORMAT)`, through `App::input_formats`, `set_default_formats`, or `Formats::open`.
- `ctl_core::input` gains the located tree, the input format trait and its registry, beside the code that reads input today.
  `input::tree` holds `Node`, `NodeKind`, `Scalar`, `ScalarStyle`, `ScalarSyntax` and `Tag`: each node keeps its byte span, each scalar its processed value, its style and, for TOML, the type its syntax gave it, and each mapping its key and value nodes in source order.
  `input::format` holds the `Format` trait with its `read` and `resolve`, `Reading`, `Wanted`, `Resolved`, `Mismatch`, `Refusal` and `Fault`, and the two built-in formats `YAML` and `TOML`.
  `input::Formats` is the registry of formats, found by extension through `Formats::of`, and `input::set_default_formats` sets the process default ([#333](https://github.com/victor-software-house/ctl-core/pull/333)).

  `Limits` gains three YAML-only fields, `max_total_replayed_events`, `max_replay_stack_depth` and `max_alias_expansions_per_anchor`, which set serde-saphyr's alias replay limits through the same tiers as the other limits, `.ctl/input.yaml` included.
  `use_repo_limits` reports one a repository raises, and a text that goes past one fails with a problem naming the field and where to raise it.
  A `Limits` literal that lists every field, without `..Limits::NONE`, no longer compiles; `Limits` documents building it with `..Limits::NONE`, which keeps compiling.

### Changed

- Declared input now lives in the new `ctl-input` crate, which `ctl_core::input` re-exports.
  The `input-yaml`, `input-toml`, `validate`, and `input` features and every path under `ctl_core::input` are unchanged, so no code that uses them changes ([#329](https://github.com/victor-software-house/ctl-core/pull/329)).
- Add a serde Deserializer over the located tree of `ctl-input`, the full set of problem types, `Messages` to word them, and `Tagged`.
  `Input` does not read through them yet.
  One existing behaviour changes: a column no longer counts a leading byte-order mark, so a problem on line 1 of a TOML file that starts with one sits one column further left ([#334](https://github.com/victor-software-house/ctl-core/pull/334)).
- Every command takes `--input <FILE>` and `--input-format <NAME>` with the new `input-document` feature and an input format (`input-yaml`, `input-json`, `input-toml` or `input`).
  `input-document` turns on `app` and `error-context`, and fails to compile without an input format.
  They read `<PREFIX>_INPUT` and `<PREFIX>_INPUT_FORMAT`, with the prefix `--format` reads its variable with.
  A YAML, JSON or TOML document, or stdin for `--input -`, fills the arguments of the command the command line selects.
  Its keys are long names and positional ids, and each value passes the argument's own parser and rules ([#348](https://github.com/victor-software-house/ctl-core/pull/348)).

  The command line wins over the environment, the environment over the document, and the document over the default.
  A key that loses is dropped.
  `App::input_conflicts(InputConflicts::Fail)` refuses it instead, with a problem on its line.
  `InputConflicts` is new.

  `App::without_input` removes both flags.
  A CLI that declares `input` or `input-format` itself must call it, or `App` panics.

  Every problem in a document names its file, line and column.
  Under `--format json`, a refused document carries each problem in `problems`, and a Clap error placed on a key adds its `parse` parts and usage line.

  `ctl-input` adds `Input::tree`, which lends the strict read's root node to a closure, and `default_formats`, the registry `Input::open` reads through.
- `input-json` reads JSON through `Input::open`, with the built-in format `input::JSON` and the extension `.json`.
  `input` does not turn it on, so a CLI that reads JSON names `input-json` beside it.
  The reader is ctl-input's own and links no parser crate ([#340](https://github.com/victor-software-house/ctl-core/pull/340)).

  `ScalarSyntax::Null` joins the tree for `null`; the enum is non-exhaustive, so a `match` on it already has a wildcard arm.
  `Formats::builtin()` holds `yaml`, `json` and `toml`, in that order, as their features allow.

  A JSON text is read by RFC 8259 with no option that loosens it:

  1. A comment, a trailing comma, a single-quoted string, an unquoted key and a number such as `01`, `+1`, `.5` or `NaN` are refused.
  2. Whitespace is space, tab, line feed and carriage return only.
  3. A raw control character in a string must be escaped, and a lone or inverted surrogate escape is refused.
  4. The text holds one value: an empty text, or text after the value, is refused.
  5. A leading byte-order mark is skipped and counts as no column.
  6. A strict reading refuses a key written twice in one object, comparing keys after their escapes are decoded, and a listing keeps both entries.
  7. A string fills only a string field, a number only a number field, and `null` only an optional or unit field.
     A number keeps its text, so an integer too large for `u128` reads as a float for a float field or a typeless read, and an integer field refuses it.
     A float that overflows is refused.
  8. `max_reader_input_bytes`, `max_depth` and `max_nodes` apply as the text is read, and a file that is not UTF-8 is refused when it is opened.
- Output formats are an open interface.
  `format::Format` defines how a result becomes output, `format::FormatName` names one, and `format::Formats` is the set a CLI registers with `App::formats`.
  `pretty` and `json` are the built-in implementations, `format::PRETTY` and `format::JSON`, and their output is unchanged.
  A CLI adds a format, replaces a built-in of the same name, or removes one.
  It also sets the default for a terminal and for a pipe.
  The terminal default is `pretty` and the piped default is `tsv`, the break that the tabular formats entry describes.
  Output on a terminal does not change, except where the breaks below say so ([#316](https://github.com/victor-software-house/ctl-core/pull/316)).

  Breaking changes:

  1. `format::OutputFormat` and `format::ParseFormatError` are removed. `FormatName::PRETTY` and `FormatName::JSON` replace its variants, and `view.format() == FormatName::JSON` replaces `is_json()`.
  2. `View::new` takes a `&'static dyn Format`, such as `&format::JSON`, in place of an `OutputFormat`. `View::requested` takes an optional name that `View::resolve` turns into a format.
  3. `View::format()` returns a `FormatName`, and `View::is_machine()` says whether the format never writes ANSI. `--quiet` hides `pretty` and a default format, and never a machine format that a flag, a shorthand, or a variable named.
  4. The `format` field of `OutputArgs`, `FormatArgs`, and `FormatLong` is an `Option<FormatName>`, and is `None` when no flag or variable named a format. `--format` has no default in help.
  5. `shell::view` takes an `Option<FormatName>`. Every format but `json` prints the script as text, and quiet never hides it.
  6. `App::new` requests no format, so an `App` without `.view(...)` now honours `--format`, `--json`, and their variables.
  7. Two variables that name different formats, such as `TOY_FORMAT=csv` and `TOY_JSON=1`, are a usage error that names both. Before, `TOY_JSON` won.
  8. `--json` moves after `--quiet` in help and in the Usage spec, and `--format` no longer shows `[default: pretty]`.
  9. `FormatName` has no `FromStr`, `Default`, `Deserialize` or `JsonSchema`.
  10. `View::format()` is no longer `const`.
  11. The `view` feature depends on `erased-serde`. It appears in `Output::data`, and `format` re-exports it.

  Migration:

  1. Replace `OutputFormat::Json` with `FormatName::JSON`, and `format.is_json()` with `view.format() == FormatName::JSON` where the branch writes JSON. Use `view.is_machine()` only where a check guards ANSI or `--quiet`.
  2. Replace `View::new(OutputFormat::X, color)` with `View::new(&format::X, color)`.
  3. Compare the `format` field with `Some(FormatName::JSON)`, or call `view()`.
  4. Pass an `Option<FormatName>` to `shell::view`, which `cli.output.format` already is.
  5. Replace `View::new(cli.output.format, color)` with `View::requested(cli.output.format, color)`.
  6. Match the text of a name against `formats.names()` where `FormatName::from_str` or `Deserialize` read one.
  7. Register any format of the CLI's own through `App::formats`. Build a `Surface` or an operator document from `formats.augment(Cli::command())`, with the same `Formats` the `App` gets.
- A command can return records, and `--stream` writes each one when the command produces it.
  `view::Records` is a header record then records, each a `Present` value that serializes with a string `type` field.
  `view::Reply` is a value or records.
  `App::run` takes either through the sealed `view::Respond`, so a handler that returns a value compiles as before.
  `View::show_records` writes records for a consumer without `App`.
  `View::stream(bool)` sets the stream setting and `View::is_stream()` reads it.
  Under it, `View::show` writes a value through the format's stream form, as a stream of one record ([#321](https://github.com/victor-software-house/ctl-core/pull/321)).

  Without `--stream`, records are collected and written as one result, so `json` writes one array.
  With `--stream`, `json` writes each record compact on one line.
  `--stream` alone uses the stream default, `json`, which `Formats::stream_default` changes.
  `Format::stream` returns the `format::RecordWriter` that writes each record.
  Its default writes each record as `write` does, so existing formats compile unchanged.
  `tsv`, `csv`, `markdown`, and `pretty` write records as they arrive, with a table's header once and again when the table changes.
  The `view` feature now depends on `anyhow`, the error type of a stream of records.

  Breaking changes:

  1. `OutputArgs`, `FormatArgs`, and `FormatLong` gain a `stream` field, and `--stream` with the variable `<PREFIX>_STREAM`, which takes `true` or `false`.
     A CLI that declares its own `--stream` must rename it.
  2. `--stream` has no short flag, so a CLI that calls `Surface::require_shorts` adds `--stream` to its allow list.
  3. `View`, with its `PartialEq` and `Debug`, carries the stream setting.

  Migration: a CLI that writes its own rows to stdout returns `Records` instead and drops that code.
- The JSON error envelope keeps a field per part of a failure, so a caller no longer parses prose.
  `ErrorBody` gains `causes`, `problems`, `usage`, and `parse`, each left out of JSON when empty and described by the schema.
  `causes` lists each further error of an `anyhow` chain, outermost first.
  `problems` holds one `InputProblem` per problem of a refused input, with its `input`, `line`, `column`, `field`, and `message`.
  `usage` is the usage line of the selected command, without the `Usage:` label, on every parse error.
  `parse` is a `ParseFailure`: Clap's error kind in snake case as `reason`, and a field for each kind of Clap's error context: `argument`, `missing`, `value`, `valid_values`, `subcommand`, `valid_subcommands`, `suggested_arguments`, `suggested_values`, `suggested_subcommands`, `suggested_commands`, `conflicts_with`, `actual_values`, `expected_values`, `min_values`, `trailing_argument`, `tips`, and `notes`.
  The new `error-context` feature turns on Clap's, so `parse` holds those parts; without it, `parse` holds only `reason`.
  `ErrorBody::text` is the text a person reads, which `pretty` and every other non-JSON format write after `{bin}: `.
  `App` sets it to what each failure printed before it had fields, so a non-JSON error reads as it did; JSON and the schema leave it out ([#347](https://github.com/victor-software-house/ctl-core/pull/347)).

  Build an `ErrorBody` with `ErrorBody::new` and the builders `kind`, `causes`, `problems`, `usage`, `parse`, and `with_text`.
  `InputProblem` and `ParseFailure` are `#[non_exhaustive]` too, built with `InputProblem::new` and `ParseFailure::new` and their builders.

  `message` is one sentence for every failure, so a script that read it for a cause or a problem reads `causes` or `problems`:

  1. A handler error's `message` is its outermost error, and the rest of its chain is in `causes`, where it was joined with `: `.
  2. A refused input's `message` is `<input> has <n> problems`, and the lines it carried are in `problems`.
  3. A parse error's `message` is Clap's first line, so the tip, the usage, and the pointer to `--help` are no longer in it.
- No column of a table narrows below its floor, and a table that cannot fit at its floors stacks.
  A column's floor is its header width or 8 cells, whichever is larger, and never more than its natural width.
  A fold column, and a keep-words column in a grid, went down to its widest cluster before, and an ellipsis column to 1 cell.
  Each now stops at its floor.
  A table whose floors and frame do not fit the width draws each row as a record, with a blank line between rows.
  A record is the first cell as a title, then a header and value line for each other column.
  A table with no header draws each row as its first cell over the others, and a table with no rows draws its grid ([#324](https://github.com/victor-software-house/ctl-core/pull/324)).

  `RenderOptions::column_floor(cells)` sets the floor, and `CTL_CORE_COLUMN_FLOOR` sets it when the consumer leaves it open.
  `0` turns the floor off, so each column goes down to the limit its overflow sets.

  `RenderOptions::table_fit(TableFit)` sets the fit, and `Table::stacking(TableFit)` sets it for one table.
  `CTL_CORE_TABLE_FIT` (`stack` or `wrap`) sets it when neither does.
  `TableFit::Wrap` keeps the grid at its floors and lets it run past the width.
  `column_floor_envs` and `table_fit_envs` replace each ordered name list, and an empty slice disables lookup.

  A table with `Table::stacked_below`, such as a help table, has no floor and keeps its threshold and form, so help output does not change.

  Public additions:

  - ctl-core: `RenderOptions::column_floor`, `column_floor_envs`, `table_fit`, `table_fit_envs`, `resolved_column_floor`, and `resolved_table_fit`.
  - ctl-core: `DEFAULT_COLUMN_FLOOR_ENV`, `DEFAULT_COLUMN_FLOOR_ENVS`, `DEFAULT_TABLE_FIT_ENV`, and `DEFAULT_TABLE_FIT_ENVS`.
  - ctl-core: `TableFit`, at `ctl_core::TableFit`, `ctl_core::prelude::TableFit`, `ctl_core::document::TableFit`, and `ctl_core::render::TableFit`.
  - ctl-core: `Table::stacking`, and its getter `Table::fit`, on the document `Table`.
  - ctl-core: `View::styles` and `App::styles` carry the floor, the fit, and their name lists, as they carry the other styles.
  - ctl-layout: `Table::column_floor`, `Table::fits`, and `Fallback`, which draws a table when it fits and another widget when it does not.

  `RenderOptions` is larger, so a CLI function that takes it by value may meet clippy's pedantic `large_types_passed_by_value`.

  Breaking changes:

  1. Every table without `stacked_below` that cannot fit at its column limits now stacks, including unframed keep-words lists that used to run past the width.
  2. No column narrows below its floor, so ellipsis and fold columns stop there, and a grid folds no column below it.
  3. In ctl-layout, `Table`'s default column floor is 8, and its `measure` minimum rises with it.

  Migration: set both `column_floor(0)` and `table_fit(TableFit::Wrap)`, or `CTL_CORE_COLUMN_FLOOR=0` and `CTL_CORE_TABLE_FIT=wrap`, to restore the old output.
  A direct ctl-layout user calls `Table::column_floor(0)` and uses no `Fallback`.
- Piped output is now `tsv` for every CLI on ctl-core.
  When stdout is not a terminal and no flag or variable names a format, a CLI writes tab-separated lines.
  Before, it wrote the pretty table.
  `--quiet` hides that default output, as it hides `pretty` ([#318](https://github.com/victor-software-house/ctl-core/pull/318)).

  `tsv`, `csv`, and `markdown` are new built-in formats: `format::TSV`, `format::CSV`, and `format::MARKDOWN`.
  Each reads the result's document, so a script gets the columns a person sees.
  `Formats::builtin()` registers all five, and `--format` accepts their names.

  Migration:

  1. A test that reads piped output passes `--format pretty`, or reads `tsv`.
  2. A CLI that needs the old piped output sets the piped default to pretty: `Formats::builtin().piped_default(FormatName::PRETTY)`.

## ctl-core 0.8.0-rc.1

- Help renders more of what Clap declares.
  `after_help` shows as an Examples section on `-h` and `--help`, and `after_long_help` replaces it on `--help`.
  `--help` and the `help` subcommand show `long_about` and each argument's `long_help`, falling back to `about` and `help`.
  A command's visible aliases follow its name, as in `ls, list` ([#303](https://github.com/victor-software-house/ctl-core/pull/303)).

  `App::command_group(heading, commands)` lists some of the root's commands under a heading of their own, in the order given.
  Each call adds one group, and the root's other commands follow under Commands.
- A CLI can report its own refusals of an input in ctl-core's format.
  `InputError::whole(name, message)` refuses the whole input, on no line.
  `InputError::try_new(name, problems)` carries a list of `Problem`s, each placed with `Input::place` or on no line, and returns `None` for an empty list.
  Both print as ctl-core's own problems do, one per line as `name:line:column: field: message` ([#304](https://github.com/victor-software-house/ctl-core/pull/304)).
- Every JSON failure now goes to stderr, and stdout stays empty after a failed run.
  That covers `View::emit_err`, a parse error, a handler error, and a result whose outcome is an error.
  A caller that read the error envelope from stdout reads it from stderr instead ([#301](https://github.com/victor-software-house/ctl-core/pull/301)).

  `ErrorBody` gains an optional `kind`, which JSON leaves out when it is unset, and `ErrorBody::kind` sets it.
  A parse error's envelope sets `kind` to `usage`.

  A handler returns the new `CommandError` to give a failure its own exit code and `kind` value.
  `App` finds it anywhere in the `anyhow` chain, as the error or as context on its cause.
  A plain `anyhow` error still exits 1 with no `kind`.
- The `json` feature enables only `serde`, and `view` enables `serde_json` with it.
  Only `view` writes JSON, so `json` alone builds no `serde_json` ([#292](https://github.com/victor-software-house/ctl-core/pull/292)).
- A parse error now reads `<bin>: <message>`, without Clap's `error:` prefix, in pretty output and in the JSON envelope's `message` ([#302](https://github.com/victor-software-house/ctl-core/pull/302)).

  `OutputArgs`, `FormatArgs`, and `FormatLong` gain `--json`, a shorthand for `--format json` that sets their `format` field.
  Naming `--json` beside a `--format` other than `json` is a usage error.
  With `env_prefix`, `--json` reads `<PREFIX>_JSON`, either flag on the command line wins over the other's variable, and `<PREFIX>_JSON` wins over `<PREFIX>_FORMAT`.
  A CLI that calls `Surface::require_shorts` adds `--json` to its allow list, since `--json` has no short form.
  A parse error after `--json` is JSON, as it is after a long `--format json`.

## ctl-core 0.7.5

- `ctl_core::input::Tagged` re-exports serde-saphyr's `Tagged<T>`, so a config field keeps its YAML tag beside its value: `styles: !override [Acme]` reads into `Tagged<T>` as `[Acme]` with the tag `!override`.
  ctl-core now requires vsh-serde-saphyr 0.0.4, whose `Tagged<T>` gives an untagged enum the plain value of a tagged node instead of the map `{override: [Acme]}` ([#294](https://github.com/victor-software-house/ctl-core/pull/294)).

## ctl-core 0.7.4

- `Input::top_level` refuses nothing that YAML allows.
  A repeated key, a merge key (`<<`) and a `.inf`, `-.inf` or `.NaN` are entries, where each gave the error that `Input::parse` gives and hid every other entry of the text.
  A key that is a number, a boolean or a null is listed by its text as written, and a sequence, mapping or alias key is still left out.
  `Input::parse` is unchanged ([#288](https://github.com/victor-software-house/ctl-core/pull/288)).

## ctl-core 0.7.3

- `Input::flattened` declares a schema field that `#[serde(flatten)]` puts directly under its parent in the file.
  `Input::check` drops that field's step from each garde path, so a problem prints as the file holds it, such as `assets.linux-x64.files`, and sits on that line.
  A path through no declared field prints and sits as before ([#284](https://github.com/victor-software-house/ctl-core/pull/284)).
- `Input::top_level` lists the entries of a YAML or TOML text's root mapping or table in the order written.
  Each `Entry` holds the key, the byte range of the value, a `Written` style and the string the parser reads, if any.
  A caller that reads a mapping it did not write, such as the front matter of a Markdown file, needs no scanner of its own.
  serde-saphyr resolves anchors, aliases and tags, yamled gives the spans, and the `toml` tree holds the TOML ones.
  A key that is not a string is left out, a root that is no mapping gives no entries, and a text past a limit gives the error `Input::parse` gives.
  yamled is now 0.0.13 ([#282](https://github.com/victor-software-house/ctl-core/pull/282)).

## ctl-core 0.7.2

- `Input::place_key` returns where the last step of a `FieldPath` is written as a key, where `Input::place` returns where its value starts.
  A fault about the key itself, such as a glob that is no glob, lands on the key and not on the first line of a block value.
  An item of a list and the root have no key, so they give the place of their value ([#280](https://github.com/victor-software-house/ctl-core/pull/280)).

## ctl-core 0.7.1

- `Input::place` turns a `FieldPath`, the steps down to a field, into the line and column where it is written, and `Input::problem_at` builds a placed `Problem` from one.
  A CLI places a problem of its own with them, as the loader places garde's.
  `FieldPath::from(&garde::Path)` reads garde's path parts, so a problem under a key that holds `.`, `*` or `/`, such as a glob `docs/**/*.md`, lands on that key's line and not on its parent's.
  `Problem::field` is still garde's printed path.
  A YAML `Input` parses its text once for placing, where each failing `check` parsed it again.
  garde is now pinned to exactly 0.23.0, because the conversion reads a doc-hidden item of its `Path` ([#278](https://github.com/victor-software-house/ctl-core/pull/278)).
- The `ctl/invocation.md.jinja` fragment writes the rule against `mise run <mount> --` and its reason as two paragraphs.
  A consumer's rendered skill and instructions then keep one sentence per line ([#269](https://github.com/victor-software-house/ctl-core/pull/269)).

## ctl-core 0.7.0

### Added

- Add a shared shell completion verb.
  The new `completion` feature gives a ctl CLI `<bin> completion <SHELL>` for bash, zsh, and fish ([#180](https://github.com/victor-software-house/ctl-core/pull/180)).

  To use it:

  - flatten `completion::CompletionCommand` into the CLI's subcommands;
  - call `App::completion()`, so the binary answers `COMPLETE=<shell>` at each TAB;
  - route the verb to `shell::view(format)` and return `command.script("<bin>")`.

  The zsh script completes on the first TAB when Homebrew installs it in `fpath`.
  The `completion` feature enables `app`, which `App::completion` needs.
  The new `shell` feature holds the `Shell` value the verb takes.
- New crate on crates.io: `ctl-layout`, terminal widgets built on `ctl-text` ([#197](https://github.com/victor-software-house/ctl-core/pull/197)).

  - A `Table` takes `Column`s with an `Overflow` and an `Align` each, a `Frame` (`Grid`, `HeaderRule`, or `Plain`), and a `RowSeparation` (`None`, `Rule`, or `Blank`).
  - Each column gets its widest cell.
    When the width runs short, the widest column shrinks first, never below the widest word of a keep-words column.
    A framed table folds instead of running past the width.
  - A numeric column aligns right, headers included.
  - `Definitions` draws each entry as a line of labels over indented description lines.
    `Breakpoint` draws one widget below a width and another from it up, so a table gives way to definitions on a narrow terminal.
  - `Record`, `Rule`, `Stack`, and `Indent` complete the set.
    A `Stack` and an `Indent` take a widget by value or by reference.
    Every widget implements `ctl_text::Renderable`, so it measures before it renders.
  - ctl-layout re-exports `ctl_text`, `Context`, `Overflow`, `Renderable`, and `Text`.
- New crate on crates.io: `ctl-term`, terminal facts and the escape encoder ([#199](https://github.com/victor-software-house/ctl-core/pull/199)).

  - `Terminal::detect` decides, for one stream and one environment lookup, whether to write colour.
    It reads `NO_COLOR`, `CLICOLOR_FORCE`, `FORCE_COLOR`, `CLICOLOR` and `TERM=dumb`, then whether the stream is a terminal.
  - It picks the colour depth from `COLORTERM`, `TERM` and `FORCE_COLOR`.
    It picks the width from an explicit width, then the stream's window, then `COLUMNS`, then a fallback of 80.
  - `DetectOptions` and `Terminal::at_depth` override each default and can turn it off.
  - `Encoder` turns `ctl_text::Line`s into bytes.
    It writes a style sequence only where the style changes, and leaves no style open at the end of a line.
    Each colour becomes the nearest one the depth can show.
    With colour off it writes the plain text byte for byte.
  - `Console` renders any `Renderable` at the width of its stream and writes it.
    It reports a closed pipe as `Error::BrokenPipe`.
    `Error` converts into `std::io::Error` without boxing. `Io` gives back the error it holds. `BrokenPipe` always gives an error of kind `io::ErrorKind::BrokenPipe`, keeping the OS error when it has one.
  - The `detect` feature, on by default, holds what asks the operating system.
    Without it the crate holds only the encoder, the colour conversion, and `Terminal::from_probe`.
  - The crate supports and tests Unix only; Windows is to come with a Windows CI job.
- New crate on crates.io: `ctl-text`, styled text for terminals over `anstyle::Style` ([#193](https://github.com/victor-software-house/ctl-core/pull/193)).

  - A `Span` removes every character `unsafe_chars::is_unsafe` refuses.
    A `Line` is a row of spans, and `Text` splits at each newline and expands tabs to the next tab stop.
  - Widths count terminal cells per grapheme cluster (`cluster_width`, `text_width`, `prefix_len`).
    `UNICODE_VERSIONS` names the Unicode version of the width and segmentation data.
  - `Overflow` fits a line to a width: `KeepWords` breaks at spaces only, `Fold` breaks a long word between clusters, and `Ellipsis` cuts the line with `…`.
    No rendered line ends in a space.
  - Widgets implement `Renderable` with `Measurement` and `Context`, and `overlay` combines two styles.

  With the `unsafe-chars` feature, `ctl_core::unsafe_chars` re-exports `ctl_text::unsafe_chars`.
- Add a shared shell integration verb.
  The new `shell-init` feature gives a ctl CLI `<bin> init <SHELL>` for bash, zsh, and fish ([#191](https://github.com/victor-software-house/ctl-core/pull/191)).

  To use it:

  - declare `shell_init::Hook::new("<bin>", "<subcommand>", "<VARIABLE>")`;
  - flatten `shell_init::InitCommand` into the CLI's subcommands;
  - route the verb to `shell::view(format)` and return `command.script(&hook)`.

  The script runs `<bin> <subcommand>` once, in the foreground, before the shell's first prompt.
  It runs only when the variable is non-empty and the binary is on `PATH`, then removes itself.
  Loading it twice installs it once, and bash keeps the `$?` its prompt shows.
  The completion and init verbs share `shell::Script` and `shell::view`.

### Changed

- The docs.rs pages for `ctl-core`, `ctl-text`, `ctl-layout`, and `ctl-term` now open on a landing page that starts with a quick start and ends with the Cargo features ([#256](https://github.com/victor-software-house/ctl-core/pull/256)).

  `ctl-core`'s page lists its modules and the prelude, so each type appears once.
  It names where to start with declared input, which feature reads each format, and the renamed `FlagClash` and `Outcome`.

  The `ctl-term` README example now renders to a string, so it shows no hidden line on GitHub.
  No API changes.
- The help text of the shared colour flags changes wording.
  A CLI that compares its help with a snapshot or a committed file updates that file when it takes this release ([#229](https://github.com/victor-software-house/ctl-core/pull/229)).

  - `--color` reads "Pretty-output colour policy".
  - `--no-color` reads "Force colourless pretty output".
  - The `pretty` format reads "Human text, optionally coloured".
  - The description of the `auto` colour value, which shell completion shows, names what it checks: whether the stream is a terminal and the environment allows colour.
- Ask Clap only for the features ctl-core calls.
  `clap` is now declared with `default-features = false`: `cli` turns on `std`, `derive`, `env`, and `string`, and `help` adds `usage` ([#183](https://github.com/victor-software-house/ctl-core/pull/183)).

  - In 0.6 usage-lib turned Clap's default features on for every consumer, and nothing in ctl-core turns them on now.
    A consumer whose own `clap` entry keeps the default features sees no change.
  - A consumer that wants Clap's error context or suggestions turns them on in its own `clap` entry: `clap = { version = "4", default-features = false, features = ["std", "derive", "error-context", "suggestions"] }`.
  - When a command misses a required argument or subcommand, `App` renders the help Clap stops for.
    It writes that help on stderr with exit code 2, in place of Clap's own.
  - The new `unsafe-chars` feature exports `unsafe_chars::is_unsafe` and `unsafe_chars::strip_unsafe`, which `Document::verbatim` filters with.
    A CLI can drop the same terminal, bidi, and separator characters from text it prints.
  - `unicode-general-category` is no longer a dependency.
- Rename the two public types whose names held "kind", and the field and method that use them ([#250](https://github.com/victor-software-house/ctl-core/pull/250)).

  This breaks every caller that names them:

  - `WarningKind` is now `FlagClash`, and `FlagWarning`'s `kind` field is now `clash`.
    A match on `warning.kind` becomes a match on `warning.clash`.
  - `MessageKind` is now `Outcome`, and `Present::message_kind` is now `Present::outcome`.
    A result that overrode `message_kind` overrides `outcome`, with the same `Success` and `Error` variants.

  Each variant and `Outcome::default_exit_code` keep their names, and no output changes.
- Lock the public API so that later releases can add variants, fields, and trait methods without breaking a consumer ([#262](https://github.com/victor-software-house/ctl-core/pull/262)).

  Each break and its migration:

  - `document::Block`, `format::OutputFormat`, and `document::NoticeLevel` are `#[non_exhaustive]`.
    A match on one of them outside ctl-core ends with a `_` arm.
  - `model::ErrorBody` and both `Envelope` variants are `#[non_exhaustive]`.
    Build them with `ErrorBody::new`, `Envelope::ok`, and `Envelope::err`, and end a pattern on a variant with `..`.
  - `flags::FlagWarning`, `Surface`, `SurfaceCommand`, and `SurfaceArgument` are `#[non_exhaustive]`.
    Read their fields as before. A struct literal of one of them no longer compiles.
  - `OutputArgs`, `FormatArgs`, `FormatLong`, `ColorLong`, `DryRunArgs`, `CompletionArgs`, and `InitArgs` are `#[non_exhaustive]`.
    Get them from the parse, and end a pattern on `CompletionArgs` or `InitArgs` with `..`.
    `CompletionCommand` and `InitCommand` are unchanged.
  - `OutputArgs` and `ColorLong` no longer expose `color` and `no_color`.
    Call `color()`, which applies `--no-color`.
  - `View`'s `format` and `quiet` fields are private.
    Read them with `View::format()` and `View::is_quiet()`. The `quiet(bool)` setter is unchanged.
  - `InputError` and `ServeError` are `#[non_exhaustive]`.
    Their public fields stay, and `io_failure()` returns the `io::ErrorKind` when a file could not be read or written.
  - `EnvPrefix` is sealed. `#[command(env_prefix = "...")]` works as before, and no type outside ctl-core can implement the trait.

  `Outcome`, `Destination`, `BrokenPipe`, `ColorMode`, `SurfaceRequirement`, `SurfaceScope`, `FlagClash`, and `Shell` stay exhaustive.
  Each one's rustdoc holds one sentence on why its set is closed.
- Render every document through the workspace's own engine in place of comfy-table and anstream.
  The engine is three new crates on crates.io, `ctl-text`, `ctl-layout`, and `ctl-term`, published at the workspace version.
  `ctl_core::{text, layout, term}` re-export them for a CLI that needs a widget directly.
  comfy-table, anstream, and unicode-width are no longer dependencies ([#212](https://github.com/victor-software-house/ctl-core/pull/212)).

  Breaking changes:

  - `kv`, `grid`, and the `ctl_core::table` module are removed.
    Compose `Fields` or `Table` inside a `Document` instead.
  - `Document::render` no longer asks a terminal for its width or colour.
    It lays out to the explicit width, then the fallback width, then the natural width.
    It writes colour only under `ColorMode::Always`.
    `View`, help, and errors still decide colour and width for the stream they write.
  - A tab in a verbatim block expands to the next 8-cell stop.
  - Table layout changes in a few places, so snapshot tests of command output may change:
    - an empty column is 2 cells wide, where it was 3;
    - columns shrink widest first, and a framed table folds only a word wider than its column;
    - two words never run past the width;
    - colour no longer changes the layout, and every line that writes a style ends with a reset.

    A wrapped column still narrows to its widest line, as before.
  - A reader can close stdout or stderr early, as `head -1` does.
    `App` then exits with the code the command would have had and writes nothing to stderr.
    In 0.6 it exited 1.
    - A command that succeeded exits 0.
      A failed command or a parse error keeps its own code, so a closed pipe never turns a failure into success.
    - `App::broken_pipe(BrokenPipe::Fail)` makes every closed pipe a failure: success becomes exit 1, and a failure keeps its own code.
  - `ListStyle` is `Frame`, and `RecordStyle`, `Frame`, and `RowSeparation` are the types of `ctl-layout`, so each is `#[non_exhaustive]` and `Hash`.
    `JsonLayout` is `#[non_exhaustive]` too.
    `RenderOptions::list_style` is `frame`; the variable `CTL_CORE_LIST_STYLE` keeps its name.
  - Every setter on `RenderOptions` and `View` takes the name of its setting: `width`, `record_style`, `quiet`, and so on.
    `RenderOptions` has no getter, because a getter would need the same name.
    `View` reads its format and quiet setting through `format()` and `is_quiet()`.
    A method that reads the environment starts with `resolved_`.
    - `record()`, `list()`, and `separation()` are `resolved_record_style()`, `resolved_frame()`, and `resolved_row_separation()`, and `View::layout()` is `resolved_json_layout()`.
    - The getters `active_theme`, `fallback`, `explicit_width`, `explicit_automatic_width_buffer`, `automatic_width_buffer_env_names`, and `automatic_width_minimum` are gone.
      Their settings are the setters `theme`, `fallback_width`, `width`, `automatic_width_buffer`, `automatic_width_buffer_envs`, and `minimum_automatic_width`.
    - `View` holds one `RenderOptions` and returns it from `render_options()`.
      `View::color(mode)` sets the colour, and the colour is not a public field.
    - `App` keeps its setter names.
  - `Stream` in the view module is `Destination`, so it no longer shares a name with `ctl_term::Stream`.
  - `Renderer` is crate-private. `Document::render` renders to a string.
  - `Document::render` and the view write through the `ctl-term` encoder, so a line ends with a reset only when a style is still open.
  - The buffer and minimum settings, `automatic_width_buffer` and the like with their `DEFAULT_COLUMN_BUFFER*` and `DEFAULT_MINIMUM_AUTOMATIC_WIDTH` constants, exist only with `view` or `help`.
    Without a stream, nothing reads them.
  - `parser::assert_help_enabled` is crate-private.

  Added:

  - `Theme` maps each role and notice label to an `anstyle::Style`: `role_style(role)` reads one and `style(role, style)` sets one.
    `RenderOptions::theme`, `View::theme`, and `App::theme` replace the default.
  - `Table::ellipsis_column` marks a title or summary column.
    A cell too wide for it keeps one line and ends in `…`, and the column gives up width before a column of whole words does.
  - `Table::numeric_column` aligns a column, header included, to the right.
- The `input` feature reads YAML through vsh-serde-saphyr 0.0.3 and yamled 0.0.12.
  In 0.6 it used serde-saphyr 2.0.0 and yamled 0.0.8 ([#244](https://github.com/victor-software-house/ctl-core/pull/244)).

  vsh-serde-saphyr keeps the library name `serde_saphyr`.
  It is serde-saphyr 2.1.1, the latest release, with two patches.
  It carries a `NOTICE` file and the fork's copyright line in `LICENSE-MIT`.

  - serde-saphyr 2.1.1 reports the same problems in the same words, and allocates less while it parses.
    The lock moves granit-parser, its YAML parser, from 2.0.0 to 2.1.0.
  - Upstream serde-saphyr formats an error's whole report while it deserializes, snippet renderer included.
    So every binary that loaded input carried annotate-snippets.
    The patched release reads the error's kind instead, and builds an alias error's message from the plain text.
  - ctl-core builds each problem's text from serde-saphyr's message formatter, and the reports keep their wording.
    The toy CLI's input tier drops 100,840 bytes of `.text` on linux-x64.
  - yamled 0.0.12 gives the location index, with its default features still off, and takes serde-saphyr from the same release.

  A consumer whose own yamled reads values takes yamled 0.0.12 too, so it links one yamled and one serde-saphyr.
  With an older yamled it links both serde-saphyr crates.
- Each declared-input format is now its own feature, so a TOML-only consumer links no YAML crate and no garde.
  With default features off and only `input-toml` on, ctl-core links none of serde-saphyr, yamled, or garde.
  `input` keeps its meaning, YAML with garde validation, so a manifest that enables it builds unchanged ([#253](https://github.com/victor-software-house/ctl-core/pull/253)).

  These break callers:

  - `input-toml` no longer enables `input`.
    A consumer that read YAML or validated through `input-toml` alone adds `input`, or `input-yaml` and `validate`.
  - `Input::check`, `Input::load`, and the `ctl_core::validate` module need the `validate` feature.
  - With `input-toml` alone, `Format::Toml` is the only variant, so its discriminant is 0, where it was 1.
  - `Format::Yaml`, `Input::new`, `Input::read`, `Input::frontmatter`, `REPO_FILE`, `use_repo_limits`, and `Loosened` need the new `input-yaml` feature.
  - `Limits` is ctl-core's own type.
    Its 17 YAML-only fields need `input-yaml`, and a caller builds it with `..Limits::NONE`.
    `Limits::budget` is no longer public.

  These are new:

  - `input-yaml`: YAML input with no garde.
  - `Input::new_with_format(name, text, format)`: text in memory in a named format.
  - `Limits::DEFAULT_MAX_READER_INPUT_BYTES`, `DEFAULT_MAX_DEPTH`, and `DEFAULT_MAX_NODES`: the defaults every format shares, equal to serde-saphyr's.

  A TOML text is held to its byte, depth, and node limits under the call and process tiers whatever the build enables.
  Without `input-yaml`, a limit message ends "the tool sets this limit", and `Input::open` on a `.yaml` or `.yml` file names the feature it needs.
- usage-lib is no longer a dependency.
  ctl-core writes the Usage KDL itself, so the `usage` feature drops usage-lib and the 19 crates it alone pulled in ([#238](https://github.com/victor-software-house/ctl-core/pull/238)).

  - The spec keeps every byte: ctl-core already turns on Clap's `usage` feature and strips help itself.
  - ctl-core declares `-h`/`--help` and the `help` subcommand itself, so they work without Clap's `help` feature.
    A CLI that parses through `parser::apply_defaults` itself calls `help::try_emit` first, since the derive does not know the `help` subcommand.
    A plain `Cli::parse()` gets Clap's minimal output and no `help` subcommand.
  - `Surface`'s Usage KDL keeps the `-h`/`--help` flag and the `help` subcommand on every command, as it did with Clap's `help` feature.
    It records a setting that turns help off only where the CLI declared one.
- `--usage-spec` writes the Usage KDL with ctl-core's own writer, built from the clap `Command` ([#236](https://github.com/victor-software-house/ctl-core/pull/236)).

  It keeps usage-lib 7.3.0's bytes except in the cases below, each now a fix.
  They are output that was not valid KDL v2 or read back as other text, and value names that mise read as usage syntax:

  - Help text or a default value that holds U+2028, U+2029, or U+0085 is written with `\u{…}` escapes.
    KDL v2 counts these characters as newlines, so mise could not read the spec, or read U+2028 back as a line feed.
  - Bidi controls and the byte order mark in help text or a default value are written with `\u{…}` escapes.
    They are U+200E, U+200F, U+202A to U+202E, U+2066 to U+2069, and U+FEFF.
    So is ESC in a default value.
    KDL v2 forbids these characters literally.
  - A default value that holds U+007F (DEL) is quoted and escaped.
    It was written bare, which KDL v2 forbids, and parsed only because the parsers miss that rule.
  - Help text with a line of only whitespace is written as a quoted string.
    A raw multi-line string read that line back empty.
  - A default value or possible value `-.5` or `+.5` is quoted.
    Written bare, it starts like a number, and KDL v2 refuses it.
  - A flag whose value name splits into placeholders, such as `<A> <B>`, is written as one value with a `value_names` child.
    The old spec repeated the name and set two values, and mise rejected the whole spec, so completion offered nothing.
  - A value name that ends in `...` or `…` keeps its name and gains `var=#false`.
    mise read a flag's value as variadic, and a positional as taking any number of values.
  - A positional whose value name splits into placeholders gets `var=#false` and a `value_names` child.
    mise read it as that many values.
  - A positional whose value name starts with `--` and a space gets `double_dash=optional`.
    mise offered it only after `--`.
  - A flag's value named `[x]` is written `<[x]>`, a required value, as clap reads it.

  The spec no longer depends on Clap features that only usage-lib turned on:

  - ctl-core strips terminal control sequences and DEL from help text itself, as Clap does with its `color` feature on.
    The spec holds the same bytes whether or not another crate turns `color` on.
  - The `usage` feature turns on Clap's `usage` feature, so the spec keeps its usage line without the `help` feature.

### Fixed

- Fix `--usage-spec` for help text that holds four quotes and a hash, `""""#`.
  The spec wrote such text as a raw string opened with one hash.
  The `"""#` inside it closed the string early, so mise could not read the spec.
  The string now opens with one hash more than any closing sequence the text holds ([#235](https://github.com/victor-software-house/ctl-core/pull/235)).

  Every other spec ctl-core writes keeps its bytes.

## ctl-core 0.6.28

- The shared `check:versions` passes with nothing to compare in a repository without `.ctl/ver.yaml` (or the file `VERCTL_CONFIG` names), so a repository that declares no releases needs neither verctl's `ver` task nor verctl ([#151](https://github.com/victor-software-house/ctl-core/pull/151)).

## ctl-core 0.6.27

- The shared `verify` runs `check:versions`, verctl's hand-edit guard, and `semver` runs `verctl semver`, which checks the API at the release type the pending fragments allow. Both run through verctl's served `ver` task, so a repository includes it at verctl 0.5.1 or later, the first that fetches the compared branches in a shallow pull request checkout, and its CI verify step sets `GITHUB_TOKEN` with `pull-requests: read` ([#149](https://github.com/victor-software-house/ctl-core/pull/149)).

## ctl-core 0.6.26

- Releases run through verctl's reusable release workflow at 0.5.0: each push to `main` publishes any version commit not yet fully released and opens or updates the Version PR on `verctl/release/main` ([#147](https://github.com/victor-software-house/ctl-core/pull/147)).

## ctl-core 0.6.25

- The shared `lint`, `test` and `test:doc` tasks pass `--workspace`, so a workspace with a root package or a `default-members` list checks every member ([#144](https://github.com/victor-software-house/ctl-core/pull/144)).

## ctl-core 0.6.24

- ctl-core edits YAML with yamled 0.0.8, which writes one-line text in the document's quotes instead of keeping a quoted node's old quotes. ctl-core's own behaviour does not change: it reads positions from yamled and does not write through it ([#140](https://github.com/victor-software-house/ctl-core/pull/140)).

## ctl-core 0.6.23

- ctl-core reads YAML with serde-saphyr 2.0.0 and edits it with yamled 0.0.7. `Limits::budget()` now returns serde-saphyr 2.0's `Budget`; a caller that passes it to serde-saphyr 1.x must move to 2.0 with it. The `comments` feature is on, so the comment limits in `Limits` still apply.

An error raised inside an aliased value keeps serde-saphyr's own wording and points at that value, followed by where the anchor is defined. ctl-core no longer parses positions out of the rendered alias text ([#136](https://github.com/victor-software-house/ctl-core/pull/136)).

## ctl-core 0.6.22

- docs.rs shows ctl-core as a landing page: a summary, a compiled quick start, how output flows, and the Cargo features, with each gated item labelled by its feature. It lists the modules and the prelude once. The crate-root re-exports are hidden from the docs but still resolve, so no import changes. Every fallible public function documents its errors ([#133](https://github.com/victor-software-house/ctl-core/pull/133)).

## ctl-core 0.6.21

- `EnvPrefix::env_prefix` binds every long option of a command and its subcommands to `<PREFIX>_<LONG>`. Put `#[command(env_prefix = "VERCTL")]` on the root parser, with the prelude in scope, and `VERCTL_CONFIG`, `VERCTL_DRY_RUN`, and `VERCTL_FORMAT` set those flags; a flag on the command line wins over its variable. An option that declares its own `env`, has no long name, is hidden, is a `--no-*` side, or is help or version stays unbound. ctl-core help prints `[env: NAME]` beside each bound option.

`flags::env_under_flags(command, argv)` makes the command line win over the variables; `App` calls it before the parse. It unbinds the variable of an option the command line set, negated with its `--no-*` side, or conflicts with, and a variable that is empty or reads as a boolean flag's default. Two remaining variables that conflict fail the parse with an error that names both ([#130](https://github.com/victor-software-house/ctl-core/pull/130)).

## ctl-core 0.6.20

- The shared `test` task runs `cargo test --lib --tests` instead of `--lib --bins --tests`. `--bins` selects every binary target and ignores its `test` flag, so a fuzz target declared with `test = false` was built and run as a test. `--tests` selects every target whose `test` flag is on, so bins keep their unit tests ([#124](https://github.com/victor-software-house/ctl-core/pull/124)).

## ctl-core 0.6.19

- New `input-toml` feature: the declared-input loader reads TOML. `Input::open(path)` takes the format from the extension (`.toml`, `.yaml`, `.yml`) and refuses any other extension; `Input::format(Format)` sets it for text in memory, and `Input::new` and `Input::read` stay YAML, so existing consumers change nothing. A TOML parse error, a serde error, and each garde field problem (a top-level key, a table, an inline table, an array of tables) report a 1-based line and column, and a TOML text is held to `max_reader_input_bytes`, `max_depth`, and `max_nodes` from the same tiers as YAML. The `input` feature keeps meaning YAML and compiles no TOML crate ([#120](https://github.com/victor-software-house/ctl-core/pull/120)).

## ctl-core 0.6.18

- The shared Rust tasks pass `--locked` to clippy, both test tasks, cargo-deny, and the toolchain check's `cargo metadata`, so `verify` fails when `Cargo.lock` no longer matches `Cargo.toml`. Before, a stale lock passed `verify` and failed only a release build that passes `--locked`. `deps` runs a locked `cargo metadata` before cargo-machete, whose own unlocked metadata call rewrote a stale lock while sibling tasks ran. A repository passes `--locked` in its own `check:features` too ([#117](https://github.com/victor-software-house/ctl-core/pull/117)).

## ctl-core 0.6.17

- New `mount_directives(task)`: the `#MISE raw=true`, `#MISE raw_args=true`, and `#USAGE mount` block a served CLI-proxy task carries. Without `raw`, mise relays the task's output and appends an SGR reset to every line whenever its own colour is on, as under `CLICOLOR_FORCE=1`, so `--format json` piped to `jq` carried an ESC byte the CLI never wrote ([#114](https://github.com/victor-software-house/ctl-core/pull/114)).

## ctl-core 0.6.16

- A parse error under a long `--format json` or `--format=json` is now a JSON error even when the bad argument comes before the flag. Clap's lenient read stopped at the bad argument, so the error printed as text, coloured when `CLICOLOR_FORCE` or `FORCE_COLOR` was set, to a caller that asked for JSON ([#112](https://github.com/victor-software-house/ctl-core/pull/112)).

## ctl-core 0.6.15

- Remove the public `baseline` module and the `baseline` feature. The shared Copier template now renders the shared Rust configuration, so ctl-core keeps no second generator. No consumer enables the feature: verctl, qctl, and forkctl pin earlier releases without it. A crate that enabled `ctl-core/baseline` takes the files from the template instead. The removal is a breaking API change released as a patch by operator decision, and CI's semver check allows it only against the 0.6.14 baseline.

## ctl-core 0.6.14

- The shared `verify` in `tasks/rust.toml` now also depends on `check:repo`, which each including repository defines for its own checks, in place of overriding `verify`. A repository that moves its include to this release must define `check:repo` (a task that runs `true` when it has no checks of its own), or `verify` fails with `task not found: check:repo`.

## ctl-core 0.6.13

- ctl-core now ships the standard Rust mise tasks (`check:toolchain`, `format`, `lint`, `test`, `test:doc`, `deny`, `deny:advisories`, `deps`, `semver`, `verify`) in `tasks/rust.toml`. Another repository includes it with `git::https://github.com/victor-software-house/ctl-core.git//tasks/rust.toml?ref=<tag>` in `task_config.includes`, and defines its own `check:features`.

## ctl-core 0.6.12

- `rust-version` is now 1.99.0: ctl-core builds, lints, and tests on that pinned stable and formats on a dated nightly only. `App`, `fixture::Suite`, and `fixture::Root` implement `Debug`.
- New feature `baseline`: `baseline::render(root, &config)` returns the Rust configuration every ctl repository shares (`rust-toolchain.toml`, `rustfmt.toml`, `clippy.toml`, `deny.toml`, `lefthook.yml`, and the marked `lints` and `tools` blocks in `Cargo.toml` and `mise.dev.toml`) as `served::Served` values, from a repository's `.ctl/rust.yaml` (`baseline::load`).

## ctl-core 0.6.11

- `input::Limits` overrides any of serde-saphyr's `Budget` fields except `enforce_alias_anchor_ratio`, which stays on, decided field by field: the call (`Input::limits`), then the repository (`limits:` in `.ctl/input.yaml`, installed by `input::use_repo_limits`), then the process (`input::set_default_limits` or `App::input_limits`), then serde-saphyr's default. `max_reader_input_bytes` now also caps text already in memory, a limit error names the limit, its value, and where to raise it, and `use_repo_limits` returns each limit the repository raises above serde-saphyr's default for the CLI to print as a warning.

## ctl-core 0.6.10

- New `fixture` feature for integration tests: `Suite::new(name)` and `suite.scenario(name)` each own a temporary directory that is removed when it drops, and `root.file(path, body)` writes a fixture with its parent directories in one call. A path outside the root panics instead of writing.
- New `served` feature: `served::render(template, source, suffix, context)` renders a template that declares its own target through top-level `{%- set path / name / executable -%}` exports, checks those declarations once, and returns a `Served` file; `Served::write(root)` writes it with its directories and the 0755 or 0644 mode, refusing a symlinked target. Which templates a repository has and what context they see stay with the CLI.

## ctl-core 0.6.9

- An alias error from `Input::parse` names where its value is defined, counted from the file, instead of repeating two positions counted from the frontmatter.
- Move the `input` feature to yamled 0.0.5, so a CLI that edits through yamled 0.0.5 resolves one yamled.

## ctl-core 0.6.8

- Move the `input` feature to yamled 0.0.4, so a CLI that edits through yamled 0.0.4 resolves one yamled.

## ctl-core 0.6.7

- Move the `input` feature to yamled 0.0.3, so a CLI that edits through yamled 0.0.3 resolves one yamled.

## ctl-core 0.6.6

- A parse problem's message no longer repeats serde-saphyr's own position, which counted from the frontmatter instead of the file.

## ctl-core 0.6.5

- Add the declared-input layer. The `validate` feature carries the garde rules every schema shares; the `input` feature reads a file, parses it with serde-saphyr (strict booleans, duplicate and merge keys refused), validates it once with garde, and reports every problem with its file and line, placed through yamled. `Input::frontmatter` splits YAML frontmatter from a Markdown body and keeps the file's line numbers.

## ctl-core 0.6.4

- Remove the `go`, `run`, `main`, `main_with`, and `main_with_help` wrappers. `App` is the only entry point. The `cli` feature no longer pulls in `anyhow`; `app` does.
- Let the environment pick each style an owner left open: `CTL_CORE_RECORD_STYLE`, `CTL_CORE_LIST_STYLE`, `CTL_CORE_ROW_SEPARATION`, and `CTL_CORE_JSON_LAYOUT`. An unaccepted value is ignored, and `App` names the accepted values on stderr.

## ctl-core 0.6.3

- JSON error envelopes follow the view JSON layout, so errors are indented like successes by default and compact when a consumer sets JsonLayout::Compact.
- Keep a word wider than its column, such as a URL or path, whole in records, paragraphs, and headings; the line runs past the width instead of splitting the word.

## ctl-core 0.6.2

- Wrap headings and section titles to the render width, so a long title stays inside the fallback width in piped output.

## ctl-core 0.6.1

- Make the operator's picks the defaults: borderless records with right-aligned keys, two-space JSON, a two-column automatic-width buffer, and an 80-column fallback width when none is detected (`fallback_width(None)` disables it). `App` now reaches the style options, JSON layout, and fallback width.

## ctl-core 0.6.0

- Add an identifier role (`Role::Id`, `Text::id`, `Table::id_column`) that renders bold in the terminal foreground colour, and mark `Role` as non-exhaustive.
- Add record style, list style, row separation, and JSON layout options to RenderOptions and View, with today's look as the default, and a gallery example that renders every candidate.

## ctl-core 0.5.2

- Expose Surface metadata, serialization, and MiniJinja rendering through separate additive feature tiers while preserving the existing full compatibility aggregate.

## ctl-core 0.5.1

- Consumers can require every long option to have an operator-visible short form, so a new long-only flag fails their surface test before release. An exact-token allow list preserves letters that shared format and color flags deliberately leave to each CLI.
- Reserve one column from automatically detected terminal widths so nested shells keep the right edge visible, with public overrides for the buffer and its environment aliases.

## ctl-core 0.5.0

- Extract a serializable operator Surface from Clap and provide shared MiniJinja fragments for skill versions, mounted invocation, and visible command inventories.

## ctl-core 0.4.2

- Add `FormatLong`, a reusable `--format` plus `-q`/`--quiet` mixin for CLIs that already own `-f`.

## ctl-core 0.4.1

- Wrap long help Usage lines within the detected width and prevent flattened output flags from replacing the root long description.

## ctl-core 0.4.0

- Allow `App` consumers to enrich mounted Usage documents without taking back stream or lifecycle ownership.

## ctl-core 0.3.0

- Add semantic verbatim document blocks for preformatted Markdown and protocol text that must bypass terminal-width wrapping.

## ctl-core 0.2.0

- Add the typed `App` chassis and fluent semantic `Document` presentation API. Pretty, colorless, JSON, help, errors, quiet behavior, and terminal layout now share one ctl-core-owned path; terminal engine types stay private.

## ctl-core 0.1.1

- `features = ["usage"]` compiles without `help`. Prelude re-exports `go` only when help is on.

## ctl-core 0.1.0

- Verify runs nextest, cargo-deny (licenses/bans/sources locally; advisories on CI), and cargo-machete.
- `usage` feature: one `--usage-spec` helper and the mise mount line so qctl, verctl, and forkctl share the same `mise run q status` form (no `--`).

## ctl-core 0.0.5

- Pin Version PR and publish at verctl 0.2.1. Publish no longer needs a full checkout; the action fetches the default branch at depth 1.
- Pin Version PR and publish at verctl 0.2.3. The action fetches the default-branch history, not a depth-1 tip, and rewrites `origin/HEAD` only from `VERCTL_DEFAULT_BRANCH`.

## ctl-core 0.0.4

- Pin Version PR and publish at verctl 0.2.0 so the first crates.io cut from this lane tags the published commit.

## ctl-core 0.0.3

- `kv` / `grid` take `ColorMode`. `--color never` is colorless; no caller strips ANSI.

## ctl-core 0.0.2

- Add `kv` / `grid` pretty tables with styled tokens for command output.
