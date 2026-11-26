## ADDED Requirements

### Requirement: Every command takes an input document

When ctl-core's `app` feature and an `input-*` feature are both on, `App` SHALL add `--input <FILE>` and `--input-format <NAME>` to every command, with no short forms.
When the command binds `--format` to `<PREFIX>_FORMAT`, the two flags SHALL read `<PREFIX>_INPUT` and `<PREFIX>_INPUT_FORMAT`.
`App::without_input()` SHALL remove both.

#### Scenario: The flags are there

- **WHEN** a CLI with `env_prefix = "QCTL"`, built with `app` and `input`, prints `qctl add --help`
- **THEN** the help lists `--input <FILE>` and `--input-format <NAME>` under "Input", reading `QCTL_INPUT` and `QCTL_INPUT_FORMAT`

#### Scenario: A CLI removes them

- **WHEN** the same CLI calls `App::without_input()` and runs `qctl add --input a.yaml`
- **THEN** the parse fails on `--input` as an unexpected argument

### Requirement: The document fills the arguments the command line left

A document SHALL be read from `--input FILE` by `--input-format` or the file's extension, or from `--input -` on stdin, which SHALL need `--input-format`.
Its root SHALL be a mapping whose keys are the selected command's long names and positional ids.
Each value SHALL pass clap's own value parser and rules for its argument.
Precedence SHALL be the command line, then the environment variable, then the document, then the default.

#### Scenario: A flag from the document

- **WHEN** `task.yaml` holds `title: Fix the cache` and `priority: 2`, and the user runs `qctl add --input task.yaml`
- **THEN** the handler gets the title `Fix the cache` and the priority 2

#### Scenario: The command line wins

- **WHEN** `task.yaml` holds `priority: 2`, and the user runs `qctl add --input task.yaml --priority 5`
- **THEN** the handler gets the priority 5

#### Scenario: The environment wins over the document

- **WHEN** `QCTL_PRIORITY=3` is set, `task.yaml` holds `priority: 2`, and the user runs `qctl add --input task.yaml`
- **THEN** the handler gets the priority 3

#### Scenario: Stdin needs a format

- **WHEN** the user runs `qctl add --input -` with no `--input-format`
- **THEN** the parse fails with one problem naming `--input-format`

### Requirement: Each document value encodes by its argument's action

A flag that sets true SHALL read `true` as given, and `false` as its `--no-` side when one exists, else as not given.
A counted flag SHALL read a non-negative integer as that many repeats.
An option taking several values SHALL read a sequence as its values.
`null` SHALL leave the key out.
A mapping, or a sequence for an option that takes one value, SHALL be refused on its key.

#### Scenario: A boolean pair

- **WHEN** `task.yaml` holds `pr: false`, and the command declares `--pr` and `--no-pr`
- **THEN** the handler reads the value `--no-pr` gives

#### Scenario: A list

- **WHEN** `task.yaml` holds `label: [cache, perf]` for an option that appends
- **THEN** the handler gets the labels `cache` and `perf`, in that order

### Requirement: A document conflict follows a stated policy

A document key SHALL lose to the command line when the command line sets the same argument, the other side of its `--x`/`--no-x` pair, or an argument it conflicts with.
It SHALL lose to a kept variable for the same argument, or one that conflicts with it.
By default a losing key SHALL be dropped.
`App::input_conflicts(InputConflicts::Fail)` SHALL turn each into a problem on the key that names what it lost to.

#### Scenario: Dropped by default

- **WHEN** `task.yaml` holds `pr: true` on line 4, and the user runs `qctl add --input task.yaml --no-pr`
- **THEN** the handler reads the value `--no-pr` gives, and no problem is reported

#### Scenario: Refused when the CLI asks

- **WHEN** the CLI calls `App::input_conflicts(InputConflicts::Fail)` and the user runs the same command
- **THEN** the parse fails with one problem on `task.yaml` line 4 that names `--no-pr`

### Requirement: The document supplies positionals only alone

The document SHALL supply a positional only when the command line gives no positional at that level and holds no `--`, and the command declares no `last(true)` positional and no `allow_missing_positional`.
Otherwise a positional key SHALL be refused on its line.

#### Scenario: A positional from the document

- **WHEN** `show.yaml` holds `id: QCTL-012` for the positional `id`, and the user runs `qctl show --input show.yaml`
- **THEN** the handler gets the id `QCTL-012`

#### Scenario: Both give positionals

- **WHEN** the user runs `qctl show QCTL-007 --input show.yaml` with the same document
- **THEN** the parse fails with one problem on `show.yaml` line 1 for `id`

### Requirement: Every document problem names its place

An unknown key, a value no argument can take, and a key refused by policy SHALL each be a problem with the document's file, line and column, all reported together.
A clap error caused by a document value SHALL be placed on that key, or on its value for an invalid value.
Each `input-*` feature SHALL turn on clap's `error-context` when `cli` is on, and `app` and `usage` without an input feature SHALL not.

#### Scenario: An unknown key

- **WHEN** `task.yaml` holds `due: friday` on line 2
- **THEN** the parse fails with one problem on `task.yaml` line 2, column 1, for the unknown key `due`

#### Scenario: A value clap refuses

- **WHEN** `task.yaml` holds `priority: high` on line 3 for a `u8` option
- **THEN** the parse fails with one problem on `task.yaml` line 3, column 11, from clap's value parser
