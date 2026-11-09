# features Specification

## Purpose

Which Clap features each ctl-core feature turns on, and why help always comes from ctl-core's renderer.
Also the one predicate that decides which characters verbatim text drops.

## Requirements

### Requirement: The cli feature turns on only the Clap features ctl-core calls

ctl-core SHALL declare `clap` with `default-features = false`. The `cli`
feature SHALL turn on only Clap's `std`, `derive`, `env`, and `string`
features. `help` and `usage` SHALL each add only Clap's `usage`, which
renders the usage line. No other ctl-core feature,
and no dependency of ctl-core, SHALL turn on Clap's `default` feature.

#### Scenario: A consumer with a minimal clap entry

- **WHEN** a consumer depends on ctl-core with `default-features = false, features = ["app"]` and on `clap` with `default-features = false, features = ["std", "derive"]`
- **THEN** `cargo tree -e features -i clap_builder` shows none of `color`, `help`, `error-context`, or `suggestions`

#### Scenario: A consumer that mounts a mise task and reads config

- **WHEN** a consumer depends on ctl-core with `default-features = false, features = ["app", "usage", "input"]` and on `clap` with `default-features = false, features = ["std", "derive"]`
- **THEN** `cargo tree -e features -i clap_builder` shows none of `default`, `color`, `help`, `error-context`, or `suggestions`

### Requirement: ctl-core declares its own help flag and subcommand

ctl-core's entry point SHALL declare `-h`/`--help` and the `help`
subcommand itself and render the help, so Clap's `help` feature stays off.
A plain `Cli::parse()` outside that entry point MAY get Clap's minimal
output and an unknown `help` subcommand.

#### Scenario: A consumer with only the cli feature

- **WHEN** a consumer builds ctl-core with `default-features = false, features = ["cli"]` and parses `-h`, `--help`, `-V`, or `help <command>` through `parser::apply_defaults`
- **THEN** Clap stops with `DisplayHelp` or `DisplayVersion`, or selects the `help` subcommand with the named command below it

### Requirement: Help always comes from ctl-core's renderer

When Clap stops a parse for help, `App` SHALL print ctl-core's help document
for the selected command. A help request SHALL print to stdout and exit 0. A
command that requires an argument or subcommand and gets neither SHALL
print to stderr and exit 2.

#### Scenario: A missing subcommand

- **WHEN** a toy CLI whose `status` subcommand sets `arg_required_else_help` runs `toy status`
- **THEN** stderr holds the help document for `status`, and the exit code is 2

#### Scenario: The version

- **WHEN** the toy CLI runs `toy --version`
- **THEN** stdout holds `toy` and its version, with no escape sequence, and the exit code is 0

### Requirement: One predicate decides which characters are unsafe to print verbatim

`ctl_core::verbatim::is_unsafe` SHALL return true for each of these
characters:

- Every member of general category Cc.
- All 12 directional formatting characters of UAX #9 at Unicode 16.0.0.
  They are ALM, LRM, RLM, LRE, RLE, PDF, LRO, RLO, LRI, RLI, FSI, and PDI.
- The one member each of Zl and Zp.

It SHALL return false for every other character.

#### Scenario: Every member of each set

- **WHEN** `is_unsafe` is called with each of the 65 Cc characters, the 12 directional formatting characters, U+2028, and U+2029
- **THEN** it returns true for each

#### Scenario: The neighbours and the other format characters

- **WHEN** `is_unsafe` is called with one of these:
  - a neighbour of each set, such as U+00A0, U+061B, U+2027, and U+206A
  - another Cf character, such as U+200B, U+200D, U+FEFF, or U+E0067
- **THEN** it returns false

#### Scenario: Nothing else

- **WHEN** `is_unsafe` is called with every Unicode scalar value
- **THEN** it returns true for exactly 79 of them

### Requirement: Verbatim document text uses the shared predicate

`Document::verbatim` text SHALL keep newlines and tabs and SHALL drop every
other character `verbatim::is_unsafe` refuses.

#### Scenario: Terminal controls in verbatim text

- **WHEN** a document holds the verbatim text `"\u{1b}]52;clipboard\u{7}\u{202e}\u{2028}\u{2029}\tvalue"`
- **THEN** the rendered text is `"]52;clipboard\tvalue"`
