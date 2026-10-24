# env-flags Specification

## Purpose
Bind every long option of a ctl CLI to an environment variable under one
prefix, so a caller that holds its settings in the environment can run the
bare verb. A flag wins over its variable, the variable over the config file,
and the config file over the default.

## Requirements

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

Before the parse, `ctl_core::flags::env_under_flags(command, argv)` SHALL
unbind the variable of each option that the command line sets, negates
(`--no-x` for `--x`, or `--x` for a bound `--no-x`), or conflicts with. It
SHALL unbind by resetting `Arg::env` on the command it returns, at the level
that declares the option, never by changing the process environment. `App` SHALL call it and parse the command
it returns.

#### Scenario: Flag and variable both set

- **WHEN** the run has `VERCTL_CONFIG=ci/ver.yaml` and `VERCTL_FORMAT=json`, and argv is `verctl --format pretty prepare --config own.yaml`
- **THEN** `config` is `own.yaml` and the format is `pretty`

#### Scenario: A global flag after the subcommand

- **WHEN** the root flattens `OutputArgs`, the run has `VERCTL_FORMAT=json`, and argv is `verctl prepare --format pretty`
- **THEN** the format is `pretty`

#### Scenario: A negation on the command line beats the variable

- **WHEN** `prepare` declares the pair `--pr`/`--no-pr` with a mutual `overrides_with`, the run has `VERCTL_PR=true`, and argv is `verctl prepare --no-pr`
- **THEN** the parse succeeds, and `pr` is `false` and `no_pr` is `true`

#### Scenario: The positive side on the command line beats the variable

- **WHEN** the run has `VERCTL_PR=false` and argv is `verctl prepare --pr`
- **THEN** the parse succeeds, and `pr` is `true`

#### Scenario: A conflicting flag beats the variable

- **WHEN** `prepare` declares `--after` and `--before` with `conflicts_with` between them, the run has `VERCTL_AFTER=a1`, and argv is `verctl prepare --before b1`
- **THEN** the parse succeeds, `before` is `b1`, and `after` is unset

### Requirement: Clap's conflicts count

Two options SHALL count as conflicting when either declares `conflicts_with`
the other, when both belong to a single-choice `ArgGroup`, or when either is
`exclusive`.

#### Scenario: A flag beats a variable in its single-choice group

- **WHEN** `prepare` flattens a group with `multiple = false` holding `--major` and `--minor`, the run has `VERCTL_MAJOR=true`, and argv is `verctl prepare --minor`
- **THEN** the parse succeeds with `major` `false` and `minor` `true`

#### Scenario: An exclusive flag beats every variable

- **WHEN** `show` declares `--config` and an `exclusive` `--list`, the run has `VERCTL_CONFIG=ci/ver.yaml`, and argv is `verctl show --list`
- **THEN** the parse succeeds with `list` `true` and `config` unset

### Requirement: Conflicting variables are refused by name

When two options whose variables both remain conflict, `env_under_flags`
SHALL return an `ArgumentConflict` error that names both variables.

#### Scenario: Two conflicting variables

- **WHEN** the run has `VERCTL_AFTER=a1` and `VERCTL_BEFORE=b1`, and argv is `verctl prepare`
- **THEN** the parse fails with `ArgumentConflict`, and the message names `VERCTL_AFTER` and `VERCTL_BEFORE`

### Requirement: An empty or default variable is unset

An option whose variable is set to the empty string SHALL take its default, as
if the variable were unset. A boolean flag SHALL read `true` and `false`. A
boolean flag's variable that reads as the flag's default SHALL be treated as
unset, so it cannot conflict with another option.

#### Scenario: A blank action input

- **WHEN** the run has `VERCTL_CONFIG=` and `VERCTL_DRY_RUN=false`
- **THEN** `verctl prepare` parses with `config` `.ctl/ver.yaml` and `dry_run` `false`

#### Scenario: A false flag variable beside a conflicting flag

- **WHEN** `prepare` declares `--frozen` conflicting with `--check`, the run has `VERCTL_CHECK=false`, and argv is `verctl prepare --frozen`
- **THEN** the parse succeeds with `frozen` `true` and `check` `false`

### Requirement: Help names each variable

ctl-core help SHALL print `[env: NAME]` after the description of each bound
option, after any `[default: …]` note.

#### Scenario: Help for prepare

- **WHEN** a user runs `verctl prepare --help` with the binding on
- **THEN** the `--config` row ends with `[default: .ctl/ver.yaml] [env: VERCTL_CONFIG]` and the `--dry-run` row with `[env: VERCTL_DRY_RUN]`
