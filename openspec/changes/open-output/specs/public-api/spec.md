## MODIFIED Requirements

### Requirement: Open enums take a wildcard arm

`document::Block` and `document::NoticeLevel` SHALL be `#[non_exhaustive]`.
Each one's rustdoc SHALL say that a match outside ctl-core ends with a wildcard arm.
The set of output formats SHALL be open through the format interface, not an enum.

#### Scenario: A consumer matches every block

- **WHEN** a crate outside ctl-core matches a `Block` on its eight variants with no wildcard arm
- **THEN** the match fails to compile with `E0004`

#### Scenario: A consumer names a format

- **WHEN** a crate outside ctl-core compares the format in force with the `json` name
- **THEN** it compares `FormatName` values, and no match on a format enum exists
