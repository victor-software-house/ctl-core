# Proposal

## Motivation

ctl-input reads declared input in YAML and TOML through an open set of formats, and the maintainer chose yaml, json and toml as the built-in set (operator, 2026-11-25).
JSON is the one built-in format no reader covers yet.
The argument document of [CTC-043][ledger] reads JSON, and a CLI that keeps its config in `.json` cannot use `Input::open` today.

The located tree needs four things from a JSON reader: a byte span for every key and value, the source text of every number, a strict RFC 8259 grammar, and limits checked before the next token.
[research.md](research.md) compares serde_json and twelve crates that report positions.
serde_json gives spans only through `raw_value`, which scans each byte once per nesting level and decodes every key twice.
The one crate that gives all four, `json_scanner` 0.1.0, has a single release and no commits since 2025-10-20.
The maintainer chose an own reader after that comparison (operator, 2026-11-26).

This change belongs to [CTC-066][ledger].

## Changes

1. ctl-input gains a strict RFC 8259 reader that builds the located tree directly, with no dependency: about 450 to 650 lines, behind a `json` feature.
2. The `JSON` format reads `.json` files and resolves each scalar by its syntax, as TOML does.
   A JSON string fills only a string field, a number only a number field, and `null` only an optional or unit field.
3. The tree gains `ScalarSyntax::Null`; the enum is already `#[non_exhaustive]`.
4. A strict reading refuses a duplicate key on the repeated key, and a listing reading keeps both entries.
5. The shared limits `max_reader_input_bytes`, `max_depth` and `max_nodes` apply, checked as the reader goes.
6. ctl-core gains an `input-json` feature.
   `input` stays YAML with validation, so a CLI opts in to JSON (operator, 2026-11-26).
7. The reader is tested against JSONTestSuite, vendored under its MIT license.

## Capabilities

### New Capabilities

None.

### Modified Capabilities

- `input-formats`: JSON joins the built-in formats, with its grammar, its scalar rules, its duplicate keys and its limits.
  The capability is new in [open-input](../open-input/proposal.md), so open-input is archived first.

## Impact

1. Code: new `ctl-input/src/json_format.rs` and a `json/` module for the scanner; `ctl-core/Cargo.toml` gains `input-json`.
2. API: additive. `input::JSON`, `ScalarSyntax::Null`, and `json` in `Formats::builtin()` when the feature is on.
3. Consumers: none change; no consumer turns `input-json` on yet.
4. Size: a new `input-json` tier in `check:size`, measured at its first build; every existing tier stays unchanged with the feature off.
5. Release: a `ctl-core: patch` changeset in the 0.8.0 candidates.

[ledger]: ../../../tasks.yaml
