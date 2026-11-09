---
ctl-core: patch
---

Each declared-input format is now its own feature, so a TOML-only consumer links no YAML crate and no garde.
With default features off and only `input-toml` on, ctl-core links none of serde-saphyr, yamled, or garde.
`input` keeps its meaning, YAML with garde validation, so a manifest that enables it builds unchanged.

These break callers:

- `input-toml` no longer enables `input`.
  A consumer that read YAML or validated through `input-toml` alone adds `input`, or `input-yaml` and `validate`.
- `Input::check`, `Input::load`, and the `ctl_core::validate` module need the `validate` feature.
- With `input-toml` alone, `Format::Toml` is the only variant, so its discriminant is 0, where it was 1.
- `Format::Yaml`, `Input::new`, `Input::read`, `Input::frontmatter`, `REPO_FILE`, `use_repo_limits`, and `Loosened` need the new `input-yaml` feature.
- `Limits` is ctl-core's own type.
  Its 17 YAML-only fields need `input-yaml`, and a caller builds it with `..Limits::NONE`.
  `Limits::budget` is no longer public.

These are new:

- `input-yaml`: YAML input with no garde.
- `Input::new_with_format(name, text, format)`: text in memory in a named format.
- `Limits::DEFAULT_MAX_READER_INPUT_BYTES`, `DEFAULT_MAX_DEPTH`, and `DEFAULT_MAX_NODES`: the defaults every format shares, equal to serde-saphyr's.

A TOML text is held to its byte, depth, and node limits under the call and process tiers whatever the build enables.
Without `input-yaml`, a limit message ends "the tool sets this limit", and `Input::open` on a `.yaml` or `.yml` file names the feature it needs.
