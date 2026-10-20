# Tasks

## 1. Remove the copies

- [ ] 1.1 Delete `tasks/rust.toml`, `.github/actions/copier-check/`, and `mise-tasks/check/rust-tasks`. Proof: `rg -n 'tasks/rust.toml|copier-check|check:rust-tasks'` outside `openspec/`, `tasks.yaml`, `CHANGELOG.md`, and `docs/research/` finds only `mise.toml`'s include of the actions repository's catalog.
- [ ] 1.2 Drop the `.taplo.toml` exclusion and the `check:local` dependency. Proof: `check:repo` passes in the pull request's Verify.
- [ ] 1.3 Point `AGENTS.md` at the actions repository. Proof: the search in 1.1.

## 2. Verify

- [ ] 2.1 No repository includes either copy. Proof: the organization scan and the code search output, attached to the pull request.
- [ ] 2.2 Verify and Template pass in CI on the pull request.

## Workflow follow-up

- Archive this change with `openspec archive remove-shared-rust-tasks`, which retires `openspec/specs/shared-rust-tasks/`, and archive CTC-026 with the pull request as evidence.
