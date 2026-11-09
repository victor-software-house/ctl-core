## ADDED Requirements

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

#### Scenario: A TOML duplicate key without garde

- **WHEN** a build with only `input-toml` parses the same TOML file
- **THEN** `Input::parse` fails with one problem on line 2, column 1

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

## REMOVED Requirements

### Requirement: The TOML path is an additive feature

**Reason**: `input-toml` no longer enables `input`, so a TOML-only build
leaves out the YAML crates and garde. "Each format is its own feature"
replaces it.

**Migration**: A consumer that enabled only `input-toml` and calls YAML API
adds `input-yaml`; one that calls `Input::check` or `Input::load` adds
`validate`.
