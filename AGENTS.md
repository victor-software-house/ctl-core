# ctl-core

Shared clap chassis for the `*ctl` CLIs (`verctl`, `qctl`, `forkctl`) and
`state-sync`. **Not a command.** Import as `ctl_core`.

This repo's queue is [`tasks.yaml`](tasks.yaml) (`CTC-###`).

## What this crate owns

- `-h` / `--help` and `-V` / `--version` (never `disable_help_flag`)
- Shared flags: `-c`/`--color`, `-f`/`--format`, `-n`/`--dry-run`/`--preview`,
  `-q`/`--quiet`, plus `--foo` / `--no-foo` negation helpers
- Styled help tables
- Pretty / JSON / colorless `View` over serializable models
- Compact command tables: `kv` / `grid` (take `ColorMode`)
- Process exit wrapper: `go` / `run` (`{bin}: {error:#}` + JSON error object)
- Wire envelope: `Envelope` / `ErrorBody` / `SCHEMA_VERSION`

Domain verbs stay in each CLI. Boolean pairs that are domain-specific
(`--pr` / `--no-pr`) stay in the CLI — use clap `overrides_with` both ways and
`warn_opposites` so the last flag wins without silence.

## Cargo features

Features are tree-shaking. A consumer that only needs the enums does not
compile `clap` or `comfy-table`. Prefer explicit feature sets over defaults
when the binary does not need help/view:

```toml
ctl-core = { version = "0.0.3", default-features = false, features = ["json"] }
```

Crate docs (`src/lib.rs` + `document_features`) are authoritative for the
feature graph. The GitHub README is not rustdoc.

## View contract

1. **Models first.** Prepare the data; the view does not own it.
2. **JSON never contains ANSI.** Pretty may.
3. **Pretty prefers Jinja.** Implement `Pretty` + `View::show_pretty`, or
   `Render` for one-liners. `formatdoc!` stays for short strings; loops and
   `{% if %}` belong in the template.
4. **Tables, not space-padded labels.** `kv` / `grid` take `ColorMode`. Tests
   pass `--color never` or call `grid(ColorMode::Never, …)`. No homemade
   `strip_ansi`.

## Strings

Multiline Rust is `indoc!` / `formatdoc!` / `writedoc!` / `printdoc!` /
`eprintdoc!` / `concatdoc!` (re-exported from this crate). **No `concat!`.**
**No escaped `\n` in a document.** Leave a raw `\n` only when that *is* the
test.

## Comments

Doc comments carry the why, on the item. Inline `//` prose inside a function
body is litter.

## Consumers

`verctl` pins this crate today. `qctl` / `forkctl` still carry their own
presentation in places — migrate them by pinning a released version, not a
path dependency, once the needed surface ships here (see `CTC-006`,
`CTC-008`).

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

## Git

Conventional commits. lefthook. No `--no-verify`. Branch `type/number-desc`.
Always open a PR — never push to `main`.
