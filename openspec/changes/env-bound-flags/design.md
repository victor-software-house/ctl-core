# Design

## Context

clap binds an option to a variable through `Arg::env`, and reads the
variable when that call runs. ctl-core's `App` builds the command from
`C::command()` in five places: the Usage spec, the pre-parse output view,
help, the bare-invocation check, and the parse. A binding must reach all five
the same way, or `VERCTL_FORMAT=json` would make the parse JSON while a parse
error still printed as text.

clap treats a value from the environment as given by the user. In clap
4.6.6, the validator checks conflicts for every argument whose value is
explicit, and a value from the environment is explicit
(`parser/validator.rs`, `validate_conflicts`). It also counts
`overrides_with` as a conflict ("Overrides are implicitly conflicts",
`gather_arg_direct_conflicts`). Only the command line resolves an override
(`remove_overrides`). The environment fills an option after that, wherever
the option is still absent (`parser.rs`, `add_env`). So with `--pr`
bound to `VERCTL_PR`:

| Run | clap 4.6.6 |
|:--|:--|
| `VERCTL_PR=true verctl prepare --no-pr` | error: `--no-pr` cannot be used with `--pr` |
| `VERCTL_PR=false verctl prepare --no-pr` | the same error |
| `QCTL_AFTER=X qctl add --before Y …` | error: `--before` cannot be used with `--after` |

Upstream tracks the `false` case as
[clap-rs/clap#5591](https://github.com/clap-rs/clap/issues/5591). It is still
open and labelled a breaking change, so no clap 4 release will change it. uv
met the same issue
([astral-sh/uv#13385](https://github.com/astral-sh/uv/issues/13385)). It moved
`UV_FROZEN` and `UV_LOCKED` out of clap and resolves them in its settings
layer, so the command line wins, with a warning such as "Ignoring
`UV_FROZEN` because `--locked` was provided".

## Decisions

1. **A derive attribute names the variables.** clap's derive turns
   `#[command(name = value)]` on a parser into the method call
   `.name(value)` on `clap::Command`, after it has added every argument and
   subcommand. A trait method in scope satisfies that call, so
   `#[command(env_prefix = "VERCTL")]` binds the whole graph inside
   `C::command()`, and every one of the five places sees the same names.
   Alternative: `App::env_prefix(prefix)` applied to the command `App`
   builds. It lost because `App` passes the type `C` to the help, parser, and
   Usage helpers, which call `C::command()` themselves. Each helper would
   need a second form that takes a command, and a CLI that calls a helper
   directly would get an unbound graph.
   Alternative: `#[arg(env = "…")]` on each option. It lost because the
   shared flags live in ctl-core and cannot carry a CLI's prefix, and because
   a CLI would repeat one attribute per option.
2. **Which options are bound.** Every option with a long name, except one
   that declares its own `env`, is hidden, is the `--no-*` side of a pair,
   or is help or version. A declared `env` is the per-option override. A
   hidden option has no help to name its variable. A `--no-*` side would
   make two variables for one setting: `VERCTL_PR=false` already says what
   `VERCTL_NO_PR=true` would.
3. **Names come from the long name.** `--dry-run` under `VERCTL` reads
   `VERCTL_DRY_RUN`. A long name means one thing across a CLI, so an option
   that several subcommands declare, such as `--config`, reads one
   variable.
4. **The command line goes first, then the variables.**
   `ctl_core::flags::env_under_flags(command, argv)` runs just before the
   parse. It parses argv once with every variable removed and errors
   ignored, which tells it which options the command line set at each
   subcommand level. It then unbinds the variable of each bound option that
   follows. Unbinding resets `Arg::env` on the command the function returns,
   which `App` then parses, at the level that declares the option: the root
   for a `global` option, whatever level the command line set it at, since
   clap copies a global into subcommands only when it builds the command.
   The process environment is never changed, since `std::env::remove_var` is
   `unsafe` in edition 2024:
   1. the command line set itself;
   2. the command line negated: `--no-x` given removes `x`'s variable, and
      `--x` given removes the variable of a `--no-x` that declares one;
   3. conflicts with an option the command line set: either side declares
      `conflicts_with` the other (`Command::get_arg_conflicts_with`), both
      belong to a single-choice `ArgGroup` (`multiple(false)`), or either
      side is `exclusive`;
   4. is empty;
   5. is the variable of a boolean flag and reads as the flag's default
      (`false` for `SetTrue`, `true` for `SetFalse`), so it changes nothing
      and cannot raise a conflict.

   Each variable that is left goes to clap unchanged. It keeps clap's
   behaviour: `ValueSource::EnvVariable`, a required option satisfied, the
   option's own value parser, and `[env: NAME]` in help. `App` calls this
   function before its parse. A CLI test that parses with its own command
   calls it too.
   Two bound options whose variables are both left, and that conflict, make
   the function return an `ArgumentConflict` error that names both
   variables. clap's own message would name two flags that the user never
   typed.
   The command line wins without a warning, as it does over a config file.
   Alternative: the variable as the option's default (`default_value`, plus
   `default_value_if` for each conflicting option). It lost on four counts.
   clap refuses a default on a required argument. Help would show
   `[default: …]` instead of the variable. The value source would read
   `DefaultValue`. And a `default_value_if` with no value leaves a
   `SetTrue` flag unset, so the derive's `get_flag` panics.
   Alternative: read the variables outside clap, as uv does. It lost
   because every CLI would then resolve each flag by hand.
   Alternative: `switch(on, off)` returning `on && !off`. It lost because
   clap refuses the run with a conflict before `switch` ever sees the
   values.
5. **Only `--x`/`--no-x` overrides are seen, and no group conflicts.** clap
   keeps an argument's `overrides_with` list private: `Arg` has no getter
   for it. A pair is therefore found by its long names. An `overrides_with`
   between options that are not a `--x`/`--no-x` pair still conflicts with
   its partner's variable. Every ctl CLI declares its overrides as such
   pairs. An `ArgGroup`'s own `conflicts_with` is private too, so a conflict
   declared on a group still fails the parse with clap's message. No ctl
   CLI declares one.
6. **Values parse as on the command line.** A variable goes through the
   option's own value parser, so a boolean flag takes `true` or `false`, and
   `1` is refused with the same error a bad flag value gets.
7. **The order is flag, variable, file, default.** An option that a config
   file can also set declares no clap default and has an `Option` type.
   `None` then means neither the flag nor the variable was given, and the
   CLI takes the file value, then its built-in default. That keeps the full
   order without `ArgMatches::value_source`, which `App` does not hand to a
   CLI.

Override: an option that declares its own `env` keeps it, and decision 4
applies to it as well. Disable: a CLI that does not add the attribute binds
nothing.

## Non-goals

1. The Usage spec does not carry the variables. usage-lib's conversion from
   clap ignores `env`, and a mounted task passes its argv and environment
   through to the CLI, which reads the variables itself.
2. No config-file layer. Each CLI owns its file and its schema.
3. No warning when the command line removes a variable.
