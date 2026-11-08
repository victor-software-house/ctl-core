# ctl-core

The shared clap chassis and presentation kernel for the `*ctl` CLIs.
**Not a command.** Import as `ctl_core`.
This guide covers this crate only; the repository's root [`AGENTS.md`](../AGENTS.md) holds the workspace rules, and they apply here too.

## What it owns

- `-h` / `--help` and `-V` / `--version` (never `disable_help_flag`)
- Shared flags: `-c`/`--color`, `-f`/`--format`, `-n`/`--dry-run`/`--preview`, `-q`/`--quiet`, plus `--foo` / `--no-foo` negation helpers
- Environment binding: `#[command(env_prefix = "VERCTL")]` on a CLI's root parser binds every long option, shared flags included, to `<PREFIX>_<LONG>` through the `EnvPrefix` trait in the prelude.
  - A flag on the command line wins over its variable, the variable over a config file, and the file over the default.
  - `App` runs `flags::env_under_flags` before the parse.
    It unbinds the variable of an option that the command line set, negated, or conflicts with.
    It also unbinds a variable that is empty or reads as a boolean flag's default.
  - A CLI test that parses its own command calls it too.
  - A caller that holds settings in the environment, such as an action, sets them and runs the bare verb.
    It never rebuilds argv in a shell.
- `App`: pre-parse hooks, Usage, help, parsing, execution, streams, errors, and exit codes.
  When the reader closes stdout early, it keeps the command's own exit code and never reports success for a failure (`App::broken_pipe`).
- `Document` and the fluent semantic primitives used by help and command output
- Pretty / JSON / colorless `View` over one serializable domain model
- The only terminal theme, width detection, wrapping, and table configuration
- Wire envelope: `Envelope` / `ErrorBody` / `SCHEMA_VERSION`
- Mise Usage spec (`usage` feature): `--usage-spec[=BIN]`, `mount_directives`, and `App::usage_spec` enrichment, so consumers run `mise run q status` with no `--`.
  - `mount_directives` are `raw`, `raw_args`, and `mount_line`, the block every served CLI-proxy task carries, so piped output stays byte for byte what the CLI wrote.
  - Completion is its own `completion` feature, below.
- Clap-derived `Surface` (`surface-model` feature): binary and mounted names, recursive commands, aliases, visibility, arguments, flags, Usage KDL, and optional audience notes.
  `surface-serde` adds serialization without a template engine; `surface-templates` adds shared MiniJinja fragments; `surface` remains the compatibility aggregate enabling all three.
- Declared input (`validate`, `input`, `input-toml` features): shared garde rules, and one loader that parses YAML or TOML, validates once, and places every problem on its file and line.
- Served files (`served` feature): render a template that declares its own target through `{%- set -%}` exports, check the declarations once, and write the file with its mode.
- Shell completion (`completion` feature): the `completion <SHELL>` verb as a flattenable `CompletionCommand`, and the registration script for bash, zsh, and fish.
  - `App::completion` answers `COMPLETE=<shell>` before parsing; the feature enables `app` for it.
  - `Shell` lives in its own `shell` feature, which later shell verbs share.
  - `clap_complete` is pinned exactly because its dynamic engine is unstable.
    A bump is a reviewed change that reruns the script tests.
  - The zsh script carries three lines so it completes on the first TAB from `fpath` ([research](../docs/research/shell-completion.md), section 6).
    Drop them when a pinned `clap_complete` writes its own.
  - `Script` and its `shell::view` live in `shell` behind `view`, shared by both shell verbs.
- Shell integration (`shell-init` feature): the `init <SHELL>` verb as a flattenable `InitCommand`, and the bash, zsh, and fish script a `Hook` writes.
  - The hook runs `<bin> <subcommand>` once, before the first prompt, when its variable is non-empty and the binary is on `PATH`, then removes itself.
  - ctl-core writes all shell syntax.
    `Hook::new` refuses any part that is not a plain word, and the script calls the binary by name, never by path ([research](../docs/research/shell-init.md)).
- Fixture trees (`fixture` feature): a temporary directory per test suite and per scenario, and one call per fixture file, for consumers' integration tests.

Domain verbs and result types stay in each CLI.
Domain handlers return data and never print, inspect the terminal, choose a view, or construct engine tables.
The engine is this workspace's `ctl-text`, `ctl-layout`, and `ctl-term`, re-exported as `ctl_core::{text, layout, term}`.
A consumer composes `Document` blocks; no engine widget type appears in `Document`'s API, and `Theme` is the one place a role becomes a style.

Boolean pairs that are domain-specific (`--pr` / `--no-pr`) stay in the CLI.
Use clap `overrides_with` both ways and `warn_opposites` so the last flag wins without silence.

## What it must not own

- Domain verbs, domain result types, and domain-specific boolean pairs.
  They stay in each CLI.
- Text measurement, wrapping, and the unsafe-character rule: `ctl-text`.
- Widget layout, such as how a table chooses column widths: `ctl-layout`.
- Terminal detection, colour depth, and escape bytes: `ctl-term`.
  ctl-core chooses a role's style through `Theme` and hands lines to `ctl-term`.
- A terminal crate the engine replaced.
  `check:tree` fails the `app` tier when comfy-table, crossterm, anstream, anstyle-parse, or console returns.

## Public API

- A public item is a promise to every consumer.
  Keep a helper `pub(crate)` until a consumer needs it.
- Every public item has a doc that adds something beyond its name, and every fallible public function documents `# Errors`.
- No engine widget type appears in `Document`'s API; a consumer reaches the engine crates through `ctl_core::{text, layout, term}`.
- Setters on `RenderOptions` and `View` are `with_<setting>`, and the getter of the same setting has the bare name.
  A method that reads the environment starts with `resolved_`, so a caller sees which methods are not pure.
  `App` is a builder that holds only setters, so it keeps the bare names.
- A break needs a `minor` changeset at the repository root while the workspace is below 1.0.
  CI runs `mise run semver` against the last release before packaging.
  The crate-docs rules under [Cargo features](#cargo-features) keep traits visible to that check.
- A change to what an operator sees moves each CLI's skill and instructions in the pull request that adopts the release.
  Examples are help text, a variable name, or an error's wording.

## Tests

| Path | Covers | Accept a change with |
|:--|:--|:--|
| `tests/snapshots/` | Rendered help, scripts, and documents, as insta file snapshots | `cargo insta review` |
| `tests/parity/` | The consumer corpus against the 0.6.x baselines | `PARITY_UPDATE=1 cargo test --locked -p ctl-core --test parity` |
| `tests/support/` | Shared helpers: shell checks and pseudo-terminals | (none) |
| `examples/size.rs` | The toy CLI that `check:size` measures | a budget change in the root `Cargo.toml` |

Read the diff of every changed snapshot or baseline before accepting it.
The sections [Snapshots](#snapshots) and [Parity corpus](#parity-corpus) hold the details.

## Decisions that bind it

These sections of [`design.md`](../openspec/changes/terminal-render-engine/design.md) state the reasons; change the code and the decision together.

- [12.
  Roles map to styles through a public theme](../openspec/changes/terminal-render-engine/design.md#12-roles-map-to-styles-through-a-public-theme)
- [13.
  A closed pipe ends the program quietly](../openspec/changes/terminal-render-engine/design.md#13-a-closed-pipe-ends-the-program-quietly)
- [15.
  Public API](../openspec/changes/terminal-render-engine/design.md#15-public-api)
- [16.
  Feature tiers and their measured sizes](../openspec/changes/terminal-render-engine/design.md#16-feature-tiers-and-their-measured-sizes)
- [17.
  Tests](../openspec/changes/terminal-render-engine/design.md#17-tests)
- [18.
  The rule against writing these from scratch is replaced](../openspec/changes/terminal-render-engine/design.md#18-the-rule-against-writing-these-from-scratch-is-replaced)

## Cargo features

Features are tree-shaking.
`document` carries no terminal engine.
`render` adds the engine and the `ctl-term` encoder, with no window query.
`view` adds JSON, `help` adds Clap help, and each turns on `ctl-term`'s `detect` feature.
`app` composes the lifecycle.

`surface-model` adds only Clap/Usage operator metadata.
`surface-serde` adds ctl-core's direct Serde edge without a template engine.
`surface-templates` adds MiniJinja, and `surface` preserves the full legacy aggregate.
`surface-model` still inherits Serde from `usage-lib`, whose KDL model requires it; that is not ctl-core serialization.
Prefer the narrowest feature set a consumer needs.
Never add serialization or rendering behavior to a lower tier.

`features = ["usage"]` does not pull `help`.
Do not add `help` only to make the prelude compile.

ctl-core asks a dependency only for the features its own code calls.
Cargo unifies features, so a consumer cannot turn off one ctl-core turned on.
`clap` is declared with `default-features = false`: `cli` asks for `std`, `derive`, `env`, and `string`, and `help` adds `usage`.
Clap's `color`, `help`, `error-context`, and `suggestions` are the consumer's choice.
ctl-core renders every help Clap stops for and never calls `Error::print` or `Command::color`.

Measure a new dependency or feature with `cargo bloat` on a release build before adding it (`openspec/changes/archive/2026-11-08-narrow-clap-features/design.md`).

The crate-root `//!` sections in `ctl-core/src/lib.rs` are the docs.rs landing page, and `document_features` there is authoritative for the feature graph.
The README is the GitHub page and does not copy them.
Crate-root re-exports stay `#[doc(hidden)]`, so the page lists modules once; the prelude documents the flat surface.

Traits are the exception, with every type a required trait method takes or returns.
cargo-semver-checks 0.51.0 treats a trait as public-API-sealed (`trait_newly_sealed`) when any `#[doc(hidden)]` path reaches it or such a type.
This holds even when another path is public.
So `Present`, `EnvPrefix`, and `Document` (returned by `Present::present`) stay visible at the root.

Each module's summary lives in its own `//!`, never also on `pub mod`.
Every fallible public function documents `# Errors`, with no `missing_errors_doc` expect.

## Machine output is never coloured

JSON and anything else meant for a parser stays plain whatever the environment says.
Agent shells set `CLICOLOR_FORCE=1`, `FORCE_COLOR=3`, and `COLORTERM=truecolor`; pretty output may honour them, and the JSON path never does.

A parse error under a long `--format json` is a JSON error even when the bad argument comes first.
A consumer test runs the binary with `CLICOLOR_FORCE=1` and asserts that JSON output holds no ESC byte.

## Library control

ctl-core supplies sane defaults, never hidden policy.
Every automatic behavior that can change rendered output has a public library override and an explicit disable path.
Environment-backed defaults expose their canonical name and let a consumer replace the ordered name list with its own aliases or disable lookup.
An explicit render width is exact and bypasses automatic-width buffering.

The automatic terminal-width buffer defaults to two columns and reads `CTL_CORE_COLUMN_BUFFER`; the automatic minimum defaults to 20.
When no width is detected, output lays out to a fallback width of 80, and `with_fallback_width(None)` disables it.
`RenderOptions`, `View`, and `App` expose the buffer, ordered environment names, minimum, and fallback; `View` returns its settings as a `RenderOptions` from `render_options`.
The buffer, its names, and the minimum exist only with `view` or `help`, since nothing reads them without a stream.
The defaults are `ctl-term`'s constants, not second copies.

Buffer zero disables subtraction; an empty environment slice disables lookup.
Explicit widths remain exact.
Public option-aware help entry points keep direct help callers under the same policy.

The record style, list frame, row separation, and JSON layout read `CTL_CORE_RECORD_STYLE`, `CTL_CORE_LIST_STYLE`, `CTL_CORE_ROW_SEPARATION`, and `CTL_CORE_JSON_LAYOUT` when the consumer has not set them.
Each name list is replaceable, and an empty slice disables lookup.

## View contract

1. **Models first.** Each command returns a serializable result; the view does not own domain data.
2. **One model, every mode.** `Present` maps that model to a semantic `Document`.
   JSON serializes the model directly.
   Colorless renders the same document.
3. **Semantic composition only.** Consumers use `Fields`, `Table`, `Section`, `Notice`, `Text`, and `Document::verbatim`; they never choose borders, ANSI, width, or streams.
   - Verbatim is for preformatted Markdown or protocol text whose line boundaries must survive rendering.
   - It removes what `unsafe_chars::is_unsafe` refuses (every Cc control other than newline and tab, the 12 UAX #9 directional formatting characters, and U+2028 and U+2029).
     It preserves the other format characters (Cf).
     Then it normalizes trailing newlines to the document.
   - A CLI that prints text it did not write filters with the same predicate.
   - The predicate and `strip_unsafe` live in `ctl-text` (`ctl_text::unsafe_chars`), and `ctl_core::unsafe_chars` re-exports them under the `unsafe-chars` feature.
4. **Quiet is human-success only.** It never hides JSON or errors.
5. **Help is a document.** Clap remains the grammar, and help uses the same renderer as command output.
   Usage lines wrap without splitting tokens.

See [`docs/presentation.md`](../docs/presentation.md).

## Operator documents

Consumer skills and installed instructions are committed rendered artifacts.
Their source templates import `ctl/version.md.jinja`, `ctl/invocation.md.jinja`, or `ctl/commands.md.jinja` and retain domain prose.
Tests build `Surface` from the consumer's Clap type and byte-compare the render with the committed file.
Do not restore token scans, copied command lists, or a second no-`--` rule.

## Consumers

Migrate one consumer at a time by pinning a released ctl-core version, never a path dependency.
Verctl goes first, then forkctl retains its protocol while deleting its local view/help/layout, then qctl turns direct printing into typed results.
The migration is tracked as `CTC-010` here and `QCTL-008` in qctl.

## Declared input

`validate` holds the garde rules every schema shares.
`input` reads a file, parses it with serde-saphyr (strict booleans, duplicate and merge keys refused), and validates it once with garde.
It reports every problem with its file and line through yamled's location index.

A message carries no position of its own, except that an alias error names where its value is defined, counted from the file.
`Input::frontmatter` splits YAML frontmatter from a Markdown body.

Limits are serde-saphyr's `Budget`, overridable field by field, in this order:

1. the call (`Input::limits`);
2. the repository (`limits:` in `.ctl/input.yaml`, installed by `use_repo_limits(root)` at startup);
3. the process (`set_default_limits` or `App::input_limits`);
4. serde-saphyr's default.

`enforce_alias_anchor_ratio` is not overridable and stays on.
`use_repo_limits` returns each limit the repository raises above the default; the CLI prints each as a warning.
A CLI parses config only through `input`; it keeps no `schema.rs` and no YAML crate of its own.
`CTC-008` tracks the consumer moves and the template renderer.

`input-toml` adds TOML on top of `input`, with the `toml` crate's span-carrying `DeTable` as parser and location index, so the same `Input` loads either format.
The format is chosen, never guessed.
`Input::open(path)` reads it from the extension (`.toml`, `.yaml`, `.yml`, any case) and refuses every other extension before reading.
`Input::new` and `Input::read` stay YAML whatever the name is, and `Input::format(Format)` sets it for text in memory.

A TOML text reads three `Limits` fields, `max_reader_input_bytes`, `max_depth`, and `max_nodes`.
Every other field names YAML syntax and has no effect on TOML.
The `toml` parser also bounds nesting at 80 levels, which no limit raises.
The repository limits file stays `.ctl/input.yaml` for a TOML-config repository.
`openspec/specs/declared-input` holds the contract and `CTC-021` the row.

## Served files

`served::render(template, source, suffix, context)` renders one template through `surface::environment()` and reads `path`, `name`, and `executable` from its top-level exports; `Served::write(root)` writes it, sets 0755 or 0644 on Unix, and refuses a symlinked target.
Which templates a repository serves and what context they see stay with the CLI. verctl moves onto it inside its redesign.

## Fixtures

Enable `fixture` on the ctl-core dev-dependency.
`Suite::new(name)` owns a temporary directory for a test file, and `suite.scenario(name)` owns one inside it; both are removed on drop.
`root.file(path, body)` writes a fixture and its parent directories and returns the path; a path outside the root panics.
A CLI keeps no fixture wrapper of its own.
`CTC-006` tracks the consumer moves.

## Snapshots

Rendered output a reviewer should read, such as a shell script or a help page, is an insta file snapshot under `ctl-core/tests/snapshots/`.
It is never a string literal or a `contains` check: a change to the output shows as a diff of the snapshot.

`insta` is pinned exactly; a bump is a reviewed change.
CI sets `INSTA_UPDATE=no` in `mise.ci.toml`, so it never writes a snapshot.
Review a change locally with `cargo insta review` (`cargo-insta` is in `mise.dev.toml`).
A test that writes a temporary file uses `fixture`, not its own temporary directory.

A rendered shell script also passes its shell's check through `ctl-core/tests/support/shells.rs`: `bash -n` and `shellcheck --enable=all`, `zsh -n`, `fish -n`.

- zsh, fish, and shellcheck are pinned in `mise.test.toml`, the `test` profile, and locked in `mise.test.lock`.
- `.miserc.toml` loads it locally after `dev`.
  CI loads it only in the Build and test job (`MISE_ENV=ci,test`, through the shared workflow's `test-profiles` input), so no other job fetches them.
- Their install directories are on the tests' `PATH`, which mr-boxington's command wrapper keeps; only mise's shims are taken off it.
- Generate the locks with the default profiles, `mise lock --platform linux-x64,macos-arm64`.
  mise also writes the pinned catalogs' task tools into `mise.test.lock`.
  The pre-push lock check runs with `MISE_ENV=dev,test`.
- Write a new lock with the pinned mise, 2026.10.25.
  A newer mise writes a new file as lockfile version 4.
  CI's mise cannot read that version.
- A missing tool fails the test.
- A script ctl-core writes passes shellcheck with no exclusions.
  Only a third-party template's lines may be excluded, by code, with the reason beside the list.

A script that changes what a shell does at run time also runs in that shell.
`ctl-core/tests/shell_init.rs` starts bash, zsh, and fish on a pseudo-terminal through `pty-process` (`ctl-core/tests/support/terminal.rs`), types each line only after the prompt appears, and reads what a fake binary logged.
A pipe is not enough: fish draws no prompt when it reads one.

## Parity corpus

`ctl-core/tests/parity/` renders a fixed corpus of the documents that verctl, qctl, forkctl, and rawr produce.

- `corpus.rs` builds each case with the public API, as a consumer does.
- `main.rs` renders every case through `Document::render` at widths 20, 40, 80, and 120, with `ColorMode::Never` and `ColorMode::Always`, and compares the output byte for byte with `ctl-core/tests/parity/baseline/<case>-w<width>-<plain|color>.txt`.
- A baseline writes each ESC byte as the six characters `\u{1b}`.
- A missing or different baseline fails the test with a line diff.
- `PARITY_UPDATE=1 cargo test --locked -p ctl-core --test parity` writes the missing and changed files.
- The baselines are plain files, not insta snapshots, because the 0.6.x files stay frozen once committed.
  The render-engine change adds the new engine's outputs beside them and lists each difference in its pull request.

Add a case to `CASES` with its baselines in the same commit.
