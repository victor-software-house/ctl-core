## MODIFIED Requirements

### Requirement: The cli feature turns on only the Clap features ctl-core calls

ctl-core SHALL declare `clap` with `default-features = false`. The `cli`
feature SHALL turn on only Clap's `std`, `derive`, `env`, and `string`
features. `help` and `usage` SHALL each add only Clap's `usage`, which
renders the usage line. No other ctl-core feature,
and no dependency of ctl-core, SHALL turn on Clap's `default` feature.

ctl-core's entry point SHALL declare `-h`/`--help` and the `help`
subcommand itself and render the help, so Clap's `help` feature stays off.
A plain `Cli::parse()` outside that entry point MAY get Clap's minimal
output and an unknown `help` subcommand.

#### Scenario: A consumer with a minimal clap entry

- **WHEN** a consumer depends on ctl-core with `default-features = false, features = ["app"]` and on `clap` with `default-features = false, features = ["std", "derive"]`
- **THEN** `cargo tree -e features -i clap_builder` shows none of `color`, `help`, `error-context`, or `suggestions`

#### Scenario: A consumer that mounts a mise task and reads config

- **WHEN** a consumer depends on ctl-core with `default-features = false, features = ["app", "usage", "input"]` and on `clap` with `default-features = false, features = ["std", "derive"]`
- **THEN** `cargo tree -e features -i clap_builder` shows none of `default`, `color`, `help`, `error-context`, or `suggestions`

#### Scenario: A consumer with only the cli feature

- **WHEN** a consumer builds ctl-core with `default-features = false, features = ["cli"]` and parses `-h`, `--help`, `-V`, or `help <command>` through `parser::apply_defaults`
- **THEN** Clap stops with `DisplayHelp` or `DisplayVersion`, or selects the `help` subcommand with the named command below it
