## ADDED Requirements

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
