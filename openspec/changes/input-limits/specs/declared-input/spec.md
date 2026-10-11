## ADDED Requirements

### Requirement: Input limits are overridable field by field in tiers

`Input::parse` SHALL use serde-saphyr's `Budget` with each field taken from
the highest tier that sets it: the call's `Input::limits`, then the
repository's `limits:` in `.ctl/input.yaml` installed by `use_repo_limits`,
then the process default from `set_default_limits`, then serde-saphyr's
default. `Limits` SHALL expose every `Budget` field except
`enforce_alias_anchor_ratio`, which stays on. `max_reader_input_bytes` SHALL
also cap text already in memory. `use_repo_limits` SHALL return each field the
repository sets looser than serde-saphyr's default, with its value and the
default, for the CLI to print as a warning.

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
