## ADDED Requirements

### Requirement: A prefix binds every eligible long option

`Command::env_prefix(prefix)`, through the `EnvPrefix` trait, SHALL bind each
long option of the command and of every subcommand, at any depth, to the
variable `<prefix>_<LONG>`, where `<LONG>` is the long name upper-cased with
`-` replaced by `_`. It SHALL leave unbound an option that declares its own
`env`, has no long name, is hidden, has a long name starting with `no-`, or is
help or version.

#### Scenario: A subcommand option and a shared flag

- **WHEN** a CLI's root parser carries `#[command(env_prefix = "VERCTL")]`, its `prepare` subcommand flattens `DryRunArgs` and declares `--config`, and the run has `VERCTL_CONFIG=ci/ver.yaml` and `VERCTL_DRY_RUN=true`
- **THEN** `verctl prepare` parses with `config` `ci/ver.yaml` and `dry_run` `true`

#### Scenario: A global flag

- **WHEN** the root flattens `OutputArgs` and the run has `VERCTL_FORMAT=json`
- **THEN** `verctl prepare` parses with format `json`

#### Scenario: Options that stay unbound

- **WHEN** the root declares `--no-color`, a hidden `--secret`, `--token` with `env = "OWN_NAME"`, and a positional `path`
- **THEN** `--no-color`, `--secret`, and `path` read no variable, and `--token` reads `OWN_NAME`

#### Scenario: No attribute

- **WHEN** a CLI's root parser does not carry `env_prefix`
- **THEN** no option reads a variable, whatever the environment holds

### Requirement: The command line wins

A value given on the command line SHALL win over the option's variable, and
the variable SHALL win over the option's default. For a `--x`/`--no-x` pair
declared with a mutual `overrides_with` and resolved through
`ctl_core::flags::switch`, a `--no-x` on the command line SHALL win over a
`true` read from `x`'s variable.

#### Scenario: Flag and variable both set

- **WHEN** the run has `VERCTL_CONFIG=ci/ver.yaml` and `VERCTL_FORMAT=json`, and argv is `verctl --format pretty prepare --config own.yaml`
- **THEN** `config` is `own.yaml` and the format is `pretty`

#### Scenario: A negation on the command line beats the variable

- **WHEN** `prepare` declares the pair `--pr`/`--no-pr`, the run has `VERCTL_PR=true`, and argv is `verctl prepare --no-pr`
- **THEN** `switch(pr, no_pr)` is `false`

#### Scenario: The positive side on the command line beats the variable

- **WHEN** the run has `VERCTL_PR=false` and argv is `verctl prepare --pr`
- **THEN** `switch(pr, no_pr)` is `true`

### Requirement: An empty variable is unset

An option whose variable is set to the empty string SHALL take its default, as
if the variable were unset. A boolean flag SHALL read `true` and `false`.

#### Scenario: A blank action input

- **WHEN** the run has `VERCTL_CONFIG=` and `VERCTL_DRY_RUN=false`
- **THEN** `verctl prepare` parses with `config` `.ctl/ver.yaml` and `dry_run` `false`

### Requirement: Help names each variable

ctl-core help SHALL print `[env: NAME]` after the description of each bound
option, after any `[default: …]` note.

#### Scenario: Help for prepare

- **WHEN** a user runs `verctl prepare --help` with the binding on
- **THEN** the `--config` row ends with `[default: .ctl/ver.yaml] [env: VERCTL_CONFIG]` and the `--dry-run` row with `[env: VERCTL_DRY_RUN]`
