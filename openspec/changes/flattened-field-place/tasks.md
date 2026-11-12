# Tasks

Every compile, lint and test step runs in a box or the pull request's CI, not on the authoring machine.

## 1. The declaration

- [ ] 1.1 Add `Input::flattened` and the rewrite in `Input::check`, with tests for YAML, TOML, an undeclared input and a key named like the flattened field.  
  Verify the tests pass in a box and the problem-report snapshot does not change.
- [ ] 1.2 Add the `declared-input` delta, the notes in `ctl-core/AGENTS.md` and the README, and a `patch` changeset.  
  Verify the OpenSpec validation and `check:docs` pass.

## 2. Integration

- [ ] 2.1 Verify the pull request's CI passes.
