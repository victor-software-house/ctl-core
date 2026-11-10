# layout Specification

## Purpose

What every widget of the rendering engine guarantees about the
width of what it draws.
This also covers how a table chooses its column widths, aligns its cells and
fits a narrow terminal.

## Requirements

### Requirement: A widget reports its narrowest and widest width

Every widget SHALL report, without drawing, the narrowest width it can be
drawn at without breaking a unit it keeps whole, and its natural width.

#### Scenario: A record

- **WHEN** a record with the keys `ledger` and `active` and the values `tasks.yaml` and `QCTL-014` is measured
- **THEN** its widest width is 18 cells, the longest key plus two spaces plus the longest value

### Requirement: A widget fits the width it is given

Every widget SHALL draw no line wider than the width it is given, except a
line that holds a word kept whole.
That line SHALL be wider only by that word.

#### Scenario: Random content

- **WHEN** any widget renders any text at any width from 1 to 200
- **THEN** every line obeys the rule, as the property tests check

### Requirement: Rendering is deterministic

The engine SHALL produce the same bytes for the same document, width and
colour decision.

#### Scenario: Two renders

- **WHEN** the same document renders twice at width 80 with colour off
- **THEN** both outputs are byte-identical

### Requirement: A table that fits takes its natural widths

When the natural widths of a table's columns, with its frame, fit the width,
the table SHALL give each column its natural width.

#### Scenario: Short rows

- **WHEN** a header-rule list of three rows whose widest line is 50 cells renders at width 80
- **THEN** the list is 50 cells wide

### Requirement: A table shrinks its widest wrappable column first

When a table does not fit, it SHALL shrink the widest wrappable column toward
the next widest, repeating until the table fits.
It SHALL NOT shrink a column below its narrowest width in this step.

#### Scenario: One long title

- **WHEN** a list with an id column of 8 cells and a title column of 70 cells renders at width 40
- **THEN** the id column keeps 8 cells and the title wraps

### Requirement: A wrapped column narrows to its widest line

After it renders the cells, a table SHALL narrow each column to its widest
rendered line, and SHALL NOT widen a column in this step.

#### Scenario: A title that wraps short of its width

- **WHEN** a grid list renders at width 40 with an id column of 8 cells. The title is eight 7-letter words and one 6-letter word
- **THEN** the title column is 23 cells, its widest wrapped line, and the grid is 38 cells wide

### Requirement: Ellipsis columns give way before words break

When the narrowest widths still do not fit, a table SHALL shrink ellipsis and
fold columns down to 1 cell.
It SHALL do so before a keep-words column runs past the width.

#### Scenario: An ellipsis title

- **WHEN** a list with a keep-words path column of 30 cells and an ellipsis title column renders at width 40
- **THEN** the path stays whole and each title that does not fit ends in `…`

### Requirement: A framed table folds instead of overflowing

A table with an outer frame SHALL fold a word wider than its column, so the
frame stays aligned.

#### Scenario: A URL in a grid

- **WHEN** a grid list holds a 60-cell URL and renders at width 40
- **THEN** every line of the grid is at most 40 cells wide

### Requirement: A numeric column aligns right

A table SHALL align every cell of a column marked numeric to the right,
headers included.

#### Scenario: Ages

- **WHEN** a list with a numeric `age` column holds `3` and `120`
- **THEN** the `3` ends in the same cell as the `120`

### Requirement: A narrow table stacks

A table with a stacking threshold SHALL render each row as indented label and
description lines when the width is below the threshold.
A table does this today.

#### Scenario: Help at width 40

- **WHEN** a help table stacked below 64 renders at width 40
- **THEN** each flag prints on its own line, indented by 2, with its description below, indented by 4
