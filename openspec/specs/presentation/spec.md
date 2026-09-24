# presentation Specification

## Purpose
How ctl-core lays out records, lists, JSON, and width, and which of those an owner or an operator can change.

## Requirements

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

### Requirement: Identifiers have their own role

ctl-core SHALL render text marked as an identifier bold in the terminal's
foreground colour, and SHALL keep the token style for flags and commands.

#### Scenario: Id column

- **WHEN** a consumer renders a list whose id column is set, with colour on
- **THEN** each id is wrapped in the bold escape and no colour escape
- **AND** a token in the same document keeps its token colour

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

### Requirement: The environment picks a style the owner left open

ctl-core SHALL read each style option from its environment variable when the
library owner has not set that option, and SHALL ignore the variable when the
owner has.

#### Scenario: Boxed record from the shell

- **WHEN** `CTL_CORE_RECORD_STYLE=boxed` is set and the consumer sets no record style
- **THEN** records render inside a full frame

#### Scenario: The owner's choice wins

- **WHEN** `CTL_CORE_JSON_LAYOUT=compact` is set and the consumer sets `JsonLayout::Pretty`
- **THEN** JSON output is indented by two spaces

### Requirement: Each variable name can be replaced or disabled

ctl-core SHALL let a consumer replace the ordered names read for each style
option, and SHALL read no variable when the list is empty.

#### Scenario: Disabled lookup

- **WHEN** a consumer sets `record_style_envs(&[])` and `CTL_CORE_RECORD_STYLE=boxed` is set
- **THEN** records render with keys right-aligned and no frame

### Requirement: An unaccepted value is named

ctl-core SHALL ignore a style variable whose value is not accepted, and `App`
SHALL print one warning line to stderr that names the accepted values.

#### Scenario: A typo in the list style

- **WHEN** `CTL_CORE_LIST_STYLE=boxes` is set and a consumer runs through `App`
- **THEN** stderr contains `CTL_CORE_LIST_STYLE=boxes is ignored; use one of grid, header-rule, plain`
- **AND** lists render in the default grid
