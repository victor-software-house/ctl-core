## ADDED Requirements

### Requirement: The column floor is a public option

ctl-core SHALL take the column floor from `RenderOptions`, `View`, and `App`, then from `CTL_CORE_COLUMN_FLOOR`, then 8 cells.
A floor of 0 SHALL leave each column at the limit its overflow sets.

#### Scenario: Floor from the environment

- **WHEN** `CTL_CORE_COLUMN_FLOOR=12` is set and a consumer sets no floor
- **THEN** no column with values wider than 12 cells renders narrower than 12 cells

#### Scenario: Floor off

- **WHEN** a consumer sets a floor of 0
- **THEN** an ellipsis column with a 5-cell header can narrow to 1 cell, as before this change

### Requirement: The table fit is a public option

ctl-core SHALL take the table fit from the table, then `RenderOptions`, `View`, and `App`, then `CTL_CORE_TABLE_FIT`, then `stack`.
Under `wrap`, a table that cannot fit at its floors SHALL keep its grid at its floors and run past the width, framed or not.

#### Scenario: Wrap from the environment

- **WHEN** `CTL_CORE_TABLE_FIT=wrap` is set and a six-column list with 8-cell floors renders at width 40
- **THEN** the grid is drawn, with every column at its floor

#### Scenario: An unaccepted value

- **WHEN** `CTL_CORE_TABLE_FIT=fold` is set
- **THEN** the run prints a warning that names the variable and the accepted values `stack` and `wrap`
