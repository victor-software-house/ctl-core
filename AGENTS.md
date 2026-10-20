# ctl-core

Shared Rust CLI chassis and presentation kernel for the `*ctl` CLIs
(`verctl`, `qctl`, `forkctl`) and `state-sync`. **Not a command.** Import as
`ctl_core`.

This repo's queue is [`tasks.yaml`](tasks.yaml) (`CTC-###`).

## What this crate owns

- `-h` / `--help` and `-V` / `--version` (never `disable_help_flag`)
- Shared flags: `-c`/`--color`, `-f`/`--format`, `-n`/`--dry-run`/`--preview`,
  `-q`/`--quiet`, plus `--foo` / `--no-foo` negation helpers
- Environment binding: `#[command(env_prefix = "VERCTL")]` on a CLI's root
  parser binds every long option, shared flags included, to
  `<PREFIX>_<LONG>` through the `EnvPrefix` trait in the prelude. A flag on
  the command line wins over its variable, the variable over a config file,
  and the file over the default. `App` runs `flags::env_under_flags` before
  the parse, which unbinds the variable of an option that the command line
  set, negated, or conflicts with, and a variable that is empty or reads as a
  boolean flag's default. A CLI test that parses its own command calls it
  too. A caller that
  holds settings in the environment, such as an action, sets them and runs
  the bare verb; it never rebuilds argv in a shell.
- `App`: pre-parse hooks, Usage, help, parsing, execution, streams, errors, and
  exit codes
- `Document` and the fluent semantic primitives used by help and command output
- Pretty / JSON / colorless `View` over one serializable domain model
- The only terminal theme, width detection, wrapping, and table configuration
- Wire envelope: `Envelope` / `ErrorBody` / `SCHEMA_VERSION`
- Mise Usage spec (`usage` feature): `--usage-spec[=BIN]`, `mount_directives`
  (`raw`, `raw_args`, and `mount_line`, the block every served CLI-proxy task
  carries, so piped output stays byte for byte what the CLI wrote), and
  `App::usage_spec` enrichment, so consumers run `mise run q status` with no
  `--`. Forkctl completion remains a composable `App::before_parse` hook.
- Clap-derived `Surface` (`surface-model` feature): binary and mounted names,
  recursive commands, aliases, visibility, arguments, flags, Usage KDL, and
  optional audience notes. `surface-serde` adds serialization without a template
  engine; `surface-templates` adds shared MiniJinja fragments; `surface` remains
  the compatibility aggregate enabling all three.
- Declared input (`validate`, `input`, `input-toml` features): shared garde
  rules, and one loader that parses YAML or TOML, validates once, and places
  every problem on its file and line.
- Served files (`served` feature): render a template that declares its own
  target through `{%- set -%}` exports, check the declarations once, and write
  the file with its mode.
- Fixture trees (`fixture` feature): a temporary directory per test suite and
  per scenario, and one call per fixture file, for consumers' integration
  tests.

Domain verbs and result types stay in each CLI. Domain handlers return data and
never print, inspect the terminal, choose a view, or construct engine tables.
Comfy Table, Anstyle, and Anstream are private ctl-core implementation details;
none of their types may cross the public semantic boundary.

Boolean pairs that are domain-specific (`--pr` / `--no-pr`) stay in the CLI.
Use clap `overrides_with` both ways and `warn_opposites` so the last flag wins
without silence.

## Cargo features

Features are tree-shaking. `document` carries no terminal engine. `render` adds
the private engine, `view` adds JSON, `help` adds Clap help, and `app` composes
the lifecycle. `surface-model` adds only Clap/Usage operator metadata;
`surface-serde` adds ctl-core's direct Serde edge without a template engine;
`surface-templates` adds MiniJinja; `surface` preserves the full legacy aggregate.
`surface-model` still inherits Serde from `usage-lib`, whose KDL model requires
it; that is not ctl-core serialization. Prefer the narrowest feature set a
consumer needs. Never add serialization or rendering behavior to a lower tier.

`features = ["usage"]` does not pull `help`. Do not add `help` only to make the
prelude compile.

The crate-root `//!` sections in `src/lib.rs` are the docs.rs landing page,
and `document_features` there is authoritative for the feature graph. The
README is the GitHub page and does not copy them. Crate-root re-exports stay
`#[doc(hidden)]`, so the page lists modules once; the prelude documents the
flat surface. Traits are the exception, with every type a required trait
method takes or returns: cargo-semver-checks 0.51.0 treats a trait as
public-API-sealed (`trait_newly_sealed`) when any `#[doc(hidden)]` path
reaches it or such a type, even when another path is public. So `Present`,
`EnvPrefix`, and `Document` (returned by `Present::present`) stay visible at
the root. Each module's summary lives in its own `//!`, never also on
`pub mod`. Every fallible public function documents `# Errors`, with no
`missing_errors_doc` expect.

## Machine output is never coloured

JSON and anything else meant for a parser stays plain whatever the
environment says. Agent shells set `CLICOLOR_FORCE=1`, `FORCE_COLOR=3`, and
`COLORTERM=truecolor`; pretty output may honour them, and the JSON path never
does. A parse error under a long `--format json` is a JSON error even when the
bad argument comes first. A consumer test runs the binary with
`CLICOLOR_FORCE=1` and asserts that JSON output holds no ESC byte.

## Library control

ctl-core supplies sane defaults, never hidden policy. Every automatic behavior
that can change rendered output has a public library override and an explicit
disable path. Environment-backed defaults expose their canonical name and let a
consumer replace the ordered name list with its own aliases or disable lookup.
An explicit render width is exact and bypasses automatic-width buffering.

The automatic terminal-width buffer defaults to two columns and reads
`CTL_CORE_COLUMN_BUFFER`; the automatic minimum defaults to 20. When no width is
detected, output lays out to a fallback width of 80, and `fallback_width(None)`
disables it. `RenderOptions`, `View`, and `App` expose the buffer, ordered
environment names, minimum, and fallback.
Buffer zero disables subtraction; an empty environment slice disables lookup.
Explicit widths remain exact. Public option-aware help entry points keep direct
help callers under the same policy.

The record style, list style, row separation, and JSON layout read
`CTL_CORE_RECORD_STYLE`, `CTL_CORE_LIST_STYLE`, `CTL_CORE_ROW_SEPARATION`, and
`CTL_CORE_JSON_LAYOUT` when the consumer has not set them. Each name list is
replaceable, and an empty slice disables lookup.

## View contract

1. **Models first.** Each command returns a serializable result; the view does
   not own domain data.
2. **One model, every mode.** `Present` maps that model to a semantic `Document`.
   JSON serializes the model directly. Colorless renders the same document.
3. **Semantic composition only.** Consumers use `Fields`, `Table`, `Section`,
   `Notice`, `Text`, and `Document::verbatim`; they never choose borders, ANSI,
   width, or streams. Verbatim is for preformatted Markdown or protocol text
   whose line boundaries must survive rendering. It removes terminal controls,
   bidi controls, and Unicode line separators; preserves other Unicode format
   characters; then normalizes trailing newlines to the document.
4. **Quiet is human-success only.** It never hides JSON or errors.
5. **Help is a document.** Clap remains the grammar, and help uses the same
   renderer as command output. Usage lines wrap without splitting tokens.

See [`docs/presentation.md`](docs/presentation.md). `kv` / `grid` and the
string-render traits are migration surfaces, not the destination.

## Strings

Multiline Rust is `indoc!` / `formatdoc!` / `writedoc!` / `printdoc!` /
`eprintdoc!` / `concatdoc!` (re-exported from this crate). **No `concat!`.**
**No escaped `\n` in a document.** Leave a raw `\n` only when that *is* the
test.

## Comments

Doc comments carry the why, on the item. Inline `//` prose inside a function
body is litter.

## Operator documents

Consumer skills and installed instructions are committed rendered artifacts.
Their source templates import `ctl/version.md.jinja`,
`ctl/invocation.md.jinja`, or `ctl/commands.md.jinja` and retain domain prose.
Tests build `Surface` from the consumer's Clap type and byte-compare the render
with the committed file. Do not restore token scans, copied command lists, or a
second no-`--` rule.

## Consumers

Migrate one consumer at a time by pinning a released ctl-core version, never a
path dependency. Verctl goes first, then forkctl retains its protocol while
deleting its local view/help/layout, then qctl turns direct printing into typed
results. The migration is tracked as `CTC-010` here and `QCTL-008` in qctl.

## Declared input

`validate` holds the garde rules every schema shares; `input` reads a file,
parses it with serde-saphyr (strict booleans, duplicate and merge keys
refused), validates it once with garde, and reports every problem with its
file and line through yamled's location index. A message carries no
position of its own, except that an alias error names where its value is
defined, counted from the file. `Input::frontmatter` splits YAML frontmatter
from a Markdown body. Limits are serde-saphyr's `Budget`, overridable field by
field: the call (`Input::limits`), then the repository (`limits:` in
`.ctl/input.yaml`, installed by `use_repo_limits(root)` at startup), then the
process (`set_default_limits` or `App::input_limits`), then serde-saphyr's
default. `enforce_alias_anchor_ratio` is not overridable and stays on.
`use_repo_limits` returns each limit the repository raises above the default;
the CLI prints each as a warning. A CLI parses config only through `input`; it keeps no `schema.rs` and
no YAML crate of its own. `CTC-008` tracks the consumer moves and the template
renderer.

`input-toml` adds TOML on top of `input`, with the `toml` crate's span-carrying
`DeTable` as parser and location index, so the same `Input` loads either
format. The format is chosen, never guessed: `Input::open(path)` reads it from
the extension (`.toml`, `.yaml`, `.yml`, any case) and refuses every other
extension before reading; `Input::new` and `Input::read` stay YAML whatever the
name says, and `Input::format(Format)` sets it for text in memory. A TOML text
reads three `Limits` fields, `max_reader_input_bytes`, `max_depth`, and
`max_nodes`; every other field names YAML syntax and has no effect on TOML. The
`toml` parser also bounds nesting at 80 levels, which no limit raises. The
repository limits file stays `.ctl/input.yaml` for a TOML-config repository.
`openspec/specs/declared-input` holds the contract and `CTC-021` the row.

## Served files

`served::render(template, source, suffix, context)` renders one template through
`surface::environment()` and reads `path`, `name`, and `executable` from its
top-level exports; `Served::write(root)` writes it, sets 0755 or 0644 on Unix,
and refuses a symlinked target. Which templates a repository serves and what
context they see stay with the CLI. verctl moves onto it inside its redesign.

## Copier template

The repository setup comes from the shared Copier template, the private
`victor-software-house/templates` repository (project `rust-crate`): the whole
files `.miserc.toml`, `.envrc`, `mise.dev.toml`, `mise.ci.toml`,
`lefthook.yml`, `committed.toml`, `rust-toolchain.toml`, `rustfmt.toml`,
`clippy.toml`, `deny.toml`, `.rumdl.toml`, `.taplo.toml`, `.typos.toml`,
`.github/workflows/ci.yml`, `.github/workflows/template.yml`,
`.github/actionlint.yaml`,
the path-named file tasks in `mise-tasks/` (`check/repo.sh`,
`commit-msg.sh`, `commits.sh`, `specs.sh`, `format/`, `lint/`, and
`hooks/secrets/`), plus the settings, toolchain, and `[task_config]` blocks
of `mise.toml` and the `[lints]` tables of `Cargo.toml`. The starters
`mise-tasks/check/local.sh` and `openspec/config.yaml` are this repository's
own, and so is `mise-tasks/check/features.sh`, which holds this crate's
feature sets.
Local edits survive an update: `.rumdl.toml` allows the README banner's
HTML, and `.typos.toml` ignores commit hashes cited in prose. Change a shared rule in the template, then take
its new tag here.
`.copier-answers.yml` records its tag and this repository's answers; never
edit it by hand. Take a new tag with
`mise x uv@0.13.1 python@3.14.8 -- uvx copier@<pin> update --defaults --vcs-ref <tag>`,
where `<pin>` is the `"pypi:copier"` version in the template's `mise.toml` at
that tag. Review the diff, and fix any conflict markers (`git diff --check`)
before committing.

`ci.yml` calls the shared `rust-ci.yml`, and `template.yml` calls the shared
`template-check.yml`, both from the public `victor-software-house/actions`
repository at the commit the template pins. Their checks report as
`verify / Verify` and `template / Template`. The drift check runs
`template:check`, which fails only a pull request that changes
`.copier-answers.yml` without reaching the newest tag; any other pull request
gets a notice. A push to `main` and a daily run keep one issue, "Template
update available", open while the newest tag would change this repository.
The drift check is skipped for pull requests from forks.

Local work loads the `dev` profile (`.miserc.toml`): lefthook, committed,
and gitleaks. Every hook command runs with `MISE_ENV=dev,hooks`. Its
`postinstall` hook runs `lefthook install` after every `mise install`, so a
fresh clone gets its git hooks from `mise install`. CI sets `MISE_ENV=ci`
(`ci,release` for release jobs) and never loads `dev`, so no runner installs
hooks.

## Fixtures

Enable `fixture` on the ctl-core dev-dependency. `Suite::new(name)` owns a
temporary directory for a test file, and `suite.scenario(name)` owns one inside
it; both are removed on drop. `root.file(path, body)` writes a fixture and its
parent directories and returns the path; a path outside the root panics. A CLI
keeps no fixture wrapper of its own. `CTC-006` tracks the consumer moves.

## Release

Human writes `.changeset/*.md` on the same PR that ships the behaviour. Never
hand-edit versions or CHANGELOG. Declarations live in [`.ctl/ver.yaml`](.ctl/ver.yaml).
`prepare` runs `cargo update --workspace` and stages `Cargo.lock` because CI
packages with `--locked`. `.github/workflows/release.yml` calls verctl's
reusable release workflow on every push to `main`: it publishes each version
commit not yet fully released, then opens or updates the Version PR on
`verctl/release/main` with auto-merge. Bump its pin together with the verctl
pin in `mise.release.toml` and the `ver` task include in `mise.toml`.

## Checks

```sh
mise run verify
```

`verify` is the toolchain pins, format, clippy, the feature sets, `test` (unit
and integration; targets with `test = false` are skipped), `test:doc`
(doc-tests), `doc` (rustdoc as docs.rs builds it), cargo-deny
licenses/bans/sources, cargo-machete, and `check:versions`, verctl's
hand-edit guard. Those tasks come from the actions repository's
`tasks/rust.toml`, which `mise.toml` includes by `git::` URL at the commit
the template pins, and `mise.toml` composes `verify` from them. This
repository's own tasks live in `mise-tasks/`:
`check:features` is the file task `mise-tasks/check/features.sh`.
`check:repo`, `commit-msg`, `commits`, and the secrets scans are the
template's files in `mise-tasks/`. `check:repo` runs taplo, rumdl, typos,
actionlint, and OpenSpec validation, then `check:local`, which
`mise-tasks/check/local.sh` leaves to this repository; ctl-core adds none.
`[task_config]` lists `mise-tasks` first, because an includes list replaces the
default task dirs. Fragment names are not namespaced by file, so each name
carries its prefix. `verify` depends on `check:features` and `check:repo` by name, so every
repository on the template defines `check:features` and takes `check:repo`
from the template. Change a shared task in the actions repository, never in
a consumer.
`check:versions` and `semver` run verctl through its served `ver` task, so
every including repository that declares releases in `.ctl/ver.yaml` (or
`VERCTL_CONFIG`) includes that task at a pinned verctl release in the
environment `verify` runs in, locks verctl there, and gives the CI Verify
step `GITHUB_TOKEN` with `pull-requests: read` (template v0.0.27 renders
both). The check reads the pull request only when a manifest version differs.
A repository without that file declares no releases, so `check:versions`
passes there with nothing to compare.
`lint`, `test`, and `test:doc` pass `--workspace`, so every member is checked,
not only the root package or the `default-members` list.
Every cargo command that resolves dependencies passes `--locked`, in the
shared tasks and in each repository's `check:features`, so a stale `Cargo.lock`
fails `verify` the way it fails a release build. cargo-machete has no
`--locked`, so `deps` runs a locked `cargo metadata` first: no task in the
parallel `verify` ever rewrites the lock before a sibling reads it.
Tests run under plain `cargo test`. Measured on Linux and macOS with doc-tests
included, cargo-nextest was 1.05 to 14 times slower (it starts one process per
test), and mbx test scheduling or a `RUST_TEST_THREADS` cap slowed concurrent
runs. Do not reintroduce them without new measurements.
`mise.toml` pins stable 1.99.0 for build, lint, and test, with
`mr_boxington = true`, which routes Cargo through mr-boxington so worktrees
share one build store. A dated nightly carries only rustfmt: `format` runs on
it for `rustfmt.toml`'s unstable options, and `doc` builds on it as docs.rs
does. The
mise pin, `rust-toolchain.toml`, and `rust-version` move together in one
commit; `check:toolchain` fails when they differ. CI uses the
same pins and caches through `jdx/mr-boxington-action`.
Do not `&&` those in a new task; `depends` is the mise form. Those cargo
invocations share `target/`; the package-cache lock serializes them. Do not
invent extra `CARGO_TARGET_DIR` trees to hide that. Advisories (and yanked
crates) are `mise run deny:advisories` on CI only, so a pre-push `verify`
does not need the network. CI also runs `mise run semver`
(`verctl semver`: cargo-semver-checks against the last release, at the
release type the pending fragments allow) before packaging. An unfixable RUSTSEC or yank is listed in
`deny.toml` `[advisories].ignore` with a reason, not by weakening `yanked`
or `unmaintained`.

## Git

Conventional commits. lefthook. No `--no-verify`. Branch `type/number-desc`.
Always open a PR — never push to `main`.

## Changes

Plan a behaviour or contract change as an OpenSpec change in
`openspec/changes/<name>/` before writing code. `openspec/config.yaml` holds
this repository's context and rules, and `openspec validate <name>` checks the
change. One change maps to one `tasks.yaml` row.
