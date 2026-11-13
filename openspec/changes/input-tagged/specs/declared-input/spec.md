## ADDED Requirements

### Requirement: A config field keeps its YAML tag

`ctl_core::input::Tagged` SHALL be serde-saphyr's `Tagged<T>`.  
`Input::parse` SHALL read a field of type `Tagged<T>` as the value of `T` and the node's tag, local tags such as `!override` as written.  
An untagged enum inside `Tagged<T>` SHALL read the plain value of a tagged node, never a map from the tag to the value.

#### Scenario: A tagged list into an untagged enum

- **WHEN** a schema field `styles` has the type `Tagged<Setting>`, where `Setting` is an untagged enum with a list variant
- **AND** the file holds `styles: !override [Acme]`
- **THEN** the field reads as the list `[Acme]` with the tag `!override`

#### Scenario: A value with no tag

- **WHEN** the same file holds `styles: [Acme]`
- **THEN** the field reads as the list `[Acme]` with no tag
