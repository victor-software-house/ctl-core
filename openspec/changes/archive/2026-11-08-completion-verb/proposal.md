# One completion verb for every ctl CLI

## Why

Only forkctl can print a shell completion script. qctl and verctl answer
`error: unrecognized subcommand 'completion'`
([output rendering comparison][rendering], item 5), and rawr, the newest
ctl CLI, has none either. forkctl's verb is about 80 lines across
`src/main.rs` and `src/cli.rs`; copying it into four more CLIs would give
four copies that drift.

The [shell completion research][research] found two more problems a copy
would carry:

1. `clap_complete` 4.6.11's zsh script completes nothing on the first TAB
   when it is installed in `fpath`, which is where Homebrew writes it.
   Reproduced under `zpty`: the first TAB rang the bell, the second
   completed.
2. A script printed by the `COMPLETE=<shell>` environment path, which
   Homebrew's `shell_parameter_format: :clap` uses, comes from
   `clap_complete` directly, so a fix applied only in the verb would miss
   it.

Queue row: CTC-030.

## What Changes

1. A new `shell` cargo feature with `ctl_core::shell::Shell`, a Clap value
   for bash, zsh, and fish. The shell-integration change builds on the same
   type.
2. A new `completion` cargo feature with `ctl_core::completion`:
   1. `CompletionCommand`, flattened into a CLI's subcommands, gives
      `<bin> completion <SHELL>` the same name, help, and argument in every
      ctl CLI.
   2. `Shell::script(bin)` returns the dynamic registration script as a
      `shell::Script` model, with the zsh first-TAB fix appended.
   3. `shell::view(format)` prints the script without colour and
      ignores `--quiet`.
3. `App::completion()` answers `COMPLETE=<shell>` before Clap parses: with
   no words it prints the same `Script` as the verb, and with words it
   prints the candidates.
4. `clap_complete` is pinned to `=4.6.11` with `unstable-dynamic`, which
   carries no semver promise.

## Capabilities

### New Capabilities

- `completion`: the verb, its shells, the registration script, the
  environment request, and how installs load the script.

### Modified Capabilities

None.

## Impact

1. Code: `src/shell.rs`, `src/completion.rs`, `App::completion` in
   `src/app.rs`, the modules in `src/lib.rs`, the features in `Cargo.toml`,
   and the feature sets in `mise-tasks/check/features.sh`.
2. Consumers: qctl, verctl, forkctl, state-sync, and rawr adopt the verb in
   their own changes after a release. forkctl's adoption drops its elvish,
   powershell, and nushell scripts, or keeps them as its own verb.
3. No change for a CLI that does not enable `completion`.

[rendering]: ../../../../docs/research/output-rendering-comparison.md
[research]: ../../../../docs/research/shell-completion.md
