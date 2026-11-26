## MODIFIED Requirements

### Requirement: The format is chosen, never guessed

`Input::open(path)` SHALL read a file and take its format from the registry by the file's extension, compared without regard to case.
A file whose extension no registered format reads, or with none, SHALL be refused before the file is read.
The refusal SHALL be a problem that names the extension and the extensions the registry accepts.

#### Scenario: A TOML file is opened by its extension

- **WHEN** `app.toml` holds `name = ''` on line 1 and a schema refuses an empty name
- **THEN** `Input::open` followed by `load` fails with one problem on line 1, column 8, for `name`

#### Scenario: An unknown extension is refused

- **WHEN** the registry holds `yaml` and `toml`, and `Input::open` is given `app.ini`
- **THEN** it fails with one problem that reads `unsupported extension .ini; name the file *.toml, *.yaml, or *.yml`, with no line

#### Scenario: A name with no extension is refused

- **WHEN** `Input::open` is given `app`
- **THEN** it fails with one problem that reads `no file extension; name the file *.toml, *.yaml, or *.yml`

### Requirement: A text in memory can name its format

`Input::new_with_format(name, text, format)` SHALL build an input of the given format, a `&'static dyn input::Format`, from text in memory.

#### Scenario: TOML text in memory without YAML

- **WHEN** a TOML-only build calls `Input::new_with_format("app.toml", "name = 'release'", &input::TOML)` and parses it into a type with a `name` field
- **THEN** `name` is `release`

### Requirement: Text in memory and a missing feature follow the same rule

`Formats::of(path)` SHALL apply the extension rule of `Input::open`.
`Input::format` SHALL set the format of an input.
With `input-yaml`, `Input::new` and `Input::read` SHALL create YAML inputs whatever name they are given.
A built-in format whose feature is off SHALL not be in `Formats::builtin()`.
A file of its extension SHALL be refused with a problem that names the feature.

#### Scenario: A name never changes the format of text in memory

- **WHEN** `Input::new("app.toml", text)` holds TOML text
- **THEN** it is read as YAML, and `Input::format(&input::TOML)` reads it as TOML

#### Scenario: A YAML file in a TOML-only build

- **WHEN** a build with only `input-toml` calls `Input::open` on `app.yaml`
- **THEN** it fails with one problem that reads `` `.yaml` needs ctl-core's input-yaml feature; name the file *.toml ``, with no line

### Requirement: Each format is its own feature

The `input-yaml` feature SHALL carry the YAML format and the `input-toml` feature the TOML format, each through the `ctl-input` crate.
Neither SHALL enable the other or `validate`, and neither SHALL link yamled.
The `input` feature SHALL enable `input-yaml` and `validate`, and nothing else.
`check:features` SHALL compile `input-yaml` alone and `input-toml` alone.

#### Scenario: A consumer of input

- **WHEN** a crate enables `ctl-core` with `features = ["input"]`
- **THEN** `cargo tree` lists `ctl-input` and no `toml` or `yamled` crate, and every call to `Input::new`, `Input::read`, `Input::parse`, `Input::check`, and `use_repo_limits` compiles unchanged

#### Scenario: A consumer of both formats

- **WHEN** a crate enables `features = ["input", "input-toml"]`
- **THEN** `Input::open` reads `.yaml`, `.yml`, and `.toml` files

## REMOVED Requirements

### Requirement: The YAML document is parsed once per input

**Reason**: Placing no longer parses with yamled; every format reads its text into the located tree once.
**Migration**: Replaced by "The tree is read once per input".

## ADDED Requirements

### Requirement: The tree is read once per input

An `Input` SHALL keep the tree it reads.
It SHALL read its text again only when the limits in force change.
It SHALL also read it again when `parse` needs a strict tree and only a listing tree is kept.
A tree read for `parse` SHALL also serve `place`, `place_key`, `check` and `top_level`.
A listing tree SHALL never serve `parse`.

#### Scenario: Two placings

- **WHEN** `place` is called twice on one input of any format
- **THEN** the second call reuses the tree the first read

#### Scenario: A load

- **WHEN** `load` parses a YAML text and places a problem garde reports
- **THEN** the text is read into the tree once

#### Scenario: A parse after a listing

- **WHEN** `top_level` lists a YAML text that holds `prefix: a` and `prefix: b`, and `parse` runs on the same input
- **THEN** the parse reads a strict tree and fails with one problem that reads `` duplicate key `prefix` ``

### Requirement: Problems use one message set

Every problem SHALL have one problem type.
The types are syntax, duplicate key, merge key, expected kind, invalid value, unknown field, unknown variant, missing field, limit and alias.
It SHALL be worded by `input::Messages`, whose English default names no parser option.
`set_messages` SHALL replace the wording for the process without changing a problem's type, line or column.

#### Scenario: A duplicate key

- **WHEN** a YAML text holds `prefix: a` on line 1 and `prefix: b` on line 2
- **THEN** parsing fails with one problem on line 2 that reads `` duplicate key `prefix` ``

### Requirement: A type's own refusal keeps its text

An error a type's own `Deserialize` raises SHALL be an invalid value that keeps the type's text.
It SHALL sit at the node whose read raised it.

#### Scenario: A type's own refusal

- **WHEN** a YAML text holds `port: 0` on line 2 and `port` is a newtype whose `try_from` refuses 0 with `port must not be 0`
- **THEN** parsing fails with one problem on line 2 for `port` that reads `port must not be 0`

### Requirement: A missing field sits on the key that holds its mapping

A missing field SHALL be placed on the key whose value is the mapping that lacks it.
When the root mapping lacks it, it SHALL be placed on line 1, in every format.

#### Scenario: A missing nested field

- **WHEN** a YAML text holds `server:` on line 3 with a mapping below it that lacks the required `port`
- **THEN** parsing fails with one problem on line 3 that reads `` missing field `port` ``
