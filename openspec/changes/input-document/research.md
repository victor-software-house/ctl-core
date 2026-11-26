# Research: an input document under clap

Retrieved 2026-11-26 (UTC).

The subject is clap 4.6.6, read at tag v4.6.6 (commit 4a622b4340d5e1fffff60c0ecefdc6882f738159).
Every statement comes from reading source, issues and published crates.

## 1. Answer

1. Clap 4.6.6 has no API for a value layer between the environment and the default.
   The need is tracked in [clap#748][issue-748], [discussion 2763][discussion-2763] and [clap#4793][issue-4793].
2. Only two mechanisms keep clap's relational checks (conflicts, requires, groups, `required_if_eq`, `exclusive`): synthesized argv tokens (3.2) and a synthetic environment variable per argument (3.3).
   Clap treats both as user-given values.
   `default_value` (3.1) loses every relational check, and editing `ArgMatches` (3.4) is not possible through the public API.
3. The recommendation is synthesized argv tokens, built from the probe parse ctl-core already runs in `env_under_flags` (section 6).
   The synthetic variable fits clap's layer order better.
   But it needs `std::env::set_var`, which is `unsafe` in edition 2024 and denied by the workspace lints.
4. Neither mechanism places an error on the document by itself.
   ctl-core must map a clap error back to a key through `InvalidArg` and `PriorArg`.
   Those exist only with clap's `error-context` feature, which ctl-core turns off (2.6).

## 2. How clap 4.6.6 fills and checks values

### 2.1 Order of layers

1. The parser reads the command line, then calls `add_env`, then `add_defaults`, then the validator ([parser.rs L49-73][parse-order]).
2. `add_env` skips an argument the matcher already holds, and marks the value `EnvVariable` ([L1414-1442][add-env]).
3. `add_defaults` runs for every argument ([L1444-1453][add-defaults]).
   A matching `default_value_if` returns before the plain default ([L1455-1531][default-if]).
4. `ValueSource` orders `DefaultValue < EnvVariable < CommandLine`, and only `DefaultValue` is not explicit ([value_source.rs][value-source]).

### 2.2 What `react` does for every source

1. `num_args` is checked only for `CommandLine` values ([L1161-1163][num-args-check]).
2. A `value_delimiter` splits values from every source ([L1178-1198][delimiter]).
3. A repeated `Set` or flag argument conflicts with itself ([L1208-1218][repeat-check], [L1251-1260][repeat-flag], [L1272-1281][repeat-action]).
4. `overrides_with` removes the other side only for a `CommandLine` value ([L1533-1551][overrides-with]).
   The validator treats overrides as conflicts ([validator.rs L504-521][validator-overrides]), so an env `--x` and an env `--no-x` fail together.
5. Groups receive a member only for an explicit source ([L1533-1551][overrides-with]).

### 2.3 Value parsers run for every source

The value parser, and so possible values, runs for command line, environment and default values alike ([L1093-1117][push-values]).
`ValueParser::parse_ref` is `pub(crate)` ([value_parser.rs L231-244][parse-ref]), so ctl-core cannot run a parser on one value outside a parse.

### 2.4 Explicit values drive every relational check

1. `check_explicit` returns false for a `DefaultValue` ([matched_arg.rs L135-151][check-explicit]).
2. The validator uses it for conflicts, `exclusive`, `requires`, `required*` and groups ([L24-60][validator-start], [L62-82][validator-else], [L196-219][validator-required], [L221-341][validator-relations]).
3. Predicates do not apply to defaults ([arg_predicate.rs L3-6][predicate-doc]).
4. Clap's tests expect a required argument with a default to fail when nothing is given ([default_vals.rs][default-tests]).
   The rule dates from clap 3.1 ([CHANGELOG][changelog-3-1]); [clap#6031][issue-6031] asks to revisit it.

### 2.5 Environment capture and flag defaults

`Arg::env(name)` reads the process variable at once and stores it in a `pub(crate)` field ([arg.rs L2205-2213][arg-env], [L87][env-field]).
No public method sets that value without the process environment.
`SetTrue`, `SetFalse` and `Count` get implicit defaults ([arg.rs L4578-4608][implicit-defaults], [action.rs L403-415][action-defaults]), so clearing `default_value` on a flag removes nothing.

### 2.6 Error context, and what ctl-core enables

1. `Error::context` exists only with the `error-context` feature ([error/mod.rs L184-196][error-context]).
2. With it, a conflict carries `InvalidArg` and `PriorArg`, and an invalid value or parser failure carries `InvalidArg` and `InvalidValue` ([L378-404][context-conflict], [L454-482][context-invalid], [L668-684][context-parser]).
   `InvalidArg` is the argument's display form, such as `--port <PORT>`.
3. ctl-core's workspace manifest enables only `std`, `derive`, `env` and `string` ([Cargo.toml L15-20][workspace-clap]), and its `cli` feature leaves `error-context` to the CLI ([L25-32][cli-feature]).
   `deny.toml` bans usage-lib because it re-enabled clap's defaults ([L36][deny-usage]).
4. The `check:tree` script fails when ctl-core turns on `error-context` ([tree.sh L75-81][check-tree]), and the changelog tells a consumer to turn it on ([L164-169][core-changelog]).
   So placing a clap error on the document works only in a CLI that enables it, unless that rule changes.
5. Clap reports an environment value's error as if the flag were typed ([clap#5202][issue-5202]).

## 3. Mechanisms

### 3.1 (a) `default_value` from the document on a cloned `Command`

1. **Precedence.** Command line and environment win.
   But a matching `default_value_if` beats the document ([L1455-1490][default-if-head]), and no public method clears it ([arg.rs L3064-3079][default-if-append]).
2. **Checks.** Value parser and possible values are kept.
   `num_args` is lost (2.2).
3. **Relational checks.** Lost (2.4): a document value neither satisfies `required` nor triggers `requires` or a conflict.
4. **Order.** `default_value_if` on other arguments sees earlier defaults, so the outcome depends on declaration order ([clap#4918][issue-4918]).
5. **Verdict.** It fails the requirement that document values pass clap's conflicts, requires and groups.

### 3.2 (b) Synthesized argv tokens at the selected subcommand level

1. **Precedence.** Tokens are `CommandLine` values and outrank the environment, because `add_env` skips a present argument ([L1414-1442][add-env]).
   ctl-core must leave out every key whose variable is kept, every key the command line set, and every key whose `--x/--no-x` partner it set.
   Otherwise the result depends on token order or fails as a conflict (2.2).
2. **Checks.** Value parser, possible values, `num_args`, `require_equals` and every relational check are kept.
3. **Placement in argv.** This is the cost of the mechanism.
   1. Subcommand names match only before `--` ([L113-127][subcommand-match]); after it every token is a positional ([L131-145][after-dashes]).
   2. A `trailing_var_arg` positional takes every later token ([L405-409][trailing-var]), and a `last(true)` positional is accepted only after `--` ([L393-404][last-positional]).
   3. Positionals fill by index in token order ([L392-437][positional-fill]).
   4. So option tokens appended at the end of argv become values once argv has `--` or a trailing positional.
      They are safe right after the selected subcommand's own name.
      Clap does not report that token's position.
   5. A document positional is safe only after the user's own.
      With `last(true)` or `allow_missing_positional`, every value after `--` goes to the highest-index positional ([L380-387][last-index]).
      The escape test runs only before the first `--` ([L113][escape-test]).
   6. A narrow rule: accept document positionals only when the command line gave none and the command has neither.
      Append them after one `--`, reusing the user's.
4. **Encoding values.**
   1. `--long=value` is exactly one value, taken as is, and satisfies `require_equals` ([L1061-1074][attached-value], [L1035-1060][require-equals]).
   2. A zero-value flag with an attached value fails ([L829-851][unneeded-value]).
      So `SetTrue` true becomes `--long`, false is left out, `Count` n becomes n copies, and `--no-x` is emitted by name.
   3. `Append` takes one `--long=v` per element, and a delimited `Set` takes `--long=v1,v2`.
   4. A `Set` option with `num_args` above one and no delimiter would absorb the user's next tokens ([L291-312][multi-value]), so it is not safely supported.
5. **Error placement.** Clap formats the error as if the user typed the token.
   ctl-core maps `InvalidArg` and `PriorArg` back to the key's span, which needs `error-context` (2.6).
6. **Unknown keys.** Refuse them before the parse, or clap names a flag the user never typed.
7. **Source.** `CommandLine`, so a handler cannot tell document values from typed ones.
8. **Interaction with `env_under_flags`.** It counts a `CommandLine` value as given ([env.rs L203-236][env-given]), so it must run on the original argv.

### 3.3 (c) A synthetic environment variable per argument

1. **How.** ctl-core would set a process variable to the document value and call `arg.env(synthetic_name)` on the selected level, only where the real variable is unset.
2. **Precedence.** Exactly the wanted order with no extra code, because the document occupies clap's environment slot (2.1).
3. **Checks.** Value parser, possible values and relational checks are kept, because `EnvVariable` is explicit; `num_args` is lost (2.2).
4. **Positionals.** No tokenizing, so `--` and `last(true)` raise no placement problem.
5. **Lists.** `add_env` passes one string ([L1427-1436][env-one-string]), so a list without a delimiter cannot be expressed.
6. **Flags.** `"true"` and `"false"` parse through the bool action ([action.rs][bool-action]), but `false` still counts as present and conflicting ([clap#5591][issue-5591]).
7. **Cost.**
   1. `set_var` is `unsafe` in edition 2024, and the workspace denies `unsafe_code` ([Cargo.toml L90-91][unsafe-lint]).
      The variables are process-global, so a threaded caller can race.
   2. `env_under_flags` reads the live environment ([env.rs L246-259][env-live]), so a variable removed after capture would be unbound.
   3. Help could print the synthetic name ([help.rs][help-env]).
   4. The source cannot tell the document from a real variable ([clap#3572][issue-3572]).

### 3.4 (d) A second parse and `update_from_arg_matches`

1. `ArgMatches` has no method that inserts a value ([arg_matches.rs][matches-api]).
2. The derived update assigns a field whenever `contains_id` is true ([L693][derive-update], [L703][derive-contains], [L753][derive-assign]), and it is true for defaults.
   So the second parse overwrites the first one's values with its defaults, and removing defaults fails for flags (2.5).
3. Two parses validate separately, so a conflict between a flag and a document key is never seen.
4. Verdict: not viable.

### 3.5 (e) Clap's API and tracker

1. [clap#748][issue-748] is open since 2016 and waiting on design.
   The maintainer points to `value_source` ([2022-08-30][comment-748-a], [2022-09-28][comment-748-b]).
2. [Discussion 2763][discussion-2763] collects requirements, including naming the layer an error came from ([comment][comment-layer]).
   The maintainer's pattern keeps clap out of the merge ([comment][comment-config]); another note lists an argv merge or an `ArgMatches` merge ([comment][comment-merge]).
3. [clap#4793][issue-4793] asks for a plugin API for application value sources.
   [clap#5559][issue-5559], [clap#5099][issue-5099] and [clap#5796][issue-5796] were closed in favour of the discussion or a separate crate.
4. [PR 6211][pull-6211] (a falsy variable still conflicts) and [PR 6162][pull-6162] (a figment example) were closed unmerged.

## 4. Comparison of mechanisms

| Property | (a) default_value | (b) argv tokens | (c) synthetic env | (d) matches merge |
|:--|:--|:--|:--|:--|
| Order CLI > env > doc > default | broken by `default_value_if` | ctl-core drops keys | native | not achievable |
| Conflicts, requires, groups | lost | kept | kept | lost across layers |
| `num_args` | lost | kept | lost | kept per parse |
| Lists without a delimiter | supported | supported | not expressible | supported |
| Positionals, `--` | no issue | placement rules | no issue | placement rules |
| `ValueSource` | `DefaultValue` | `CommandLine` | `EnvVariable` | mixed |
| Unsafe or global state | no | no | yes | no |
| Fits `env_under_flags` | yes | yes, on original argv | no | n/a |

## 5. Prior art

| Crate (version) | Clap checks file values | Error location |
|:--|:--|:--|
| twelf 0.15.0 | no, `serde_json` ([merge][twelf-merge], [skip][twelf-skip], [JSON][twelf-json]) | none after the merge |
| clap_config 0.1.4 | no, serde ([order][clap-config-order], [keep][clap-config-keep]) | format crate message only |
| clap-serde-derive 1.0.0 | no, serde into `Option` ([lib][serde-derive-lib], [README][serde-derive-readme], [macro][serde-proc-lib]) | format crate message only |
| ortho_config 0.11.0 | no, figment ([order][ortho-order], [absent][ortho-absent]) | key path, no line ([figment][figment-error]) |
| confique 0.5.0 | no, serde ([example][confique-example]) | source name plus format error ([error][confique-error]) |
| clap_conf 0.2.0 | no, `FromStr` ([first][conf-first], [default][conf-default]) | none, falls back to default |
| clap-config-file 0.6.0 | no, config-rs ([presence][config-file-presence]) | warning, falls back ([warning][config-file-warning]) |
| argfile 1.0.0 | yes, file tokens are argv ([expand][argfile-expand]) | none, clap sees argv |

1. Several crates treat a clap value as absent when its `value_source` is `DefaultValue`.
2. Only argfile passes file values through all of clap's checks, because `@path` is replaced in place by tokens.
   The user chooses the position.
3. No crate examined runs file values through clap's value parsers and relational rules while keeping the file's line and column.

## 6. Recommendation

Use synthesized argv tokens (3.2), built from one probe parse.

1. **Why this one.**
   1. It is the only mechanism without `unsafe` code that keeps every relational check and `num_args`.
   2. ctl-core already runs the probe: `env_under_flags` parses a clone with `ignore_errors(true)` and walks the selected subcommand path ([env.rs L122-187][env-probe]).
   3. Every check is clap's own, so the document cannot drift from the command line's rules.
2. **Steps.**
   1. Probe the original argv.
      Record the subcommand path, the ids the command line set, and the variables `env_under_flags` keeps.
   2. Refuse a document key that names no long option or positional of the selected command, citing its line.
   3. Drop a key the command line set, a key whose `--x/--no-x` partner it set, and a key whose variable is kept.
   4. Encode the rest as in 3.2 point 4, and record each emitted id's key span.
      Refuse a `Set` option with `num_args` above one and no delimiter.
   5. Insert option tokens right after the selected subcommand's name token, found by probing growing prefixes of argv until the probe reaches it (inference, not tested).
      At the root the position is right after the binary name.
   6. Append positionals only under the narrow rule of 3.2 point 3, item 6.
   7. Run `env_under_flags` on the original argv, then parse the final argv once.
   8. On error, match `InvalidArg` to an argument's display form and place the error on the recorded key span.
3. **Lost or changed.**
   1. `ValueSource` reports `CommandLine`, so ctl-core must keep its own record.
   2. Clap stops at the first error; ctl-core's input loader reports every problem.
   3. A missing-required error has no document position, and a custom value parser that returns a context-free error cannot be placed.
   4. Placement needs `error-context`, which ctl-core currently leaves off.
4. **When (c) would be better.** If one `#[expect(unsafe_code)]` around `set_var` and the race risk is acceptable, (c) removes all placement rules.
   It still loses `num_args` and undelimited lists.

## 7. Not verified

1. Nothing was compiled or run; every behaviour above is read from source.
2. The prefix-probe method (6, step 5) is untested, including with `infer_subcommands` and `args_conflicts_with_subcommands`.
3. Which consumer CLIs enable `error-context` was not checked.
4. The text of clap's errors for synthesized tokens, and whether the usage line could leak document values, was not checked.
5. `num_args(0..=1)` and `allow_hyphen_values` with an attached value were not traced.
6. Prior-art links use each crate's recorded commit, except confique's `error.rs` and clap-serde-derive, which cite docs.rs pages that were not compared with the published files.
7. The ctl-core links point at commit 5e7a420c189073eb5cf97712e7f87d167645d92d.
8. Of the 21 comments in discussion 2763, only those cited were read in full.

[parse-order]: https://github.com/clap-rs/clap/blob/4a622b4340d5e1fffff60c0ecefdc6882f738159/clap_builder/src/parser/parser.rs#L49-L73
[add-env]: https://github.com/clap-rs/clap/blob/4a622b4340d5e1fffff60c0ecefdc6882f738159/clap_builder/src/parser/parser.rs#L1414-L1442
[add-defaults]: https://github.com/clap-rs/clap/blob/4a622b4340d5e1fffff60c0ecefdc6882f738159/clap_builder/src/parser/parser.rs#L1444-L1453
[default-if]: https://github.com/clap-rs/clap/blob/4a622b4340d5e1fffff60c0ecefdc6882f738159/clap_builder/src/parser/parser.rs#L1455-L1531
[default-if-head]: https://github.com/clap-rs/clap/blob/4a622b4340d5e1fffff60c0ecefdc6882f738159/clap_builder/src/parser/parser.rs#L1455-L1490
[value-source]: https://github.com/clap-rs/clap/blob/4a622b4340d5e1fffff60c0ecefdc6882f738159/clap_builder/src/parser/matches/value_source.rs#L1-L17
[num-args-check]: https://github.com/clap-rs/clap/blob/4a622b4340d5e1fffff60c0ecefdc6882f738159/clap_builder/src/parser/parser.rs#L1161-L1163
[delimiter]: https://github.com/clap-rs/clap/blob/4a622b4340d5e1fffff60c0ecefdc6882f738159/clap_builder/src/parser/parser.rs#L1178-L1198
[repeat-check]: https://github.com/clap-rs/clap/blob/4a622b4340d5e1fffff60c0ecefdc6882f738159/clap_builder/src/parser/parser.rs#L1208-L1218
[repeat-flag]: https://github.com/clap-rs/clap/blob/4a622b4340d5e1fffff60c0ecefdc6882f738159/clap_builder/src/parser/parser.rs#L1251-L1260
[repeat-action]: https://github.com/clap-rs/clap/blob/4a622b4340d5e1fffff60c0ecefdc6882f738159/clap_builder/src/parser/parser.rs#L1272-L1281
[overrides-with]: https://github.com/clap-rs/clap/blob/4a622b4340d5e1fffff60c0ecefdc6882f738159/clap_builder/src/parser/parser.rs#L1533-L1551
[validator-overrides]: https://github.com/clap-rs/clap/blob/4a622b4340d5e1fffff60c0ecefdc6882f738159/clap_builder/src/parser/validator.rs#L504-L521
[push-values]: https://github.com/clap-rs/clap/blob/4a622b4340d5e1fffff60c0ecefdc6882f738159/clap_builder/src/parser/parser.rs#L1093-L1117
[parse-ref]: https://github.com/clap-rs/clap/blob/4a622b4340d5e1fffff60c0ecefdc6882f738159/clap_builder/src/builder/value_parser.rs#L231-L244
[check-explicit]: https://github.com/clap-rs/clap/blob/4a622b4340d5e1fffff60c0ecefdc6882f738159/clap_builder/src/parser/matches/matched_arg.rs#L135-L151
[validator-start]: https://github.com/clap-rs/clap/blob/4a622b4340d5e1fffff60c0ecefdc6882f738159/clap_builder/src/parser/validator.rs#L24-L60
[validator-else]: https://github.com/clap-rs/clap/blob/4a622b4340d5e1fffff60c0ecefdc6882f738159/clap_builder/src/parser/validator.rs#L62-L82
[validator-required]: https://github.com/clap-rs/clap/blob/4a622b4340d5e1fffff60c0ecefdc6882f738159/clap_builder/src/parser/validator.rs#L196-L219
[validator-relations]: https://github.com/clap-rs/clap/blob/4a622b4340d5e1fffff60c0ecefdc6882f738159/clap_builder/src/parser/validator.rs#L221-L341
[predicate-doc]: https://github.com/clap-rs/clap/blob/4a622b4340d5e1fffff60c0ecefdc6882f738159/clap_builder/src/builder/arg_predicate.rs#L3-L6
[default-tests]: https://github.com/clap-rs/clap/blob/4a622b4340d5e1fffff60c0ecefdc6882f738159/tests/builder/default_vals.rs#L1097-L1128
[changelog-3-1]: https://github.com/clap-rs/clap/blob/4a622b4340d5e1fffff60c0ecefdc6882f738159/CHANGELOG.md#L1822-L1822
[issue-6031]: https://github.com/clap-rs/clap/issues/6031
[arg-env]: https://github.com/clap-rs/clap/blob/4a622b4340d5e1fffff60c0ecefdc6882f738159/clap_builder/src/builder/arg.rs#L2205-L2213
[env-field]: https://github.com/clap-rs/clap/blob/4a622b4340d5e1fffff60c0ecefdc6882f738159/clap_builder/src/builder/arg.rs#L87-L87
[implicit-defaults]: https://github.com/clap-rs/clap/blob/4a622b4340d5e1fffff60c0ecefdc6882f738159/clap_builder/src/builder/arg.rs#L4578-L4608
[action-defaults]: https://github.com/clap-rs/clap/blob/4a622b4340d5e1fffff60c0ecefdc6882f738159/clap_builder/src/builder/action.rs#L403-L415
[error-context]: https://github.com/clap-rs/clap/blob/4a622b4340d5e1fffff60c0ecefdc6882f738159/clap_builder/src/error/mod.rs#L184-L196
[context-conflict]: https://github.com/clap-rs/clap/blob/4a622b4340d5e1fffff60c0ecefdc6882f738159/clap_builder/src/error/mod.rs#L378-L404
[context-invalid]: https://github.com/clap-rs/clap/blob/4a622b4340d5e1fffff60c0ecefdc6882f738159/clap_builder/src/error/mod.rs#L454-L482
[context-parser]: https://github.com/clap-rs/clap/blob/4a622b4340d5e1fffff60c0ecefdc6882f738159/clap_builder/src/error/mod.rs#L668-L684
[workspace-clap]: https://github.com/victor-software-house/ctl-core/blob/5e7a420c189073eb5cf97712e7f87d167645d92d/Cargo.toml#L15-L20
[cli-feature]: https://github.com/victor-software-house/ctl-core/blob/5e7a420c189073eb5cf97712e7f87d167645d92d/ctl-core/Cargo.toml#L25-L32
[deny-usage]: https://github.com/victor-software-house/ctl-core/blob/5e7a420c189073eb5cf97712e7f87d167645d92d/deny.toml#L36-L36
[check-tree]: https://github.com/victor-software-house/ctl-core/blob/5e7a420c189073eb5cf97712e7f87d167645d92d/mise-tasks/check/tree.sh#L75-L81
[core-changelog]: https://github.com/victor-software-house/ctl-core/blob/5e7a420c189073eb5cf97712e7f87d167645d92d/CHANGELOG.md#L164-L169
[issue-5202]: https://github.com/clap-rs/clap/issues/5202
[default-if-append]: https://github.com/clap-rs/clap/blob/4a622b4340d5e1fffff60c0ecefdc6882f738159/clap_builder/src/builder/arg.rs#L3064-L3079
[issue-4918]: https://github.com/clap-rs/clap/issues/4918
[subcommand-match]: https://github.com/clap-rs/clap/blob/4a622b4340d5e1fffff60c0ecefdc6882f738159/clap_builder/src/parser/parser.rs#L113-L127
[after-dashes]: https://github.com/clap-rs/clap/blob/4a622b4340d5e1fffff60c0ecefdc6882f738159/clap_builder/src/parser/parser.rs#L131-L145
[trailing-var]: https://github.com/clap-rs/clap/blob/4a622b4340d5e1fffff60c0ecefdc6882f738159/clap_builder/src/parser/parser.rs#L405-L409
[last-positional]: https://github.com/clap-rs/clap/blob/4a622b4340d5e1fffff60c0ecefdc6882f738159/clap_builder/src/parser/parser.rs#L393-L404
[positional-fill]: https://github.com/clap-rs/clap/blob/4a622b4340d5e1fffff60c0ecefdc6882f738159/clap_builder/src/parser/parser.rs#L392-L437
[last-index]: https://github.com/clap-rs/clap/blob/4a622b4340d5e1fffff60c0ecefdc6882f738159/clap_builder/src/parser/parser.rs#L380-L387
[escape-test]: https://github.com/clap-rs/clap/blob/4a622b4340d5e1fffff60c0ecefdc6882f738159/clap_builder/src/parser/parser.rs#L113-L113
[attached-value]: https://github.com/clap-rs/clap/blob/4a622b4340d5e1fffff60c0ecefdc6882f738159/clap_builder/src/parser/parser.rs#L1061-L1074
[require-equals]: https://github.com/clap-rs/clap/blob/4a622b4340d5e1fffff60c0ecefdc6882f738159/clap_builder/src/parser/parser.rs#L1035-L1060
[unneeded-value]: https://github.com/clap-rs/clap/blob/4a622b4340d5e1fffff60c0ecefdc6882f738159/clap_builder/src/parser/parser.rs#L829-L851
[multi-value]: https://github.com/clap-rs/clap/blob/4a622b4340d5e1fffff60c0ecefdc6882f738159/clap_builder/src/parser/parser.rs#L291-L312
[env-probe]: https://github.com/victor-software-house/ctl-core/blob/5e7a420c189073eb5cf97712e7f87d167645d92d/ctl-core/src/flags/env.rs#L122-L187
[env-given]: https://github.com/victor-software-house/ctl-core/blob/5e7a420c189073eb5cf97712e7f87d167645d92d/ctl-core/src/flags/env.rs#L203-L236
[env-live]: https://github.com/victor-software-house/ctl-core/blob/5e7a420c189073eb5cf97712e7f87d167645d92d/ctl-core/src/flags/env.rs#L246-L259
[env-one-string]: https://github.com/clap-rs/clap/blob/4a622b4340d5e1fffff60c0ecefdc6882f738159/clap_builder/src/parser/parser.rs#L1427-L1436
[bool-action]: https://github.com/clap-rs/clap/blob/4a622b4340d5e1fffff60c0ecefdc6882f738159/clap_builder/src/builder/action.rs#L431-L443
[issue-5591]: https://github.com/clap-rs/clap/issues/5591
[unsafe-lint]: https://github.com/victor-software-house/ctl-core/blob/5e7a420c189073eb5cf97712e7f87d167645d92d/Cargo.toml#L90-L91
[help-env]: https://github.com/victor-software-house/ctl-core/blob/5e7a420c189073eb5cf97712e7f87d167645d92d/ctl-core/src/help.rs#L498-L498
[issue-3572]: https://github.com/clap-rs/clap/issues/3572
[matches-api]: https://github.com/clap-rs/clap/blob/4a622b4340d5e1fffff60c0ecefdc6882f738159/clap_builder/src/parser/matches/arg_matches.rs#L118-L1240
[derive-update]: https://github.com/clap-rs/clap/blob/4a622b4340d5e1fffff60c0ecefdc6882f738159/clap_derive/src/derives/args.rs#L693-L693
[derive-contains]: https://github.com/clap-rs/clap/blob/4a622b4340d5e1fffff60c0ecefdc6882f738159/clap_derive/src/derives/args.rs#L703-L703
[derive-assign]: https://github.com/clap-rs/clap/blob/4a622b4340d5e1fffff60c0ecefdc6882f738159/clap_derive/src/derives/args.rs#L753-L753
[issue-748]: https://github.com/clap-rs/clap/issues/748
[comment-748-a]: https://github.com/clap-rs/clap/issues/748#issuecomment-1232161125
[comment-748-b]: https://github.com/clap-rs/clap/issues/748#issuecomment-1261378965
[discussion-2763]: https://github.com/clap-rs/clap/discussions/2763
[comment-layer]: https://github.com/clap-rs/clap/discussions/2763#discussioncomment-1806349
[comment-config]: https://github.com/clap-rs/clap/discussions/2763#discussioncomment-1301698
[comment-merge]: https://github.com/clap-rs/clap/discussions/2763#discussioncomment-1374967
[issue-4793]: https://github.com/clap-rs/clap/issues/4793
[issue-5559]: https://github.com/clap-rs/clap/issues/5559
[issue-5099]: https://github.com/clap-rs/clap/issues/5099
[issue-5796]: https://github.com/clap-rs/clap/issues/5796
[pull-6211]: https://github.com/clap-rs/clap/pull/6211
[pull-6162]: https://github.com/clap-rs/clap/pull/6162
[twelf-merge]: https://github.com/bnjjj/twelf/blob/cdd3b6720a1e5e6c8a28485e67d1a3de1a641c09/config-derive/src/lib.rs#L157-L177
[twelf-skip]: https://github.com/bnjjj/twelf/blob/cdd3b6720a1e5e6c8a28485e67d1a3de1a641c09/config-derive/src/lib.rs#L221-L256
[twelf-json]: https://github.com/bnjjj/twelf/blob/cdd3b6720a1e5e6c8a28485e67d1a3de1a641c09/config-derive/src/lib.rs#L304-L307
[clap-config-order]: https://github.com/gibfahn/clap_config/blob/dd5700b31f721a7f2e2f378c60bdda6c93783061/clap_config/src/lib.rs#L232-L238
[clap-config-keep]: https://github.com/gibfahn/clap_config/blob/dd5700b31f721a7f2e2f378c60bdda6c93783061/clap_config/src/lib.rs#L318-L333
[serde-derive-lib]: https://docs.rs/crate/clap-serde-derive/1.0.0/source/src/lib.rs
[serde-derive-readme]: https://docs.rs/crate/clap-serde-derive/1.0.0/source/README.md
[serde-proc-lib]: https://docs.rs/crate/clap-serde-proc/0.3.0/source/src/lib.rs
[ortho-order]: https://github.com/leynos/ortho-config/blob/239f53cd5754f9e00a2b294ed4f2832147ba418d/ortho_config/src/lib.rs#L213-L219
[ortho-absent]: https://github.com/leynos/ortho-config/blob/239f53cd5754f9e00a2b294ed4f2832147ba418d/ortho_config/src/merge.rs#L134-L140
[figment-error]: https://github.com/SergioBenitez/Figment/blob/dae213f0f31d764a0c00aeb39f145222050bc657/src/error.rs#L89-L97
[confique-example]: https://github.com/LukasKalbertodt/confique/blob/39179571813e6aa42b010b82b2bee433b5f6446b/examples/clap.rs#L18-L48
[confique-error]: https://docs.rs/crate/confique/0.5.0/source/src/error.rs
[conf-first]: https://github.com/storyfeet/clap_conf/blob/3e7f197c6cdc5c7f0bd7d11a98a32ec7d7f9e71e/src/grabber.rs#L24-L40
[conf-default]: https://github.com/storyfeet/clap_conf/blob/3e7f197c6cdc5c7f0bd7d11a98a32ec7d7f9e71e/src/grabber.rs#L69-L72
[config-file-presence]: https://github.com/bodo-run/clap-config-file/blob/5a761f8f0ceda088f78a24b41fc98b000afb14b4/src/lib.rs#L280-L296
[config-file-warning]: https://github.com/bodo-run/clap-config-file/blob/5a761f8f0ceda088f78a24b41fc98b000afb14b4/src/lib.rs#L219-L235
[argfile-expand]: https://github.com/rust-cli/argfile/blob/16aaf789b32edef9490f52f95f9ed51bd21064f7/src/lib.rs#L83-L111
