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

### 3. The writer departs from 7.3.0 only where KDL v2 or mise requires it

The same probe parsed each output back with kdl 6.7.1. In the cases
below, 7.3.0's output fails that parse or reads back as a different
value. In the DEL row, it breaks a KDL v2 rule both parsers miss. 6.0.0
behaves the same in every one.

| Field | Value holds | 7.3.0 writes | KDL v2 rule | Result |
|:--|:--|:--|:--|:--|
| help | U+2028 or U+2029, no LF | a quoted string, character literal | both are Newline; a single-line string holds none | parse fails |
| help | U+2028 and LF | a raw multiline string | a literal Newline reads back as LF | U+2028 becomes LF |
| help | U+200E, U+202E, U+2066, or U+FEFF | the character literal, quoted or raw | disallowed literal code points; a raw string cannot escape them | parse fails |
| help | a whitespace-only line, and LF | a raw multiline string | a whitespace-only line reads back empty | the spaces are lost |
| default | U+2028, U+2029, or U+0085 | a quoted string, character literal | all three are Newline | parse fails |
| default | U+200E, U+202E, U+2066, U+FEFF, or ESC | a quoted string, character literal | disallowed literal code points | parse fails |
| default | U+007F (DEL) | a bare identifier, character literal | a disallowed literal code point | parses only because both parsers miss it |
| default, choice | `-.5` or `+.5` | a bare identifier | a sign, then `.` and a digit, starts a number, and no identifier may start that way | parse fails |

In each case the writer uses the escaped quoted form. 7.3.0 already writes
that form for a value with a control character. It is a quoted string
with `\n`, `\t`, `\r`, `\"`, and `\\`, and `\u{…}` for every other Newline
or disallowed code point. Each departure has a test that parses the
output with kdl 6.7.1 and compares the value.

The DEL row is the one case a parse cannot catch. The [KDL v2 spec][kdl-v2]
lists U+007F among the disallowed literal code points.

kdl 6.7.1's [`is_disallowed_unicode`][kdl-671-disallowed] and usage-lib
7.3.0's [copy][usage-730-disallowed] leave it out of the match, although
their doc comments list it. So 7.3.0 judges `a<DEL>b` a plain identifier
and writes it bare, and both parsers read it back.

The omission is an upstream defect in kdl 6.7.1, which usage-lib copied.
It is not reported upstream: a report needs the maintainer's approval.

- The writer writes `default="a\u{7f}b"`.
- Beside the parse, its tests check every output for two things:
  - a literal code point from the whole disallowed set;
  - a Newline inside a single-line string.

  A later missed character then fails a test without a new row.
- Every other bare default in the corpus is a valid identifier: `-x` and
  `--x` are signed identifiers.
- The corpus also holds one default and possible value per case of the
  identifier grammar:
  - number-like words: `1`, `-1`, `+1`, `.5`, `-.5`, `+.5`, `1e3`,
    `0x1F`, and `1_000`;
  - the keywords and their `#` forms;
  - the empty string, whitespace, `=`, brackets, `/`, `\`, `;`, and
    quotes.

  Each reads back as a string with the same text. 7.3.0 quotes all of
  them except `-.5` and `+.5`, the row above, and `'`, which is a valid
  identifier.

7.3.0 strips terminal escape sequences and DEL from help text before
writing it: help `a<ESC>[1mb` is written as `ab`. The writer keeps that,
by its own code. Default values are not stripped, so they need the
escapes.

Two clap features decide those bytes, and only usage-lib turns them on,
through clap's defaults:

1. clap's `Display` strips help only with clap's `color` feature on. The
   writer strips help itself, by the rules of anstream 1.0.0's
   `strip_str`, which clap calls with `color` on. Text clap already
   stripped comes through unchanged. So the spec holds the same bytes
   whether or not another crate turns `color` on.
2. clap's `render_usage` returns an empty string without clap's `usage`
   feature. ctl-core's `usage` feature turns `clap/usage` on itself.

`test:usage` runs the spec tests with only the `usage` feature, so the
plain tier is held to the same snapshots.

Alternative considered: reproduce 7.3.0 exactly, invalid cases included.
It lost because mise refuses a spec that does not parse. And a spec that
parses to other text shows the user something the CLI never said.

#### Value names that read as usage syntax

mise reads an `arg` node's name as usage syntax, through usage-lib's
[`SpecArg::from`][usage-730-arg-from]. mise 2026.11.4 embeds usage-lib
7.3.0 ([`Cargo.lock`][mise-lock]). clap gives a value name no syntax, so
a name such as `x...` means one value named `x...`. The parser reads
three things into the name:

1. a label that splits into two or more `<…>` or `[…]` words reads as
   that many values ([`fixed_placeholders`][usage-730-placeholders]);
2. a name that ends in `...` or `…` reads as any number of values;
3. a name that starts with `--` and a space reads as a value that only
   follows `--`.

The parser applies the node's properties after the name, so a property
can state clap's meaning again. A `value_names` child replaces the name
and its split.

The probe ran under mise 2026.11.4 on macOS, with one file task per
form. Each task mounted a spec with that value name on a flag or on a
positional. A positional with the choices `p1` and `p2` came next.

zsh completion (`mise __complete_word__ --shell zsh --line`) then shows
how many values mise reads. When it offers `p1 p2` after one value, mise
reads one, as clap does.

| Where | Value name | 7.3.0 writes | mise reads | The writer writes | mise reads |
|:--|:--|:--|:--|:--|:--|
| flag | `-- X` | `"<-- X>"` | one value | the same | one value |
| flag | `[x]` | `"[x]" required=#false` | one value | `"<[x]>"` | one value |
| flag | `<A> <B>` | `"<<A> <B>> <<A> <B>>" var=#true var_min=2 var_max=2` | nothing: mise rejects the whole spec, and completion offers only files | `"<<A> <B>>" var=#false` and `value_names "<A> <B>"` | one value |
| flag | `x...` or `x…` | `<x>… var=#true` | any number | `<x...> var=#false` | one value |
| positional | `[x]` | `"<[x]>"` | one value | the same | one value |
| positional | `-- X` | `"<-- X>"` | one value, only after `--` | `"<-- X>" double_dash=optional` | one value, before or after `--` |
| positional | `<A> <B>` | `"<<A> <B>>"` | two values | `"<<A> <B>>" var=#false` and `value_names "<A> <B>"` | one value |
| positional | `x...` or `x…` | `<x...>` | any number | `<x...> var=#false` | one value |

So the writer keeps clap's value name as written and adds a property
only where mise would read more into it:

- `var=#false` for a name that splits into placeholders or ends in
  `...` or `…`;
- a `value_names` child holding the whole name, for a name that splits;
- `double_dash=optional` for a positional's name that starts with `--`
  and a space.
  A flag's value keeps the prefix with no property: mise reads it as one
  value either way.

An optional positional is wrapped as `[…]` before mise reads it, and the
parser strips that before the split. So `[<A> <B>]` reads as one value
and gains nothing, while `[[A] [B]]` splits and gains both.

The `value_names` snapshot holds one case per form, for a flag and a
positional. Its first capture is 7.3.0's output, and the writer's
commit shows the change.

7.3.0 has three defects here, none reported upstream; a report needs the
maintainer's approval:

1. A flag's value name that splits into placeholders becomes two values
   under a repeated label, and mise rejects the whole spec.
2. A flag's value name that ends in `...` or `…` becomes variadic.
3. A positional's value name gets no property, so mise misreads it.

Alternatives considered:

1. Write 7.3.0's bytes. It lost because mise rejects a spec with the
   first defect, and the whole command loses completion.
2. Write the literal name alone. It lost because mise reads the forms
   above as syntax, and no quoting in the name prevents that.

### 4. The writer covers every attribute 7.3.0 reads, not only today's

The writer maps every clap getter 7.3.0's bridge reads:

1. command metadata, and the settings 7.3.0 writes as properties;
2. flags, with their aliases, values, actions, defaults, and visibility.
   Environment variables are not read, as in 7.3.0. Writing `env=` would
   be a separate decision;
3. positional arguments, possible values, value hints, and groups.

The corpus includes the every-attribute command from the probe, so each
mapping has a byte comparison.

Alternative considered: map only what the consumers use today. It lost
because a consumer that later sets another clap attribute would get a
spec that silently drops it.

### 5. A KDL v2 parser runs in tests only

The tests parse the writer's output with [kdl 6.7.1][kdl-671], the KDL
project's own KDL v2 parser. It is a dev-dependency, pinned with `=`, and
built without default features. kdl pulls winnow 0.7, miette, and
unicode-width 0.1 into the dev graph.

1. `deny.toml` bans usage-lib outright.
2. It bans kdl and winnow below 1.0 with `wrappers`. Only ctl-core may
   depend on kdl, and only kdl on winnow 0.7.
3. Neither second version needs a `skip`. cargo-deny 0.20.2 leaves a crate
   reached only through dev-dependencies out of two checks:
   - duplicates, by [`multiple-versions-include-dev`][deny-bans-0202],
     default false;
   - licences, by [`include-dev`][deny-licenses-0202], default false.

   The advisory, ban, and source checks still cover it. A
   dev-dependency never ships, so that default is the policy here.
4. `syn@2` is the one `skip`: garde_derive 0.23.0, the latest release,
   requires syn 2, while serde, clap, thiserror, and schemars use syn 3.
5. The templates repository renders `deny.toml`. Each entry is an answer
   in `.copier-answers.yml` (`deny_bans`, `deny_skip`), applied with
   `copier update`, never a hand edit.
6. `check:tree` fails when usage-lib, kdl, or winnow below 1.0 appears in
   ctl-core's normal dependencies with every feature. The dev-dependency
   is the one path left for kdl and winnow 0.7.

Alternative considered: set `exclude-dev = true` in `deny.toml`'s
`[graph]`. It lost because it would also drop every other dev-dependency
from the advisory, ban, and source checks.

Alternative considered: a `skip-tree` entry for kdl. It lost because the
duplicate check never sees kdl's tree, and the template renders no
`skip-tree`.

### 6. Nothing turns clap's defaults back on

At [d05500f][d05500f], `cargo tree -e features -i clap_builder
--all-features` shows one crate enabling clap's `default` feature:
usage-lib, through its `clap` feature. clap 4.6.7's defaults are `std`,
`color`, `help`, `usage`, `error-context`, and `suggestions`. Once
usage-lib is gone the same command must show no `default`, and the
`features` spec gains a scenario for `app`, `usage`, and `input` together.
If something else still enables it, the work cuts it or records why it
cannot.

With usage-lib removed, nothing else enables `default`. One of its
features was in use: `usage::spec` writes each command's usage line from
clap's `render_usage`, which needs clap's `usage` feature. The writer's
change already turned `clap/usage` on in ctl-core's `usage` feature
([decision 3](#3-the-writer-departs-from-730-only-where-kdl-v2-or-mise-requires-it)),
so the removal changes no spec byte.

A second one was in use too. Without clap's `help` feature, clap turns
off the `-h`/`--help` flag and the `help` subcommand, unless a command
overrides its help ([clap_builder 4.6.6, `Command::_build_self`][clap-help-gate]).
ctl-core owns that flag and renders the help itself.

usage-lib had hidden the gap: the tests run with every feature, so
usage-lib's defaults gave clap `help` there. CI's first run without
usage-lib failed 8 tests on it.

Turning on `clap/help` fixed them and grew every size tier by 37 to 41 KB,
8 to 12 percent. That is clap's help writer, which no ctl-core binary runs.
So ctl-core declares both itself, as clap words them:

- `parser::apply_defaults` and `help::document` add an `ArgAction::Help`
  flag, `-h`/`--help`, which stops the parse with `DisplayHelp` without the
  feature ([`ArgAction::Help`][clap-help-action]).
- They add a `help` subcommand holding a copy of each subcommand name, as
  clap's own does when a command is built. `App` answers it before the
  derive's `from_arg_matches` sees it.

A plain `Cli::parse()` outside ctl-core's entry point gets clap's minimal
output for `--help` and no `help` subcommand. `test:cli` runs the parser
tests with `cli` alone, so no other crate can hide a gap again.

`check:tree` holds these results:

1. With `cli`, clap's `help` feature is off.
2. With `usage`, clap's `usage` feature is on.
3. With `app`, `usage`, and `input`, clap's `default`, `color`, `help`,
   `error-context`, and `suggestions` are off.

`check:features` compiles `app,usage,input` warning-free.

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
it.

qctl 0.6.0 is built on ctl-core 0.6.24, whose loader has the same
two calls ([0.6.24 input.rs][input-options-0624]). The run checked three
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
[clap-help-gate]: https://docs.rs/crate/clap_builder/4.6.6/source/src/builder/command.rs#4415
[clap-help-action]: https://docs.rs/crate/clap_builder/4.6.6/source/src/parser/parser.rs#1302
[d16]: ../terminal-render-engine/design.md#16-feature-tiers-and-their-measured-sizes
[qctl-head]: https://github.com/victor-software-house/qctl/commit/cd06a58610646d6ebd9e2ec3ce2bdc1a7a59e4b7
[verctl-head]: https://github.com/victor-software-house/verctl/commit/d087741fd60190757141761eb8410a1a73875c53
[forkctl-head]: https://github.com/victor-software-house/forkctl/commit/7b987070f9069842c2e9afdd17fca28353b81bc5
[usage-730]: https://docs.rs/crate/usage-lib/7.3.0
[usage-730-disallowed]: https://docs.rs/crate/usage-lib/7.3.0/source/src/kdl/v2_parser.rs#1239
[usage-730-arg-from]: https://docs.rs/crate/usage-lib/7.3.0/source/src/spec/arg.rs#804
[usage-730-placeholders]: https://docs.rs/crate/usage-lib/7.3.0/source/src/spec/arg.rs#914
[mise-lock]: https://github.com/jdx/mise/blob/4dc53ec645558e448f7f36a7b1f02ccbd69446d1/Cargo.lock
[usage-730-toml]: https://docs.rs/crate/usage-lib/7.3.0/source/Cargo.toml
[usage-notice]: https://docs.rs/crate/usage-lib/7.3.0/source/NOTICE.md
[helpers-600]: https://docs.rs/crate/usage-lib/6.0.0/source/src/spec/helpers.rs
[helpers-730]: https://docs.rs/crate/usage-lib/7.3.0/source/src/spec/helpers.rs
[kdl-v2]: https://github.com/kdl-org/kdl/blob/2.0.0/SPEC.md
[kdl-671]: https://docs.rs/crate/kdl/6.7.1
[deny-bans-0202]: https://github.com/EmbarkStudios/cargo-deny/blob/bca0dde53651ee946720e4540b5ce2610bec8f06/docs/src/checks/bans/cfg.md#the-multiple-versions-include-dev-field-optional
[deny-licenses-0202]: https://github.com/EmbarkStudios/cargo-deny/blob/bca0dde53651ee946720e4540b5ce2610bec8f06/docs/src/checks/licenses/cfg.md#the-include-dev-field-optional
[kdl-671-disallowed]: https://docs.rs/crate/kdl/6.7.1/source/src/v2_parser.rs#1571
[clap-467]: https://docs.rs/crate/clap/4.6.7/source/Cargo.toml
[saphyr-211]: https://docs.rs/crate/serde-saphyr/2.1.1
[saphyr-211-toml]: https://docs.rs/crate/serde-saphyr/2.1.1/source/Cargo.toml
[saphyr-error]: https://docs.rs/crate/serde-saphyr/2.1.1/source/src/de/error.rs
[saphyr-budget]: https://docs.rs/crate/serde-saphyr/2.1.1/source/src/de/budget.rs
[annotate-01217]: https://docs.rs/crate/annotate-snippets/0.12.17
[mise-release]: https://github.com/jdx/mise/releases/tag/v2026.11.4
