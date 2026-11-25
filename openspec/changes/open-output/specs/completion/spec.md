## MODIFIED Requirements

### Requirement: The script survives quiet and colour settings

`shell::view(format)` SHALL print a `Script` in `format`, with no colour, whatever quiet setting the CLI's own view would apply.
Every format other than `json`, built-in or registered, SHALL print the script text unchanged, ending in exactly one newline.
JSON output SHALL be an object with the shell's name under `shell` and the script under `text`.

#### Scenario: Pretty output

- **WHEN** the `Script` for `Shell::Zsh` and `toy` is captured through `shell::view` with `pretty`
- **THEN** the captured stdout is the script text with one trailing newline and no ESC byte

#### Scenario: Piped output

- **WHEN** `eval "$(toy completion zsh)"` runs, so stdout is a pipe and the format is `tsv`
- **THEN** the captured stdout is the same script text as in `pretty`

#### Scenario: CSV output

- **WHEN** `TOY_FORMAT=csv toy completion zsh` runs
- **THEN** the captured stdout is the same script text as in `pretty`

#### Scenario: JSON output

- **WHEN** the same script is captured through `shell::view` with `json`
- **THEN** stdout parses as JSON with `shell` equal to `"zsh"` and `text` equal to the script text
