# Proposal: served files

Queue row: CTC-008 (template renderer).

## Why

verctl renders the files a repository serves from templates that declare
their own target. The renderer sits in verctl's `src/templates.rs` (833 lines,
555 of them tests), so no other ctl CLI can serve a file the same way. CTC-008
asks for that renderer to be available to any ctl CLI.

## What Changes

1. A new `served` feature exposes `ctl_core::served`.
2. `render(template, source, suffix, context)` renders one template through
   `surface::environment()` and reads its top-level `{%- set -%}` exports as
   one schema: `path` (a directory inside the repository, the root when
   unsaid), `name` (one file name, the template's own name minus `suffix` when
   unsaid), and `executable` (0755 or 0644). It returns a checked `Served`.
3. `Served::write(root)` writes the file and its directories, sets its mode
   on Unix, and refuses a symlinked target.
4. Discovering which templates a repository has, and building their context,
   stay with the CLI.

## Capabilities

### New Capabilities

- `served-files`: rendering a self-declaring template and writing its file.

### Modified Capabilities

None.

## Impact

1. A patch release of ctl-core, as for `input`: a new feature, nothing
   existing changes. It reuses `minijinja`, `garde`, and `serde`, already
   optional dependencies, adding their `deserialization` and `derive`
   features under `served`.
2. verctl moves its renderer onto this inside its redesign, keeping its
   `templates::write`, `plan`, and `any` interface and its served bytes.
