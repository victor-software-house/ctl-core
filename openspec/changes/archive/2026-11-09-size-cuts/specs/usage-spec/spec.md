## Purpose

The Usage KDL text ctl-core writes for a clap command. It names the
reference output, the strings that must stay intact, and what mise must do
with the spec.

## ADDED Requirements

### Requirement: The spec matches usage-lib 7.3.0

For every command in the committed corpus, `usage::spec` SHALL return the
bytes usage-lib 7.3.0 returns for the same command and bin name. The
values the next requirement covers are the only exception.

#### Scenario: The corpus

- **WHEN** the corpus test renders the every-attribute command, the toy `App` command mounted as `toy`, and each hostile-string command
- **THEN** each output equals its committed snapshot, captured from usage-lib 7.3.0 in a real run

#### Scenario: A multiline value holding four quotes and a hash

- **WHEN** a flag's help is `x`, a newline, `""""#`, a newline, and `y`
- **THEN** the help is written as a raw string opened with `##"""` and closed with `"""##`

### Requirement: Every string reads back as written

Every string the spec holds SHALL be valid KDL v2 and SHALL read back,
under a KDL v2 parser, as the value written. A value that a plain or raw
multiline string cannot hold SHALL be written as a quoted string with
escapes.

#### Scenario: A line separator in help text

- **WHEN** a flag's help is `a`, U+2028, and `b`
- **THEN** the spec writes the help as `"a\u{2028}b"`, and a KDL v2 parser reads back `a`, U+2028, `b`

#### Scenario: A direction control in a default value

- **WHEN** a flag's default value is `a`, U+202E, and `c`
- **THEN** the spec writes the default as `"a\u{202e}c"`, and a KDL v2 parser reads back `a`, U+202E, `c`

#### Scenario: A whitespace-only line in help text

- **WHEN** a flag's help is `a`, a newline, three spaces, a newline, and `b`
- **THEN** a KDL v2 parser reads back the help with the three spaces kept

#### Scenario: Quotes, backslashes, tabs, and non-ASCII text

- **WHEN** flags' help texts are `say "hi"`, `a\b`, `a`, a tab, and `b`, and `café 日本`
- **THEN** a KDL v2 parser reads back each help text unchanged

### Requirement: mise reads the spec

mise SHALL parse the spec of a CLI mounted through `mount_directives`, show
its help, and complete its arguments.

#### Scenario: Help and completion through mise 2026.11.4

- **WHEN** a mise file task carries `mount_directives("toy")` and runs the toy CLI, under mise 2026.11.4
- **THEN** `mise run toy --help` prints the toy's subcommands and flags, and zsh completion after `mise run toy` and a space offers the toy's subcommands

### Requirement: The usage feature compiles no KDL crate

ctl-core with the `usage` feature SHALL have no normal dependency on
usage-lib, kdl, or winnow below 1.0.

#### Scenario: The dependency tree with every feature

- **WHEN** `cargo tree --package ctl-core --edges normal --all-features --target all` runs
- **THEN** it lists none of usage-lib, kdl, or a winnow version below 1.0
