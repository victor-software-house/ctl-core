# Proposal

## Motivation

ctl-core reads declared input in two formats, YAML and TOML, and the set is closed: `input::Format` is an enum, and each operation matches on it.
Parsing, placing a field, listing top-level entries and reading a tag each have a YAML half and a TOML half ([input.rs][input]).

A CLI cannot add a format, and the argument document of [CTC-043][ledger] needs JSON, which no half reads.

The maintainer chose an open set of input formats, as output formats are, with yaml, json and toml built in (operator, 2026-11-25).
Erasing serde behind a trait object was rejected.
It turns the YAML error into a string and loses its line and column (operator, 2026-11-26).

The YAML half also reads its text twice.
serde-saphyr parses it into the target type, and yamled parses it again to place each problem ([yaml_text.rs][yaml-text]).
The `input` tier is the largest one ctl-core measures, about 577 KB over `app` ([Cargo.toml][cargo]).

This change belongs to [CTC-065][ledger].
It also carries out [CTC-038][ledger], the move of declared input into a `ctl-input` leaf crate.

## Changes

1. A new leaf crate, `ctl-input`, holds declared input; ctl-core's `input` features re-export it, so `ctl_core::input` paths stay.
2. Every format reads its text into one located tree.
   Each node keeps its byte span, each scalar its value, style and tag, and each mapping its keys in order.
3. **BREAKING** `input::Format` becomes a trait.
   A format reads text into the tree under the limits in force.
   It also resolves a scalar for the type its field needs.
   `input::Formats` is the registry, with `YAML` and `TOML` built in, and a CLI adds, replaces or removes a format.
4. ctl-input owns, once, what sits above the tree.
   That is the serde Deserializer, placing a field path, listing top-level entries, garde checks, `Tagged`, and the wording of every problem.
5. The YAML format reads through vsh-serde-saphyr, whose new located reader keeps its limits, its duplicate and merge key refusals and its scalar rules.
   yamled leaves ctl-core's graph.
6. **BREAKING** Problem messages come from one message set for every format, with no option names of a parser in them.
7. The JSON format, with its own reader, is a separate change.

## Capabilities

### New Capabilities

- `input-formats`: the located tree, the format trait and its registry, and how a scalar resolves.

### Modified Capabilities

- `declared-input`: the format comes from the registry, the tree is read once, problems use one message set, and the input features turn on `ctl-input` without yamled.

## Impact

1. Code: a new `ctl-input/` crate built from `ctl-core/src/input/`; vsh-serde-saphyr gains a located reader and public scalar rules in a fork patch released first.
2. API: `input::Format`, `Format::of` and `Input::new_with_format` change; `Input`, `InputError`, `Problem`, `FieldPath`, `Limits` and `Tagged` keep their paths.
3. Consumers: rgbctl calls only `Input::open`; verctl's and hakkiri's tests pin serde-saphyr wording that changes.
4. Size: the `input` budgets move by the measured difference, in both directions.
5. Release: a `ctl-core: minor` changeset in the 0.8.0 candidates.

[cargo]: https://github.com/victor-software-house/ctl-core/blob/0d8e5a40c6ce8d7d3e1a0d6b1cfc8c836825a7e1/Cargo.toml
[input]: https://github.com/victor-software-house/ctl-core/blob/0d8e5a40c6ce8d7d3e1a0d6b1cfc8c836825a7e1/ctl-core/src/input.rs
[ledger]: ../../../tasks.yaml
[yaml-text]: https://github.com/victor-software-house/ctl-core/blob/0d8e5a40c6ce8d7d3e1a0d6b1cfc8c836825a7e1/ctl-core/src/input/yaml_text.rs
