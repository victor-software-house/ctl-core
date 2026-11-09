# Design

## Context

ctl-core's `usage` feature renders a clap command as Usage KDL, the format
mise reads to mount a CLI as a task. Today [`usage::spec`][usage-rs] sets
the command's bin name and returns `Spec::from(&command).to_string()` from
usage-lib. ctl-core's lock holds usage-lib 6.0.0. The consumers resolve
6.1.1.

Versions read for this plan, each the latest release on 2026-11-08:

| Crate or tool | Version | Read for |
|:--|:--|:--|
| [usage-lib][usage-730] | 7.3.0 | the reference output, and what its clap bridge reads |
| [KDL spec][kdl-v2] | 2.0.0 | quoting, raw strings, newlines, disallowed code points |
| [kdl][kdl-671] | 6.7.1 | the test-only KDL v2 parser |
| [clap][clap-467] | 4.6.7 | the default feature set |
| [serde-saphyr][saphyr-211] | 2.1.1 | which feature pulls annotate-snippets, and its error renderer |
| [annotate-snippets][annotate-01217] | 0.12.17 | what the loader would stop linking |
| [mise][mise-release] | 2026.11.4 | reading the spec, help, and completion |

## Goals and non-goals

Goals:

1. Write the Usage KDL with no third-party KDL crate.
2. Remove usage-lib, kdl, and winnow 0.7 from ctl-core's graph.
3. Keep clap's default features off in every ctl-core feature set.
4. Stop linking annotate-snippets, if no problem report changes.
5. Hold the result with size and dependency gates.

Non-goals:

1. No change to the public API of the `usage` module or of `App`.
2. No parser for Usage KDL. ctl-core only writes it.
3. No change to the documents `Surface` renders, beyond the spec text
   they embed.
4. No upstream pull request to usage-lib or serde-saphyr without the
   maintainer's approval.

## Decisions

### 1. ctl-core writes the KDL itself

A private writer under `usage` walks the clap `Command` and writes KDL
text directly. It holds no document model and no parser.

Alternatives considered:

1. **Stay on usage-lib 6.x.** It lost on size: 269,548 bytes in qctl
   ([decision 16][d16]), and clap's defaults on.
2. **Move to usage-lib 7.3.0 with default features off.** 7.3.0 no longer
   depends on the kdl crate. It carries a trimmed copy of kdl-rs 6.7.1
   instead ([NOTICE.md][usage-notice]). It uses winnow 1.0.4, the version
   toml already pulls in. So it removes kdl and winnow 0.7.
   - It lost because it still declares clap without
     `default-features = false` ([manifest][usage-730-toml]). clap's
     defaults stay on.
   - It also still links a KDL parser and the whole spec model, which
     ctl-core never reads.
3. **Ask usage-lib to declare clap with `default-features = false`.** That
   fixes the clap defaults only. The parser and model stay. It remains a
   reasonable upstream proposal, made separately with approval.

The maintainer chose the writer in ctl-core.

### 2. usage-lib 7.3.0 is the reference output

The new writer matches what usage-lib 7.3.0 writes, byte for byte. Before
the writer exists, a corpus test captures 7.3.0's output in a real run,
and the capture is committed as the baseline.

A probe on the measurement host compared today's output with 7.3.0's on
2026-11-08:

1. Three consumers were built against ctl-core [d05500f][d05500f]: qctl
   at [cd06a58][qctl-head], verctl at [d087741][verctl-head], and forkctl
   at [7b98707][forkctl-head]. Each was built with usage-lib 6.1.1, then
   with 7.3.0. Each pair printed byte-identical specs: 259, 131, and 289
   lines.
2. Two commands were rendered with 6.0.0 and with 7.3.0. One sets every
   clap attribute 7.3.0's bridge reads. The other holds 35 hostile strings
   in help text and default values.

The outputs differ in one case only: a multiline value that contains
`""""#`.

1. 6.0.0 opens the raw string with one `#`. The `"""#` inside the value
   then closes it early, and the document fails to parse.
2. 7.3.0 counts `"""` at every offset and opens with `##`.
3. The cause is `raw_multiline_hash_count` in `spec/helpers.rs`.
   [6.0.0][helpers-600] scans with `match_indices`, which skips
   overlapping matches. [7.3.0][helpers-730] scans every byte offset. The
   changelog between the two releases does not name the fix.

7.3.0's count is correct under KDL v2, so the writer follows it. The
changeset lists it as a fix against today's output.

### 3. The writer departs from 7.3.0 only where KDL v2 requires it

The same probe parsed each output back with kdl 6.7.1. In the cases
below, 7.3.0's output does not parse, or parses to a different value.
6.0.0 behaves the same in every one.

| Field | Value holds | 7.3.0 writes | KDL v2 rule | Result |
|:--|:--|:--|:--|:--|
| help | U+2028 or U+2029, no LF | a quoted string, character literal | both are Newline; a single-line string holds none | parse fails |
| help | U+2028 and LF | a raw multiline string | a literal Newline reads back as LF | U+2028 becomes LF |
| help | U+200E, U+202E, U+2066, or U+FEFF | the character literal, quoted or raw | disallowed literal code points; a raw string cannot escape them | parse fails |
| help | a whitespace-only line, and LF | a raw multiline string | a whitespace-only line reads back empty | the spaces are lost |
| default | U+2028, U+2029, or U+0085 | a quoted string, character literal | all three are Newline | parse fails |
| default | U+200E, U+202E, U+2066, U+FEFF, or ESC | a quoted string, character literal | disallowed literal code points | parse fails |

In each case the writer uses the escaped quoted form. 7.3.0 already writes
that form for a value with a control character. It is a quoted string
with `\n`, `\t`, `\r`, `\"`, and `\\`, and `\u{…}` for every other Newline
or disallowed code point. Each departure has a test that parses the
output with kdl 6.7.1 and compares the value.

7.3.0 strips terminal escape sequences and DEL from help text before
writing it: help `a<ESC>[1mb` is written as `ab`. The writer keeps that.
Default values are not stripped, so they need the escapes.

Alternative considered: reproduce 7.3.0 exactly, invalid cases included.
It lost because mise refuses a spec that does not parse. And a spec that
parses to other text shows the user something the CLI never said.

### 4. The writer covers every attribute 7.3.0 reads, not only today's

The writer maps every clap getter 7.3.0's bridge reads:

1. command metadata, and the settings 7.3.0 writes as properties;
2. flags, with their aliases, values, actions, defaults, environment
   variables, and visibility;
3. positional arguments, possible values, value hints, and groups.

The corpus includes the every-attribute command from the probe, so each
mapping has a byte comparison.

Alternative considered: map only what the consumers use today. It lost
because a consumer that later sets another clap attribute would get a
spec that silently drops it.

### 5. A KDL v2 parser runs in tests only

The tests parse the writer's output with [kdl 6.7.1][kdl-671], the KDL
project's own KDL v2 parser. It is a dev-dependency, pinned with `=`. kdl
pulls winnow 0.7 into the dev graph.

1. `deny.toml` bans usage-lib outright.
2. It bans kdl and winnow below 1.0 with `wrappers`. Only ctl-core may
   depend on kdl, and only kdl on winnow 0.7.
3. `check:tree` fails when either appears in ctl-core's normal
   dependencies under any feature set. The dev-dependency is the one path
   left.

Alternative considered: set `exclude-dev = true` in `deny.toml`'s
`[graph]`. It lost because it would also drop every other dev-dependency
from the licence, source, and advisory checks.

### 6. Nothing turns clap's defaults back on

At [d05500f][d05500f], `cargo tree -e features -i clap_builder
--all-features` shows one crate enabling clap's `default` feature:
usage-lib, through its `clap` feature. clap 4.6.7's defaults are `std`,
`color`, `help`, `usage`, `error-context`, and `suggestions`. Once
usage-lib is gone the same command must show no `default`, and the
`features` spec gains a scenario for `app`, `usage`, and `input` together.
If something else still enables it, the work cuts it or records why it
cannot.

### 7. The YAML loader stops reaching annotate-snippets, or the work stops

serde-saphyr 2.1.1 ties annotate-snippets to its `deserialize` feature
([manifest][saphyr-211-toml]), and the loader needs `deserialize`, so no
feature choice removes it. Link-time optimization can drop it only when
no reachable code calls the snippet renderer.

The loader calls `Error::render_with_formatter`
([input.rs][input-render], [again][input-render-alias]). That function
goes through serde-saphyr's `fmt_error_rendered`, whose `WithSnippet` arm
calls the snippet renderer, so the renderer stays reachable even with
`with_snippet: false` ([error.rs][saphyr-error]). `Display` on the error
takes the same path.

Users see no snippet today. The loader parses with `with_snippet: false`
and passes `error.without_snippet()` to the message
([input.rs][input-options]), so every report is one line: file, line,
column, and serde-saphyr's text. A real run on 2026-11-08 (-03:00) showed
it. qctl 0.6.0 is built on ctl-core 0.6.24, whose loader has the same
two calls ([0.6.24 input.rs][input-options-0624]). It checked three
malformed ledgers:

| Ledger | Report after the path |
|:--|:--|
| `prefix` given twice | `3:1: duplicate mapping key: prefix, set DuplicateKeyPolicy in Options if acceptable` |
| A block item inside `[` | `4:3: "-" is only valid inside a block` |
| `queue: [` left open | `3:8: unclosed bracket '['`, the same text under `--format json` |

None held a source line or a caret. So the cut removes code that renders
nothing a user sees.

The candidate cut builds each message from `MessageFormatter::format_message`
directly, with the same control-character handling the renderer applies.
No ctl-core path then reaches the renderer. The proof has two parts:

1. The existing input tests, and snapshots of real-run problem reports
   taken before the change, pass unchanged. Every report keeps its file,
   line, and message text.
2. qctl's release build has no annotate-snippets symbol.

If either fails, the work stops and reports the finding. It does not trade
a message for bytes. An upstream proposal to split snippets from
`deserialize` would then be the next step, made only with approval.

### 8. serde-saphyr keeps `deserialize` and `comments`

`deserialize` is the parser. `comments` gates the comment limits in
serde-saphyr's `Budget` ([budget.rs][saphyr-budget]), which
`.ctl/input.yaml` exposes as `limits:` keys, and which the loader names in
a limit error. Dropping it would change what the repository limits file
accepts. The gate task records that evidence. The lock moves from 2.0.0 to
2.1.1, the latest, if the input tests pass unchanged.

### 9. The size gates cover the cuts

The `size` example gains code behind `usage` and `input`, so link-time
optimization keeps the code those tiers add. `[workspace.metadata.size]`
gains a `usage` tier (`app,usage`) and an `input` tier (`app,input`).
Every tier's budget is set to the `.text` CI measures on linux-x64 for the
commit that lands the cuts. A budget is lowered, never raised, and each
tier carries its reason.

### 10. One stack, one pull request per step

1. This plan.
2. The corpus test and its baseline, captured with 7.3.0.
3. The writer, passing the baseline unchanged except for decision 3.
4. usage-lib, kdl, and winnow 0.7 removed and banned, and the clap check.
5. The annotate-snippets cut, or the report that stops it.
6. The gates, docs, and changeset.
7. The consumer measurement in decision 16.

## Risks

1. **mise reads a spec differently from usage-lib's parser.** The writer
   and baseline steps each run mise 2026.11.4 against a mounted toy task:
   it must parse the spec, print `--help`, and complete arguments.
2. **A clap attribute the corpus misses.** Decision 4 maps every getter
   7.3.0 reads, and the every-attribute command pins each one.
3. **The annotate-snippets cut changes a message.** Decision 7 stops the
   work instead.

[usage-rs]: https://github.com/victor-software-house/ctl-core/blob/d05500fa0a1689038dc16b58d5c1fa2227c68b78/ctl-core/src/usage.rs#L26-L32
[input-render]: https://github.com/victor-software-house/ctl-core/blob/d05500fa0a1689038dc16b58d5c1fa2227c68b78/ctl-core/src/input.rs#L339
[input-render-alias]: https://github.com/victor-software-house/ctl-core/blob/d05500fa0a1689038dc16b58d5c1fa2227c68b78/ctl-core/src/input.rs#L351
[input-options]: https://github.com/victor-software-house/ctl-core/blob/d05500fa0a1689038dc16b58d5c1fa2227c68b78/ctl-core/src/input.rs#L294-L311
[input-options-0624]: https://github.com/victor-software-house/ctl-core/blob/v0.6.24/src/input.rs#L262-L279
[d05500f]: https://github.com/victor-software-house/ctl-core/commit/d05500fa0a1689038dc16b58d5c1fa2227c68b78
[d16]: ../terminal-render-engine/design.md#16-feature-tiers-and-their-measured-sizes
[qctl-head]: https://github.com/victor-software-house/qctl/commit/cd06a58610646d6ebd9e2ec3ce2bdc1a7a59e4b7
[verctl-head]: https://github.com/victor-software-house/verctl/commit/d087741fd60190757141761eb8410a1a73875c53
[forkctl-head]: https://github.com/victor-software-house/forkctl/commit/7b987070f9069842c2e9afdd17fca28353b81bc5
[usage-730]: https://docs.rs/crate/usage-lib/7.3.0
[usage-730-toml]: https://docs.rs/crate/usage-lib/7.3.0/source/Cargo.toml
[usage-notice]: https://docs.rs/crate/usage-lib/7.3.0/source/NOTICE.md
[helpers-600]: https://docs.rs/crate/usage-lib/6.0.0/source/src/spec/helpers.rs
[helpers-730]: https://docs.rs/crate/usage-lib/7.3.0/source/src/spec/helpers.rs
[kdl-v2]: https://github.com/kdl-org/kdl/blob/2.0.0/SPEC.md
[kdl-671]: https://docs.rs/crate/kdl/6.7.1
[clap-467]: https://docs.rs/crate/clap/4.6.7/source/Cargo.toml
[saphyr-211]: https://docs.rs/crate/serde-saphyr/2.1.1
[saphyr-211-toml]: https://docs.rs/crate/serde-saphyr/2.1.1/source/Cargo.toml
[saphyr-error]: https://docs.rs/crate/serde-saphyr/2.1.1/source/src/de/error.rs
[saphyr-budget]: https://docs.rs/crate/serde-saphyr/2.1.1/source/src/de/budget.rs
[annotate-01217]: https://docs.rs/crate/annotate-snippets/0.12.17
[mise-release]: https://github.com/jdx/mise/releases/tag/v2026.11.4
