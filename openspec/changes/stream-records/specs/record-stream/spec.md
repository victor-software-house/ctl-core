# Spec Delta

## Purpose

Lets a command write its records as it produces them, in the order produced, in any registered format.

## ADDED Requirements

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

### Requirement: Every format streams

Every format SHALL have a stream form that writes each record when it is produced, in the order produced.
A format that defines no stream form of its own SHALL write each record as it writes a whole result.
A CLI's own format SHALL stream through the same interface as a built-in.

#### Scenario: A CLI's format streams

- **WHEN** a CLI registers `yaml` with no stream form of its own and runs `toy events --stream --format yaml`
- **THEN** each record is written as a YAML document when the command produces it

#### Scenario: tsv writes the header once

- **WHEN** `toy events --stream --format tsv` runs and three records of one table arrive
- **THEN** stdout holds one header line, then one line per record as it arrives

#### Scenario: Pretty streams

- **WHEN** `toy events --stream --format pretty` runs and the command yields three records a second apart
- **THEN** each record is rendered before the next one is produced, with a blank line between records

### Requirement: --stream alone uses the stream default

`--stream` with no format named SHALL use the stream default, `json`, and a CLI SHALL be able to change it.

#### Scenario: --stream on a terminal

- **WHEN** `toy events --stream` runs on a terminal with no format variable set
- **THEN** stdout holds one JSON object per line, the header first

### Requirement: A single value streams as it serializes

Under `--stream`, a command that returns one value SHALL write it as one record, in the form the value serializes to.
ctl-core SHALL NOT add a `type` field to it; the `type` field is required only of the records a command returns as `Records`.

#### Scenario: A single value under --stream

- **WHEN** `toy status --stream --json` runs and the command returns one value with no `type` field
- **THEN** stdout holds that value as one compact JSON value on one line, with no `type` field

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

### Requirement: --quiet hides the stream default

`--quiet` SHALL treat the stream default as a default format: it SHALL hide the stream default's records, and SHALL NOT hide a machine format named for the stream.

#### Scenario: Quiet with the stream default

- **WHEN** `toy events --stream --quiet` runs with no format named and succeeds
- **THEN** stdout is empty

#### Scenario: Quiet with a named stream format

- **WHEN** `toy events --stream --json --quiet` runs and succeeds
- **THEN** stdout holds one JSON object per record
