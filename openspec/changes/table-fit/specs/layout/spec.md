## MODIFIED Requirements

### Requirement: Ellipsis columns give way before words break

When the narrowest widths still do not fit, a table SHALL shrink ellipsis and fold columns down to their floor.
It SHALL do so before a keep-words column runs past the width.

#### Scenario: An ellipsis title

- **WHEN** a list with a keep-words path column of 30 cells and an ellipsis title column renders at width 40
- **THEN** the path stays whole, the title column is at least 8 cells, and each title that does not fit ends in `…`

## ADDED Requirements

### Requirement: No column narrows below its floor

A column's floor SHALL be its header width or the floor setting, 8 cells by default, whichever is larger, and no more than its natural width.
A table SHALL NOT render a column narrower than its floor.

#### Scenario: A wide list at 80 columns

- **WHEN** a grid list of six columns, each with 30-cell values and a 4-cell header, renders at width 80
- **THEN** no column is narrower than 8 cells

#### Scenario: A short column keeps its width

- **WHEN** a column's widest cell and header are 3 cells
- **THEN** the column is 3 cells wide

### Requirement: A table that cannot fit at its floors stacks

When a table's floors and frame do not fit the width, the table SHALL draw each row as a record: the first column's value as its title, then one header and value line per other column, rows separated by a blank line.
A table with a `stacked_below` threshold SHALL keep that threshold's form.

#### Scenario: Six columns at width 40

- **WHEN** a list with six 8-cell floors renders at width 40 in a grid
- **THEN** each row renders as a record whose lines name each column's header

#### Scenario: Help keeps its threshold

- **WHEN** a help table stacked below 64 renders at width 40
- **THEN** each flag prints on its own line, indented by 2, with its description below, indented by 4
