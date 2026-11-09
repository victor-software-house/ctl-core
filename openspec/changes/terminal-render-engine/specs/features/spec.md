## MODIFIED Requirements

### Requirement: One predicate decides which characters are unsafe to print verbatim

`ctl_core::unsafe_chars::is_unsafe`, with the `unsafe-chars` feature, SHALL
return true for each of these characters:

- Every member of general category Cc.
- All 12 directional formatting characters of UAX #9 at Unicode 16.0.0.
  They are ALM, LRM, RLM, LRE, RLE, PDF, LRO, RLO, LRI, RLI, FSI, and PDI.
- The one member each of Zl and Zp.

It SHALL return false for every other character.

#### Scenario: Every member of each set

- **WHEN** `is_unsafe` is called with each of the 65 Cc characters, the 12 directional formatting characters, U+2028, and U+2029
- **THEN** it returns true for each

#### Scenario: The neighbours and the other format characters

- **WHEN** `is_unsafe` is called with one of these:
  - a neighbour of each set, such as U+00A0, U+061B, U+2027, and U+206A
  - another Cf character, such as U+200B, U+200D, U+FEFF, or U+E0067
- **THEN** it returns false

#### Scenario: Nothing else

- **WHEN** `is_unsafe` is called with every Unicode scalar value
- **THEN** it returns true for exactly 79 of them

### Requirement: Verbatim document text uses the shared predicate

`Document::verbatim` text SHALL keep newlines and tabs and SHALL drop every
other character `unsafe_chars::is_unsafe` refuses, through
`unsafe_chars::strip_unsafe`.

#### Scenario: Terminal controls in verbatim text

- **WHEN** a document holds the verbatim text `"\u{1b}]52;clipboard\u{7}\u{202e}\u{2028}\u{2029}\tvalue"`
- **THEN** the rendered text is `"]52;clipboard\tvalue"`
