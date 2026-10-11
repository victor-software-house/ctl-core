## ADDED Requirements

### Requirement: A template declares the file it serves

`served::render` SHALL read a template's top-level exports `path`, `name`,
and `executable` as one schema after rendering it. An unsaid `path` SHALL mean
the repository root, an unsaid `name` the template's file name without the
suffix, and an unsaid `executable` false. Any other export SHALL be ignored.

#### Scenario: Nothing declared

- **WHEN** `.ctl/templates/q.jinja` declares nothing and renders `served`
- **THEN** the target is `q` and the body is `served` with its newline

#### Scenario: A directory and a name

- **WHEN** a template sets `path = "examples"` and `name = "mise.toml"`
- **THEN** the target is `examples/mise.toml`

#### Scenario: An unknown export

- **WHEN** a template sets `tool` and uses it in its body
- **THEN** rendering succeeds and `tool` is not a declaration

### Requirement: A declaration that does not fit names the template and the field

`render` SHALL fail with an error that starts with the template's name when
`path` is outside the repository, `name` is not one file name, a value has the
wrong type, the file name does not end in the suffix, or the template uses an
undefined name.

#### Scenario: A climbing path

- **WHEN** a template sets `path = "../escaped"`
- **THEN** the error starts with `.ctl/templates/q.jinja: ` and contains
  `path: must stay inside the repository`

### Requirement: Writing sets the one mode bit git records

`Served::write(root)` SHALL create the target's directories, write the body,
and on Unix set mode 0755 when `executable` is true and 0644 otherwise,
correcting a stale mode. It SHALL refuse a target, or any directory above it
under `root`, that is a symlink, and a created directory that does not
resolve inside `root`.

#### Scenario: An executable task and a stale README

- **WHEN** a template with `executable = true` and one without are written,
  and the README was 0755 before
- **THEN** the task file is 0755 and the README is 0644

#### Scenario: A symlinked target

- **WHEN** the target path is a symlink to a file outside the root
- **THEN** `write` fails with `must not be a symlink` and the outside file is
  unchanged

#### Scenario: A symlinked directory above the target

- **WHEN** `tasks` under the root is a symlink to a directory outside it, and
  a template declares `path = "tasks/q"`
- **THEN** `write` fails with `tasks must not be a symlink` and nothing is
  written outside the root
