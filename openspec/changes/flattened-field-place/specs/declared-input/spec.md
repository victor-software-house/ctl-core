## ADDED Requirements

### Requirement: A problem under a flattened field sits where the file writes it

`Input::flattened` SHALL take the `FieldPath` of a schema field that serde flattens into its parent.  
`Input::check` SHALL drop that field's step from each garde path whose leading steps equal the declared path.  
It SHALL then place the problem and print its field from the resulting path.  
A problem whose path starts with no declared path SHALL keep garde's printed field and its place.

#### Scenario: A flattened map in YAML

- **WHEN** `ver.yaml` has `assets:` on line 1, `linux-x64:` on line 2 and `files: []` on line 3
- **AND** the schema holds the targets in a flattened map field `targets`, and the input declares `assets`, `targets`
- **THEN** the problem on `files` reads `ver.yaml:3:12: assets.linux-x64.files: must declare at least one file`

#### Scenario: The same schema without the declaration

- **WHEN** the input of the previous scenario declares nothing
- **THEN** the problem reads `ver.yaml:2:3: assets.targets.linux-x64.files: must declare at least one file`, as before

#### Scenario: A flattened map in TOML

- **WHEN** `ver.toml` has `[assets.linux-x64]` on line 1 and `files = []` on line 2, and the input declares `assets`, `targets`
- **THEN** the problem reads `ver.toml:2:9: assets.linux-x64.files: must declare at least one file`

#### Scenario: A key with the flattened field's name

- **WHEN** a target is named `targets`, so garde reports `assets.targets.targets.files`, and the input declares `assets`, `targets`
- **THEN** the printed field is `assets.targets.files`, and the problem sits on that target's `files` line
