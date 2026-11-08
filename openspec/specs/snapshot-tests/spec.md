# snapshot-tests Specification

## Purpose

How ctl-core tests rendered scripts and help: reviewed snapshots that CI never writes, and a shell check for each script on a closed set of shells.

## Requirements

### Requirement: Rendered output is a reviewed snapshot

A ctl-core test that checks a rendered shell script or a rendered help page
SHALL compare it with an insta file snapshot under `tests/snapshots/`, and
SHALL NOT hold the expected text in a string literal.

#### Scenario: A script changes

- **WHEN** a change alters one line of the bash completion script for `toy`
- **THEN** `tests/snapshots/completion__bash.snap` differs by that line, and the test fails until the snapshot is accepted

#### Scenario: The help changes

- **WHEN** the `completion` verb's help renders at width 100 with colour off
- **THEN** it equals `tests/snapshots/completion__help.snap`

### Requirement: CI never writes a snapshot

CI SHALL run the tests with `INSTA_UPDATE=no`, so a missing or changed
snapshot fails the test and no snapshot file is written.

#### Scenario: A snapshot is missing in CI

- **WHEN** a test asserts a snapshot that has no file and `MISE_ENV=ci` is loaded
- **THEN** the test fails and no `.snap` or `.snap.new` file is created

### Requirement: A rendered script passes its shell's check

A test that renders a shell script SHALL run it through its shell's syntax
check: `bash -n` and `shellcheck --shell=bash --enable=all` for bash,
`zsh -f -n` for zsh, and `fish --no-config -n` for fish.

#### Scenario: The fish script

- **WHEN** the fish completion script for `toy` is rendered
- **THEN** `fish --no-config -n` exits 0 on it

#### Scenario: The bash script from clap_complete

- **WHEN** the bash completion script for `toy` is rendered
- **THEN** `bash -n` exits 0, and shellcheck exits 0 with only SC2207, SC2181, and SC2250 excluded

### Requirement: A missing tool fails the check

The shell check SHALL fail a test, naming the tool, when it cannot start
the shell or shellcheck it needs.

#### Scenario: fish is not installed

- **WHEN** `fish` is not on `PATH` and a test checks a fish script
- **THEN** the test fails with a message that names `fish` and `mise run test`

### Requirement: The shell set is closed

`ctl_core::shell::Shell` SHALL NOT be `#[non_exhaustive]`, and its rustdoc
SHALL state that adding a shell is a breaking release.

#### Scenario: A consumer matches every shell

- **WHEN** a consumer matches `Shell::Bash`, `Shell::Zsh`, and `Shell::Fish` with no wildcard arm
- **THEN** the match compiles
