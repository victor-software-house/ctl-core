# Changelog

## ctl-core 0.0.5

- Pin Version PR and publish at verctl 0.2.1. Publish no longer needs a full checkout; the action fetches the default branch at depth 1.
- Pin Version PR and publish at verctl 0.2.3. The action fetches the default-branch history, not a depth-1 tip, and rewrites `origin/HEAD` only from `VERCTL_DEFAULT_BRANCH`.

## ctl-core 0.0.4

- Pin Version PR and publish at verctl 0.2.0 so the first crates.io cut from this lane tags the published commit.

## ctl-core 0.0.3

- `kv` / `grid` take `ColorMode`. `--color never` is colorless; no caller strips ANSI.

## ctl-core 0.0.2

- Add `kv` / `grid` pretty tables with styled tokens for command output.
