# ctl-input

Declared input for CLIs: parse YAML, JSON or TOML, validate once with garde, and place every problem on its file and line.
This guide covers this crate only.
The repository's root [`AGENTS.md`](../AGENTS.md) holds the workspace-wide rules.

## What it owns

- `Input`, `Limits`, `FieldPath`, `InputError`, and `Problem`, with the YAML, JSON and TOML formats behind the `yaml`, `json` and `toml` features.
- The garde loader, `Input::check` and `Input::load`, behind `validate`.
- The located tree (`tree`), the `Format` trait with its `read` and `resolve` (`format`), and the `Formats` registry with its process default (`formats`).
  `yaml_format`, `json_format` and `toml_format` are the three built-in formats, `YAML`, `JSON` and `TOML`, and each keeps its own scalar rules.
  `json` is the own RFC 8259 reader behind `json_format`: a scanner of spanned tokens and one loop with an explicit stack that builds the tree, with no dependency.
- The tree an `Input` keeps (`kept`): the text is read into it once, under the limits and the reading it needs, and `parse`, `place`, `place_key`, `check` and `top_level` all work on it.
  `Input` holds its format as a `&'static dyn Format`, so a CLI's own format is read, placed and worded like the built-in ones.
  `Input::open` takes the process's `Formats`, `Formats::open` a registry of the caller's.

It depends on no other crate in this repository.
The crate is empty unless `yaml`, `json` or `toml` is on.

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
The tests of the tree, the trait, the registry, and the kept tree are one binary, `tests/it`, and its `fixtures.rs` reads the texts in `tests/fixtures/` and keeps a snapshot of how each is read and placed.
`json_suite.rs` reads every file of the vendored JSONTestSuite in `tests/fixtures/json-test-suite/` and checks it against a table of verdicts.
The integration tests, `input.rs`, `input_json.rs`, `input_json_off.rs`, `input_limits.rs`, `input_toml.rs`, and `input_toml_alone.rs`, live in `ctl-core/tests/` and read the crate through `ctl_core::input`.
`README.md` is the crates.io page.
