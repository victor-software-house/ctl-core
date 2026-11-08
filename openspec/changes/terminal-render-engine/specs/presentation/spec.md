## MODIFIED Requirements

### Requirement: Automatic width keeps a two-column buffer

ctl-core SHALL subtract 2 columns from an automatically detected width by
default, and SHALL keep an explicit width exact. The detected width SHALL come
from the stream being written, stdout for a result and stderr for help printed
after a usage error.

#### Scenario: Detected width

- **WHEN** stdout is not a terminal and `COLUMNS=119`
- **THEN** no rendered line is wider than 117 columns

#### Scenario: Explicit width

- **WHEN** a consumer sets an explicit width of 100
- **THEN** the renderer lays out to 100 columns with no buffer

#### Scenario: Help on a narrow stderr

- **WHEN** stdout is a pipe, stderr is a terminal 50 columns wide, and a usage error prints help to stderr
- **THEN** no line of the help is wider than 48 columns

### Requirement: Undetected width uses a fallback width

ctl-core SHALL lay out to the fallback width when neither the stream being
written nor `COLUMNS` gives a width, and SHALL let a consumer change or
disable it. The fallback SHALL NOT change the minimum applied to a detected
width.

#### Scenario: Everything piped

- **WHEN** stdout is a pipe and `COLUMNS` is unset
- **THEN** no rendered line of a schema-4 `qctl show` is wider than 80 columns

#### Scenario: Fallback disabled

- **WHEN** a consumer sets the fallback width to `None` and no width is detected
- **THEN** the table renders at its natural width
