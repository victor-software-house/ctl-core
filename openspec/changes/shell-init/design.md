# Design

## Context

The [research][research] read zoxide, starship, mise, direnv, and atuin at
pinned tags, and the hook code of bash 5.3, zsh 5.9.2, and fish 4.9.3.
rawr's first design also asked for a shell function that evaluates what
some subcommands print; rawr now runs the follow-up command itself, so that
function and a per-shell quoting API have no consumer and are left out.

## Decisions

1. **A static script from a declared hook.**
   The CLI declares `Hook::new(bin, subcommand, variable)` once.
   ctl-core writes every line of shell syntax, and the script holds no path
   or setting read at `init` time, so a saved copy survives upgrades.
   - Alternative: let each CLI pass its own script text.
     Each CLI would then own three shell languages and their tests.
2. **Every part is a plain word, checked once.**
   `bin` and `subcommand` take letters, digits, `-`, and `_`, starting with
   a letter or digit; `variable` is a shell variable name.
   The script can then use each part unquoted, and no input can inject
   shell syntax.
   - Alternative: quote each part per shell.
     That is the quoting API the narrowed scope dropped, for names that
     never need it.
3. **The hook removes itself, then runs the command.**
   bash filters its function out of `PROMPT_COMMAND`, as a string or as the
   array bash 5.1 accepts; zsh filters `precmd_functions`; fish erases its
   `fish_prompt` handler.
   Each shell walks a copy of its hook list, so the removal takes effect at
   the next prompt.
   Removing it first means a command that fails, or that the operator
   interrupts, does not run again at the next prompt.
   bash's hook also sets `<function>_ran` and returns at once when it is
   already set, because another tool can rewrite `PROMPT_COMMAND` so the
   removal no longer finds the hook.
   bash-preexec 0.7.0 does this when it loads
   ([`__bp_install_after_session_init`][bp-install]): on bash 5.1 and later
   it appends its install command as an array element, which turns a string
   `PROMPT_COMMAND` into element zero, `<function>;…`, and on older bash it
   strips a trailing `;`.
   zsh and fish need no guard: zsh removes the function's name from
   `precmd_functions`, whose elements are names that a rewrite keeps whole,
   and fish erases the handler function itself.
   - Alternative: match more of the forms a rewrite leaves.
     Each tool rewrites differently, and the guard holds whatever the
     rewrite; the removal stays as cleanup.
   - Alternative: `add-zsh-hook`. It is autoloaded, and `zsh -f` does not
     load it; `precmd_functions` is the array it edits.
   - Alternative: rewrite a string `PROMPT_COMMAND` as an array. That
     breaks bash below 5.1, which runs only element zero.
4. **Guards run at the first prompt, not at load.**
   The command runs only when the variable is non-empty and a lookup on
   `PATH` alone finds the binary: `type -P` in bash, `whence -p` in zsh, and
   `command -q` in fish.
   Checking at the prompt lets a startup file set the variable after the
   line that loads the script, and keeps a missing binary silent.
   - Alternative: `command -v`. It also matches an alias or a function
     named `<bin>`, and `command <bin>` then prints "command not found".
5. **A second load installs nothing.**
   bash checks `PROMPT_COMMAND` for its function first, zsh checks
   `precmd_functions[(I)…]`, and a second fish definition replaces the
   first.
6. **The bash hook returns the status it found.**
   It is placed first in `PROMPT_COMMAND`, and commands in one string share
   `$?`, so it saves `$?` on entry and returns it, as zoxide does.
   zsh and fish restore the status around a hook themselves.
7. **Each script's first line names how to load it.**
   ctl-core help renders no text after the arguments, so the loading line
   for each shell lives in the script's first comment and in the module
   docs, in the form all five surveyed CLIs document:
   `eval "$(<bin> init bash)"`, `eval "$(<bin> init zsh)"`, and
   `<bin> init fish | source`.
8. **`Script` and `view` live in `shell`.**
   Both verbs print one script for one shell, with no colour and never
   quiet.
   They move behind the `view` feature, and `shell-init` enables
   `shell` and `view` only, so it does not pull in `clap_complete`.
9. **Tests run each shell on a pseudo-terminal.**
   `ctl-core/tests/support/terminal.rs` starts the shell on a terminal from
   [pty-process][pty-process] 0.5.4, waits for a prompt that ends in
   `READY> `, and types each line only after it.
   The startup file loads the script twice; the fake binary logs its
   arguments and exits 7, and the prompt shows the last status, so the
   tests prove the hook ran once and left `$?` at 0.
   The environment is cleared down to `PATH`, `HOME`, `TERM`, and the test's
   own variables.
   fish runs with `fish_features=no-query-term`, which stops it waiting 10
   seconds on terminal queries.
   - Alternative: drive the shell with `script`.
     Under `script`, fish 4.9.3 dropped every line typed ahead of its first
     prompt and exited at end of input, so the timing would need sleeps.
   - Alternative: open the terminal with `rustix` directly.
     Making the terminal the child's controlling terminal needs a
     `pre_exec` closure, which is `unsafe`, and ctl-core forbids unsafe
     code.
   - Alternative: `portable-pty`, `expectrl`, or `rexpect`.
     Each brings `nix` or more crates; `pty-process` needs only `rustix`.
   The pin is exact for the same reason as insta's: a bump is a reviewed
   change.

[bp-install]: https://github.com/rcaloras/bash-preexec/blob/d866eeefdb8dfce075cbd4e37dc73c4deb4b0bd2/bash-preexec.sh#L538-L561
[pty-process]: https://github.com/doy/pty-process/tree/v0.5.4/src/blocking
[research]: ../../../docs/research/shell-init.md
