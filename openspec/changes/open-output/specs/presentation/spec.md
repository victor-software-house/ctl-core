## MODIFIED Requirements

### Requirement: Undetected width uses a fallback width

ctl-core SHALL lay out `pretty` output to the fallback width when neither the stream being written nor `COLUMNS` gives a width.
It SHALL let a consumer change or disable the fallback width.
The fallback SHALL NOT change the minimum applied to a detected width.

#### Scenario: Everything piped

- **WHEN** stdout is a pipe, `COLUMNS` is unset, and the format is `pretty`
- **THEN** no rendered line of a schema-4 `qctl show` is wider than 80 columns

#### Scenario: Fallback disabled

- **WHEN** a consumer sets the fallback width to `None` and no width is detected
- **THEN** the table renders at its natural width

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
