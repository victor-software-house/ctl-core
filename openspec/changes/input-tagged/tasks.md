# Tasks

Every compile, lint and test step runs in a box or the pull request's CI, not on the authoring machine.

## 1. The re-export

- [ ] 1.1 Require vsh-serde-saphyr 0.0.4, re-export `Tagged` from `ctl_core::input`, and test a tagged list and an untagged one into `Tagged<an untagged enum>` through `Input::parse`.  
  Verify the test passes in the pull request's CI.
- [ ] 1.2 Add the `declared-input` delta, the notes in `ctl-core/AGENTS.md` and the README, and a `patch` changeset.  
  Verify the OpenSpec validation and `check:docs` pass.

## 2. Integration

- [ ] 2.1 Verify the pull request's CI passes.
