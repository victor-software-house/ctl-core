# Design

## Context

ctl-input reads every format into one located tree ([tree.rs][tree]), and each format implements `Format` with a `read` and a `resolve` ([format.rs][format]).
YAML reads through the vsh-serde-saphyr fork and TOML through the `toml` crate ([toml_format.rs][toml]).
Above the tree, one Deserializer, placing, listing, checks and wording serve every format.
So a JSON format only has to build the tree and resolve its scalars.

[research.md](research.md) compares the readers available on 2026-11-26.

## Goals and non-goals

Goals:

1. A JSON format that reads, places, lists and checks exactly as YAML and TOML do.
2. Strict RFC 8259, with no option that loosens it.
3. No new dependency, and a measured size for the new tier.

Non-goals:

1. JSON with comments, trailing commas or other JSON5 forms.
2. JSON Lines input.
3. Reading the argument document of CTC-043, which uses this format in its own change.

## Decisions

### 1. An own reader inside ctl-input

The reader is a scanner that yields tokens with byte spans, driven by one loop that builds the tree.
It is about 450 to 650 lines without tests, by comparison with three readers of the same shape (research.md, section 6.1).

Alternatives considered:

1. serde_json, already a dependency of ctl-core's JSON output.
   Its spans come only through `raw_value` and pointer arithmetic, which scans a byte once per nesting level and decodes every key twice.
   Its deserializer is also new code in the binary: the output path links only its serializer.
2. `json_scanner` 0.1.0 gives spans, number text and a strict grammar.
   It has one release, no commits since 2025-10-20, and a closed error enum ctl-input would map by hand.
   It remains the fallback if the reader's lines are not wanted.
3. struson, json-event-parser and actson can give spans, but each needs a derivation that was not run, and struson's duplicate error has no end offset.
4. jsonc-parser accepts raw control characters and Unicode whitespace with every option off, and nojson fixes its depth limit at 128.
   Both build their whole structure before ctl-input can apply its node limit.

### 2. One loop with an explicit stack

The loop keeps a stack of open containers.
It checks `max_depth` when it opens a container and `max_nodes` when it adds a node, so a refusal comes before the next allocation.

Recursive descent lost: its depth is the call stack, so the depth check and the stack overflow guard become one concern.

### 3. The text is a `str`

`Input` holds a `String`, so the reader never sees invalid UTF-8.
A file that is not UTF-8, such as one written in UTF-16, is refused when `Input::open` reads it.

Taking bytes lost: the reader would validate UTF-8 a second time.

### 4. Strict grammar

1. Whitespace is space, tab, line feed and carriage return only.
2. A string refuses a raw control character, U+0000 to U+001F, and accepts only the escapes RFC 8259 lists.
3. A high surrogate escape must be followed by a low one; a lone or inverted surrogate is refused, because a Rust `String` cannot hold it.
4. Numbers follow the RFC grammar: no leading `+`, no leading zero, no bare `.`, and no `Infinity` or `NaN`.
5. Literals are `true`, `false` and `null` in lower case.
6. The root is any one value; an empty text, or text after the value, is refused.
7. A leading byte-order mark is skipped, as RFC 8259 section 8.1 allows.
   Its column is not counted, as for YAML and TOML.

Refusing the mark lost: editors on Windows write it, and YAML and TOML already accept it.

### 5. The tree a JSON text builds

1. An object is a mapping whose entries keep their key nodes and value nodes in written order.
2. An array is a sequence.
3. A string is a scalar with `ScalarStyle::DoubleQuoted` and `ScalarSyntax::Text`, its value unescaped.
   Each key is the same kind of scalar.
4. A number is a scalar with `ScalarStyle::Plain` and its source text as its value.
   Its syntax is `Integer` with no `.`, `e` or `E`, and `Float` otherwise.
5. `true` and `false` are `Boolean`, and `null` is the new `ScalarSyntax::Null`.
6. No node has a tag.

Every node keeps its byte span: a string's span includes its quotes.

### 6. Scalars resolve by syntax

`Json::resolve` follows TOML's rule: the scalar's syntax decides, never its text.

1. `Text` fills a string field.
2. `Boolean` fills a bool field.
3. `Integer` fills an integer field when an `i128` or a `u128` holds it, and a float field.
4. `Float` fills a float field when `f64` holds it finitely; a value that overflows to infinity is refused.
5. `Null` fills a unit or optional field, and an optional field of any type reads `None`.
6. A typeless read gives each value its natural kind; an integer no `u128` holds reads as a float when it is finite.

Quoted numbers filling number fields, as YAML allows, lost: JSON writers type their values, and TOML already sets the strict rule.

### 7. Duplicate keys

Keys compare after unescaping, so `"a"` and `"\u0061"` are the same key.
A strict reading refuses the second one with `Fault::DuplicateKey` on its span.
A listing reading keeps both entries, as YAML's does, so `top_level` and placing still work.

Last-one-wins, as serde_json does, lost: it drops a value without a word.

### 8. Limits

The format reads the shared fields `max_reader_input_bytes`, `max_depth` and `max_nodes` through the same tiers as YAML and TOML: the call, the repository's `.ctl/input.yaml`, the process, then the default.
`max_reader_input_bytes` is checked before the first token.
A refusal names the field and where to raise it, through `Fault::Limit` and `Fault::TooLarge`.
To lift a limit, a caller sets it to `usize::MAX` in `Limits`; JSON adds no limit of its own.

### 9. Wording of a syntax error

A syntax error is `Fault::Syntax` at the span of the token that broke the grammar, worded by `Messages` with no parser names:

| Case | Words |
|:--|:--|
| A value was expected | `expected a value` |
| An array is not closed | ``expected `,` or `]` `` |
| An object is not closed | ``expected `,` or `}` `` |
| A key was expected | `expected a key in double quotes` |
| A string ends early | `a string is not closed` |
| A bad escape | ``unknown escape `\q` `` |
| A raw control character | `control character U+0009 must be escaped` |
| A lone surrogate | ``escape `\ud800` is half of a surrogate pair`` |
| A bad number | ``number `01` is not valid JSON`` |
| Text after the value | `text after the value` |

### 10. An opt-in feature

ctl-input gains a `json` feature, and ctl-core an `input-json` feature that turns it on.
`Formats::builtin()` holds `yaml`, `json` and `toml` as their features allow.
`input` stays `input-yaml` with `validate`, so a CLI that reads JSON names `input-json` (operator, 2026-11-26).

Adding JSON to `input` lost: every binary with `input` would carry the reader whether it reads JSON or not.

### 11. Conformance

JSONTestSuite at commit [1ef36fa][suite] is vendored under `ctl-input/tests/fixtures/json-test-suite/` with its MIT license.
One test reads each file through `Input` and compares the outcome with a verdict table:

1. Every `y_` file reads, except the two duplicate-key files, which a strict reading refuses.
2. Every `n_` file is refused.
3. Every `i_` file has the verdict of research.md, section 6.3: numbers read, lone surrogates are refused, invalid UTF-8 and UTF-16 are refused, the byte-order mark is skipped, and 500 nested arrays read only when `max_depth` allows 500.

## Risks

1. An own reader can miss a grammar case.
   The suite covers the cases readers most often miss, and the test fails on any file without a verdict.
2. The `input-json` tier has no budget yet.
   Its first measurement sets it, and later changes move it by measured differences.

[format]: https://github.com/victor-software-house/ctl-core/blob/53dfdf02e14a128bc8b3a41501a1541c54c0b51b/ctl-input/src/format.rs
[suite]: https://github.com/nst/JSONTestSuite/tree/1ef36fa01286573e846ac449e8683f8833c5b26a
[toml]: https://github.com/victor-software-house/ctl-core/blob/53dfdf02e14a128bc8b3a41501a1541c54c0b51b/ctl-input/src/toml_format.rs
[tree]: https://github.com/victor-software-house/ctl-core/blob/53dfdf02e14a128bc8b3a41501a1541c54c0b51b/ctl-input/src/tree.rs
