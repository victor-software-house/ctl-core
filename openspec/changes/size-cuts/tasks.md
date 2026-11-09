# Tasks

## 1. Evidence before the plan

- [x] 1.1 Find why 6 shell tests failed on a macOS arm64 host while CI
  passed. Verify: the same tests pass on that host once the cause is gone.

  | Target | Test |
  |:--|:--|
  | `completion` | `fish_registers_one_line_that_calls_the_binary_by_name` |
  | `shell_init` | `fish_runs_the_hook_on_the_prompt_event` |
  | `shell_init` | `fish_runs_the_hook_once` |
  | `shell_init` | `no_shell_runs_the_hook_without_its_variable` |
  | `shell_init` | `no_shell_runs_the_hook_without_the_binary` |
  | `shell_init` | `no_shell_takes_a_function_for_the_binary` |

  2026-11-08 (-03:00), on macOS 26.7.1 (Darwin 25.6.0, arm64) at
  [9161cfc][9161cfc]. Not a macOS bug:
  1. The failing run used plain `cargo test` outside mise. The shells
     `mise.test.toml` pins were not on `PATH`, and the host has no fish of
     its own.
  2. All 6 failures were spawn errors: the program was missing.
  3. The `dev` and `test` profiles put conda-fish 4.9.3 and conda-zsh
     5.9.2 on `PATH`. The command:

     ```sh
     mise x -- cargo test --locked --workspace --all-features -p ctl-core --test completion --test shell_init
     ```

  4. `completion` passed 11 of 11, and `shell_init` passed 14 of 14. CI's
     Build and test job loads the same profile.

- [x] 1.2 Compare today's `--usage-spec` output with usage-lib 7.3.0's,
  and check both against KDL v2. Verify: [design.md](design.md), decisions
  2 and 3, lists every difference with its cause.

  2026-11-08 (-03:00), on the measurement host with rustc 1.99.1:
  1. Three consumers were built against ctl-core [d05500f][d05500f]:
     qctl [cd06a58][qctl-head], verctl [d087741][verctl-head], and forkctl
     [7b98707][forkctl-head]. Each was built with usage-lib 6.1.1, then
     with 7.3.0. Each pair printed byte-identical specs: 259, 131, and 289
     lines.
  2. A probe crate rendered two commands with usage-lib 6.0.0 and 7.3.0.
     One sets every attribute 7.3.0 reads. The other holds 35 hostile
     strings, in help text and in default values. kdl 6.7.1 parsed each
     output back.
  3. The every-attribute output was identical, 87 lines. The hostile
     outputs differed only for `""""#`, where 6.0.0's output fails to
     parse. Both versions failed in the cases decision 3 lists.
  4. Not verified yet: mise reading either output. Tasks 2.3 and 3.5 cover
     it.

## 2. Baseline from usage-lib 7.3.0

- [ ] 2.1 Add a corpus test of three kinds of command. One sets every
  attribute, one is a toy `App` command mounted as `toy`, and the rest
  hold one hostile string each. Verify: the snapshots come from a run of the
  current code on the measurement host. insta accepts them under CI's
  flags, and none is written by hand.
- [ ] 2.2 Move usage-lib to 7.3.0 and accept the snapshots again. Verify:
  the snapshot diff shows only decision 2's `""""#` change. The changeset
  lists it as a fix.
- [ ] 2.3 Mount the toy through `mount_directives` under mise 2026.11.4.
  Verify: `mise run toy --help` prints the toy's subcommands and flags.
  zsh completion after `mise run toy` and a space offers them. The note
  records the mise version.
- [ ] 2.4 Verify: CI passes on the pushed commit.

## 3. The writer

- [ ] 3.1 Write the KDL from the clap `Command` in a private module under
  `usage`, covering every getter decision 4 lists. Verify: `usage::spec`
  no longer calls usage-lib, and the corpus snapshots pass unchanged
  except for the values decision 3 lists.
- [ ] 3.2 Add `kdl = "=6.7.1"` as a dev-dependency and a test per row of
  decision 3 that parses the output and compares the value. Verify: each
  test fails against usage-lib 7.3.0's output and passes against the
  writer's.
- [ ] 3.3 Add tests for quotes, backslashes, newlines, tabs, non-ASCII
  text, a leading dash, and the words `true`, `null`, and `inf`. Verify:
  each value reads back unchanged.
- [ ] 3.4 Build qctl, verctl, and forkctl against the branch. Verify: each
  `--usage-spec` output equals its usage-lib 7.3.0 output.
- [ ] 3.5 Repeat task 2.3 against the writer. Verify: the same result,
  under the same mise version.
- [ ] 3.6 Verify: CI passes on the pushed commit, and the changeset lists
  each departure as a fix.

## 4. Remove usage-lib and keep clap's defaults off

- [ ] 4.1 Remove usage-lib from `[workspace.dependencies]`,
  `ctl-core/Cargo.toml`, and the machete ignore list. Verify:
  `Cargo.lock` holds no usage-lib, and holds kdl and winnow 0.7 only
  through the dev-dependency.
- [ ] 4.2 Ban usage-lib in `deny.toml`. Ban kdl and winnow below 1.0 with
  `wrappers`, as decision 5 describes. Give each ban a reason, in the
  syntax the pinned cargo-deny accepts. Verify: `mise run deny` passes in
  CI.
- [ ] 4.3 Extend `check:tree` to fail on usage-lib, kdl, or winnow below
  1.0 in ctl-core's normal dependencies with every feature. Verify: CI's
  `check:tree` passes, and fails on a commit that adds usage-lib back.
- [ ] 4.4 Verify: `cargo tree -e features -i clap_builder` for ctl-core
  with `app`, `usage`, and `input` shows no `default`, `color`, `help`,
  `error-context`, or `suggestions`. If one shows, cut it or record why it
  cannot be cut.
- [ ] 4.5 Verify: CI's `deps` (cargo-machete) passes.

## 5. annotate-snippets

- [ ] 5.1 List annotate-snippets symbols in a qctl release build with the
  branch patched in. Verify: the note names the call path that keeps
  them.
- [ ] 5.2 Capture real-run problem reports from the input tests and from
  qctl on malformed ledgers, before the change. Verify: the captures are
  committed as snapshots, accepted with insta, none written by hand.
- [ ] 5.3 Build messages without the snippet renderer, as decision 7
  describes. Verify: every input test and snapshot passes unchanged, and
  qctl's release build has no annotate-snippets symbol. If either fails,
  stop and report instead of changing a message.

## 6. Gates and docs

- [ ] 6.1 Move serde-saphyr to 2.1.1 and keep `deserialize` and
  `comments`. Verify: the input tests pass unchanged, and the note records
  what `comments` gates.
- [ ] 6.2 Verify: `deny.toml`'s `[bans.workspace-dependencies]` check
  passes, so each dependency is declared once in
  `[workspace.dependencies]`.
- [ ] 6.3 Add the `usage` and `input` tiers to the `size` example and to
  `[workspace.metadata.size]`. Set every budget to the `.text` CI prints
  for the commit, with a reason per tier. Verify: no budget is higher than
  before, and CI's `check:size` passes.
- [ ] 6.4 Update `ctl-core/AGENTS.md`, `docs/presentation.md`, the `usage`
  module docs, and the feature docs in `ctl-core/Cargo.toml` where they
  name usage-lib or its Serde edge. Verify: `rg usage-lib` finds only
  `deny.toml`, the changeset, and this change.
- [ ] 6.5 Verify: Vale reports zero problems, no lint exception lacks a
  reason, and the changeset covers every behaviour change in the stack.

## 7. Measure again

- [ ] 7.1 Measure qctl and rawr with the stack patched in, by decision
  16's method. Verify: decision 16 holds the table, linked to the commits
  and runs.

[9161cfc]: https://github.com/victor-software-house/ctl-core/commit/9161cfc74d150a9e9ad467b2ac1d2312261f49a2
[d05500f]: https://github.com/victor-software-house/ctl-core/commit/d05500fa0a1689038dc16b58d5c1fa2227c68b78
[qctl-head]: https://github.com/victor-software-house/qctl/commit/cd06a58610646d6ebd9e2ec3ce2bdc1a7a59e4b7
[verctl-head]: https://github.com/victor-software-house/verctl/commit/d087741fd60190757141761eb8410a1a73875c53
[forkctl-head]: https://github.com/victor-software-house/forkctl/commit/7b987070f9069842c2e9afdd17fca28353b81bc5
