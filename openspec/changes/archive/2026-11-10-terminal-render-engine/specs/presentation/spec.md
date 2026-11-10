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
written nor `COLUMNS` gives a width.
It SHALL let a consumer change or disable the fallback width.
The fallback SHALL NOT change the minimum applied to a detected width.

#### Scenario: Everything piped

- **WHEN** stdout is a pipe and `COLUMNS` is unset
- **THEN** no rendered line of a schema-4 `qctl show` is wider than 80 columns

#### Scenario: Fallback disabled

- **WHEN** a consumer sets the fallback width to `None` and no width is detected
- **THEN** the table renders at its natural width

### Requirement: List style is a public option

ctl-core SHALL render a `Table` list in the frame set on `RenderOptions`
(`frame`), and SHALL use `Frame::Grid` when none is set.

#### Scenario: Header rule

- **WHEN** a consumer renders a three-row list with the `HeaderRule` frame
- **THEN** exactly one horizontal rule appears, directly under the header
- **AND** no vertical rule appears

### Requirement: Each variable name can be replaced or disabled

ctl-core SHALL let a consumer replace the ordered names read for each style
option, and SHALL read no variable when the list is empty.

#### Scenario: Disabled lookup

- **WHEN** a consumer sets `record_style_envs(&[])` and `CTL_CORE_RECORD_STYLE=boxed` is set
- **THEN** records render with keys right-aligned and no frame

## ADDED Requirements

### Requirement: A notice separator takes the plain role's style

ctl-core SHALL draw the separators of a notice in the style the theme gives
`Role::Plain`, as it draws the plain text of a paragraph.

#### Scenario: A styled plain role

- **WHEN** a theme gives `Role::Plain` an underline and a document holds a warning notice, rendered with colour at width 60
- **THEN** the ` · ` after the label is written inside an underline sequence
