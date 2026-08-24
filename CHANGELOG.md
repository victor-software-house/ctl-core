# Changelog

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
