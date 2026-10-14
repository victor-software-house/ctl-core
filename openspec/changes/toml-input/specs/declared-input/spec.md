## MODIFIED Requirements

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

## ADDED Requirements

### Requirement: The format is chosen, never guessed

`Input::open(path)` SHALL read a file and take its format from the extension,
compared without regard to case: `.toml` is TOML, and `.yaml` and `.yml` are
YAML. A file with any other extension, or none, SHALL be refused with a
problem that names the extension and the accepted ones, before the file is
read. `Format::of(path)` SHALL apply the same rule. `Input::format` SHALL set
the format of an input, and `Input::new` and `Input::read` SHALL create YAML
inputs whatever the name says. Without the `input-toml` feature, a `.toml`
file SHALL be refused with a problem that names the feature.

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
The depth and node limits are checked on the parsed tree, at the line and
column of the value that crosses the limit. Every other `Limits` field names
a YAML syntax feature and SHALL have no effect on TOML. The `toml` parser
SHALL keep its own bound of 80 nested levels, which no limit raises.

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
