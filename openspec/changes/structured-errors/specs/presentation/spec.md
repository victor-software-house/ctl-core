## ADDED Requirements

### Requirement: A JSON error keeps a field per part

The JSON error envelope SHALL carry each part of a failure in its own field: `message` as one sentence, `causes`, `problems`, `usage` and `parse`, each left out when empty.
No field SHALL carry a part that has a field of its own, and no field SHALL contain ANSI.

#### Scenario: An unknown argument

- **WHEN** a CLI built with `app` runs `toy show --bogus --format json`
- **THEN** stderr is one envelope whose `message` is one sentence, whose `parse.reason` is `unknown_argument`, and whose `usage` is `toy show [OPTIONS] <ID>`

#### Scenario: The parts clap's error-context gives

- **WHEN** the same CLI is built with ctl-core's `error-context` feature and runs the same command
- **THEN** `parse.argument` is `--bogus` and `parse.tips` holds clap's tip, and `message` holds neither

#### Scenario: Missing arguments

- **WHEN** a CLI built with ctl-core's `error-context` runs `toy copy a` and `copy` needs `<FROM>` and `<TO>`
- **THEN** `parse.missing` is `["<TO>"]`, and pretty prints clap's list as it did before

#### Scenario: A refused input

- **WHEN** an input with two problems is refused under `--format json`
- **THEN** `problems` holds two objects, each with its `input`, `line`, `column`, `field` and `message`, and `message` names the input and the two problems

#### Scenario: A handler error with causes

- **WHEN** a handler returns an error `reading config` caused by `no such file`
- **THEN** `message` is `reading config` and `causes` is `["no such file"]`

### Requirement: Pretty errors read as before

The `pretty` error SHALL print the text a person reads, kept beside the parts, so its output is byte-identical to what it printed before this change, in a build with ctl-core's `error-context` and in one without.

#### Scenario: A handler error in pretty

- **WHEN** the same handler error is written by `pretty`
- **THEN** stderr reads `toy: reading config: no such file`, as it does today
