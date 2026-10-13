# Changelog

## ctl-core 0.6.17

- New `mount_directives(task)`: the `#MISE raw=true`, `#MISE raw_args=true`, and `#USAGE mount` block a served CLI-proxy task carries. Without `raw`, mise relays the task's output and appends an SGR reset to every line whenever its own colour is on, as under `CLICOLOR_FORCE=1`, so `--format json` piped to `jq` carried an ESC byte the CLI never wrote ([#114](https://github.com/victor-software-house/ctl-core/pull/114)).

## ctl-core 0.6.16

- A parse error under a long `--format json` or `--format=json` is now a JSON error even when the bad argument comes before the flag. Clap's lenient read stopped at the bad argument, so the error printed as text, coloured when `CLICOLOR_FORCE` or `FORCE_COLOR` was set, to a caller that asked for JSON ([#112](https://github.com/victor-software-house/ctl-core/pull/112)).

## ctl-core 0.6.15

- Remove the public `baseline` module and the `baseline` feature. The shared Copier template now renders the shared Rust configuration, so ctl-core keeps no second generator. No consumer enables the feature: verctl, qctl, and forkctl pin earlier releases without it. A crate that enabled `ctl-core/baseline` takes the files from the template instead. The removal is a breaking API change released as a patch by operator decision, and CI's semver check allows it only against the 0.6.14 baseline.

## ctl-core 0.6.14

- The shared `verify` in `tasks/rust.toml` now also depends on `check:repo`, which each including repository defines for its own checks, in place of overriding `verify`. A repository that moves its include to this release must define `check:repo` (a task that runs `true` when it has no checks of its own), or `verify` fails with `task not found: check:repo`.

## ctl-core 0.6.13

- ctl-core now ships the standard Rust mise tasks (`check:toolchain`, `format`, `lint`, `test`, `test:doc`, `deny`, `deny:advisories`, `deps`, `semver`, `verify`) in `tasks/rust.toml`. Another repository includes it with `git::https://github.com/victor-software-house/ctl-core.git//tasks/rust.toml?ref=<tag>` in `task_config.includes`, and defines its own `check:features`.

## ctl-core 0.6.12

- `rust-version` is now 1.99.0: ctl-core builds, lints, and tests on that pinned stable and formats on a dated nightly only. `App`, `fixture::Suite`, and `fixture::Root` implement `Debug`.
- New feature `baseline`: `baseline::render(root, &config)` returns the Rust configuration every ctl repository shares (`rust-toolchain.toml`, `rustfmt.toml`, `clippy.toml`, `deny.toml`, `lefthook.yml`, and the marked `lints` and `tools` blocks in `Cargo.toml` and `mise.dev.toml`) as `served::Served` values, from a repository's `.ctl/rust.yaml` (`baseline::load`).

## ctl-core 0.6.11

- `input::Limits` overrides any of serde-saphyr's `Budget` fields except `enforce_alias_anchor_ratio`, which stays on, decided field by field: the call (`Input::limits`), then the repository (`limits:` in `.ctl/input.yaml`, installed by `input::use_repo_limits`), then the process (`input::set_default_limits` or `App::input_limits`), then serde-saphyr's default. `max_reader_input_bytes` now also caps text already in memory, a limit error names the limit, its value, and where to raise it, and `use_repo_limits` returns each limit the repository raises above serde-saphyr's default for the CLI to print as a warning.

## ctl-core 0.6.10

- New `fixture` feature for integration tests: `Suite::new(name)` and `suite.scenario(name)` each own a temporary directory that is removed when it drops, and `root.file(path, body)` writes a fixture with its parent directories in one call. A path outside the root panics instead of writing.
- New `served` feature: `served::render(template, source, suffix, context)` renders a template that declares its own target through top-level `{%- set path / name / executable -%}` exports, checks those declarations once, and returns a `Served` file; `Served::write(root)` writes it with its directories and the 0755 or 0644 mode, refusing a symlinked target. Which templates a repository has and what context they see stay with the CLI.

## ctl-core 0.6.9

- An alias error from `Input::parse` names where its value is defined, counted from the file, instead of repeating two positions counted from the frontmatter.
- Move the `input` feature to yamled 0.0.5, so a CLI that edits through yamled 0.0.5 resolves one yamled.

## ctl-core 0.6.8

- Move the `input` feature to yamled 0.0.4, so a CLI that edits through yamled 0.0.4 resolves one yamled.

## ctl-core 0.6.7

- Move the `input` feature to yamled 0.0.3, so a CLI that edits through yamled 0.0.3 resolves one yamled.

## ctl-core 0.6.6

- A parse problem's message no longer repeats serde-saphyr's own position, which counted from the frontmatter instead of the file.

## ctl-core 0.6.5

- Add the declared-input layer. The `validate` feature carries the garde rules every schema shares; the `input` feature reads a file, parses it with serde-saphyr (strict booleans, duplicate and merge keys refused), validates it once with garde, and reports every problem with its file and line, placed through yamled. `Input::frontmatter` splits YAML frontmatter from a Markdown body and keeps the file's line numbers.

## ctl-core 0.6.4

- Remove the `go`, `run`, `main`, `main_with`, and `main_with_help` wrappers. `App` is the only entry point. The `cli` feature no longer pulls in `anyhow`; `app` does.
- Let the environment pick each style an owner left open: `CTL_CORE_RECORD_STYLE`, `CTL_CORE_LIST_STYLE`, `CTL_CORE_ROW_SEPARATION`, and `CTL_CORE_JSON_LAYOUT`. An unaccepted value is ignored, and `App` names the accepted values on stderr.

## ctl-core 0.6.3

- JSON error envelopes follow the view JSON layout, so errors are indented like successes by default and compact when a consumer sets JsonLayout::Compact.
- Keep a word wider than its column, such as a URL or path, whole in records, paragraphs, and headings; the line runs past the width instead of splitting the word.

## ctl-core 0.6.2

- Wrap headings and section titles to the render width, so a long title stays inside the fallback width in piped output.

## ctl-core 0.6.1

- Make the operator's picks the defaults: borderless records with right-aligned keys, two-space JSON, a two-column automatic-width buffer, and an 80-column fallback width when none is detected (`fallback_width(None)` disables it). `App` now reaches the style options, JSON layout, and fallback width.

## ctl-core 0.6.0

- Add an identifier role (`Role::Id`, `Text::id`, `Table::id_column`) that renders bold in the terminal foreground colour, and mark `Role` as non-exhaustive.
- Add record style, list style, row separation, and JSON layout options to RenderOptions and View, with today's look as the default, and a gallery example that renders every candidate.

## ctl-core 0.5.2

- Expose Surface metadata, serialization, and MiniJinja rendering through separate additive feature tiers while preserving the existing full compatibility aggregate.

## ctl-core 0.5.1

- Consumers can require every long option to have an operator-visible short form, so a new long-only flag fails their surface test before release. An exact-token allow list preserves letters that shared format and color flags deliberately leave to each CLI.
- Reserve one column from automatically detected terminal widths so nested shells keep the right edge visible, with public overrides for the buffer and its environment aliases.

## ctl-core 0.5.0

- Extract a serializable operator Surface from Clap and provide shared MiniJinja fragments for skill versions, mounted invocation, and visible command inventories.

## ctl-core 0.4.2

- Add `FormatLong`, a reusable `--format` plus `-q`/`--quiet` mixin for CLIs that already own `-f`.

## ctl-core 0.4.1

- Wrap long help Usage lines within the detected width and prevent flattened output flags from replacing the root long description.

## ctl-core 0.4.0

- Allow `App` consumers to enrich mounted Usage documents without taking back stream or lifecycle ownership.

## ctl-core 0.3.0

- Add semantic verbatim document blocks for preformatted Markdown and protocol text that must bypass terminal-width wrapping.

## ctl-core 0.2.0

- Add the typed `App` chassis and fluent semantic `Document` presentation API. Pretty, colorless, JSON, help, errors, quiet behavior, and terminal layout now share one ctl-core-owned path; terminal engine types stay private.

## ctl-core 0.1.1

- `features = ["usage"]` compiles without `help`. Prelude re-exports `go` only when help is on.

## ctl-core 0.1.0

- Verify runs nextest, cargo-deny (licenses/bans/sources locally; advisories on CI), and cargo-machete.
- `usage` feature: one `--usage-spec` helper and the mise mount line so qctl, verctl, and forkctl share the same `mise run q status` form (no `--`).

## ctl-core 0.0.5

- Pin Version PR and publish at verctl 0.2.1. Publish no longer needs a full checkout; the action fetches the default branch at depth 1.
- Pin Version PR and publish at verctl 0.2.3. The action fetches the default-branch history, not a depth-1 tip, and rewrites `origin/HEAD` only from `VERCTL_DEFAULT_BRANCH`.

## ctl-core 0.0.4

- Pin Version PR and publish at verctl 0.2.0 so the first crates.io cut from this lane tags the published commit.

## ctl-core 0.0.3

- `kv` / `grid` take `ColorMode`. `--color never` is colorless; no caller strips ANSI.

## ctl-core 0.0.2

- Add `kv` / `grid` pretty tables with styled tokens for command output.
