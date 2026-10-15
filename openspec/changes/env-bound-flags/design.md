# Design

## Context

clap binds an option to a variable through `Arg::env`, and reads the
variable when that call runs. ctl-core's `App` builds the command from
`C::command()` in five places: the Usage spec, the pre-parse output view,
help, the bare-invocation check, and the parse. A binding must reach all five
the same way, or `VERCTL_FORMAT=json` would make the parse JSON while a parse
error still printed as text.

## Decisions

1. **A derive attribute through an extension trait.** clap's derive turns
   `#[command(name = value)]` on a parser into the method call
   `.name(value)` on `clap::Command`, after it has added every argument and
   subcommand. A trait method in scope satisfies that call, so
   `#[command(env_prefix = "VERCTL")]` binds the whole graph inside
   `C::command()`, and every one of the five places sees it with no change to
   `App`.
   Alternative: `App::env_prefix(prefix)` applied to the command `App`
   builds. It lost because `App` passes the type `C` to the help, parser, and
   Usage helpers, which call `C::command()` themselves; each would need a
   second, command-taking form, and a CLI that calls a helper directly would
   get an unbound graph.
   Alternative: `#[arg(env = "…")]` on each option. It lost because the
   shared flags live in ctl-core and cannot carry a CLI's prefix, and because
   a CLI would repeat one attribute per option.
2. **Which options are bound.** Every option with a long name, except one
   that declares its own `env`, is hidden, is the `--no-*` side of a pair,
   or is help or version. A declared `env` is the per-option override. A
   hidden option has no help to name its variable. A `--no-*` side would
   make two variables for one setting: `VERCTL_PR=false` already says what
   `VERCTL_NO_PR=true` would.
   Binding only `--x` leaves one hole: clap fills a variable into an option
   only after the command-line parse, and only where the option is absent.
   A command-line `--no-x` clears `x` through `overrides_with`, so
   `VERCTL_X=true` then fills `x` back in and both sides read `true`.
   `ctl_core::flags::switch(on, off)` therefore returns `on && !off`: with
   the mutual override, both sides are `true` only when `on` came from the
   variable and `off` from the command line, and the command line wins.
   Alternative: read `ArgMatches::value_source` for the pair. It lost
   because `App` does not hand `ArgMatches` to a CLI.
3. **Names come from the long name.** `--dry-run` under `VERCTL` reads
   `VERCTL_DRY_RUN`. A long name means one thing across a CLI, so an option
   that several subcommands declare, such as `--config`, reads one
   variable.
4. **An empty variable is unset.** clap passes an empty variable through as
   the value `""`. A composite action sets every input it declares, so an
   input left blank arrives as `""`, and `-c ""` would fail. `env_prefix`
   leaves an option unbound for the run when its variable is empty, so the
   option takes its default. clap reads the variable when `Arg::env` runs,
   so the check happens while the command is built, and help omits the
   `[env: …]` note for that run.
   Alternative: a value parser per option that maps `""` to the default. It
   lost because every option would need one.
   Alternative: remove empty variables before the parse. It lost because
   `set_var` is unsafe in edition 2024 and the crate forbids `unsafe`.
5. **Values parse as on the command line.** A variable goes through the
   option's own value parser, so a boolean flag takes `true` or `false`, and
   `1` is refused with the same error a bad flag value gets.
6. **Flags win.** clap applies the command line, then the variable, then the
   default. An option that a config file can also set declares no clap
   default and has an `Option` type, so `None` means neither the flag nor the
   variable was given; the CLI then takes the file value, then its built-in
   default. That keeps the full order flag, variable, file, default without
   reading `ArgMatches::value_source`, which `App` does not hand to a CLI.

Override: an option that declares its own `env` keeps it. Disable: a CLI
that does not add the attribute binds nothing.

## Non-goals

1. The Usage spec does not carry the variables. usage-lib's conversion from
   clap ignores `env`, and a mounted task passes its argv and environment
   through to the CLI, which reads the variables itself.
2. No config-file layer. Each CLI owns its file and its schema.
