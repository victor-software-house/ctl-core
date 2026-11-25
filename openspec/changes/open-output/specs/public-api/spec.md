# Spec Delta

## MODIFIED Requirements

### Requirement: View settings are read through methods

`View`'s format and quiet setting SHALL be private.
`View::format()` SHALL return the name of the format in force, and `View::is_quiet()` the quiet setting.
`View::quiet(bool)` SHALL stay the setter.

#### Scenario: A quiet JSON view

- **WHEN** a caller builds a `json` view with `ColorMode::Never` and `quiet(true)`
- **THEN** `format()` returns the `json` name and `is_quiet()` returns `true`

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
