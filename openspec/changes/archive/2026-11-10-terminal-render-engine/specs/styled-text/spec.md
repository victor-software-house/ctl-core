## Purpose

How the rendering engine turns text into styled spans.
This covers which characters from outside the program it removes, and how it
counts width in terminal cells.
It also covers how it wraps, folds and truncates a line.

## ADDED Requirements

### Requirement: Unsafe characters are removed from every span

The engine SHALL remove these characters from the text of every span:

- the C0 and C1 control characters, including ESC;
- the twelve UAX #9 directional formatting characters;
- U+2028 and U+2029.

The exceptions are newline and tab, which the engine handles as the next
requirements state.
The other line breaks are also exceptions, as the requirement after them
states.

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

### Requirement: A line break other than a newline keeps the words apart

The engine SHALL treat newline, carriage return, vertical tab, form feed, next
line (U+0085), U+2028, and U+2029 as line breaks. A span SHALL replace each
with a space, and a text SHALL replace each with a newline. A carriage return
directly before a line feed SHALL be one break with it.

#### Scenario: A separator in a span

- **WHEN** a span holds `a\u{2028}b`
- **THEN** its text is `a b`

#### Scenario: A newline in a span

- **WHEN** a span holds `first\nsecond`
- **THEN** its text is `first second`

#### Scenario: A break in a paragraph

- **WHEN** a paragraph holds `first\rsecond` at width 80
- **THEN** the output is the two lines `first` and `second`

#### Scenario: A carriage return and a line feed

- **WHEN** a paragraph holds `first\r\nsecond` at width 80
- **THEN** the output is the two lines `first` and `second`, with no blank line between

### Requirement: Empty text renders nothing

The engine SHALL render a paragraph with no characters in it to no lines, as
it renders a paragraph that was never given text.

#### Scenario: An empty string

- **WHEN** a paragraph holds the empty string at width 80
- **THEN** the output has no lines

### Requirement: A tab expands to the next tab stop

The engine SHALL replace each tab with spaces up to the next multiple of 8
cells from the start of its line.
It SHALL let a caller set another tab width.

#### Scenario: Default tab stop

- **WHEN** a paragraph holds `ab\tc`
- **THEN** the output is `ab` followed by 6 spaces and `c`

#### Scenario: A tab after a cluster that continues from an earlier piece

- **WHEN** a paragraph holds `❤`, and then, pushed apart from it, U+FE0F, a
  tab, and `x`
- **THEN** `x` starts at cell 8, because the heart and the selector are one
  2-cell cluster

### Requirement: Width is counted per grapheme cluster

The engine SHALL count the width of text as the sum, over its extended
grapheme clusters, of each cluster's width in terminal cells.
It SHALL count East Asian ambiguous characters as 1 cell.

#### Scenario: A family emoji

- **WHEN** the engine measures the ZWJ sequence for a family of four
- **THEN** its width is 2

#### Scenario: Wide characters

- **WHEN** the engine measures `日本`
- **THEN** its width is 4

#### Scenario: Letters a font joins

- **WHEN** the engine measures the Arabic letters lam and alef side by side
- **THEN** its width is 2, the sum of two 1-cell clusters

### Requirement: The Unicode versions are stated

The engine SHALL expose the Unicode version of its width data and of its
segmentation data.

#### Scenario: A dependency upgrade

- **WHEN** a dependency upgrade changes either Unicode version
- **THEN** the test that pins both versions fails until the pin is updated

### Requirement: A word is kept whole unless the caller allows a fold

When wrapping with the keep-words policy, the engine SHALL break lines only at
whitespace or at a zero width space.
It SHALL let a word wider than the width run past it.

#### Scenario: A long URL

- **WHEN** a paragraph holds `see`, a space, and `https://example.test/releases/tag/pkg@1.2.3` at width 30
- **THEN** the URL appears whole on one line
- **AND** that line is the only line wider than 30 cells

#### Scenario: A zero width space

- **WHEN** a paragraph holds `ab` and `cd` with a zero width space between
  them, at width 2
- **THEN** the output is the two lines `ab` and `cd`

### Requirement: A fold never splits a grapheme cluster

When folding, the engine SHALL break a word only between grapheme clusters.
It SHALL carry a cluster that does not fit at the end of a line to the next
line whole.

#### Scenario: Folding wide text

- **WHEN** `日本語` is folded to width 3
- **THEN** the lines are `日`, `本` and `語`

#### Scenario: Folding an emoji sequence

- **WHEN** `ab` followed by the ZWJ family emoji is folded to width 3
- **THEN** the lines are `ab` and the whole family emoji

### Requirement: Truncation ends in an ellipsis

When truncating, the engine SHALL cut the text to one cell less than the
width and append `…`, and SHALL leave text that fits unchanged. A wide
cluster that straddles the cut SHALL be replaced by a space, so the result is
exactly the width.

#### Scenario: Truncated title

- **WHEN** `Let a ledger declare row separation` is truncated to width 12
- **THEN** the result is `Let a ledge…`

#### Scenario: A wide character at the cut

- **WHEN** `日本語` is truncated to width 4
- **THEN** the result is `日`, a space, and `…`, which is 4 cells

### Requirement: No line ends in a space

The engine SHALL remove trailing spaces from every rendered line.

#### Scenario: Borderless padding

- **WHEN** a borderless record with a short value renders at width 80
- **THEN** no line of the output ends in a space
