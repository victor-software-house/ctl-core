# Spec Delta

## MODIFIED Requirements

### Requirement: JSON layout is a public option and never carries ANSI

The `json` format SHALL print a whole document in the layout set on `View`, and SHALL use `Pretty` when none is set.
Under `--stream` it SHALL write each record compact on one line, whatever the layout.
It SHALL never write an escape sequence into JSON output.

#### Scenario: Pretty JSON

- **WHEN** a consumer prints a model with `--format json` and the `Pretty` layout
- **THEN** the output is valid JSON indented by two spaces

#### Scenario: Forced colour

- **WHEN** a consumer prints JSON with `--color always` and `CLICOLOR_FORCE=1`
- **THEN** the output contains no `ESC` byte

#### Scenario: A stream ignores the layout

- **WHEN** a consumer sets the `Pretty` layout and runs a command with `--stream --json`
- **THEN** each record is on one line
