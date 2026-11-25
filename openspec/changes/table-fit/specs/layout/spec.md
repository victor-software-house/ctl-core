# Spec Delta

## MODIFIED Requirements

### Requirement: Ellipsis columns give way before words break

When the narrowest widths still do not fit, a table SHALL shrink ellipsis and fold columns down to their floor.
It SHALL do so before a keep-words column runs past the width.

#### Scenario: An ellipsis title

- **WHEN** an unframed list with a keep-words path column of 30 cells and an ellipsis title column renders at width 40
- **THEN** the path stays whole, the title column is at least 8 cells, and each title that does not fit ends in `…`

### Requirement: A framed table folds instead of overflowing

A table with an outer frame SHALL fold a word wider than its column, so the frame stays aligned.
It SHALL NOT fold a column below its floor.
Under the `wrap` table fit, a framed table that cannot fit at its floors SHALL keep its grid at its floors and run past the width.

#### Scenario: A URL in a grid

- **WHEN** a grid list holds a 60-cell URL and renders at width 40
- **THEN** every line of the grid is at most 40 cells wide

#### Scenario: Six paths in a grid

- **WHEN** a grid list of six columns of 30-cell paths renders at width 40
- **THEN** no column folds below 8 cells, and the rows render as stacked records

## ADDED Requirements

### Requirement: No column narrows below its floor

When the floor setting is above 0, a column's floor SHALL be its header width or the floor setting, 8 cells by default, whichever is larger, and no more than its natural width.
When the floor setting is 0, every column's floor SHALL be 0.
A table SHALL NOT render a column narrower than its floor.

#### Scenario: A wide list at 80 columns

- **WHEN** a grid list of six columns, each with 30-cell values and a 4-cell header, renders at width 80
- **THEN** no column is narrower than 8 cells

#### Scenario: A short column keeps its width

- **WHEN** a column's widest cell and header are 3 cells
- **THEN** the column is 3 cells wide

### Requirement: A table that cannot fit at its floors stacks

When a table's floors and frame do not fit the width, the table SHALL draw each row as a record.
A record SHALL show the first column's value as its title, then one header and value line per other column.
A blank line SHALL separate the records.

A table with no headers SHALL stack each row as its first cell over the others.
A table with a `stacked_below` threshold SHALL keep that threshold's form.

#### Scenario: Six columns at width 40

- **WHEN** a list with six 8-cell floors renders at width 40 in a grid
- **THEN** each row renders as a record whose lines name each other column's header

#### Scenario: A headerless table

- **WHEN** a headerless table of six 8-cell-floor columns renders at width 40
- **THEN** each row prints its first cell indented by 2 and the other cells below it, indented by 4

#### Scenario: Help keeps its threshold

- **WHEN** a help table stacked below 64 renders at width 40
- **THEN** each flag prints on its own line, indented by 2, with its description below, indented by 4

### Requirement: A table with no rows draws its grid

A table with no rows SHALL draw its grid, header only, and SHALL NOT stack.

#### Scenario: A table with no rows

- **WHEN** a table with headers and no rows renders at a width where its floors do not fit
- **THEN** it draws its grid, header only, and never stacks
