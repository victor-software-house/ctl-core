## Purpose

How the rendering engine turns text into styled spans: which characters from
outside the program it removes, how it counts width in terminal cells, and how
it wraps, folds and truncates a line.

## ADDED Requirements

### Requirement: Unsafe characters are removed from every span

The engine SHALL remove from the text of every span the C0 and C1 control
characters, including ESC, the twelve UAX #9 directional formatting
characters, and U+2028 and U+2029, except newline and tab, which it handles as
the next two requirements state.

#### Scenario: An escape sequence in a title

- **WHEN** a table cell holds `fix \u{1b}[2Jthe build`
- **THEN** the rendered cell reads `fix [2Jthe build`
- **AND** the output holds no ESC byte other than the engine's own styling

#### Scenario: A right-to-left override

- **WHEN** a record value holds `abc\u{202e}def`
- **THEN** the rendered value reads `abcdef`

### Requirement: A newline starts a new line

The engine SHALL split span text at each newline, and SHALL render each part
as its own line.

#### Scenario: Two-line value

- **WHEN** a paragraph holds `first\nsecond` at width 80
- **THEN** the output is the two lines `first` and `second`

### Requirement: A tab expands to the next tab stop

The engine SHALL replace each tab with spaces up to the next multiple of 8
cells from the start of its line, and SHALL let a caller set another tab
width.

#### Scenario: Default tab stop

- **WHEN** a paragraph holds `ab\tc`
- **THEN** the output is `ab` followed by 6 spaces and `c`

### Requirement: Width is counted per grapheme cluster

The engine SHALL count the width of text as the sum, over its extended
grapheme clusters, of each cluster's width in terminal cells, and SHALL count
East Asian ambiguous characters as 1 cell.

#### Scenario: A family emoji

- **WHEN** the engine measures the ZWJ sequence for a family of four
- **THEN** its width is 2

#### Scenario: Wide characters

- **WHEN** the engine measures `日本`
- **THEN** its width is 4

### Requirement: The Unicode versions are stated

The engine SHALL expose the Unicode version of its width data and of its
segmentation data.

#### Scenario: A dependency upgrade

- **WHEN** a dependency upgrade changes either Unicode version
- **THEN** the test that pins both versions fails until the pin is updated

### Requirement: A word is kept whole unless the caller allows a fold

When wrapping with the keep-words policy, the engine SHALL break lines only at
whitespace, and SHALL let a word wider than the width run past it.

#### Scenario: A long URL

- **WHEN** a paragraph holds `see https://example.test/releases/tag/pkg@1.2.3` at width 30
- **THEN** the URL appears whole on one line
- **AND** that line is the only line wider than 30 cells

### Requirement: A fold never splits a grapheme cluster

When folding, the engine SHALL break a word only between grapheme clusters,
and SHALL carry a cluster that does not fit at the end of a line to the next
line whole.

#### Scenario: Folding wide text

- **WHEN** `日本語` is folded to width 3
- **THEN** the lines are `日`, `本` and `語`

#### Scenario: Folding an emoji sequence

- **WHEN** `ab` followed by the ZWJ family emoji is folded to width 3
- **THEN** the lines are `ab` and the whole family emoji

### Requirement: Truncation ends in an ellipsis

When truncating, the engine SHALL cut the text to one cell less than the
width and append `…`, and SHALL leave text that fits unchanged.

#### Scenario: Truncated title

- **WHEN** `Let a ledger declare row separation` is truncated to width 12
- **THEN** the result is `Let a ledge…`

### Requirement: No line ends in a space

The engine SHALL remove trailing spaces from every rendered line.

#### Scenario: Borderless padding

- **WHEN** a borderless record with a short value renders at width 80
- **THEN** no line of the output ends in a space
