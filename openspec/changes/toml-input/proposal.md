# Proposal: read TOML config files through the declared-input loader

## Why

The operator decided on 2026-10-14 that TOML is better for config files and
asked for native TOML support in ctl-core (CTC-021). Today the `input`
feature reads YAML only. A CLI that wants a TOML config has to keep its own
parser and its own mapping from a parser error and a garde field path to a
line, which is the drift [CTC-008][ctc-008] removed for YAML.

A TOML text cannot be told apart from YAML by reading it: `name = "release"`
is a valid YAML plain scalar. So the format has to be chosen, not guessed.

## What Changes

1. A new `input-toml` feature reads TOML through the `toml` crate's
   span-carrying tree. `Input::load`, `parse`, and `check` keep their
   signatures and work on a TOML input.
2. `Format` names the two formats. `Input::open(path)` reads a file and
   chooses its format from the extension: `.toml`, `.yaml`, or `.yml`. Any
   other extension is refused with a problem. `Input::format(Format)` sets
   the format of text in memory.
3. `Input::new` and `Input::read` stay YAML, so the YAML consumers change
   nothing.
4. A TOML text is held to `max_reader_input_bytes`, `max_depth`, and
   `max_nodes`, decided by the same tiers as YAML.
5. The garde field path (`outputs[1].path`) is split once, for both formats.

## Capabilities

### New Capabilities

None.

### Modified Capabilities

- `declared-input`: a TOML input, format selection, and TOML limits.

## Impact

1. Additive public API: `input::Format` and `Input::{format, open}`. The
   `input` feature keeps its name and meaning (YAML). A YAML-only consumer
   compiles no TOML crate.
2. `input-toml` also compiles serde-saphyr, because `Limits` is a view of
   serde-saphyr's `Budget` and the repository limits file is YAML. See the
   design for the alternative and why it lost.
3. New dependency `toml` 1.1.8 with `toml_parser`, `toml_datetime`,
   `serde_spanned`, and `winnow`. All are MIT or Apache-2.0, so `deny.toml`
   needs no new licence.

[ctc-008]: https://github.com/victor-software-house/ctl-core/blob/4c178e8/tasks.yaml#L27
