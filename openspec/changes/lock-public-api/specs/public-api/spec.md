## ADDED Requirements

### Requirement: Open enums take a wildcard arm

`document::Block`, `format::OutputFormat`, and `document::NoticeLevel` SHALL be `#[non_exhaustive]`.
Each one's rustdoc SHALL say that a match outside ctl-core ends with a wildcard arm.

#### Scenario: A consumer matches every block

- **WHEN** a crate outside ctl-core matches a `Block` on its eight variants with no wildcard arm
- **THEN** the match fails to compile with `E0004`

### Requirement: Growing structs are built by ctl-core

These structs SHALL be `#[non_exhaustive]`: `model::ErrorBody`, `flags::FlagWarning`, `Surface`, `SurfaceCommand`, `SurfaceArgument`, `input::InputError`, and `served::ServeError`.
Both variants of `model::Envelope` SHALL be `#[non_exhaustive]`.
`Envelope::ok`, `Envelope::err`, and `ErrorBody::new` SHALL build the envelope.

#### Scenario: A consumer builds an error body

- **WHEN** a crate outside ctl-core writes `ErrorBody { bin, message }`
- **THEN** it fails to compile, and `ErrorBody::new(bin, message)` builds the same value

#### Scenario: A consumer reads an envelope

- **WHEN** a crate outside ctl-core matches `Envelope::Err { error, .. }`
- **THEN** the match compiles and binds the error body

### Requirement: Flattened flag structs grow without a break

These clap `Args` structs SHALL be `#[non_exhaustive]`: `OutputArgs`, `FormatArgs`, `FormatLong`, `ColorLong`, `DryRunArgs`, `CompletionArgs`, and `InitArgs`.
`CompletionCommand` and `InitCommand` SHALL stay exhaustive.

#### Scenario: A test matches the completion verb

- **WHEN** a test matches `CompletionCommand::Completion(CompletionArgs { shell: Shell::Fish, .. })`
- **THEN** the match compiles and holds for `toy completion fish`

### Requirement: The colour flags have one reader

`OutputArgs` and `ColorLong` SHALL keep `color` and `no_color` private.
`color()` SHALL be the only way to read the colour mode they parse.

#### Scenario: --no-color after --color always

- **WHEN** a CLI that flattens `OutputArgs` parses `--color always --no-color`
- **THEN** `color()` returns `ColorMode::Never`, and no public field reads `always`

### Requirement: An error keeps the kind of its I/O failure

`InputError::io_kind()` and `ServeError::io_kind()` SHALL return the `io::ErrorKind` of the I/O failure that caused the error.
They SHALL return `None` for a refusal no I/O failure caused.
Both types SHALL keep deriving `Clone`, `PartialEq`, and `Eq`.

#### Scenario: A missing input file

- **WHEN** `Input::open` reads `missing.yaml`, which does not exist
- **THEN** the error's `io_kind()` is `Some(io::ErrorKind::NotFound)`

#### Scenario: A refused value

- **WHEN** `Input::open` reads a file whose text fails to parse
- **THEN** the error's `io_kind()` is `None`

### Requirement: View settings are read through methods

`View`'s format and quiet setting SHALL be private.
`View::format()` SHALL return the format, and `View::is_quiet()` the quiet setting.
`View::quiet(bool)` SHALL stay the setter.

#### Scenario: A quiet JSON view

- **WHEN** a caller builds `View::new(OutputFormat::Json, ColorMode::Never).quiet(true)`
- **THEN** `format()` returns `OutputFormat::Json` and `is_quiet()` returns `true`

### Requirement: EnvPrefix is sealed

`flags::EnvPrefix` SHALL have a supertrait that no crate outside ctl-core can name.
`clap::Command` SHALL be its only implementation.
The trait SHALL keep its path and its prelude export.

#### Scenario: A consumer implements EnvPrefix

- **WHEN** a crate outside ctl-core writes `impl EnvPrefix for MyCommand`
- **THEN** it fails to compile, because `MyCommand` does not implement the sealed supertrait

### Requirement: A closed enum says why it is closed

These enums SHALL NOT be `#[non_exhaustive]`: `Outcome`, `Destination`, `BrokenPipe`, `ColorMode`, `SurfaceRequirement`, `SurfaceScope`, and `FlagClash`.
Each one's rustdoc SHALL say in one sentence why its set is closed.

#### Scenario: A consumer matches every outcome

- **WHEN** a crate outside ctl-core matches `Outcome::Success` and `Outcome::Error` with no wildcard arm
- **THEN** the match compiles
