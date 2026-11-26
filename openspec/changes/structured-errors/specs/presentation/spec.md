## ADDED Requirements

### Requirement: A JSON error keeps a field per part

The JSON error envelope SHALL carry each part of a failure in its own field: `message` as one sentence, `causes`, `problems`, `usage` and `parse`, each left out when empty.
No field SHALL carry a part that has a field of its own, and no field SHALL contain ANSI.

#### Scenario: An unknown argument

- **WHEN** a CLI built with `app` runs `toy show --bogus --format json`
- **THEN** stderr is one envelope whose `message` is one sentence, whose `parse.reason` is `unknown_argument`, and whose `usage` is `toy show [OPTIONS] <ID>`

#### Scenario: The parts clap's error-context gives

- **WHEN** the same CLI is built with clap's `error-context` and runs the same command
- **THEN** `parse.argument` is `--bogus` and `parse.tips` holds clap's tip, and `message` holds neither

#### Scenario: A refused input

- **WHEN** an input with two unknown keys is refused under `--format json`
- **THEN** `problems` holds two objects, each with its `input`, `line`, `column`, `field` and `message`, and `message` names the input and the two problems

#### Scenario: A handler error with causes

- **WHEN** a handler returns an error `reading config` caused by `no such file`
- **THEN** `message` is `reading config` and `causes` is `["no such file"]`

### Requirement: Pretty errors read as before

The `pretty` error SHALL render `message`, then each problem as `{bin}: {input}:{line}:{column}: {field}: {message}`, then each cause, then the tips and the usage line.

#### Scenario: A handler error in pretty

- **WHEN** the same handler error is written by `pretty`
- **THEN** stderr reads `toy: reading config`, then `caused by: no such file`
