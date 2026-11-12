# declared-input Specification

## Purpose

How a ctl CLI reads a config file.
It has shared garde rules and one loader that parses YAML or TOML and validates it once.
Every problem is placed on its file and line.

## Requirements

### Requirement: Shared validators complain in the repository's words

The `validate` feature SHALL provide garde `custom` rules generic over the
context:

- a path that stays inside the repository;
- one file name;
- a list that needs at least one entry;
- a value that cannot be empty.

Each message SHALL say what a person changes.

#### Scenario: An empty list names what it needs

- **WHEN** a schema field uses `at_least_one("label")`
- **AND** the file has `labels: []`
- **THEN** the problem reads `must declare at least one label`

### Requirement: One entry point loads a file

`Input::load` SHALL read the text, parse its shape with the parser of the
input's format, and validate it once with garde. For YAML, the parser is
`serde-saphyr`, which SHALL accept only `true` and `false` as booleans and
SHALL refuse duplicate keys and merge keys. For TOML, the parser is the
`toml` crate, which refuses duplicate keys by the TOML specification.

#### Scenario: A duplicate key is refused with its line

- **WHEN** a YAML file has `flag: true` on line 1 and `flag: false` on line 2
- **THEN** loading fails with one problem on line 2

#### Scenario: A TOML duplicate key is refused with its line

- **WHEN** a TOML file has `flag = true` on line 1 and `flag = false` on
  line 2
- **THEN** loading fails with one problem on line 2, column 1

#### Scenario: A TOML duplicate key without garde

- **WHEN** a build with only `input-toml` parses the same TOML file
- **THEN** `Input::parse` fails with one problem on line 2, column 1

### Requirement: Every schema problem names its file and line

A validation failure SHALL report every problem garde finds.
Each problem SHALL carry:

- the input's name;
- the line and column of the field it names, or of its nearest written
  ancestor, placed from the parts of garde's path and never from its printed
  string;
- the field path, as garde prints it, for display only;
- the message.

#### Scenario: Two problems in one file

- **WHEN** `ver.yaml` has `name: ''` on line 1 and `labels: []` on line 2
- **THEN** the error lists `ver.yaml:1:7: name: cannot be empty ...` and
  `ver.yaml:2:9: labels: must declare at least one label`

#### Scenario: A problem under a key that holds dots

- **WHEN** a map keyed by glob has the key `docs/**/*.md` on line 2 and an empty `label` below it on line 3
- **THEN** the problem for `files.docs/**/*.md.label` is on line 3, not on the `files` line

### Requirement: Frontmatter keeps the file's line numbers

`Input::frontmatter` SHALL split a text that starts with a `---` line into
the YAML up to the next `---` line and the body after it. A problem in the
frontmatter SHALL carry the line number it has in the whole file.

#### Scenario: A changeset fragment with an empty package

- **WHEN** a fragment's second line is `package: ''`
- **THEN** the problem is reported on line 2

### Requirement: An alias error names where its value is defined

When a value reached through an alias fails to parse, the problem SHALL sit
on the alias.
Its message SHALL name where the failing value is defined, counted from the
file, with no other position in the text.

#### Scenario: A value copied by an alias in frontmatter

- **WHEN** a Markdown file's frontmatter defines `base: &b` on line 2 with
  `port: eighty` on line 3, and line 4 is `copy: *b`, read into a type where
  `copy.port` is a `u16`
- **THEN** the problem is on line 4, column 7, and reads
  `invalid u16 (defined at line 3, column 9)`

### Requirement: Input limits are overridable field by field in tiers

`Input::parse` SHALL take each limit from the highest tier that sets it, in
this order:

1. the call's `Input::limits`;
2. with `input-yaml`, the repository's `limits:` in `.ctl/input.yaml`
   installed by `use_repo_limits`;
3. the process default from `set_default_limits`;
4. ctl-core's stated default for the three shared fields, and serde-saphyr's
   default for every YAML-only field.

#### Scenario: The process default applies

- **WHEN** `set_default_limits` sets `max_depth: 4` and a text nests six deep
- **THEN** the parse fails naming `max_depth (4)`

#### Scenario: The repository beats the process

- **WHEN** `.ctl/input.yaml` sets `limits: {max_depth: 8}` and
  `use_repo_limits(root)` ran
- **THEN** the six-deep text parses

#### Scenario: The call beats the repository

- **WHEN** the input also carries `limits(Limits { max_depth: Some(5), .. })`
- **THEN** the parse fails naming `max_depth (5)`

#### Scenario: An unset field falls through

- **WHEN** the process sets `max_nodes: 20` and the repository sets only
  `max_depth`
- **THEN** a 30-item sequence fails naming `max_nodes (20)`

#### Scenario: A repository file with an unknown field

- **WHEN** `.ctl/input.yaml` sets `limits: {max_dept: 8}`
- **THEN** `use_repo_limits` fails naming the file and `max_dept`

#### Scenario: The tiers of a TOML-only build

- **WHEN** a build with only `input-toml` sets `max_depth: 4` as the process
  default, and the call sets `max_depth: 5`
- **THEN** a TOML text six deep fails naming `max_depth (5)`

### Requirement: The limits cover the whole budget, and a loosened one is reported

With `input-yaml`, `Limits` SHALL expose every `Budget` field except
`enforce_alias_anchor_ratio`, which stays on. `max_reader_input_bytes` SHALL
also cap text already in memory.

`use_repo_limits` SHALL return each field the
repository sets looser than its default.
A shared field's default is ctl-core's, and a YAML-only field's is
serde-saphyr's.
It SHALL return each field with its value and the default, for the CLI to
print as a warning.

#### Scenario: The alias ratio cannot be turned off

- **WHEN** `.ctl/input.yaml` sets `limits: {enforce_alias_anchor_ratio: false}`
- **THEN** `use_repo_limits` fails naming `enforce_alias_anchor_ratio`

#### Scenario: A repository loosens a limit

- **WHEN** `.ctl/input.yaml` sets `max_depth: 128`, `max_nodes: 10`,
  `max_reader_input_bytes: 536870912`, and `alias_anchor_ratio_multiplier: 1000`
- **THEN** `use_repo_limits` returns `max_reader_input_bytes`, `max_depth`, and
  `alias_anchor_ratio_multiplier` with their values and defaults, not
  `max_nodes`, and the `max_depth` one
  reads `.ctl/input.yaml raises max_depth to 128, above the default 64`

### Requirement: A limit error shows how to raise the limit

A refusal caused by a limit SHALL name the file, the line when known, the
limit's field and value, and `raise limits.<field> in .ctl/input.yaml`. A limit
hit while reading `.ctl/input.yaml` itself SHALL point at the tool's process
default instead. Without `input-yaml`, the message SHALL end with
`the tool sets this limit`.

#### Scenario: Depth

- **WHEN** `deep.yaml` nests six deep under `max_depth: 4`
- **THEN** the error reads `deep.yaml:<line>:<column>: exceeds max_depth (4);
  raise limits.max_depth in .ctl/input.yaml`

#### Scenario: Text already in memory

- **WHEN** an 18-byte text is parsed under `max_reader_input_bytes: 10`
- **THEN** the one problem has no line and reads `is 18 bytes, which exceeds
  max_reader_input_bytes (10); raise limits.max_reader_input_bytes in
  .ctl/input.yaml`

#### Scenario: A limit hit while reading the repository file

- **WHEN** the process sets `max_depth: 1` and `.ctl/input.yaml` nests two
  deep
- **THEN** `use_repo_limits` fails with `raise max_depth in the tool's process
  default, which reads .ctl/input.yaml`

#### Scenario: A limit in a TOML-only build

- **WHEN** a build with only `input-toml` parses
  `a = { b = { c = { d = { e = 1 } } } }` under `max_depth: 4`
- **THEN** the problem is on line 1, column 23, and reads `exceeds max_depth
  (4); the tool sets this limit`

### Requirement: The format is chosen, never guessed

`Input::open(path)` SHALL read a file and take its format from the extension,
compared without regard to case: `.toml` is TOML, and `.yaml` and `.yml` are
YAML.
A file with any other extension, or none, SHALL be refused before the file is
read.
The refusal SHALL be a problem that names the extension and the accepted
ones.

#### Scenario: A TOML file is opened by its extension

- **WHEN** `app.toml` holds `name = ''` on line 1 and a schema refuses an
  empty name
- **THEN** `Input::open` followed by `load` fails with one problem on
  line 1, column 8, for `name`

#### Scenario: An unknown extension is refused

- **WHEN** `Input::open` is given `app.json`
- **THEN** it fails with one problem that reads
  `unsupported extension .json; name the file *.toml, *.yaml, or *.yml`,
  with no line

#### Scenario: A name with no extension is refused

- **WHEN** `Input::open` is given `app`
- **THEN** it fails with one problem that reads
  `no file extension; name the file *.toml, *.yaml, or *.yml`

### Requirement: Text in memory and a missing feature follow the same rule

`Format::of(path)` SHALL apply the extension rule of `Input::open`.
`Input::format` SHALL set the format of an input. With `input-yaml`,
`Input::new` and `Input::read` SHALL create YAML inputs whatever name they
are given. A file whose format's feature is off SHALL be refused with a
problem that names the feature.

#### Scenario: A name never changes the format of text in memory

- **WHEN** `Input::new("app.toml", text)` holds TOML text
- **THEN** it is read as YAML, and `Input::format(Format::Toml)` reads it as
  TOML

#### Scenario: A YAML file in a TOML-only build

- **WHEN** a build with only `input-toml` calls `Input::open` on `app.yaml`
- **THEN** it fails with one problem that reads `` `.yaml` needs ctl-core's
  input-yaml feature; name the file *.toml ``, with no line

### Requirement: A TOML problem names its file, line, and column

A TOML parse error, a serde error, and each garde problem SHALL carry:

- the input's name;
- a 1-based line;
- a 1-based column counted in characters;
- a message.

A garde problem SHALL sit on the value of the field it names, or on its
nearest written ancestor.
This SHALL hold for a key at the top level, in a table, in an inline table,
and in an array of tables.

#### Scenario: A top-level key

- **WHEN** `app.toml` has `name = ''` on line 1
- **THEN** the problem is `app.toml:1:8: name: cannot be empty ...`

#### Scenario: A table

- **WHEN** `app.toml` has `[release]` on line 3 and `tag = ''` on line 4
- **THEN** the problem for `release.tag` is on line 4, column 7

#### Scenario: An inline table

- **WHEN** `app.toml` has `owner = { name = '' }` on line 2
- **THEN** the problem for `owner.name` is on line 2, column 18

#### Scenario: An array of tables

- **WHEN** `app.toml` has two `[[outputs]]` tables, and the second holds
  `path = '../outside'` on line 13
- **THEN** the problem for `outputs[1].path` is on line 13, column 8

#### Scenario: Several problems in one file

- **WHEN** `app.toml` breaks four rules
- **THEN** the error lists every problem, each on its own line

#### Scenario: A type error

- **WHEN** `port = 'eighty'` is on line 1 and the field is a `u16`
- **THEN** the problem is on line 1, column 8, and reads
  `invalid type: string "eighty", expected u16`

#### Scenario: A parse error

- **WHEN** line 2 is `labels = ['ship'` and never closes
- **THEN** the problem is on line 2 and has no field

#### Scenario: A field the file leaves out

- **WHEN** a field with a garde rule has a serde default and the file
  omits it
- **THEN** the problem sits on the nearest table that is written

### Requirement: A TOML text is held to the size and nesting limits

A TOML input SHALL be held to `max_reader_input_bytes`, `max_depth`, and
`max_nodes`.
Each limit SHALL be decided by the tiers `Input::limits`, the repository's
`.ctl/input.yaml` with `input-yaml`, the process default, and ctl-core's
stated default.
The limit messages SHALL be the same as for YAML.
Depth counts tables and arrays, and the root
table is 1. Nodes count the root table and every table, array, and value.

#### Scenario: The input size

- **WHEN** an 18-byte TOML text is parsed under `max_reader_input_bytes: 10`
- **THEN** the one problem has no line and reads `is 18 bytes, which exceeds
  max_reader_input_bytes (10); raise limits.max_reader_input_bytes in
  .ctl/input.yaml`

#### Scenario: Depth

- **WHEN** `a = { b = { c = { d = { e = 1 } } } }` is parsed under
  `max_depth: 4`
- **THEN** the problem is on line 1, column 23, and reads `exceeds max_depth
  (4); raise limits.max_depth in .ctl/input.yaml`

#### Scenario: A text at the depth limit

- **WHEN** the same text is parsed under `max_depth: 5`
- **THEN** it parses

### Requirement: A TOML limit sits on the value that crosses it

The depth and node limits SHALL be checked on the parsed tree, at the line and
column of the value that crosses the limit. Every other `Limits` field names
a YAML syntax feature and SHALL have no effect on TOML. The `toml` parser
SHALL keep its own bound of 80 nested levels, which no limit raises.

#### Scenario: Nodes

- **WHEN** a TOML text holds `values` with 30 items under `max_nodes: 20`
- **THEN** the problem is on the 19th item and reads `exceeds max_nodes (20);
  raise limits.max_nodes in .ctl/input.yaml`

#### Scenario: A YAML-only limit

- **WHEN** a TOML text is parsed under `max_aliases: 0`
- **THEN** it parses

#### Scenario: Nesting past the parser's bound

- **WHEN** a TOML text nests arrays 100 deep under `max_depth: 1000`
- **THEN** it fails with the `toml` parser's problem, on a line

### Requirement: Each format is its own feature

The `input-yaml` feature SHALL carry the YAML format and the `input-toml`
feature the TOML format. Neither SHALL enable the other or `validate`.
The `input` feature SHALL enable `input-yaml` and `validate`, and nothing
else. `check:features` SHALL compile `input-yaml` alone and `input-toml`
alone.

#### Scenario: A consumer of input

- **WHEN** a crate enables `ctl-core` with `features = ["input"]`
- **THEN** `cargo tree` lists no `toml` crate, and every existing call to
  `Input::new`, `Input::read`, `Input::parse`, `Input::check`, and
  `use_repo_limits` compiles and behaves as before

#### Scenario: A consumer of both formats

- **WHEN** a crate enables `features = ["input", "input-toml"]`
- **THEN** `Input::open` reads `.yaml`, `.yml`, and `.toml` files

### Requirement: A TOML-only build links no YAML crate and no garde

With default features off and only `input-toml` on, ctl-core's normal
dependencies SHALL include none of `serde-saphyr`, `yamled`, or `garde`.
`check:tree` SHALL fail when one of them appears.

#### Scenario: The dependency tree of a TOML-only build

- **WHEN** `cargo tree --package ctl-core --edges normal
  --no-default-features --features input-toml` runs
- **THEN** its output names no `serde-saphyr`, `yamled`, or `garde`

### Requirement: Validation needs the validate feature

`Input::check` and `Input::load` SHALL exist only with the `validate`
feature. Without it, `Input::parse` SHALL read a text into a type with no
validation.

#### Scenario: A TOML consumer that validates

- **WHEN** a crate enables `features = ["input-toml", "validate"]`
- **THEN** `Input::open` followed by `load` reports each garde problem on its
  line and column

#### Scenario: A TOML consumer that does not validate

- **WHEN** a crate enables only `input-toml`
- **THEN** `Input::parse` reads a TOML text, and `garde` is not compiled

### Requirement: ctl-core states the shared limit defaults

`Limits` SHALL expose `max_reader_input_bytes`, `max_depth`, and `max_nodes`
in every build, with the defaults `Limits::DEFAULT_MAX_READER_INPUT_BYTES`
(268,435,456), `Limits::DEFAULT_MAX_DEPTH` (64), and
`Limits::DEFAULT_MAX_NODES` (250,000). With `input-yaml`, a YAML text SHALL
meet the same three defaults.

#### Scenario: The default depth of a TOML text

- **WHEN** a TOML text nests 65 tables deep and no tier sets `max_depth`
- **THEN** the parse fails naming `max_depth (64)`

#### Scenario: The defaults match the YAML parser

- **WHEN** ctl-core is built with `input-yaml`
- **THEN** each of the three constants equals the YAML parser's own default

### Requirement: A text in memory can name its format

`Input::new_with_format(name, text, format)` SHALL build an input of the
given format from text in memory, in any build with that format's feature.

#### Scenario: TOML text in memory without YAML

- **WHEN** a TOML-only build calls `Input::new_with_format("app.toml",
  "name = 'release'", Format::Toml)` and parses it into a type with a
  `name` field
- **THEN** `name` is `release`

### Requirement: A field path names where a problem sits

ctl-core SHALL export `FieldPath`, a list of steps from the root, each a mapping or table key or a sequence or array index.
A key SHALL be any text, including text that holds `.`, `*`, `/`, or `[`.
With the `validate` feature, `FieldPath` SHALL be buildable from garde's path by reading its parts, and SHALL NOT be built by splitting the printed path.

#### Scenario: A key that holds dots, stars, and slashes

- **WHEN** a path is built from garde's path for the field `vocab[0]` under the key `docs/**/*.md` under `files`
- **THEN** its steps are the keys `files`, `docs/**/*.md`, and `vocab`, then the index 0

#### Scenario: A numeric key stays a key

- **WHEN** garde's path holds the key `2` under `files`
- **THEN** the second step is the key `2`, not the index 2

### Requirement: Input places a path

`Input::place` SHALL take a `FieldPath` and return the 1-based line and a 1-based column counted in characters where the path is written, or where its nearest written ancestor is.
It SHALL add the input's frontmatter offset to the line.
It SHALL return `None` when the text does not parse.
It SHALL exist with `input-yaml` or `input-toml`, and not need `validate`.

#### Scenario: A key that holds dots, stars, and slashes in YAML

- **WHEN** `app.yaml` has `files:` on line 1, `'docs/**/*.md':` on line 2, and `label: ''` on line 3, and the path is `files`, `docs/**/*.md`, `label`
- **THEN** `place` returns line 3, column 12

#### Scenario: A key that holds dots, stars, and slashes in TOML

- **WHEN** `app.toml` has `[files."docs/**/*.md"]` on line 1 and `label = ''` on line 2, and the path is `files`, `docs/**/*.md`, `label`
- **THEN** `place` returns line 2, column 9

#### Scenario: A path the file lacks

- **WHEN** the path names a key the file does not hold under `files`
- **THEN** `place` returns the place of `files`

### Requirement: Placing keeps to the limits in force

`Input::place` SHALL return `None` for a text that goes past the limits in force, as `parse` would refuse it.  
It SHALL NOT give such a text to a parser that has no budget of its own.

#### Scenario: A text over the size limit

- **WHEN** a YAML or TOML input is longer than `max_reader_input_bytes` and `place` is called
- **THEN** `place` returns `None`, and a YAML input has built no document

#### Scenario: Limits that change after a placing

- **WHEN** an input was placed under one set of limits, then the repository file, the process default or `Input::limits` on a clone gives it a set the text goes past, or a set it passes after a refusal
- **THEN** `place` gives the result of the limits in force at that call

#### Scenario: A YAML text over the depth or node limit

- **WHEN** a YAML input nests deeper than `max_depth`, or holds more nodes than `max_nodes`, and `place` is called
- **THEN** `place` returns `None`, and the input keeps no document

### Requirement: A caller builds a placed problem

`Input::problem_at` SHALL take a `FieldPath` and a message and return a `Problem` with the place of the path, the path printed as garde prints a field, and the message.

#### Scenario: A CLI's own check

- **WHEN** a CLI calls `problem_at` for the path `files`, `docs/**/*.md`, `vocab`, index 1
- **THEN** the problem's field is `files.docs/**/*.md.vocab[1]` and its line and column are those of `place`

### Requirement: The YAML document is parsed once per input

An `Input` SHALL parse the text with yamled at most once for placing, whatever the number of calls to `place` and `check`.

#### Scenario: Two placings

- **WHEN** `place` is called twice on one YAML input
- **THEN** the second call reuses the document the first built

### Requirement: Input places a key

`Input::place_key` SHALL take a `FieldPath` and return the 1-based line and a 1-based column counted in characters where the last step of the path is written as a key, or where the nearest written ancestor's key is.  
A step that is an index, and the root, SHALL give the place of their value.  
It SHALL add the input's frontmatter offset to the line, and SHALL return `None` for a text that does not parse or goes past the limits in force.

#### Scenario: A key that holds a block mapping in YAML

- **WHEN** `app.yaml` has `files:` on line 1, `'docs/**/*.md':` on line 2 and `label: ''` on line 3, and the path is `files`, `docs/**/*.md`
- **THEN** `place_key` returns line 2, column 3, where `place` returns line 3, column 5

#### Scenario: A scalar value

- **WHEN** the path of the previous scenario ends with `label`
- **THEN** `place_key` returns line 3, column 5, the key, where `place` returns line 3, column 12

#### Scenario: A key in a flow mapping

- **WHEN** a line holds `flow: {styles: [x], vocab: []}` and the path is `flow`, `styles`
- **THEN** `place_key` returns that line and the column of `styles`

#### Scenario: A key in TOML

- **WHEN** `app.toml` has `[files."docs/**/*.md"]` on line 1, and the path is `files`, `docs/**/*.md`
- **THEN** `place_key` returns line 1, column 8, where the key starts

#### Scenario: An item has no key

- **WHEN** the path ends with an index
- **THEN** `place_key` returns what `place` returns

#### Scenario: A path the file lacks

- **WHEN** the path names a key the file does not hold under `files`
- **THEN** `place_key` returns the place of the key `files`
