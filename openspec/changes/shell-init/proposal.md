# One init verb that hooks a ctl CLI into the shell

## Why

rawr needs the operator's shell to run one of its commands before the first
prompt of a new shell, in the foreground, and only inside the terminal
context an environment variable marks.
Each CLI that wrote that hook itself would write three scripts in three
shell languages, and the [research][research] found four ways to get one
wrong:

1. A script that embeds the binary's absolute path breaks after a Homebrew
   upgrade.
2. A startup file that loads the script twice installs the hook twice.
3. A hook placed first in bash's `PROMPT_COMMAND` string changes the `$?`
   the commands after it read.
4. A test that only parses the script, or drives the shell through a pipe,
   never sees the hook run: fish draws no prompt when it reads a pipe.

Queue row: CTC-033.

## What Changes

1. A new `shell-init` cargo feature with `ctl_core::shell_init`:
   1. `Hook::new(bin, subcommand, variable)` declares the command, and
      refuses any part that is not a plain word.
   2. `Hook::script(shell)` writes the bash, zsh, or fish script: the hook
      runs `<bin> <subcommand>` once, before the first prompt, when
      `variable` is non-empty and `bin` is on `PATH`, then never again.
   3. `InitCommand`, flattened into a CLI's subcommands, gives
      `<bin> init <SHELL>` the same name, help, and argument in every ctl
      CLI.
2. `Script` and `view` move from `completion` to `shell`, behind the `view`
   feature, so both verbs share them.
   `completion` is not yet released, so no old path is kept.
3. Tests snapshot each script and the verb help, check each script with its
   shell and with shellcheck for bash, and run each shell on a
   pseudo-terminal through `pty-process`, pinned to `=0.5.4` as a
   dev-dependency.

## Capabilities

### New Capabilities

- `shell-init`: the `init <SHELL>` verb and the first-prompt hook script it
  prints.

### Modified Capabilities

None.
The `completion` change's own spec moves with it, since neither is
archived.

## Impact

1. Code: `ctl-core/src/shell_init.rs`, `ctl-core/src/shell.rs`,
   `ctl-core/src/completion.rs`, `ctl-core/src/lib.rs`, and the `shell-init`
   feature in `ctl-core/Cargo.toml`.
2. Tests: `ctl-core/tests/shell_init.rs`, `ctl-core/tests/support/terminal.rs`,
   `ctl-core/tests/snapshots/shell_init__*.snap`, and the imports in
   `ctl-core/tests/completion.rs`.
3. `Cargo.lock` adds `pty-process` as a test-only crate and moves `rustix`
   from 1.1.4 to 1.1.6, which `pty-process` requires.
4. Docs: `README.md`, `AGENTS.md`, `docs/research/shell-init.md`, and the
   `completion-verb` change, which named `completion::view`.

[research]: ../../../docs/research/shell-init.md
