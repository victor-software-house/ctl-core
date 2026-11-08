# Shell integration: how current CLIs ship an init script

Surveyed 2026-11-08 from source at the pinned tags below.
The question is what `init <SHELL>` verb ctl-core should give a ctl CLI that
needs one shell hook: a command of the CLI that runs once, in the
foreground, before the shell's first prompt, when an environment variable
is set.
rawr is the first consumer.

**Answer.**
Print a static script for bash, zsh, and fish, built by ctl-core from the
hook the CLI declares.
Call the binary by name, never by an absolute path.
Run the command only when the variable is non-empty and the binary is on
`PATH`.
Have the hook remove itself before it runs the command, and guard the
install against a second `source`.
Test each script by running its shell on a pseudo-terminal, not only by
parsing it.

## 1. What the surveyed CLIs do

| CLI | Verb | User's line, bash and zsh | User's line, fish |
|:--|:--|:--|:--|
| [zoxide][z-cmd] | `zoxide init <SHELL> [--cmd] [--hook] [--no-cmd]` | `eval "$(zoxide init zsh)"`, after `compinit` ([README][z-readme]) | `zoxide init fish \| source` |
| [starship][s-cmd] | `starship init <SHELL> [--print-full-init]` | `eval "$(starship init zsh)"` ([docs][s-readme]) | `starship init fish \| source` |
| [mise][m-cmd] | `mise activate [SHELL] [--shims] [--quiet]` | `eval "$(mise activate zsh)"` ([shell setup][m-setup]) | `mise activate fish \| source` |
| [direnv][d-cmd] | `direnv hook SHELL` | `eval "$(direnv hook zsh)"` ([hook.md][d-hook]) | `direnv hook fish \| source` |
| [atuin][a-cmd] | `atuin init <SHELL> [--disable-ctrl-r] [--disable-up-arrow]` | `eval "$(atuin init zsh)"` ([installation][a-install]) | `atuin init fish \| source` |

Every one takes the shell as its one positional argument, and every one
documents the same two lines: `eval "$(<bin> <verb> <shell>)"` for bash and
zsh, and `<bin> <verb> fish | source` for fish.

## 2. Static or not

None of the five is fully static:

1. zoxide bakes in `_ZO_ECHO` and `_ZO_RESOLVE_SYMLINKS` as read at `init`
   time ([init.rs][z-init]), and calls `\command zoxide` by name.
2. starship embeds its absolute path from `which` or `current_exe`
   ([init/mod.rs][s-init]).
3. mise embeds its absolute path and snapshots `PATH` into
   `__MISE_ORIG_PATH` ([activate.rs][m-activate]).
4. direnv embeds its own path, from `DIRENV_EXE_PATH` or the executable
   ([config.go][d-config]).
5. atuin merges its settings file and environment into the script
   ([init.rs][a-init]).

An absolute path ties the script to one install: a Homebrew upgrade moves
the keg, and a saved script then calls a removed binary.
Calling the binary by name keeps a saved script valid across upgrades, and
lets a user save it to a file and source that file without starting any
process at shell start.

## 3. Hooks per shell

| Need | bash | zsh | fish |
|:--|:--|:--|:--|
| Before a prompt | `PROMPT_COMMAND`, a string, or from bash 5.1 an array | `precmd_functions` | `function … --on-event fish_prompt` |
| Guard against a second `source` | zoxide and direnv test `PROMPT_COMMAND` for their hook first ([zoxide bash][z-bash-hook], [direnv bash][d-bash]) | direnv tests `precmd_functions[(I)…]` ([direnv zsh][d-zsh]) | A second definition replaces the first ([zoxide fish][z-fish-hook]) |
| Keep the operator's `$?` | zoxide saves `$?` and returns it ([zoxide bash][z-bash-status]) | the shell restores it | the shell restores it |

`add-zsh-hook` would also add a function once, but it is an autoloaded
function, and `zsh -f` does not load it; `precmd_functions` is the array it
edits.

A hook that removes itself edits the list the shell is walking.
Each shell walks a copy, so the edit takes effect at the next prompt:

1. bash 5.3 copies a `PROMPT_COMMAND` array into an argument vector before
   running its elements (`execute_array_command`, `eval.c` lines 286-302 of
   the [bash 5.3 release][bash-5.3]), and parses a string value from a copy
   (`execute_variable_command`, `parse.y` lines 3011-3027).
2. zsh 5.9.2 runs `precmd_functions` from `arrdup` of the array
   ([utils.c L1499][zsh-callhook]).
3. fish 4.9.3 erases the function from inside its own handler; the
   behaviour test below shows the handler runs once.

bash restores `$?` around each `PROMPT_COMMAND` element through
`save_parser_state` and `restore_parser_state` (`parse.y` lines 7221 and
7285 of the [release][bash-5.3]), but commands inside one string share it,
so a hook placed first in a string must return the status it found.
zsh restores it after each hook function ([exec.c L6087-6091][zsh-restore]),
and fish around each event handler ([event.rs L501-504][fish-restore]).

## 4. Testing the scripts

1. zoxide renders every shell for each flag combination and runs each
   script in the real shell: bash with `-e -u -o pipefail`, zsh with
   `--no-rcs`, fish with `--no-config --private`.
   It also runs `shellcheck --enable=all` and `shfmt`, and its CI installs
   the shells through Nix ([shell.rs][z-tests], [ci.yml][z-ci]).
2. direnv runs its bash and zsh hooks in the real shell and skips a missing
   shell ([shell_hook_test.go][d-tests]).
3. mise snapshots each `activate` output and runs per-shell end-to-end
   tests.
4. starship and atuin run no test on their generated scripts.

A parse-only check is weaker: `bash -n` and `zsh -n` read without executing
([The Set Builtin][bash-set], [options.yo L1793-1799][zsh-noexec]), and
`fish --no-execute` "only perform[s] syntax checking"
([fish.rst L47-48][fish-n]).
None of them catches a wrong command or variable name.

A prompt hook runs only in an interactive shell that draws a prompt, and
feeding one a pipe is not enough:

1. bash `-i` and zsh `-i` read lines from a pipe and run their prompt hooks.
2. fish `-i` reading a pipe draws no prompt, so its `fish_prompt` handlers
   never run.
3. Under `script`, fish 4.9.3 draws its prompt, but first waits 10 seconds
   on a terminal query that `script`'s terminal does not answer;
   `fish_features=no-query-term` turns the queries off.
   Lines typed ahead of the first prompt were echoed and then dropped, and
   fish exited at end of input without running them.

So the test runs each shell on a pseudo-terminal, sets a prompt it can
recognise, and types each line only after the prompt appears.
[pty-process][pty-process] 0.5.4 opens the terminal and starts the shell as
its session leader; its only dependency is `rustix`, which ctl-core already
locks.

## 5. Recommendation

1. A `shell-init` feature with a flattenable `init <SHELL>` verb, sharing
   the `Shell` type and the script view the `completion` feature uses.
2. A hook the CLI declares once, `<bin> <subcommand>` guarded by a variable,
   validated as plain words, from which ctl-core writes all shell syntax.
3. The binary called by name, the hook removed before it runs, the install
   guarded against a second `source`, and the bash hook returning the
   status it found.
4. Each script's first line names the line that loads it, in the form all
   five CLIs document.
5. Tests that snapshot each script, check it with its shell and with
   `shellcheck` for bash, and run it in bash, zsh, and fish on a
   pseudo-terminal against a fake binary.

[a-cmd]: https://github.com/atuinsh/atuin/blob/v18.24.5/crates/atuin/src/command/client/init.rs#L13-L30
[a-init]: https://github.com/atuinsh/atuin/blob/v18.24.5/crates/atuin/src/command/client/init.rs#L74-L82
[a-install]: https://github.com/atuinsh/atuin/blob/v18.24.5/docs/docs/guide/installation.md#L186
[bash-5.3]: https://ftp.gnu.org/gnu/bash/bash-5.3.tar.gz
[bash-set]: https://www.gnu.org/software/bash/manual/html_node/The-Set-Builtin.html
[d-bash]: https://github.com/direnv/direnv/blob/v2.38.2/internal/cmd/shell_bash.go#L11-L30
[d-cmd]: https://github.com/direnv/direnv/blob/v2.38.2/internal/cmd/cmd_hook.go#L19-L47
[d-config]: https://github.com/direnv/direnv/blob/v2.38.2/internal/cmd/config.go#L103-L115
[d-hook]: https://github.com/direnv/direnv/blob/v2.38.2/docs/hook.md#L24
[d-tests]: https://github.com/direnv/direnv/blob/v2.38.2/internal/cmd/shell_hook_test.go#L13-L60
[d-zsh]: https://github.com/direnv/direnv/blob/v2.38.2/internal/cmd/shell_zsh.go#L11-L29
[fish-n]: https://github.com/fish-shell/fish-shell/blob/4.9.3/doc_src/cmds/fish.rst#L47-L48
[fish-restore]: https://github.com/fish-shell/fish-shell/blob/4.9.3/src/event.rs#L501-L504
[m-activate]: https://github.com/jdx/mise/blob/v2026.11.4/src/cli/activate.rs#L84-L100
[m-cmd]: https://github.com/jdx/mise/blob/v2026.11.4/src/cli/activate.rs#L14-L81
[m-setup]: https://github.com/jdx/mise/blob/v2026.11.4/docs/shell-setup.md#L43
[pty-process]: https://github.com/doy/pty-process/tree/v0.5.4/src/blocking
[s-cmd]: https://github.com/starship/starship/blob/v1.27.0/src/main.rs#L85-L90
[s-init]: https://github.com/starship/starship/blob/v1.27.0/src/init/mod.rs#L19-L38
[s-readme]: https://github.com/starship/starship/blob/v1.27.0/docs/README.md#L105
[z-bash-hook]: https://github.com/ajeetdsouza/zoxide/blob/v0.10.0/templates/bash.txt#L64-L74
[z-bash-status]: https://github.com/ajeetdsouza/zoxide/blob/v0.10.0/templates/bash.txt#L38-L44
[z-ci]: https://github.com/ajeetdsouza/zoxide/blob/v0.10.0/.github/workflows/ci.yml#L39-L57
[z-cmd]: https://github.com/ajeetdsouza/zoxide/blob/v0.10.0/src/cmd/cmd.rs#L124-L145
[z-fish-hook]: https://github.com/ajeetdsouza/zoxide/blob/v0.10.0/templates/fish.txt#L55-L69
[z-init]: https://github.com/ajeetdsouza/zoxide/blob/v0.10.0/src/cmd/init.rs#L13-L16
[z-readme]: https://github.com/ajeetdsouza/zoxide/blob/v0.10.0/README.md#L316-L321
[z-tests]: https://github.com/ajeetdsouza/zoxide/blob/v0.10.0/src/shell.rs#L36-L350
[zsh-callhook]: https://github.com/zsh-users/zsh/blob/zsh-5.9.2/Src/utils.c#L1499
[zsh-noexec]: https://github.com/zsh-users/zsh/blob/zsh-5.9.2/Doc/Zsh/options.yo#L1793-L1799
[zsh-restore]: https://github.com/zsh-users/zsh/blob/zsh-5.9.2/Src/exec.c#L6087-L6091
