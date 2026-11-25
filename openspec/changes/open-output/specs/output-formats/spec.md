# Spec Delta

## Purpose

Lets a CLI choose, add, replace, or remove the formats its results are written in, and keeps machine formats plain.

## ADDED Requirements

### Requirement: A format is an implementation of a public interface

ctl-core SHALL define how a result becomes output through one public interface.
Its built-in formats SHALL be implementations of that interface, with no path a CLI's own format cannot take.

#### Scenario: A CLI adds a format

- **WHEN** a CLI registers a format named `yaml` and runs `toy status --format yaml`
- **THEN** the `yaml` format writes the result to stdout, with no change to ctl-core

#### Scenario: A CLI replaces a built-in

- **WHEN** a CLI registers its own format named `csv` and runs `toy status -f csv`
- **THEN** its format writes the result, and the built-in `csv` does not run

### Requirement: The built-in formats

ctl-core SHALL ship five formats: `pretty` for people, `json` for one JSON document, `tsv` for one record per line, `csv`, and `markdown`.
Every format except `pretty` SHALL be a machine format.

#### Scenario: Built-in names in help

- **WHEN** a CLI that registers no format of its own prints `--help`
- **THEN** `--format` lists `pretty`, `json`, `tsv`, `csv`, and `markdown`

### Requirement: The registered names are the only accepted names

`--format` SHALL accept exactly the registered names.
Help, completion, the usage spec, and a parse error SHALL list the same names.

#### Scenario: A removed format

- **WHEN** a CLI removes `markdown` and runs `toy status --format markdown`
- **THEN** the run exits 2 with a usage error that lists the remaining names

#### Scenario: Completion lists a custom format

- **WHEN** a CLI registers `yaml` and a shell completes the value after `toy status --format`
- **THEN** the candidates include `yaml`

### Requirement: The default follows the terminal

With no format named, output SHALL be the terminal default, `pretty`, when stdout is a terminal, and the piped default, `tsv`, when it is not.
A CLI SHALL be able to set either default, and setting the piped default to `pretty` SHALL turn the piped switch off.

#### Scenario: Piped list

- **WHEN** `toy list | cat` runs with no `--format`
- **THEN** stdout holds a tab-separated header line and one line per row, with no ESC byte

#### Scenario: A CLI keeps pretty when piped

- **WHEN** a CLI sets its piped default to `pretty` and `toy list | cat` runs
- **THEN** stdout holds the colourless pretty table

### Requirement: A name on the command line wins

The format in force SHALL be the one named on the command line, then the one named by a variable, then the default.
Two variables that name different formats SHALL be a usage error that names both.

#### Scenario: Variable under a flag

- **WHEN** `TOY_FORMAT=csv toy status --format json` runs
- **THEN** the output is JSON

#### Scenario: Two format variables

- **WHEN** `TOY_FORMAT=csv TOY_JSON=1 toy status` runs
- **THEN** the run exits 2 with a usage error that names `TOY_FORMAT` and `TOY_JSON`

#### Scenario: Variable over the default

- **WHEN** `TOY_FORMAT=csv toy status` runs on a terminal
- **THEN** the output is CSV

### Requirement: Shorthand flags

A format SHALL be able to declare a shorthand long flag that selects it.
`json` SHALL declare `--json`.
Two different format flags on one command line, such as `--json` and `--format csv`, SHALL be a usage error, exit 2.
A repeated `--format` keeps its last value, as every repeated flag does.

#### Scenario: --json

- **WHEN** `toy status --json` runs
- **THEN** the output is the same as `toy status --format json`

#### Scenario: Conflict

- **WHEN** `toy status --json --format csv` runs
- **THEN** the run exits 2 with a usage error naming both flags

#### Scenario: Same format twice

- **WHEN** `toy status --json --format json` runs
- **THEN** the output is JSON and the run exits 0

### Requirement: Machine formats never carry ANSI

A machine format SHALL write no escape sequence, whatever the colour mode and the colour variables say.

#### Scenario: Forced colour, piped

- **WHEN** `CLICOLOR_FORCE=1 FORCE_COLOR=3 toy list --format tsv --color always` runs
- **THEN** stdout contains no ESC byte

### Requirement: --quiet never hides a machine format that was named

`--quiet` SHALL hide a successful result in `pretty` and in a default machine format.
It SHALL NOT hide a machine format named by a flag, a shorthand, or a variable.

#### Scenario: Quiet with a named format

- **WHEN** `toy status --format csv --quiet` runs and succeeds
- **THEN** stdout holds the CSV output

#### Scenario: Quiet with a format variable

- **WHEN** `TOY_FORMAT=json toy status --quiet` runs and succeeds
- **THEN** stdout holds the JSON document

#### Scenario: Quiet with the piped default

- **WHEN** `toy apply --quiet | cat` runs with no format named and succeeds
- **THEN** stdout is empty

#### Scenario: Quiet with a named pretty

- **WHEN** `toy status --format pretty --quiet` runs and succeeds
- **THEN** stdout is empty

### Requirement: Tabular formats follow the view

`tsv`, `csv`, and `markdown` SHALL take their rows and columns from the result's view, in its column order.
A tab, line feed, carriage return, or backslash inside a `tsv` value SHALL be written as `\t`, `\n`, `\r`, or `\\`.

#### Scenario: A value with a tab

- **WHEN** a row's cell holds `a<TAB>b` and `toy list --format tsv` runs
- **THEN** that cell is written as `a\tb` and the row stays on one line

#### Scenario: CSV quoting

- **WHEN** a cell holds `a, "b"` and `toy list --format csv` runs
- **THEN** that cell is written as `"a, ""b"""`

### Requirement: Each format writes its own error form

A failure SHALL go to stderr in the format's error form, and stdout SHALL stay empty when no record was written.
`json` SHALL keep the error envelope with `error.kind`.

#### Scenario: A failure in csv

- **WHEN** `toy status --format csv` fails with the message `ledger missing`
- **THEN** stderr holds `toy: ledger missing`, stdout is empty, and the exit code is 1

#### Scenario: A parse error after a shorthand

- **WHEN** `toy status --json --bogus` runs
- **THEN** stderr holds a JSON error envelope and the exit code is 2
