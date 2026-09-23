# Presentation

## ADDED Requirements

### Requirement: Record style is a public option

ctl-core SHALL render a `Fields` record in the record style set on
`RenderOptions`, and SHALL use the default recorded in the design when none is
set.

#### Scenario: Borderless record

- **WHEN** a consumer renders a two-field record with the `Borderless` record style at width 80
- **THEN** the output contains no box-drawing characters
- **AND** each key is right-aligned in one column

#### Scenario: Consumer keeps the box

- **WHEN** a consumer sets the `Boxed` record style
- **THEN** the record renders inside a full frame, as before this change

### Requirement: List style is a public option

ctl-core SHALL render a `Table` list in the list style set on `RenderOptions`.

#### Scenario: Header rule

- **WHEN** a consumer renders a three-row list with the `HeaderRule` style
- **THEN** exactly one horizontal rule appears, directly under the header
- **AND** no vertical rule appears

### Requirement: Row separation is a public option

ctl-core SHALL separate rows only when row separation is set, and SHALL
default to no separation.

#### Scenario: Rule between rows

- **WHEN** a consumer renders a three-row list with row separation `Rule`
- **THEN** a horizontal rule appears between each pair of rows

#### Scenario: Default

- **WHEN** row separation is not set
- **THEN** rows print on consecutive lines with no rule and no blank line

### Requirement: JSON layout is a public option and never carries ANSI

ctl-core SHALL print JSON in the layout set on `RenderOptions`, and SHALL
never write an escape sequence into JSON output.

#### Scenario: Pretty JSON

- **WHEN** a consumer prints a model with `--format json` and the `Pretty` layout
- **THEN** the output is valid JSON indented by two spaces

#### Scenario: Forced colour

- **WHEN** a consumer prints JSON with `--color always` and `CLICOLOR_FORCE=1`
- **THEN** the output contains no `ESC` byte

### Requirement: Automatic width keeps a two-column buffer

ctl-core SHALL subtract 2 columns from an automatically detected width by
default, and SHALL keep an explicit width exact.

#### Scenario: Detected width

- **WHEN** stdout is not a terminal and `COLUMNS=119`
- **THEN** no rendered line is wider than 117 columns

#### Scenario: Explicit width

- **WHEN** a consumer sets an explicit width of 100
- **THEN** the renderer lays out to 100 columns with no buffer

### Requirement: Undetected width uses a fallback width

ctl-core SHALL lay out to the fallback width when neither stdout nor
`COLUMNS` gives a width, and SHALL let a consumer change or disable it. The
fallback SHALL NOT change the minimum applied to a detected width.

#### Scenario: Everything piped

- **WHEN** stdout is a pipe and `COLUMNS` is unset
- **THEN** no rendered line of a schema-4 `qctl show` is wider than 80 columns

#### Scenario: Fallback disabled

- **WHEN** a consumer sets the fallback width to `None` and no width is detected
- **THEN** the table renders at its natural width
