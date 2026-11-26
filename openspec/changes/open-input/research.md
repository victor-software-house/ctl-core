# Research

What the design rests on, read from source on 2026-11-26.
Crate citations name the crate version and file; ctl-core citations are pinned to [0d8e5a4][base].

## 1. Routes to an open set of formats

Typed loading, `Input::parse::<T>`, is generic, so a format behind `&dyn` cannot call it directly.

| Route | Result |
|:--|:--|
| erased-serde 0.4.11 | `erase_de` is `serde::de::Error::custom(e)` (`src/error.rs:16`), so serde-saphyr's located error reaches ctl-core as a string. Rejected (operator, 2026-11-26). |
| A tree built from granit-parser 2.1.0 events in ctl-input | Feasible, but rejected: the fork owns YAML (operator, 2026-11-26). Details follow the table. |
| A tree built by the fork's own located reader | Chosen. A reader that emits nodes instead of serde calls reuses the fork's pipeline. Details follow the table. |

On the granit-parser route, events each carry a byte span, style, tag and anchor ([granit-parser][granit] `src/parser.rs:113-198`).
That route copies the fork's budget, duplicate-key identity and scalar rules into ctl-input.

The fork, [vsh-serde-saphyr v0.0.4][fork], already runs granit's events through its budget (`src/de/budget.rs:587-668`), duplicate keys (`src/de/key_nodes.rs:42-144`) and merge keys (`:603-622`).
The fork's located reader reuses that pipeline.

## 2. YAML scalar rules to keep

The fork resolves a plain scalar in `deserialize_any` in this order: tag, null, quoted or string tag, YAML 1.2 bool, integer, float, string (`src/de/deserializer.rs:931-1101`).
Typed methods ignore the style: a quoted `"8"` fills an integer and a quoted `'true'` a bool (`:1107-1197`).

| Written | Typeless read today | String field | Typed field |
|:--|:--|:--|:--|
| `1.10` | float 1.1 | `1.10` | float 1.1, not an integer or bool |
| `010` | float 10.0 (`src/parse_scalars.rs:207-215`) | `010` | integer refused, float 10.0 |
| `0x10`, `0o17`, `0b101` | integer | the text | integer |
| `.inf`, `.nan` | refused for a typeless read in a config | the text | float |
| `true`, `True`, `TRUE` and the three `false` | bool | the text | bool |
| `no`, `yes`, `on` | string | the text | bool refused |
| `~`, `null`, empty | null | refused | `None` for `Option` |
| `!!str 1` | string | the text | parsed |

The design keeps this table, except that a typeless `010` reads as integer 10, as YAML 1.2's core schema reads it.
qctl reads its ledger typeless and hakkiri reads `params` into an untagged enum, so both see the typeless column.

## 3. TOML

`DeTable` is `Map<Spanned<DeString>, Spanned<DeValue>>` ([toml 1.1.8][toml] `src/de/parser/detable.rs:10-16`), so every key and value has a span.
`DeString` holds the processed value only; the style comes from the first bytes of its span, as `toml_entries.rs` reads it today.
Keys come back sorted; the tree orders them by span.

The `toml` Deserializer forwards every typed method to `deserialize_any` (`src/de/deserializer/value.rs:247-251`).
So `name = 1` into a `String` is refused today, and the TOML format keeps that.

A datetime is `toml_datetime::Datetime`, passed to serde through a private map.
The tree holds it as a plain scalar with its source text.
A `Datetime` target is not supported.

## 4. Consumers and pinned wording

| Consumer | Uses |
|:--|:--|
| forkctl | `use_repo_limits`; writes YAML with serde-saphyr directly |
| qctl | typeless reads into `serde_json::Value`; keeps yamled for edits |
| hakkiri | `place_key`, `Tagged` (`!override`), untagged `params` |
| rgbctl | `Input::open` only |
| verctl | `frontmatter`, `problem_at` |

Text pinned in tests, which the message set rewords:

| Today | Pinned in |
|:--|:--|
| `duplicate mapping key: …, set DuplicateKeyPolicy in Options if acceptable` | ctl-core `input__problem_reports` snapshot; hakkiri `rule_parameters.rs` |
| `invalid boolean (strict mode expects true/false)` | ctl-core snapshot |
| `YAML merge keys are not allowed by configured policy` | ctl-core snapshot |
| `expected mapping start` | verctl `runners.rs`, `fragment.rs`; hakkiri `faults.tsv` |
| `` unknown field `x`, expected one of … `` | ctl-core snapshot; hakkiri `check.rs`, `package_settings.rs`, `test_command.rs`, `faults.tsv` |
| `non-finite float … rejected by the configured policy` | ctl-core `input.rs` |

## 5. Size

The budgets give `app` 526,377 bytes, `input` 1,103,677 and `input-toml` 647,502 ([Cargo.toml][cargo]).
How the 577 KB of `input` divides between serde-saphyr, yamled and garde is not measured.
Dropping yamled and instantiating serde-saphyr once for the tree should shrink `input`; the tree Deserializer adds code per target type.
The change measures every tier before and after in one box.

## 6. JSON readers, for the json change

Seven ways to read strict JSON with a span on every value and key were compared.

| Reader | Spans | Strict | Limits while reading | Cost |
|:--|:--|:--|:--|:--|
| Our own reader | yes | yes | yes | 500 to 800 lines, no dependency |
| [json_scanner 0.1.0][json-scanner] | yes | yes | the caller counts | one release, one user |
| [struson 0.8.0][struson] | inferred ends | yes, number limit off | depth only | adds `thiserror`, 15.6k lines |
| [jsonc-parser 0.34.0][jsonc] | yes | no: extra whitespace, raw control characters, empty input | depth fixed at 512 | none |
| [json-syntax 0.13.1][json-syntax] | yes | yes | none | about 22 crates |
| jiter, simd-json, sonic-rs | no node spans | yes | yes | unsafe SIMD code |
| granit-parser with refusals | yes | no: refuses U+007F to U+009F and keys over 1024 characters | yes | a second tokenizer for YAML escapes |

Our own reader is the only one that is strict, adds no dependency, and applies the limits while reading.
json_scanner's design is the model, and the JSONTestSuite cases its repository ships are the tests.

[base]: https://github.com/victor-software-house/ctl-core/tree/0d8e5a40c6ce8d7d3e1a0d6b1cfc8c836825a7e1
[cargo]: https://github.com/victor-software-house/ctl-core/blob/0d8e5a40c6ce8d7d3e1a0d6b1cfc8c836825a7e1/Cargo.toml
[fork]: https://github.com/victor-software-house/serde-saphyr/tree/v0.0.4
[granit]: https://docs.rs/crate/granit-parser/2.1.0
[json-scanner]: https://docs.rs/crate/json_scanner/0.1.0
[json-syntax]: https://docs.rs/crate/json-syntax/0.13.1
[jsonc]: https://docs.rs/crate/jsonc-parser/0.34.0
[struson]: https://docs.rs/crate/struson/0.8.0
[toml]: https://docs.rs/crate/toml/1.1.8
