# ctl-input

Declared input for CLIs: parse YAML or TOML, validate once with garde, and place every problem on its file and line.
This guide covers this crate only.
The repository's root [`AGENTS.md`](../AGENTS.md) holds the workspace-wide rules.

## What it owns

- `Input`, `Format`, `Limits`, `FieldPath`, `InputError`, and `Problem`, with the YAML and TOML readers behind the `yaml` and `toml` features.
- The garde loader, `Input::check` and `Input::load`, behind `validate`.

It depends on no other crate in this repository.
The crate is empty unless `yaml` or `toml` is on.

## What it must not own

- Presentation of an `InputError`.
  ctl-core renders it.
- Shared garde rules such as `at_least_one`.
  They live in ctl-core's `validate` module.

## Public API

`ctl_core::input` re-exports this crate, so a public item here is an item of ctl-core's API too.
A break needs a minor changeset at the repository root while the workspace is below 1.0.

## Tests

The unit tests sit beside their modules.
The integration tests, `input.rs`, `input_limits.rs`, `input_toml.rs`, and `input_toml_alone.rs`, live in `ctl-core/tests/` and read the crate through `ctl_core::input`.
`README.md` is the crates.io page.
