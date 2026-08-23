# ctl-core

Shared Rust CLI chassis and presentation kernel for the `*ctl` CLIs
(`verctl`, `qctl`, `forkctl`) and `state-sync`. **Not a command.** Import as
`ctl_core`.

This repo's queue is [`tasks.yaml`](tasks.yaml) (`CTC-###`).

## What this crate owns

- `-h` / `--help` and `-V` / `--version` (never `disable_help_flag`)
- Shared flags: `-c`/`--color`, `-f`/`--format`, `-n`/`--dry-run`/`--preview`,
  `-q`/`--quiet`, plus `--foo` / `--no-foo` negation helpers
- `App`: pre-parse hooks, Usage, help, warnings, parsing, execution, streams,
  errors, and exit codes
- `Document` and the fluent semantic primitives used by help and command output
- Pretty / JSON / colorless `View` over one serializable domain model
- The only terminal theme, width detection, wrapping, and table configuration
- Wire envelope: `Envelope` / `ErrorBody` / `SCHEMA_VERSION`
- Mise Usage spec (`usage` feature): `--usage-spec[=BIN]`, `mount_line`, so
  consumers run `mise run q status` with no `--`. Forkctl completion remains a
  composable `App::before_parse` hook.

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
the lifecycle. Prefer explicit feature sets when a consumer needs less than the
complete chassis.

`features = ["usage"]` does not pull `help`. Do not add `help` only to make the
prelude compile.

Crate docs (`src/lib.rs` + `document_features`) are authoritative for the
feature graph. The GitHub README is not rustdoc.

## View contract

1. **Models first.** Each command returns a serializable result; the view does
   not own domain data.
2. **One model, every mode.** `Present` maps that model to a semantic `Document`.
   JSON serializes the model directly. Colorless renders the same document.
3. **Semantic composition only.** Consumers use `Fields`, `Table`, `Section`,
   `Notice`, and `Text`; they never choose borders, ANSI, width, or streams.
4. **Quiet is human-success only.** It never hides JSON or errors.
5. **Help is a document.** Clap remains the grammar, and help uses the same
   renderer as command output.

See [`docs/presentation.md`](docs/presentation.md). `kv` / `grid`, the `go`
wrapper, and string-render traits are migration surfaces, not the destination.

## Strings

Multiline Rust is `indoc!` / `formatdoc!` / `writedoc!` / `printdoc!` /
`eprintdoc!` / `concatdoc!` (re-exported from this crate). **No `concat!`.**
**No escaped `\n` in a document.** Leave a raw `\n` only when that *is* the
test.

## Comments

Doc comments carry the why, on the item. Inline `//` prose inside a function
body is litter.

## Consumers

Migrate one consumer at a time by pinning a released ctl-core version, never a
path dependency. Verctl goes first, then forkctl retains its protocol while
deleting its local view/help/layout, then qctl turns direct printing into typed
results. The migration is tracked as `CTC-010` here and `QCTL-008` in qctl.

## Declared input (queued)

`CTC-008` moves the shared "read, parse shape, validate once, complain in the
repo's own words" layer here so each CLI stops owning a drifting copy. Do that
row before any CLI-side "move my schema.rs" follow-up.

## Release

Human writes `.changeset/*.md` on the same PR that ships the behaviour. Never
hand-edit versions or CHANGELOG. Declarations live in [`.ctl/ver.yaml`](.ctl/ver.yaml).
`prepare` runs `cargo update --workspace` and stages `Cargo.lock` because CI
packages with `--locked`. Prove the lane by shipping through it (`CTC-009`).

## Checks

```sh
mise run verify
```

`verify` is format, clippy, nextest, doc-tests, cargo-deny licenses/bans/sources, and cargo-machete.
Do not `&&` those in a new task; `depends` is the mise form. Those cargo
invocations share `target/`; the package-cache lock serializes them. Do not
invent extra `CARGO_TARGET_DIR` trees to hide that. Advisories (and yanked
crates) are `mise run deny:advisories` on CI only, so a pre-push `verify`
does not need the network. An unfixable RUSTSEC or yank is listed in
`deny.toml` `[advisories].ignore` with a reason, not by weakening `yanked`
or `unmaintained`.

## Git

Conventional commits. lefthook. No `--no-verify`. Branch `type/number-desc`.
Always open a PR — never push to `main`.
