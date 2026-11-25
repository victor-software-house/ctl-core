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
Naming two different formats on one command line SHALL be a usage error, exit 2.

#### Scenario: --json

- **WHEN** `toy status --json` runs
- **THEN** the output is the same as `toy status --format json`

#### Scenario: Conflict

- **WHEN** `toy status --json --format csv` runs
- **THEN** the run exits 2 with a usage error naming both flags

#### Scenario: Same format twice

- **WHEN** `toy status --json --format json` runs
- **THEN** the output is JSON and the run exits 0

### Requirement: There is no JSON Lines format

ctl-core SHALL NOT register a `jsonl` format or a `--jsonl` flag.
JSON one object per line SHALL be the `json` format's stream delivery.

#### Scenario: jsonl is unknown

- **WHEN** `toy status --format jsonl` runs
- **THEN** the run exits 2 with a usage error

### Requirement: --stream writes records as they are produced

Under `--stream`, a command that returns records SHALL write each record when it is produced, header record first.
In `json`, each record SHALL be one compact JSON object with a string `type` field, on its own line ending in LF.

#### Scenario: Records arrive one by one

- **WHEN** `toy events --stream --json` runs and the command yields three records a second apart
- **THEN** stdout holds four lines, the header first, and each line is written before the next record is produced

#### Scenario: Every record is typed

- **WHEN** a reader parses each line of `toy events --stream --json`
- **THEN** each line is a JSON object whose `type` field is a string

### Requirement: Any format can give a stream form

A format SHALL give a stream form through the public format interface, on the same terms for a CLI's own format as for a built-in.
`json` SHALL have a stream form.
`--stream` with a format that has none SHALL be a usage error, exit 2, before the command runs.

#### Scenario: A CLI's format streams

- **WHEN** a CLI registers `yaml` with a stream form and runs `toy events --stream --format yaml`
- **THEN** the `yaml` format receives each record when the command produces it

#### Scenario: Pretty does not stream

- **WHEN** `toy events --stream --format pretty` runs
- **THEN** the run exits 2 with a usage error, and the command does not run

### Requirement: --stream alone uses the stream default

`--stream` with no format named SHALL use the stream default, `json`, and a CLI SHALL be able to change it.

#### Scenario: --stream on a terminal

- **WHEN** `toy events --stream` runs on a terminal with no format variable set
- **THEN** stdout holds one JSON object per line, the header first

### Requirement: A stream written whole is one document

Without `--stream`, a command that returns records SHALL write them as one document once they are all produced.
In `json`, that document SHALL be one array of the same records, header first.

#### Scenario: Whole JSON of a stream

- **WHEN** `toy events --json` runs and the command yields three records
- **THEN** stdout holds one JSON array of four objects, each with a `type` field

### Requirement: A failure during a stream

When a record fails after others were written, the written records SHALL stay on stdout.
The failure SHALL go to stderr in the format's error form, and the run SHALL exit with the failure's code.
A `json` array SHALL be closed before the failure is written.

#### Scenario: The third record fails

- **WHEN** `toy events --json` runs and the third record is an error with exit code 3
- **THEN** stdout parses as a JSON array of the header and two records, stderr holds the error envelope, and the exit code is 3

### Requirement: A closed reader ends a stream quietly

When the reader closes stdout during a stream, ctl-core SHALL stop asking the command for records and exit as the closed-pipe policy says.

#### Scenario: head

- **WHEN** `toy events --stream --json | head -n 1` runs against an endless stream
- **THEN** the run ends, writes nothing to stderr, and exits 0 under the default policy

### Requirement: Machine formats never carry ANSI

A machine format SHALL write no escape sequence, whatever the colour mode and the colour variables say.

#### Scenario: Forced colour, piped

- **WHEN** `CLICOLOR_FORCE=1 FORCE_COLOR=3 toy list --format tsv --color always` runs
- **THEN** stdout contains no ESC byte

### Requirement: --quiet never hides a machine format that was named

`--quiet` SHALL hide a successful result in `pretty` and in a default format.
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
