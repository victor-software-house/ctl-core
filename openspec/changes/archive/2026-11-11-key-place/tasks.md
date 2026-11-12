# Tasks

Every compile, lint and test step runs in a box or the pull request's CI, not on the authoring machine.

## 1. The key place

- [x] 1.1 Add `Input::place_key`, for YAML through `Node::key` and for TOML through the key span of the entry.  
  Verify the tests for a block mapping, a scalar, a flow mapping, a list item, a missing path and the root fail to compile before the change and pass after it.  
  Evidence, 2026-11-11 (UTC): the tests call `place_key`, so they do not compile without it. In a box, `a_key_is_placed_where_it_is_written_and_its_value_where_it_starts` for YAML and TOML, `an_item_has_no_key_and_a_flow_mapping_is_placed_on_its_line`, `an_array_item_has_no_key` and the missing-path tests pass, with the doctest of `place_key`.
- [x] 1.2 Add the `declared-input` delta, the notes in `ctl-core/AGENTS.md` and the README, and the changeset.  
  Verify the OpenSpec validation and `check:docs` pass.  
  Evidence, 2026-11-11 (UTC): `format`, `lint`, `check:features`, `test`, `test:input-toml`, `test:doc`, the three doc tasks, `check:size`, `check:tree`, `specs`, `specs:budget`, `lint:md`, `lint:typos`, `lint:prose`, `check:docs`, `check:repo`, `check:versions` and `semver` pass in a box.  
  The `input` tier measured 1,051,134 bytes against 1,043,133, 8,001 above its budget and inside the tolerance, and the budget moved to it.

## 2. Integration

- [x] 2.1 Verify the pull request's CI passes: lint, workflow policy, prose lint, OpenSpec, size and the dependency checks.  
  Evidence, 2026-11-12 (UTC): [PR 280][ctl-core#280] merged as 60e91fa with `verify / Verify`, `verify / Static checks`, `verify / Build and test` and `verify / Publish checks` green.
[ctl-core#280]: https://github.com/victor-software-house/ctl-core/pull/280
