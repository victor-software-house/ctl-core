# declared-input Specification

## Purpose

How a ctl CLI reads a config file: shared garde rules, one loader that parses YAML and validates it once, and every problem placed on its file and line.

## Requirements

### Requirement: Shared validators complain in the repository's words

The `validate` feature SHALL provide garde `custom` rules generic over the
context: a path that stays inside the repository, one file name, a list that
needs at least one entry, and a value that cannot be empty. Each message SHALL
say what a person changes.

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

### Requirement: Every schema problem names its file and line

A validation failure SHALL report every problem garde finds, each with the
input's name, the line and column of the field it names (or of its nearest
written ancestor), the field path, and the message.

#### Scenario: Two problems in one file

- **WHEN** `ver.yaml` has `name: ''` on line 1 and `labels: []` on line 2
- **THEN** the error lists `ver.yaml:1:7: name: cannot be empty ...` and
  `ver.yaml:2:9: labels: must declare at least one label`

### Requirement: Frontmatter keeps the file's line numbers

`Input::frontmatter` SHALL split a text that starts with a `---` line into
the YAML up to the next `---` line and the body after it. A problem in the
frontmatter SHALL carry the line number it has in the whole file.

#### Scenario: A changeset fragment with an empty package

- **WHEN** a fragment's second line is `package: ''`
- **THEN** the problem is reported on line 2

### Requirement: An alias error names where its value is defined

When a value reached through an alias fails to parse, the problem SHALL sit
on the alias, and its message SHALL name where the failing value is defined,
counted from the file, with no other position in the text.

#### Scenario: A value copied by an alias in frontmatter

- **WHEN** a Markdown file's frontmatter defines `base: &b` on line 2 with
  `port: eighty` on line 3, and line 4 is `copy: *b`, read into a type where
  `copy.port` is a `u16`
- **THEN** the problem is on line 4, column 7, and reads
  `invalid u16 (defined at line 3, column 9)`

### Requirement: Input limits are overridable field by field in tiers

`Input::parse` SHALL use serde-saphyr's `Budget` with each field taken from
the highest tier that sets it: the call's `Input::limits`, then the
repository's `limits:` in `.ctl/input.yaml` installed by `use_repo_limits`,
then the process default from `set_default_limits`, then serde-saphyr's
default.

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

### Requirement: The limits cover the whole budget, and a loosened one is reported

`Limits` SHALL expose every `Budget` field except
`enforce_alias_anchor_ratio`, which stays on. `max_reader_input_bytes` SHALL
also cap text already in memory. `use_repo_limits` SHALL return each field the
repository sets looser than serde-saphyr's default, with its value and the
default, for the CLI to print as a warning.

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

### Requirement: A limit error says how to raise the limit

A refusal caused by a limit SHALL name the file, the line when known, the
limit's field and value, and `raise limits.<field> in .ctl/input.yaml`. A limit
hit while reading `.ctl/input.yaml` itself SHALL point at the tool's process
default instead.

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

### Requirement: The format is chosen, never guessed

`Input::open(path)` SHALL read a file and take its format from the extension,
compared without regard to case: `.toml` is TOML, and `.yaml` and `.yml` are
YAML. A file with any other extension, or none, SHALL be refused with a
problem that names the extension and the accepted ones, before the file is
read.

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
`Input::format` SHALL set the format of an input, and `Input::new` and
`Input::read` SHALL create YAML inputs whatever the name says. Without the
`input-toml` feature, a `.toml` file SHALL be refused with a problem that
names the feature.

#### Scenario: A name never changes the format of text in memory

- **WHEN** `Input::new("app.toml", text)` holds TOML text
- **THEN** it is read as YAML, and `Input::format(Format::Toml)` reads it as
  TOML

### Requirement: A TOML problem names its file, line, and column

A TOML parse error, a serde error, and each garde problem SHALL carry the
input's name, a 1-based line, a 1-based column counted in characters, and
a message. A garde problem SHALL sit on the value of the field it names,
or on its nearest written ancestor, for a key at the top level, in a table,
in an inline table, and in an array of tables.

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
`max_nodes`, each decided by the tiers `Input::limits`, the repository's
`.ctl/input.yaml`, the process default, and serde-saphyr's default, with the
same limit messages as YAML. Depth counts tables and arrays, and the root
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

### Requirement: The TOML path is an additive feature

The `input-toml` feature SHALL enable `input` and the `toml` crate, and
nothing else. The `input` feature SHALL keep enabling YAML only, and a
consumer that enables `input` without `input-toml` SHALL compile no TOML
crate. `check:features` SHALL compile `input-toml` alone.

#### Scenario: A YAML-only consumer

- **WHEN** a crate enables `ctl-core` with `features = ["input"]`
- **THEN** `cargo tree` lists no `toml` crate, and every existing call
  compiles and behaves as before

#### Scenario: A TOML consumer

- **WHEN** a crate enables `features = ["input-toml"]`
- **THEN** `Format::Toml` and `Input::open` read `.toml` files, and the
  `input` API is available
