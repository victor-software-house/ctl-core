# Tasks

## 1. The flags

- [ ] 1.1 Add `--input` and `--input-format` to every command from `App` when `app` and `input-document` are on, under an "Input" heading, bound to `<PREFIX>_INPUT` and `<PREFIX>_INPUT_FORMAT`, and add `App::without_input()`.
  Verify: tests for "The flags are there" and "A CLI removes them" pass on the build host, and the help snapshot shows the heading.
- [ ] 1.2 Read the document by `--input-format` or extension through the input format registry and limits, and refuse stdin without a format.
  Verify: tests read one document in YAML, JSON and TOML, and the stdin scenario passes.

## 2. Joining the command line

- [ ] 2.1 From the probe `env_under_flags` runs, find the selected command, its keys, the arguments the command line set and the kept variables, and refuse unknown keys on their lines.
  Verify: the unknown-key scenario and a subcommand two levels deep pass.
- [ ] 2.2 Encode each value by design decision 5 and place the tokens by decision 6, probing prefixes for the subcommand's position.
  Verify: tests for every row of the encoding table, `infer_subcommands`, a flag subcommand, and both positional scenarios pass.
- [ ] 2.3 Apply the precedence and `App::input_conflicts` of decision 7.
  Verify: the precedence and conflict scenarios pass, including a key against its own `--no-` side under both policies.

## 3. Placing problems

- [ ] 3.1 Collect ctl-core's own refusals and report them together, and place a clap error on the key or value behind it.
  Verify: the value-clap-refuses scenario passes, and a document with two unknown keys reports both.
- [ ] 3.2 Add the `input-document` feature, which turns on `app` and ctl-core's `error-context`, and change `check:tree` to require clap's `error-context` there and refuse it for `app,usage,input`, `app,input-toml` and `app,input-json`.
  Verify: `check:tree` passes on the build host, and it fails when any of those three turns `error-context` on.

## 4. Size, docs and release

- [ ] 4.1 Add an `input-document` size tier, cut the measured hot spots (the two generic sorts, the `--input-format` parser, the cloned tree), and measure every tier at the base and the head in one box.
  Verify: `check:size` passes in CI, every existing tier is unchanged, and the pull request lists both measurements per tier.
- [ ] 4.2 Update the crate docs, `ctl-core/AGENTS.md`, the README, the env-flags docs and the usage spec output for the two flags.
  Verify: the doc and prose tasks pass in CI.
- [ ] 4.3 Write a `ctl-core: patch` changeset naming the flags, their variables, `App::without_input`, `App::input_conflicts` and the precedence.
  Verify: `openspec validate input-document --strict` passes, and the Version PR lists the entry.
