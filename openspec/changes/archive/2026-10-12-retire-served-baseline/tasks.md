# Tasks

- [x] 1. Remove the `baseline` feature and module, `templates/baseline/`,
  `tests/baseline.rs`, `.ctl/rust.yaml`, `Served::with_body`, and the
  `served-baseline` spec. Proof: `git grep baseline` finds none of them,
  and `mise run verify` passes on a build runner.
- [x] 2. Drop the `baseline` set from `check:features` and from
  `.copier-answers.yml`. Proof: the files in the diff, and `check:features`
  passes inside `verify`.
- [x] 3. Archive CTC-015 as dropped and update CTC-018 and AGENTS.md.
  Proof: the files in the diff, and `q check` passes inside `verify`.
- [ ] 4. Release as 0.6.15, a patch by operator decision, with CI's semver
  check allowing the removal only against the 0.6.14 baseline (`semver:ci`).
  Proof: `mise run semver:ci` passes, and the tag exists.

## Evidence

2026-10-12 UTC, tasks 1 to 3. On a fresh Linux x86_64 build runner with
mise 2026.10.8, `mise run verify` exited 0 on [3248712], in the pre-push
hook as well, and `cargo package --locked --all-features` exited 0.

2026-10-12 UTC, task 4, not done. `mise run semver` against ctl-core
0.6.14 exited 100 with "semver requires new major version: 6 major and 0
minor checks failed", on the removed module, its structs, its functions,
and `REPO_FILE`.

[3248712]: https://github.com/victor-software-house/ctl-core/commit/3248712ed77d72ac64dc8a402e239644beade9a3
