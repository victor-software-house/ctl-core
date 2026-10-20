# Design

## Context

ctl-core's `mise.toml` already includes the shared Rust tasks from the [actions] repository, and its `template.yml` already calls the shared drift-check workflow.
The local copies run nowhere except `check:rust-tasks`, which tests `tasks/rust.toml` itself.

## Decisions

1. Delete both copies in one pull request.
   The alternative was a stub `tasks/rust.toml` that points at the actions repository.
   It lost because an include pinned to an old tag still gets the old file, and a later tag has no consumer to read the stub.
2. Retire the `shared-rust-tasks` capability instead of rewriting it.
   The alternative was a requirement that ctl-core includes the catalog from the actions repository.
   It lost because the template renders that include, so it is repository configuration, not behaviour ctl-core owns.
3. Return `mise-tasks/check/local.sh` to the starter body, with no dependency.
   The alternative was keeping an empty `check:rust-tasks`.
   It lost because a check with nothing to test only adds a task name.

This change adds no automatic behaviour, so it has no override or disable path.

## Risks

A repository that still included ctl-core's catalog from `main` would lose its tasks.
The scan in the proposal found none; every include names a commit or a tag.

`check:rust-tasks` ran the catalog's `check:versions` against a stub `ver` task, with and without a release config.
The actions repository's `check:catalog` compares its catalog field by field with ctl-core's copy at a pinned commit, which this deletion does not break, but it does not run `check:versions`.
That behavioural check has no new home in this change.

[actions]: https://github.com/victor-software-house/actions/tree/1b6e52f1c2783dec2481fe281850aeb0161e92bd
