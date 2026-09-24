# Presentation

## ADDED Requirements

### Requirement: The environment picks a style the owner left open

ctl-core SHALL read each style option from its environment variable when the
library owner has not set that option, and SHALL ignore the variable when the
owner has.

#### Scenario: Boxed record from the shell

- **WHEN** `CTL_CORE_RECORD_STYLE=boxed` is set and the consumer sets no record style
- **THEN** records render inside a full frame

#### Scenario: The owner's choice wins

- **WHEN** `CTL_CORE_JSON_LAYOUT=compact` is set and the consumer sets `JsonLayout::Pretty`
- **THEN** JSON output is indented by two spaces

### Requirement: Each variable name can be replaced or disabled

ctl-core SHALL let a consumer replace the ordered names read for each style
option, and SHALL read no variable when the list is empty.

#### Scenario: Disabled lookup

- **WHEN** a consumer sets `record_style_envs(&[])` and `CTL_CORE_RECORD_STYLE=boxed` is set
- **THEN** records render with keys right-aligned and no frame

### Requirement: An unaccepted value is named

ctl-core SHALL ignore a style variable whose value is not accepted, and `App`
SHALL print one warning line to stderr that names the accepted values.

#### Scenario: A typo in the list style

- **WHEN** `CTL_CORE_LIST_STYLE=boxes` is set and a consumer runs through `App`
- **THEN** stderr contains `CTL_CORE_LIST_STYLE=boxes is ignored; use one of grid, header-rule, plain`
- **AND** lists render in the default grid
