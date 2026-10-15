---
ctl-core: patch
---

`EnvPrefix::env_prefix` binds every long option of a command and its subcommands to `<PREFIX>_<LONG>`. Put `#[command(env_prefix = "VERCTL")]` on the root parser, with the prelude in scope, and `VERCTL_CONFIG`, `VERCTL_DRY_RUN`, and `VERCTL_FORMAT` set those flags; a flag on the command line wins over its variable. An option that declares its own `env`, has no long name, is hidden, is a `--no-*` side, or is help or version stays unbound. ctl-core help prints `[env: NAME]` beside each bound option.

`flags::env_under_flags(command, argv)` makes the command line win over the variables; `App` calls it before the parse. It unbinds the variable of an option the command line set, negated with its `--no-*` side, or conflicts with, and a variable that is empty or reads as a boolean flag's default. Two remaining variables that conflict fail the parse with an error that names both.
