# Design: served Rust baseline

## Decisions

### 1. Templates are embedded in the crate

Each template is a `.jinja` file under `templates/baseline/`, compiled in
with `include_str!`. The baseline version is then the ctl-core version a
runner links. Fetching templates from a URL or a git ref at run time was
considered. It lost because it needs the network, and it adds a second
version that can disagree with the crate's.

### 2. Whole files, plus marked blocks for shared files

Five files are wholly the baseline's. `Cargo.toml` and `mise.dev.toml` hold
repository content (dependencies, features, tasks), so only a named region is
served:

```toml
# ctl-core baseline: begin lints
[lints.rust]
...
# ctl-core baseline: end lints
```

`render` reads the file, replaces what lies between the markers, and returns
the whole file. A runner compares or writes whole files only. A TOML editor
dependency was considered. It lost because the regions are contiguous
tables, so text splicing is enough, and the repository keeps its own
formatting. The lints region never contains `[package]` or
`[workspace.package]`, which verctl's version step writes.

### 3. One schema per repository, owned by ctl-core

`.ctl/rust.yaml` is read through `input`, so errors carry a file and a line.
The fields are:

1. `layout`: `crate` or `workspace`. A workspace gets `[workspace.lints.*]`.
2. `published`: whether the crate goes to crates.io, which adds
   cargo-semver-checks to the toolchain block.
3. `tokio`: adds the tokio guard types and the unbounded-channel ban to
   `clippy.toml`.
4. Append-only lists, each entry with a `reason`: clippy
   `disallowed_methods` and `disallowed_types`; cargo-deny `licenses`,
   `bans`, `skip`, and advisory `ignore`.
5. `own`: files the repository keeps for itself, each with a `reason`.

Putting the fields in `.ctl/ver.yaml` was considered. It lost because the
schema belongs to the baseline, not to versioning, and nothing under `.ctl/`
belongs to a CLI only because of where it sits.

### 4. Overrides

A repository changes the baseline in two ways only: by adding entries to an
append-only list, or by owning a whole file. An owned file is not rendered,
and the runner reports it as owned, so the exception stays visible. A
per-key override was considered. It lost because it makes every rendered
file a merge of two sources, which is the drift this change removes.

### 5. ctl-core checks itself

`tests/baseline.rs` renders ctl-core's own `.ctl/rust.yaml` and asserts that
each rendered body equals the file in the checkout. A template change
without the matching file change, or the reverse, fails `verify`. This needs
no runner, so ctl-core does not depend on verctl.

### 6. The runner is verctl

verctl already serves files in every ctl repository and already reports
drift. The baseline's files join that one mechanism. A binary in ctl-core
was considered. It lost because ctl-core is a library and because it would
add a second served-file mechanism.

### 7. Lockfiles stay with their owner

When the toolchain region changes, `mise.dev.lock` must be regenerated in
full through mise. ctl-core does not run mise. The runner does.

## Automatic behaviour, override, and disable

1. Rendering a file: override by adding to an append-only list. Disable by
   listing the file under `own`.
2. Splicing a block: a file without its markers fails `render` and names the
   file and the marker. Disable by owning the file.
3. The self-check in ctl-core: there is no disable. Change the template and
   the file together.
