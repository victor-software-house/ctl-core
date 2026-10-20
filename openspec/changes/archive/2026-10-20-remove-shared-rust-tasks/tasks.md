# Tasks

## 1. Remove the copies

- [x] 1.1 Delete `tasks/rust.toml`, `.github/actions/copier-check/`, and `mise-tasks/check/rust-tasks`. Proof: `rg -n 'tasks/rust.toml|copier-check|check:rust-tasks'` outside `openspec/`, `tasks.yaml`, `CHANGELOG.md`, and `docs/research/` finds only `mise.toml`'s include of the actions repository's catalog.
  2026-10-20 (UTC-3): [ctl-core#156][pr-156] at [a3bc554][a3bc554]. The search outside the excluded paths finds only `mise.toml`'s include of the actions repository's catalog.
- [x] 1.2 Drop the `.taplo.toml` exclusion and the `check:local` dependency. Proof: `check:repo` passes in the pull request's Verify.
  2026-10-20 (UTC-3): `check:repo` passed inside [Verify][verify-run] on [a3bc554][a3bc554].
- [x] 1.3 Point `AGENTS.md` at the actions repository. Proof: the search in 1.1.
  2026-10-20 (UTC-3): the search in 1.1 finds no mention of either copy in `AGENTS.md`.

## 2. Verify

- [x] 2.1 No repository includes either copy. Proof: the organization scan and the code search output, attached to the pull request.
  2026-10-20 (UTC-3): the scan read every non-archived repository's mise configuration, `.miserc.toml`, workflow, and `action.yml` files on its default branch and found no reference; a positive control on a repository that includes the actions catalog found its references. `gh search code` finds the names only in ctl-core, OpenSpec history, work ledgers, and documentation. Both are summarised in [ctl-core#156][pr-156].
- [x] 2.2 Verify and Template pass in CI on the pull request.
  2026-10-20 (UTC-3): [Verify][verify-run] and [Template][template-run] passed on [a3bc554][a3bc554].

## Workflow follow-up

- Archive this change with `openspec archive remove-shared-rust-tasks`, which retires `openspec/specs/shared-rust-tasks/`, and archive CTC-026 with the pull request as evidence.

[pr-156]: https://github.com/victor-software-house/ctl-core/pull/156
[a3bc554]: https://github.com/victor-software-house/ctl-core/commit/a3bc55495dfaf06a0774b106496eb744b53ea8ae
[verify-run]: https://github.com/victor-software-house/ctl-core/actions/runs/39377518918
[template-run]: https://github.com/victor-software-house/ctl-core/actions/runs/39377519118
