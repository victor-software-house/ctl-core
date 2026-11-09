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

- [x] 2.1 Add a corpus test of three kinds of command. One sets every
  attribute, one is a toy `App` command mounted as `toy`, and the rest
  hold one hostile string each. Verify: the snapshots come from a run of the
  current code on the measurement host. insta accepts them under CI's
  flags, and none is written by hand.

  2026-11-08 (-03:00), on the measurement host with rustc 1.99.1, at
  [a1e2d43][a1e2d43] with usage-lib 6.0.0:
  1. [`ctl-core/tests/usage_spec.rs`](../../../ctl-core/tests/usage_spec.rs)
     renders four snapshots: `every`, `toy`, `hostile_help`, and
     `hostile_default`. The toy goes through `App::mounted_as("toy")` and
     `--usage-spec=toy`.
  2. `INSTA_UPDATE=always` wrote them. A second run with
     `INSTA_UPDATE=no`, CI's setting, passed 4 of 4.
  3. Each control, bidi, separator, and byte order mark character shows
     as `<U+XXXX>`. The spec already writes some as KDL's `\u{..}`.
- [x] 2.2 Move usage-lib to 7.3.0 and accept the snapshots again. Verify:
  the snapshot diff shows only decision 2's `""""#` change. The changeset
  lists it as a fix.

  2026-11-08 (-03:00), same host: after `cargo update -p usage-lib
  --precise 7.3.0`, only `hostile_help` changed, in two lines. The
  `--quad-hash` string opens with `##"""` and closes with `"""##` in place
  of one hash. The run with `INSTA_UPDATE=no` passed the corpus and the
  91 library tests. The changeset lists the fix.
- [x] 2.3 Mount the toy through `mount_directives` under mise 2026.11.4.
  Verify: `mise run toy --help` prints the toy's subcommands and flags.
  zsh completion after `mise run toy` and a space offers them. The note
  records the mise version.

  2026-11-09 (-03:00), same host, mise 2026.11.4 macos-arm64 from its
  release binary, ctl-core at usage-lib 7.3.0:
  1. The toy binary declares the corpus's `Toy` and calls
     `App::mounted_as("toy")`. A mise file task runs it and carries the
     three lines `mount_directives("toy")` returns.
  2. `mise run toy --help` printed the toy's help: `status`, `check`, and
     `help`, and the flags `--format`, `--color`, `--no-color`, `--quiet`,
     `--version`, and `--help`. `mise run toy status` ran the verb.
  3. zsh completion runs `mise __complete_word__ --shell zsh --line` with
     the text before the cursor. For `mise run toy` and a space, it
     offered `check` and `status`. For `mise run toy --`, it offered the
     four output flags and `--help`. For `mise run toy check --`, it added
     `--strict`.
  4. The spec has no `--version` flag and no `help` command, so
     completion offers neither.
- [ ] 2.4 Verify: CI passes on the pushed commit.

## 3. The writer

- [x] 3.1 Write the KDL from the clap `Command` in a private module under
  `usage`, covering every getter decision 4 lists. Verify: `usage::spec`
  no longer calls usage-lib, and the corpus snapshots pass unchanged
  except for the values decision 3 lists.

  2026-11-09 (-03:00), on the measurement host with rustc 1.99.1:
  [`usage/kdl.rs`](../../../ctl-core/src/usage/kdl.rs) writes nodes and
  strings, and [`usage/nodes.rs`](../../../ctl-core/src/usage/nodes.rs)
  maps the `Command`. `usage::spec` calls neither usage-lib nor its types.
  `INSTA_UPDATE=always`, then `INSTA_UPDATE=no`, on `--test usage_spec`
  passed 10 of 10. `every` and `toy` kept their bytes. The changed entries
  are decision 3's rows:
  - the KDL v2 cases;
  - `-.5` and `+.5` in `identifier`;
  - the value names in `value_names`, whose 7.3.0 capture passed 6 of 6
    on the same host first.
- [x] 3.2 Add `kdl = "=6.7.1"` as a dev-dependency and these tests:
  - one per row of decision 3, which parses the output and compares the
    value;
  - one over every corpus output, which looks for a literal code point
    from KDL v2's whole disallowed set, U+007F included;
  - one over every corpus output, which looks for a Newline inside a
    single-line string.

  Verify: each test fails against usage-lib 7.3.0's output and passes
  against the writer's.

  2026-11-09 (-03:00), same run: the parse-back and literal checks pass
  against the writer's output.

  2026-11-09 (-03:00), same host: the branch's `usage_spec.rs` over the
  `src/` of [7f4e123][7f4e123], which calls usage-lib 7.3.0, failed all
  four checks:
  1. `defaults_read_back_unchanged`, on the U+2028 default;
  2. `help_reads_back_as_clap_renders_it`, on the whitespace-only line,
     read back empty;
  3. `identifier_words_read_back_as_strings`, on `-.5`;
  4. `every_spec_parses_with_no_forbidden_literal`, on the U+2028 help.

  Each check stops at its first failing case. A row past it shows only
  in the writer's pass.
- [x] 3.3 Add tests for quotes, backslashes, newlines, tabs, non-ASCII
  text, a leading dash, and the words `true`, `null`, and `inf`. Verify:
  each value reads back unchanged.

  2026-11-09 (-03:00), same run: `defaults_read_back_unchanged` and
  `help_reads_back_as_clap_renders_it` cover every hostile string, and
  `identifier_words_read_back_as_strings` every identifier case. All pass.
- [x] 3.4 Build qctl, verctl, and forkctl against the branch. Verify: each
  `--usage-spec` output equals its usage-lib 7.3.0 output.

  2026-11-09 (-03:00), same host: each CLI at its head
  ([qctl][qctl-head], [verctl][verctl-head], [forkctl][forkctl-head]) was
  built twice. Its ctl-core line pointed first at [7f4e123][7f4e123],
  then at the branch. Each pair of `--usage-spec` outputs is
  byte-identical, at 259, 131, and 289 lines.
- [x] 3.5 Repeat task 2.3 against the writer. Also mount one task per
  value-name form of decision 3, on a flag and on a positional. A
  positional with two choices follows each. Verify:
  - task 2.3's result, under the same mise version;
  - for each form, zsh completion after one value offers the two
    choices, and after two values offers only files.

  2026-11-09 (-03:00), same host, mise 2026.11.4 macos-arm64 from its
  release binary:
  1. Task 2.3's toy under the writer: `mise run toy --help` printed
     `status`, `check`, `help`, and the same flags, and
     `mise run toy status` ran the verb. Completion offered the same
     words as in task 2.3 after `mise run toy`, `mise run toy --`, and
     `mise run toy check --`.
  2. Each value-name form under the writer, on a flag and on a
     positional: completion offered `p1` and `p2` after one value. After
     two values it offered only files. Decision 3's table holds the 7.3.0
     results.
- [ ] 3.6 Verify: CI passes on the pushed commit, and the changeset lists
  each departure as a fix.

## 4. Remove usage-lib and keep clap's defaults off

- [x] 4.1 Remove usage-lib from `[workspace.dependencies]`,
  `ctl-core/Cargo.toml`, and the machete ignore list. Verify:
  `Cargo.lock` holds no usage-lib, and holds kdl and winnow 0.7 only
  through the dev-dependency.

  On 2026-11-09 (-03:00), the lock went from 119 packages to 99. It
  dropped usage-lib, usage-argv, regex, itertools, thiserror, semver,
  strsim, anstream and its parsers, and the crates only they pulled in.
  `cargo tree --all-features --edges normal` shows no kdl and no winnow
  0.7; `--edges dev` shows kdl under ctl-core alone.
- [x] 4.2 Ban usage-lib in `deny.toml` through the `deny_bans` answer,
  with a reason. Verify: `mise run deny` passes in CI with no warning.

  The writer's change already applied the rest of decision 5 through
  `copier update` at template v0.0.72. It added the kdl and winnow below
  1.0 `wrappers` bans, and the `syn@2` skip. On 2026-11-09 (-03:00), with
  cargo-deny 0.20.2:
  1. `cargo deny check` reported advisories, bans, licenses, and sources
     ok, with no warning.
  2. With each ban's wrappers pointed elsewhere, it failed with `banned`
     for kdl 6.7.1 and winnow 0.7.15.

  The usage-lib ban went in as a `deny_bans` answer through the same
  `copier update`. On 2026-11-09 (-03:00), `mise run deny` reported bans,
  licenses, and sources ok, with no warning. CI's advisory run is still
  to come on the pull request.
- [x] 4.3 Extend `check:tree` to fail on usage-lib, kdl, or winnow below
  1.0 in ctl-core's normal dependencies with every feature. Verify: CI's
  `check:tree` passes, and fails on a commit that adds usage-lib back.

  On 2026-11-09 (-03:00), `mise run check:tree` passed on the branch.
  With usage-lib added back to both manifests and the lock re-resolved,
  it failed on usage-lib and on clap's `color`, `help`, `error-context`,
  and `suggestions`. With `clap/usage` taken out of the `usage` feature,
  it failed on the missing `usage`. CI's run is still to come.
- [x] 4.4 Verify: `cargo tree -e features -i clap_builder` for ctl-core
  with `app`, `usage`, and `input` shows no `default`, `color`, `help`,
  `error-context`, or `suggestions`. If one shows, cut it or record why it
  cannot be cut.

  Without `help`, clap drops `-h`/`--help` and the `help` subcommand, and
  CI's first run without usage-lib failed 8 tests on that. Turning on
  `clap/help` grew every size tier by 8 to 12 percent, so ctl-core declares
  both itself and `help` stays off
  ([decision 6](design.md#6-nothing-turns-claps-defaults-back-on)).
  On 2026-11-09 (-03:00), the command showed `env`, `std`, `string`, and
  `usage` for `app,usage,input` and for `usage`, and `env`, `std`, and
  `string` for `cli`.
- [x] 4.5 Verify: CI's `deps` (cargo-machete) passes.

  [PR 238][ctl-core#238] merged at [4eaa0fa][4eaa0fa]. Its last head,
  [b8c0b9a][b8c0b9a], passed `deps` in the Static checks job of
  [run 42568402969][run-42568402969].
- [x] 4.6 Declare `-h`/`--help` and the `help` subcommand in ctl-core, and
  add `test:cli`, which runs the parser tests with only the `cli` feature.
  Verify: CI's `test:cli` passes with `-h`, `--help`, `-V`, and
  `help <command>`, no snapshot or parity file changes, and `check:size`
  reports each tier within its budget.

  In [run 42568402969][run-42568402969] on [b8c0b9a][b8c0b9a], `test:cli`
  passed its 6 tests, `help_and_version_shorts` and
  `help_subcommand_names_the_command` among them. No snapshot or parity
  file changed in [PR 238][ctl-core#238]. `check:size` passed every tier
  within its limit:

  | Tier | `.text` | Budget | Limit |
  |:--|--:|--:|--:|
  | `cli` | 343,888 | 343,888 | 350,765 |
  | `help` | 446,886 | 444,582 | 453,473 |
  | `view` | 408,998 | 408,998 | 417,177 |
  | `app` | 471,974 | 469,654 | 479,047 |

- [x] 4.7 Give `Surface::from_command` the same `-h`/`--help` flag and
  `help` subcommand, so its Usage KDL holds them as it did with Clap's
  `help` feature, and turns help off only where the CLI declared that.
  Verify: a unit test fails without the change and passes in CI with it.

  On 2026-11-09 (-03:00), `usage_kdl_keeps_the_declared_help` failed at
  [e7861c6] with no help flag and no `help` command line, and with
  `disable_help_flag` on every declared command. With the change it
  passed, and it passed in CI in [run 42568402969][run-42568402969] on
  [b8c0b9a][b8c0b9a].

## 5. annotate-snippets

- [x] 5.1 List annotate-snippets symbols in a qctl release build with the
  branch patched in. Verify: the note names the call path that keeps
  them.

  2026-11-09 (-03:00), on a macOS arm64 host, by decision 16's method:
  1. [qctl][qctl-7d4e2ed] was built with its release profile,
     unstripped, against [4eaa0fa][4eaa0fa]. The binary holds 95
     `annotate_snippets` symbols.
  2. The path that keeps them runs through serde-saphyr itself.
     `deserialize_str` calls `to_string` on a visitor's error to look
     for "expected a borrowed string"
     ([deserializer.rs, 2.0.0][saphyr-deserializer], lines 1149 and
     1196). That `Display` runs the renderer and its snippet code.
  3. In the disassembly, each caller of that `to_string` is serde-saphyr's
     `deserialize_recorded_key`, once per field visitor. The visitors
     are qctl's ledger and row types, and serde_json's key classifier.
     Any struct with string keys reaches it inside `deserialize`.
- [x] 5.2 Capture real-run problem reports from the input tests and from
  qctl on malformed ledgers, before the change. Verify: the captures are
  committed as snapshots, accepted with insta, none written by hand.

  `problem_reports_keep_their_wording` in `ctl-core/tests/input.rs`
  snapshots nine reports. Three of them are the reports qctl printed for
  malformed ledgers: a duplicate key, a stray `-`, and an unclosed `[`.
  On 2026-11-09 (-03:00), insta accepted the snapshot at
  [4eaa0fa][4eaa0fa] with the test added. qctl's own output was compared
  on the host and is not committed here: qctl's tests own it.
- [ ] 5.3 Build messages without the snippet renderer, as decision 7
  describes. Verify: every input test and snapshot passes unchanged, and
  qctl's release build has no annotate-snippets symbol. If either fails,
  stop and report instead of changing a message.

  Failed on 2026-11-09 (-03:00), so the work stopped and the change was
  dropped:
  1. Messages built from `MessageFormatter::format_message` left the
     snapshot and the input tests unchanged. qctl printed the same
     reports in pretty and JSON output.
  2. qctl's release build still held 95 `annotate_snippets` symbols.
     Its `__text` grew from 3,232,292 to 3,232,484 bytes.
  3. The renderer stays reachable through serde-saphyr's
     `deserialize_str`, as task 5.1 records. ctl-core cannot avoid it.

  Asking serde-saphyr to stop calling `Display` there is for the
  operator to decide.

## 6. Gates and docs

- [x] 6.1 Move serde-saphyr to 2.1.1 and keep `deserialize` and
  `comments`. Verify: the input tests pass unchanged, and the note records
  what `comments` gates.

  In [run 42575754249][run-42575754249] on [9d25529][9d25529], the input
  suites passed with no test or snapshot changed: `input` 8,
  `input_limits` 3, and `input_toml` 16, with
  `problem_reports_keep_their_wording` among them. `comments` gates
  `Budget::max_total_comment_bytes` and `max_buffered_comment_events`
  ([budget.rs][saphyr-budget-211]). The first is the `limits:` key that
  `.ctl/input.yaml` exposes and that a limit error names. The lock moved
  granit-parser from 2.0.0 to 2.1.0 with it.
- [x] 6.2 Verify: `deny.toml`'s `[bans.workspace-dependencies]` check
  passes, so each dependency is declared once in
  `[workspace.dependencies]`.

  On 2026-11-09 (-03:00), `cargo deny --all-features check bans` reported
  bans ok on the branch, and CI's `deny` passed in
  [run 42575754249][run-42575754249].
- [ ] 6.3 Add the `usage` and `input` tiers to the `size` example and to
  `[workspace.metadata.size]`. Set every budget to the `.text` CI prints
  for the commit, with a reason per tier. Verify: a budget follows a
  measured, explained change and never rises to hide an unexplained one,
  and CI's `check:size` passes.

  In [run 42575754249][run-42575754249] on [9d25529][9d25529], CI printed
  these `.text` sizes. Each budget now equals CI's number:

  | Tier | Features | Old budget | `.text` and new budget |
  |:--|:--|--:|--:|
  | `cli` | `cli` | 343,888 | 343,888 |
  | `help` | `help` | 444,582 | 446,886 |
  | `view` | `cli,view` | 408,998 | 408,998 |
  | `app` | `app` | 469,654 | 471,974 |
  | `usage` | `app,usage` | none | 525,798 |
  | `input` | `app,input` | none | 1,141,709 |

  `help` and `app` rose by 2,304 and 2,320 bytes. That is the cost of
  declaring `-h`/`--help` and the `help` subcommand in ctl-core
  ([decision 6](design.md#6-nothing-turns-claps-defaults-back-on)), which
  [PR 238][ctl-core#238] already measured. A linux-x64 measurement with
  1.99.1 set the first `usage` and `input` budgets, and it matched CI's
  numbers to the byte.
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

[a1e2d43]: https://github.com/victor-software-house/ctl-core/commit/a1e2d438f2ce4abbd1f652f59d88cb245cd1fd76
[9161cfc]: https://github.com/victor-software-house/ctl-core/commit/9161cfc74d150a9e9ad467b2ac1d2312261f49a2
[d05500f]: https://github.com/victor-software-house/ctl-core/commit/d05500fa0a1689038dc16b58d5c1fa2227c68b78
[7f4e123]: https://github.com/victor-software-house/ctl-core/commit/7f4e123d0caca6e4a9c87f5d1a0cc4fae33d88db
[qctl-head]: https://github.com/victor-software-house/qctl/commit/cd06a58610646d6ebd9e2ec3ce2bdc1a7a59e4b7
[verctl-head]: https://github.com/victor-software-house/verctl/commit/d087741fd60190757141761eb8410a1a73875c53
[forkctl-head]: https://github.com/victor-software-house/forkctl/commit/7b987070f9069842c2e9afdd17fca28353b81bc5
[e7861c6]: https://github.com/victor-software-house/ctl-core/commit/e7861c6c3dfb8760f6007bcdd657b30d64158eb8
[4eaa0fa]: https://github.com/victor-software-house/ctl-core/commit/4eaa0faf6ff60aba20f99adaefdb1258f72aa6d0
[b8c0b9a]: https://github.com/victor-software-house/ctl-core/commit/b8c0b9a271d4842ab1e006794abe822e31ebe033
[ctl-core#238]: https://github.com/victor-software-house/ctl-core/pull/238
[run-42568402969]: https://github.com/victor-software-house/ctl-core/actions/runs/42568402969
[run-42575754249]: https://github.com/victor-software-house/ctl-core/actions/runs/42575754249
[9d25529]: https://github.com/victor-software-house/ctl-core/commit/9d25529b4d73dd158b29e38bcbed0f50370cb1b7
[saphyr-budget-211]: https://docs.rs/crate/serde-saphyr/2.1.1/source/src/de/budget.rs
[qctl-7d4e2ed]: https://github.com/victor-software-house/qctl/commit/7d4e2ed8cfec82f640812a5cf1cd121b9824db0e
[saphyr-deserializer]: https://docs.rs/crate/serde-saphyr/2.0.0/source/src/de/deserializer.rs
