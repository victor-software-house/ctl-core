# Read TOML input without the YAML and garde crates

## Why

A CLI that reads only TOML config still compiles a YAML parser, a YAML
location index, and garde.
The `input-toml` feature enables `input`, and `input` enables
`serde-saphyr`, `yamled`, and `validate`.

At the `input` size tier, `app,input`, those crates add 568,804 bytes of
`.text` over `app`: 1,040,778 against 471,974 in
[`[workspace.metadata.size]`][size].
A consumer moving onto ctl-core's `App` and `input-toml` reads only TOML,
as the notes of [CTC-025][ledger] record, so it carries all three for
nothing.

The split is not a feature edit alone.
`Limits` is defined as overrides of serde-saphyr's `Budget`, and the TOML path
reads `max_depth` and `max_nodes` from that `Budget`
([`limits.rs`][limits], [`toml_text.rs`][toml-text]).

The repository's limits file, `.ctl/input.yaml`, is YAML.
`Input::check` and `Input::load` are garde.

This is [CTC-025][ledger], in the scope of ctl-core 0.7.0.

## What Changes

- A new `input-yaml` feature carries the YAML format, the 17 YAML-only
  `Limits` fields, `.ctl/input.yaml`, `use_repo_limits`, `Input::new`,
  `Input::read`, and `Input::frontmatter`.
- **BREAKING** `input-toml` no longer enables `input`. With it alone,
  ctl-core links neither `serde-saphyr`, `yamled`, nor `garde`.
- `input` keeps its meaning: YAML input with validation. It becomes
  `input-yaml` plus `validate`, so qctl, verctl, and forkctl need no
  manifest change.
- **BREAKING** `Input::check` and `Input::load` exist only with `validate`.
- ctl-core owns `Limits`. Three fields, `max_reader_input_bytes`,
  `max_depth`, and `max_nodes`, exist in every build, with defaults
  ctl-core states as constants. They equal serde-saphyr's defaults, so a YAML
  text meets the same limits as before.
- **BREAKING** `Limits::budget` is no longer public: it returns a
  serde-saphyr type.
- `Input::new_with_format` builds an input of any enabled format from text in
  memory, for a build without YAML.
- A TOML-only build applies the call and process tiers, then ctl-core's
  defaults. Its limit message ends with `the tool sets this limit`, since
  there is no repository file to raise it in.
- `check:tree` fails when `input-toml` alone reaches `serde-saphyr`,
  `yamled`, or `garde`, and `check:features` compiles `input-yaml` and
  `input-toml` alone.
- A new size tier measures `app,input-toml`.

## Capabilities

### New Capabilities

None.

### Modified Capabilities

- `declared-input`: the format features, which API each feature carries,
  who owns the limits and their defaults, and the limit message without a
  repository file.

## Impact

- `ctl-core/src/input.rs`, `ctl-core/src/input/limits.rs`,
  `ctl-core/src/input/toml_text.rs`, `ctl-core/src/app.rs`
  (`App::input_limits`), and `ctl-core/src/lib.rs`.
- `ctl-core/Cargo.toml` features, `check:features`, `check:tree`, and the
  size tiers in the root `Cargo.toml`.
- `AGENTS.md`, the crate docs, and a changeset naming the break.
- No change to the serde-saphyr fork.

[size]: ../../../../Cargo.toml
[limits]: https://github.com/victor-software-house/ctl-core/blob/9141d9f9a5fa5f7b32995945cd0101011f311486/ctl-core/src/input/limits.rs
[toml-text]: https://github.com/victor-software-house/ctl-core/blob/9141d9f9a5fa5f7b32995945cd0101011f311486/ctl-core/src/input/toml_text.rs
[ledger]: ../../../../tasks.yaml
