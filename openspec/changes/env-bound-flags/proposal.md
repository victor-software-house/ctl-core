# Bind every flag to an environment variable

## Why

A ctl CLI can be set only from its command line. Callers that hold their
settings in the environment, such as a composite action or a reusable
workflow, rebuild argv in bash to pass them on. verctl's three actions do so in
three `run` blocks: each turns `VERCTL_CONFIG` back into `-c` and
`VERCTL_DRY_RUN` back into `--dry-run` through a bash array, and the
`config` input has to restate verctl's default, `.ctl/ver.yaml`, because an
empty `-c ""` would fail.

clap reads a variable for any option that declares `env`, and ctl-core
already enables clap's `env` feature, but no option declares one. The shared
flags (`--dry-run`, `--format`, `--color`) are derive structs in ctl-core, so
a CLI cannot add a variable to them at all.

Queue row: CTC-022.

## What Changes

1. A new trait, `ctl_core::flags::EnvPrefix`, gives `clap::Command` a method
   `env_prefix(prefix)`. It binds each eligible long option of the command
   and of every subcommand to `<PREFIX>_<LONG>`, with `-` turned into `_`
   and the name upper-cased.
2. A CLI opts in with one attribute on its root parser,
   `#[command(env_prefix = "VERCTL")]`. The prelude exports the trait, so
   the attribute resolves without another import.
3. A flag on the command line wins over its variable, which wins over the
   option's default. clap alone does not give that order: it treats a
   variable as given by the user, so a variable that conflicts with a flag
   on the command line, or with its `--no-*` side, fails the parse. A new
   function, `ctl_core::flags::env_under_flags`, runs before the parse and
   removes each variable the command line overrules. `App` calls it.
4. ctl-core help prints `[env: NAME]` beside each bound option.
5. Operator rule: every ctl CLI is configurable through the environment, with
   precedence flag, then variable, then config file, then default. A caller
   that holds settings in the environment sets them and runs the bare verb.

## Capabilities

### New Capabilities

- `env-flags`: which options a prefix binds, the variable names, the
  precedence over the command line, conflicting variables, empty values,
  and help.

### Modified Capabilities

None.

## Impact

1. Code: `src/flags/env.rs`, the parse in `src/app.rs`, the help
   description in `src/help.rs`, and the exports in `src/flags/mod.rs`,
   `src/lib.rs`, and `src/prelude.rs`.
2. Consumers: verctl, qctl, and forkctl adopt with one attribute after the
   release that ships this. verctl's actions then run `verctl <verb>` with
   only `env:` set.
3. No change for a CLI that does not add the attribute.
