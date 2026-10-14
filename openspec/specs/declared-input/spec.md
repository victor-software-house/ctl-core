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

`Input::load` SHALL read the text, parse its shape with `serde-saphyr`, and
validate it once with garde. The parser SHALL accept only `true` and `false`
as booleans, and SHALL refuse duplicate keys and merge keys.

#### Scenario: A duplicate key is refused with its line

- **WHEN** a file has `flag: true` on line 1 and `flag: false` on line 2
- **THEN** loading fails with one problem on line 2

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
