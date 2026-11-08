## ADDED Requirements

### Requirement: A shell is a Clap value

`ctl_core::shell::Shell` SHALL parse from the Clap values `bash`, `zsh`,
and `fish`, and SHALL reject any other value with Clap's invalid-value
error.

#### Scenario: A known shell

- **WHEN** a CLI's argument has type `Shell` and the command line passes `zsh`
- **THEN** the argument parses as `Shell::Zsh`

#### Scenario: An unknown shell

- **WHEN** the command line passes `elvish`
- **THEN** the parse fails with `ErrorKind::InvalidValue`

### Requirement: Every ctl CLI has the same completion verb

A CLI that flattens `CompletionCommand` into its subcommands SHALL accept
`<bin> completion <SHELL>`, whose help SHALL read "Print the script that
loads shell completion for this command." and SHALL name bash, zsh, and fish
in the argument's help.

#### Scenario: The verb parses

- **WHEN** a toy CLI named `toy` flattens `CompletionCommand` and runs `toy completion fish`
- **THEN** it parses as `CompletionCommand::Completion` with shell `Shell::Fish`

#### Scenario: The verb's help

- **WHEN** ctl-core renders help for `toy completion`
- **THEN** the help holds the about line, `<SHELL>`, and "bash, zsh, or fish"

### Requirement: The script is the dynamic registration

`Shell::script(bin)` SHALL return `clap_complete`'s dynamic registration for
that shell, with the variable `COMPLETE`, the registered command `bin`, and
the completer `bin` called by name.

#### Scenario: fish

- **WHEN** `Shell::Fish.script("toy")` runs
- **THEN** its text is the line `complete --keep-order --exclusive --command toy --arguments "(COMPLETE=fish toy -- (commandline --current-process --tokenize --cut-at-cursor) (commandline --current-token))"`

#### Scenario: bash

- **WHEN** `Shell::Bash.script("toy")` runs
- **THEN** its text sets `COMPLETE="bash"`, calls `"toy" -- "${words[@]}"`, and registers with `complete -o nospace -o bashdefault -o nosort -F _clap_complete_toy toy`

### Requirement: The zsh script completes on the first TAB from fpath

The zsh script SHALL start with `#compdef <bin>` and SHALL end with lines
that run the completer when zsh autoloads the file as the function
`_<bin>`, and do nothing when the file is sourced.

#### Scenario: The appended lines

- **WHEN** `Shell::Zsh.script("state-sync")` runs
- **THEN** its text starts with `#compdef state-sync` and ends with the three lines `if [[ ${funcstack[1]} == _state-sync ]]; then`, `_clap_dynamic_completer_state_sync "$@"` indented four spaces, and `fi`

### Requirement: The script survives quiet and colour settings

`completion::view(format)` SHALL print a `Script` in `format`, with no
colour, whatever quiet setting the CLI's own view would apply. Pretty output
SHALL be the script text ending in exactly one newline. JSON output SHALL be
an object with the shell's name under `shell` and the script under `text`.

#### Scenario: Pretty output

- **WHEN** the `Script` for `Shell::Zsh` and `toy` is captured through `completion::view(OutputFormat::Pretty)`
- **THEN** the captured stdout is the script text with one trailing newline and no ESC byte

#### Scenario: JSON output

- **WHEN** the same script is captured through `completion::view(OutputFormat::Json)`
- **THEN** stdout parses as JSON with `shell` equal to `"zsh"` and `text` equal to the script text

### Requirement: The environment prints the registration

`App::completion()` SHALL add a pre-parse hook. When `COMPLETE` names a
supported shell and no word follows a `--` in argv, the hook SHALL print the
same text as `Shell::script(bin)` and exit 0.

#### Scenario: A registration request

- **WHEN** `COMPLETE=zsh` is set and `toy` runs with no arguments
- **THEN** stdout holds `Shell::Zsh.script("toy")`'s text and the exit code is 0

### Requirement: The environment prints candidates

When `COMPLETE` names a supported shell and words follow `--`, the hook
SHALL print `clap_complete`'s candidates for those words and exit 0.

#### Scenario: A candidate request

- **WHEN** `COMPLETE=fish` is set and `toy -- toy st` runs
- **THEN** stdout holds a line starting with `status` and the exit code is 0

### Requirement: An ordinary run is left alone

When `COMPLETE` is unset, empty, or `0`, the run SHALL continue to the parse.

#### Scenario: A disabled request

- **WHEN** `COMPLETE=0` is set and `toy status` runs
- **THEN** the CLI runs `status` as without the hook

### Requirement: An unsupported shell is an error

When `COMPLETE` names an unsupported shell, the hook SHALL report the error
on stderr, naming bash, zsh, and fish, and exit non-zero.

#### Scenario: An unsupported shell

- **WHEN** `COMPLETE=elvish` is set and `toy` runs with no arguments
- **THEN** stderr names bash, zsh, and fish, and the exit code is not 0
