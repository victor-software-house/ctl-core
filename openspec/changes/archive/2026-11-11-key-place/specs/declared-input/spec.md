## ADDED Requirements

### Requirement: Input places a key

`Input::place_key` SHALL take a `FieldPath` and return the 1-based line and a 1-based column counted in characters where the last step of the path is written as a key, or where the nearest written ancestor's key is.  
A step that is an index, and the root, SHALL give the place of their value.  
It SHALL add the input's frontmatter offset to the line, and SHALL return `None` for a text that does not parse or goes past the limits in force.

#### Scenario: A key that holds a block mapping in YAML

- **WHEN** `app.yaml` has `files:` on line 1, `'docs/**/*.md':` on line 2 and `label: ''` on line 3, and the path is `files`, `docs/**/*.md`
- **THEN** `place_key` returns line 2, column 3, where `place` returns line 3, column 5

#### Scenario: A scalar value

- **WHEN** the path of the previous scenario ends with `label`
- **THEN** `place_key` returns line 3, column 5, the key, where `place` returns line 3, column 12

#### Scenario: A key in a flow mapping

- **WHEN** a line holds `flow: {styles: [x], vocab: []}` and the path is `flow`, `styles`
- **THEN** `place_key` returns that line and the column of `styles`

#### Scenario: A key in TOML

- **WHEN** `app.toml` has `[files."docs/**/*.md"]` on line 1, and the path is `files`, `docs/**/*.md`
- **THEN** `place_key` returns line 1, column 8, where the key starts

#### Scenario: An item has no key

- **WHEN** the path ends with an index
- **THEN** `place_key` returns what `place` returns

#### Scenario: A path the file lacks

- **WHEN** the path names a key the file does not hold under `files`
- **THEN** `place_key` returns the place of the key `files`
