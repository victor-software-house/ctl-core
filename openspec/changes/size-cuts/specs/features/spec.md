## MODIFIED Requirements

### Requirement: The cli feature turns on only the Clap features ctl-core calls

ctl-core SHALL declare `clap` with `default-features = false`. The `cli`
feature SHALL turn on only Clap's `std`, `derive`, `env`, `string`, and
`help` features; without `help`, Clap drops the `-h`/`--help` flag.
`help` and `usage` SHALL each add only Clap's `usage`, which
renders the usage line. No other ctl-core feature,
and no dependency of ctl-core, SHALL turn on Clap's `default` feature.

#### Scenario: A consumer with a minimal clap entry

- **WHEN** a consumer depends on ctl-core with `default-features = false, features = ["app"]` and on `clap` with `default-features = false, features = ["std", "derive"]`
- **THEN** `cargo tree -e features -i clap_builder` shows none of `color`, `error-context`, or `suggestions`

#### Scenario: A consumer that mounts a mise task and reads config

- **WHEN** a consumer depends on ctl-core with `default-features = false, features = ["app", "usage", "input"]` and on `clap` with `default-features = false, features = ["std", "derive"]`
- **THEN** `cargo tree -e features -i clap_builder` shows none of `default`, `color`, `error-context`, or `suggestions`

#### Scenario: A consumer with only the cli feature

- **WHEN** a consumer builds ctl-core with `default-features = false, features = ["cli"]` and parses `-h`, `--help`, or `-V`
- **THEN** Clap stops with `DisplayHelp` or `DisplayVersion`
