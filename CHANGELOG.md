# Changelog

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
