# entry Specification

## Purpose
How a consumer binary enters ctl-core: `App` owns parsing, help, rendering, and the exit code.

## Requirements

### Requirement: App is the only entry point

ctl-core SHALL expose `App` as the only way for a consumer binary to parse,
render help, run, and map a result to an exit code. It SHALL NOT expose a
second wrapper for any of those steps.

#### Scenario: A consumer main

- **WHEN** a consumer writes `fn main() -> ExitCode { App::<Cli>::new("toy").run(execute) }`
- **THEN** help, parsing, rendering, and the exit code all come from `App`

#### Scenario: The removed wrappers

- **WHEN** a consumer imports `ctl_core::run`, `ctl_core::run::go`, or `ctl_core::prelude::main_with_help`
- **THEN** the build fails with an unresolved import
