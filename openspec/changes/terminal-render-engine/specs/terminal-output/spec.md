## Purpose

How the rendering engine decides, for one output stream, whether to write
colour and at what depth, how wide the output is, where escape sequences are
written, and what happens when the reader closes the stream.

## ADDED Requirements

### Requirement: An explicit colour mode ignores the colour variables

The engine SHALL write colour under `ColorMode::Always` and SHALL write none
under `ColorMode::Never`, whatever `NO_COLOR`, `CLICOLOR`, `CLICOLOR_FORCE`,
`FORCE_COLOR`, and `TERM=dumb` hold. Both modes SHALL still read the variables
that set the colour depth and the width.

#### Scenario: Never beats a forcing variable

- **WHEN** a document renders with `--color never` and `CLICOLOR_FORCE=1`
- **THEN** the output holds no ESC byte

#### Scenario: Always beats NO_COLOR

- **WHEN** a document renders with `--color always`, `NO_COLOR=1`, and stdout is a pipe
- **THEN** the output holds SGR sequences

#### Scenario: Always still follows the depth variable

- **WHEN** a document renders with `--color always`, `COLORTERM=truecolor`, and stdout is a pipe
- **THEN** a 24-bit colour is written as a 24-bit sequence

#### Scenario: Never still follows COLUMNS

- **WHEN** a document renders with `--color never`, `COLUMNS=100`, and stdout is a pipe
- **THEN** the output lays out to 98 cells

### Requirement: Automatic colour follows a fixed order

Under `ColorMode::Auto` the engine SHALL take the first rule that applies:
`NO_COLOR` non-empty turns colour off; `CLICOLOR_FORCE` non-empty and not `0`
turns it on; `FORCE_COLOR` non-empty and not `0` or `false` turns it on;
`CLICOLOR=0` turns it off; `TERM=dumb` turns it off; otherwise colour is on
only when the stream is a terminal.

#### Scenario: NO_COLOR wins over CLICOLOR_FORCE

- **WHEN** `NO_COLOR=1` and `CLICOLOR_FORCE=1` are both set and stdout is a terminal
- **THEN** the output holds no ESC byte

#### Scenario: Forced colour into a pipe

- **WHEN** `FORCE_COLOR=3` is set and stdout is a pipe
- **THEN** the output holds SGR sequences

#### Scenario: FORCE_COLOR=0

- **WHEN** `FORCE_COLOR=0` is the only colour variable and stdout is a pipe
- **THEN** the output holds no ESC byte

#### Scenario: FORCE_COLOR=0 turns nothing off

- **WHEN** `FORCE_COLOR=0` is set and stdout is a terminal
- **THEN** the output holds SGR sequences

#### Scenario: A NO_COLOR that is not UTF-8

- **WHEN** the process environment holds `NO_COLOR` with bytes that are not valid UTF-8 and stdout is a terminal
- **THEN** the output holds no ESC byte

#### Scenario: A pipe with no variable

- **WHEN** no colour variable is set and stdout is a pipe
- **THEN** the output holds no ESC byte

### Requirement: Colour is decided per stream

The engine SHALL decide colour and width for the stream the output is written
to, so stdout and stderr can differ.

#### Scenario: Help on stderr

- **WHEN** stdout is a pipe, stderr is a terminal, and a usage error prints help to stderr under `--color auto`
- **THEN** the help on stderr holds SGR sequences

### Requirement: Colour depth follows the environment

When colour is on, the engine SHALL use 24-bit colour when `COLORTERM` is
`truecolor` or `24bit` or `FORCE_COLOR=3`, 256 colours when `TERM` contains
`256color` or `FORCE_COLOR=2`, and 16 colours otherwise, and SHALL convert
every colour to the nearest colour at that depth. At 16 colours it SHALL drop
the underline colour.

#### Scenario: A 24-bit colour on a 16-colour terminal

- **WHEN** a span styled with RGB `(255, 0, 0)` renders with colour on, `TERM=xterm` and no `COLORTERM`
- **THEN** the span is written with the 16-colour code for bright red or red

#### Scenario: An underline colour at 16 colours

- **WHEN** a span with an underline colour renders with colour on at 16 colours
- **THEN** the output holds no SGR 58 sequence for that span

### Requirement: Escape sequences are written only by the writer

The engine SHALL write no escape sequence when colour is off, and SHALL end
every line with no style open, so each line stands alone. It SHALL write a
reset after the last styled span of a line, and none after a line that already
ended in plain text. It SHALL write nothing for a span with no text.

#### Scenario: A coloured table cut by head

- **WHEN** a coloured list of 10 rows renders and the reader keeps only the first 3 lines
- **THEN** in each of the 3 lines the last escape sequence is `ESC[0m`, or the line carries none

#### Scenario: A style that ends mid-line

- **WHEN** a line holds a red span followed by a plain span
- **THEN** the output holds one reset, before the plain text, and none after it

#### Scenario: A span with no text

- **WHEN** a line holds a red span, an empty green span, and a red span
- **THEN** the output holds one red sequence and one reset

#### Scenario: Captured output under Auto

- **WHEN** a test captures a view under `ColorMode::Auto` and no stream is a terminal
- **THEN** the captured text holds no ESC byte

### Requirement: Width comes from the stream being written

When no explicit width is set, the engine SHALL take the width of the
terminal behind the stream being written, then a non-zero `COLUMNS`, then the
fallback width, and SHALL apply the column buffer and minimum only to a detected
width. The width SHALL be at least 1, and the column buffer SHALL read no
variable unless the caller names one.

#### Scenario: Output piped, COLUMNS set

- **WHEN** stdout is a pipe and `COLUMNS=100`
- **THEN** no rendered line of a wrapped paragraph is wider than 98 cells

#### Scenario: Nothing to detect

- **WHEN** stdout is a pipe and `COLUMNS` is unset
- **THEN** the output lays out to 80 cells

### Requirement: A closed pipe ends the program quietly

When the reader closes stdout before the output of a successful command is
written, `App` SHALL exit with code 0 and write nothing to stderr, unless the
consumer chose to treat a closed pipe as a failure.

#### Scenario: Piped to head

- **WHEN** a CLI built on `App` prints 10,000 rows and its stdout is piped to `head -1`
- **THEN** the CLI exits with code 0
- **AND** stderr is empty

#### Scenario: The consumer wants the error

- **WHEN** the consumer sets the closed-pipe policy to fail and the reader closes stdout
- **THEN** the CLI exits with a non-zero code

### Requirement: A closed pipe never hides a failure

When the reader closes the stream that carries a failed command's error or a
parse error, `App` SHALL exit with that failure's own code under either
policy.

#### Scenario: A failed command whose error reader stops early

- **WHEN** a command fails with a long error and its stderr is piped to `head -1`
- **THEN** the CLI exits with code 1

#### Scenario: A parse error whose reader stops early

- **WHEN** a parse error with a long message is written to a stderr piped to `head -1`
- **THEN** the CLI exits with code 2 under the default policy
- **AND** it exits with code 2 when the consumer sets the closed-pipe policy to fail
