# served-baseline Specification

## Purpose
The Rust configuration every ctl repository shares, rendered from templates in the ctl-core release a repository links, with marked blocks for shared files and a self-check that keeps ctl-core on its own baseline.

## Requirements

### Requirement: The baseline renders five whole files

`baseline::render(root, &config)` SHALL return one `served::Served` for each
of `rust-toolchain.toml`, `rustfmt.toml`, `clippy.toml`, `deny.toml`, and
`lefthook.yml` that the repository does not own, rendered from the template
embedded in the ctl-core release.

#### Scenario: A crate with defaults

- **WHEN** `.ctl/rust.yaml` sets `published: true`
- **THEN** `render` returns those five files and both block files,
  `rust-toolchain.toml` pins `channel = "1.99.0"`, and the `tools` block
  lists cargo-semver-checks

#### Scenario: A tokio crate

- **WHEN** `.ctl/rust.yaml` sets `tokio: true`
- **THEN** the rendered `clippy.toml` lists `tokio::sync::MutexGuard` under
  `await-holding-invalid-types` and bans
  `tokio::sync::mpsc::unbounded_channel`

### Requirement: Append-only entries reach their file with their reason

Each entry of an append-only list SHALL appear in the rendered file with its
`reason`. An entry without a `reason` SHALL fail the load of
`.ctl/rust.yaml`, naming the field and its line.

#### Scenario: An extra license

- **WHEN** `deny.licenses` lists `BSD-3-Clause` with a reason
- **THEN** the rendered `deny.toml` allows `BSD-3-Clause` and carries the
  reason as a comment

#### Scenario: A missing reason

- **WHEN** a `deny.bans` entry names a crate and no reason
- **THEN** loading `.ctl/rust.yaml` fails naming `reason` and the entry's
  line

#### Scenario: A quote in a reason

- **WHEN** a `clippy.disallowed_methods` reason contains `"`
- **THEN** loading fails on the field `clippy.disallowed_methods[0].reason`

### Requirement: A marked block is replaced and the rest of the file kept

For `Cargo.toml` (block `lints`) and `mise.dev.toml` (block `tools`),
`render` SHALL return the whole file with only the text between
`# ctl-core baseline: begin <name>` and `# ctl-core baseline: end <name>`
replaced. A file without both markers SHALL fail `render`, naming the file
and the missing marker.

#### Scenario: Repository content survives

- **WHEN** `Cargo.toml` has `[package]`, `[dependencies]`, and a stale lints
  block between the markers
- **THEN** the rendered `Cargo.toml` keeps `[package]` and `[dependencies]`
  byte for byte and holds the current lints

#### Scenario: A workspace

- **WHEN** `.ctl/rust.yaml` sets `layout: workspace`
- **THEN** the lints block uses `[workspace.lints.rust]`,
  `[workspace.lints.rustdoc]`, and `[workspace.lints.clippy]`

#### Scenario: Missing markers

- **WHEN** `mise.dev.toml` has no `# ctl-core baseline: begin tools`
- **THEN** `render` fails naming `mise.dev.toml` and that marker

### Requirement: A repository can own a file

A file listed under `own` with a `reason` SHALL NOT be rendered, and
`render` SHALL report it as owned with its reason.

#### Scenario: An owned rustfmt.toml

- **WHEN** `own` lists `rustfmt.toml` with a reason
- **THEN** `render` returns no `rustfmt.toml` and reports it as owned

### Requirement: ctl-core matches its own baseline

ctl-core's test suite SHALL render ctl-core's `.ctl/rust.yaml` and fail when
any rendered body differs from the file in the checkout.

#### Scenario: A template changes alone

- **WHEN** `templates/baseline/clippy.toml.jinja` changes and ctl-core's
  `clippy.toml` does not
- **THEN** `mise run verify` fails naming `clippy.toml`
