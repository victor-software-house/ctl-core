# Tasks

Every compile, lint and test step runs in a box or the pull request's CI, not on the authoring machine.

## 1. The declaration

- [x] 1.1 Add `Input::flattened` and the rewrite in `Input::check`, with tests for YAML, TOML, an undeclared input and a key named like the flattened field.  
  Verify the tests pass in a box and the problem-report snapshot does not change.  
  Evidence, 2026-11-11 (UTC): in a box, `a_problem_under_a_declared_flattened_field_sits_where_the_file_writes_it`, `an_undeclared_or_unmatched_flattened_field_places_as_garde_names_it`, the TOML test and the `flattened` doctest pass.  
  `problem_reports_keep_their_wording` passes with the snapshot unchanged.
- [x] 1.2 Add the `declared-input` delta, the notes in `ctl-core/AGENTS.md` and the README, and a `patch` changeset.  
  Verify the OpenSpec validation and `check:docs` pass.  
  Evidence, 2026-11-11 (UTC): `mise run -c verify` passes in a box, `specs`, `check:docs` and `check:size` included.  
  The `input` tier measured 1,053,230 bytes, 2,096 above its budget of 1,051,134.  
  That is inside the limit of 1,072,156, so the budget stays.

## 2. Integration

- [ ] 2.1 Verify the pull request's CI passes.
