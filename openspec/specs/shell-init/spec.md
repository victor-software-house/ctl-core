# shell-init Specification

## Purpose

The `init <SHELL>` verb a ctl CLI gets from ctl-core: the hook script it prints for bash, zsh, and fish, and when that hook runs.

## Requirements

### Requirement: A hook is declared from plain words

`Hook::new(bin, subcommand, variable)` SHALL return a `Hook` when `bin`
and `subcommand` are letters, digits, `-`, and `_` starting with a letter
or digit, and `variable` is letters, digits, and `_` not starting with a
digit.
Otherwise it SHALL return the `HookError` variant that names the first
part that fails.

#### Scenario: A subcommand with shell syntax

- **WHEN** `Hook::new("toy-cli", "pick;ls", "TOY_BLOCK")` runs
- **THEN** it returns `HookError::Subcommand("pick;ls")`

### Requirement: The verb prints the hook script for one shell

`InitCommand`, flattened into a CLI's subcommands, SHALL parse
`<bin> init <SHELL>` for `bash`, `zsh`, and `fish`, and
`InitCommand::script(hook)` SHALL return `hook.script(shell)`.

#### Scenario: fish

- **WHEN** `toy-cli init fish` is parsed and its command's script is taken for the toy hook
- **THEN** it equals `hook.script(Shell::Fish)`

### Requirement: The script calls the binary by name

Each script SHALL call the binary as `command <bin> <subcommand>`, and
SHALL contain no path and no value read when it was rendered.
Its first line SHALL be a comment naming the startup file and the line
that loads it.

#### Scenario: The bash script

- **WHEN** the bash script for `toy-cli`, `pick`, and `TOY_BLOCK` is rendered
- **THEN** it equals `ctl-core/tests/snapshots/shell_init__bash.snap`, whose first line is `# Load in ~/.bashrc: eval "$(toy-cli init bash)"`

### Requirement: Each script passes its shell's check

Each script SHALL pass `bash -n` and `shellcheck --shell=bash --enable=all`
with no exclusions, `zsh -f -n`, or `fish --no-config -n`.

#### Scenario: The bash script

- **WHEN** the bash script is rendered
- **THEN** `bash -n` and shellcheck exit 0 on it with nothing excluded

### Requirement: The hook runs once before the first prompt

A shell whose startup file loads the script SHALL run `<bin> <subcommand>`
once, in the foreground, before its first prompt, when `variable` is
non-empty and `bin` is on `PATH`, and SHALL NOT run it at a later prompt.

#### Scenario: Loaded twice, three prompts

- **WHEN** bash, zsh, or fish starts on a terminal with a startup file that loads the script twice, with `TOY_BLOCK` set and `toy-cli` on `PATH`, and two lines are typed before `exit`
- **THEN** `toy-cli` ran exactly once, with the argument `pick`

#### Scenario: Another tool rewrites PROMPT_COMMAND

- **WHEN** bash loads the script, then appends to `PROMPT_COMMAND` as bash-preexec 0.7.0 does when it loads, and two lines are typed before `exit`
- **THEN** `toy-cli` ran exactly once

### Requirement: The hook keeps the last exit status

The hook SHALL leave the exit status the next prompt reads unchanged.

#### Scenario: The binary fails

- **WHEN** the hook runs a `toy-cli` that exits 7, and the prompt shows the last exit status
- **THEN** the first prompt shows 0 in bash, zsh, and fish

### Requirement: The hook needs its variable and its binary

The hook SHALL run nothing, and print nothing, when `variable` is unset or
empty, or when `bin` is not on `PATH`.

#### Scenario: The variable is unset

- **WHEN** a shell loads the script without `TOY_BLOCK` set
- **THEN** `toy-cli` never runs

#### Scenario: The binary is missing

- **WHEN** a shell loads the script with `TOY_BLOCK` set and no `toy-cli` on `PATH`
- **THEN** nothing runs, and the terminal shows no "not found" or "Unknown command" error

#### Scenario: A function has the binary's name

- **WHEN** a shell loads the script with `TOY_BLOCK` set, no `toy-cli` on `PATH`, and a shell function named `toy-cli`
- **THEN** nothing runs, and the terminal shows no "not found" error
