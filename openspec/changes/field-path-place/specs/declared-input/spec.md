## ADDED Requirements

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

## MODIFIED Requirements

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
